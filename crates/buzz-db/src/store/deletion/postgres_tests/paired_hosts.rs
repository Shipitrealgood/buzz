use super::*;

const HOST_ROWS: &[(&str, &str)] = &[
    (
        "paired_host_events",
        "INSERT INTO paired_host_events
         (community_id, generation, owner_key, host_key, event_id, operation, expires_at, event_json, route_json)
         VALUES ($1, $2, $2, $3, $2, 'pair-claim', 1, '{}', '{}')",
    ),
    (
        "paired_host_bindings",
        "INSERT INTO paired_host_bindings
         (community_id, generation, owner_key, host_key, claim_id, ack_id, state)
         VALUES ($1, $2, $2, $3, $2, $2, 'active')",
    ),
    (
        "paired_host_requests",
        "INSERT INTO paired_host_requests
         (community_id, generation, owner_key, host_key, request_id, intent_id, head_phase, intent_expiry)
         VALUES ($1, $2, $2, $3, $2, $2, 'create', 1)",
    ),
    (
        "paired_agent_placements",
        "INSERT INTO paired_agent_placements
         (community_id, agent_pubkey, owner_pubkey, host_pubkey, generation, revision, unit_id, state, last_operation_id)
         VALUES ($1, $2, $2, $3, $2, 1, 'unit', 'pending', 'operation')",
    ),
    (
        "paired_agent_operations",
        "INSERT INTO paired_agent_operations
         (community_id, owner_pubkey, operation_id, agent_pubkey, host_pubkey, generation,
          action, expected_revision, expires_at, content_sha256, event_json, state)
         VALUES ($1, $2, 'operation', $2, $3, $2, 'deploy', 0, 1, 'hash', '{}', 'pending')",
    ),
    (
        "paired_host_deployment_status",
        "INSERT INTO paired_host_deployment_status
         (community_id, generation, owner_pubkey, host_pubkey, expires_at, event_json)
         VALUES ($1, $2, $2, $3, 1, '{}')",
    ),
];

async fn seed_host_rows(db: &Db, community: CommunityId) -> Result<()> {
    let mut tx = crate::begin_community_write_transaction(
        &db.pool,
        community,
        crate::observability::WriterOperation::Authorization,
    )
    .await?;
    for &(_, insert) in HOST_ROWS {
        sqlx::query(insert)
            .bind(community.as_uuid())
            .bind(&[1_u8; 32][..])
            .bind(&[2_u8; 32][..])
            .execute(tx.conn())
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub(super) async fn assert_host_purge_is_scoped() {
    let (db, store) = store().await;
    let (request, inventory) = inventoried_request(&db, &store).await;
    let target = request.community_id;
    let bystander = db
        .ensure_configured_community(&format!("bystander-{}.example", Uuid::new_v4().simple()))
        .await
        .expect("create bystander community")
        .id;
    for community in [target, bystander] {
        seed_host_rows(&db, community)
            .await
            .expect("seed host rows");
    }

    store.approve(request.id, "approver", None).await.unwrap();
    let claim = store
        .claim_specific(request.id, "executor", DEFAULT_LEASE_DURATION)
        .await
        .unwrap()
        .unwrap();
    store.begin_quiescing(&claim.lease).await.unwrap();
    let err = seed_host_rows(&db, target)
        .await
        .expect_err("quiescing blocks fresh write admission");
    assert!(matches!(err, DbError::AccessDenied(ref msg) if msg.contains("write-fenced")));
    let generation = store.fence(&claim.lease).await.unwrap();
    let token = LeaseToken {
        fence_generation: Some(generation),
        ..claim.lease
    };
    store
        .freeze_destructive_storage_manifest(&token, &inventory.storage)
        .await
        .unwrap();
    store.mark_drained(&token).await.unwrap();
    store
        .mark_bindings_removed(&token, serde_json::json!({"keys": 0}))
        .await
        .unwrap();
    let deleted = store.purge_postgres(&token).await.expect("purge postgres");
    store
        .mark_cache_purged(&token, serde_json::json!({"keys": 0}))
        .await
        .unwrap();
    store
        .verify_postgres_logically_deleted(&token)
        .await
        .unwrap();

    for &(table, _) in HOST_ROWS {
        assert_eq!(deleted.get(table), Some(&1), "{table}");
        let owners: Vec<Uuid> = sqlx::query_scalar(AssertSqlSafe(format!(
            "SELECT community_id FROM {table} WHERE community_id IN ($1, $2)"
        )))
        .bind(target.as_uuid())
        .bind(bystander.as_uuid())
        .fetch_all(&db.pool)
        .await
        .unwrap();
        assert_eq!(owners, [*bystander.as_uuid()], "{table}");
    }
}
