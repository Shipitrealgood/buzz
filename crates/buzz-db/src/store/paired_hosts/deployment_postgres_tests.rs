use super::*;
use buzz_core::{
    host_deployment::{wire, Authority},
    host_registry as registry,
    paired_message::Scope,
};
use nostr::{Event, Keys};
use sha2::{Digest, Sha256};

async fn enroll(
    db: &Db,
    c: CommunityId,
    owner: &Keys,
    host: &Keys,
    origin: &str,
) -> (Scope, Event) {
    let _ = db
        .add_relay_member(
            c,
            &owner.public_key().to_hex(),
            "member",
            None,
            &Keys::generate(),
        )
        .await
        .unwrap();
    let scope = Scope {
        community: origin.into(),
        generation: uuid::Uuid::new_v4().simple().to_string().repeat(2),
        owner: owner.public_key(),
        host: host.public_key(),
    };
    let n = nostr::Timestamp::now().as_secs();
    let claim = registry::seal(
        owner,
        &scope,
        registry::Body::PairClaim {
            proof: "ab".repeat(32),
        },
        "cc".repeat(32),
        None,
        n,
        n + 500,
    )
    .unwrap();
    db.post_registry_event(
        c,
        origin,
        owner.public_key(),
        &serde_json::to_vec(&claim).unwrap(),
        false,
        &[0; 32],
    )
    .await
    .unwrap();
    let ack = registry::seal(
        host,
        &scope,
        registry::Body::PairAck {
            claim_id: claim.id.to_hex(),
        },
        "dd".repeat(32),
        Some(claim.id.to_hex()),
        n,
        n + 500,
    )
    .unwrap();
    db.post_registry_event(
        c,
        origin,
        host.public_key(),
        &serde_json::to_vec(&ack).unwrap(),
        false,
        &[0; 32],
    )
    .await
    .unwrap();
    (scope, ack)
}
fn command(
    owner: &Keys,
    scope: &Scope,
    agent: PublicKey,
    op: &str,
    rev: u64,
    act: Action,
) -> (Authority, Event) {
    let n = nostr::Timestamp::now().as_secs();
    // The DB intentionally sees only a real NIP44 ciphertext; host validates plaintext.
    let content = nostr::nips::nip44::encrypt(
        owner.secret_key(),
        &scope.host,
        b"{\"fixture\":true}",
        nostr::nips::nip44::Version::V2,
    )
    .unwrap();
    let a = Authority {
        v: 1,
        scope: scope.clone(),
        agent,
        action: act,
        operation_id: op.into(),
        expected_revision: rev,
        expires_at: n + 500,
        content_sha256: hex::encode(Sha256::digest(content.as_bytes())),
    };
    let e = wire::sign_command(owner, &a, content.as_bytes(), n).unwrap();
    (a, e)
}
#[tokio::test]
#[ignore = "requires Postgres"]
async fn deployment_atomic_replay_placement_permissions_cleanup_and_revocation() {
    let db = Db::from_pool(
        sqlx::PgPool::connect(&crate::test_support::database_url())
            .await
            .unwrap(),
    );
    let name = format!("deployment-{}.test", uuid::Uuid::new_v4());
    let c = db.ensure_configured_community(&name).await.unwrap().id;
    let origin = format!("https://{name}");
    let owner = Keys::generate();
    let host = Keys::generate();
    let (scope, ack) = enroll(&db, c, &owner, &host, &origin).await;
    let agent = Keys::generate().public_key();
    let (a, e) = command(&owner, &scope, agent, &"11".repeat(32), 0, Action::Deploy);
    let raw = serde_json::to_vec(&e).unwrap();
    db.post_host_deployment(c, &origin, owner.public_key(), &raw, false)
        .await
        .unwrap();
    db.post_host_deployment(c, &origin, owner.public_key(), &raw, false)
        .await
        .unwrap();
    let (_, conflict) = command(&owner, &scope, agent, &a.operation_id, 0, Action::Deploy);
    assert!(db
        .post_host_deployment(
            c,
            &origin,
            owner.public_key(),
            &serde_json::to_vec(&conflict).unwrap(),
            false
        )
        .await
        .is_err());
    assert!(db
        .post_host_deployment(c, "https://foreign.test", owner.public_key(), &raw, false)
        .await
        .is_err());
    let (_, stale) = command(&owner, &scope, agent, &"12".repeat(32), 0, Action::Deploy);
    assert!(db
        .post_host_deployment(
            c,
            &origin,
            owner.public_key(),
            &serde_json::to_vec(&stale).unwrap(),
            false
        )
        .await
        .is_err());
    let other_host = Keys::generate();
    let (other, _) = enroll(&db, c, &owner, &other_host, &origin).await;
    let (_, duplicate) = command(&owner, &other, agent, &"13".repeat(32), 0, Action::Deploy);
    assert!(db
        .post_host_deployment(
            c,
            &origin,
            owner.public_key(),
            &serde_json::to_vec(&duplicate).unwrap(),
            false
        )
        .await
        .is_err());
    let g = bytes(&scope.generation).unwrap();
    let mut q = wire::Query {
        v: 1,
        scope: scope.clone(),
        view: "permit".into(),
        nonce: "21".repeat(32),
        operation_id: Some(a.operation_id.clone()),
    };
    assert!(db
        .read_host_deployment(c, owner.public_key(), None, false, &q)
        .await
        .is_err());
    let admitted = db
        .read_host_deployment(c, host.public_key(), Some(&g), false, &q)
        .await
        .unwrap();
    assert_eq!(
        admitted.data["authority"],
        serde_json::to_value(&a).unwrap()
    );
    assert_eq!(admitted.data["unit_id"], a.operation_id);
    assert!(
        db.read_host_deployment(c, host.public_key(), Some(&g), false, &q)
            .await
            .is_err(),
        "challenge cannot authorize another attempt"
    );
    let (_, cleanup) = command(&owner, &scope, agent, &"14".repeat(32), 1, Action::Cleanup);
    db.post_host_deployment(
        c,
        &origin,
        owner.public_key(),
        &serde_json::to_vec(&cleanup).unwrap(),
        false,
    )
    .await
    .unwrap();
    q.nonce = "22".repeat(32);
    assert!(
        db.read_host_deployment(c, host.public_key(), Some(&g), false, &q)
            .await
            .is_err(),
        "queued deploy fenced by cleanup"
    );
    q.operation_id = Some("14".repeat(32));
    let cleanup_permit = db
        .read_host_deployment(c, host.public_key(), Some(&g), false, &q)
        .await
        .unwrap();
    assert_eq!(cleanup_permit.data["unit_id"], a.operation_id);
    let (_, resurrect) = command(&owner, &scope, agent, &"15".repeat(32), 2, Action::Deploy);
    assert!(db
        .post_host_deployment(
            c,
            &origin,
            owner.public_key(),
            &serde_json::to_vec(&resurrect).unwrap(),
            false
        )
        .await
        .is_err());
    let n = nostr::Timestamp::now().as_secs();
    let revoke = registry::seal(
        &owner,
        &scope,
        registry::Body::Unpair {
            ack_id: ack.id.to_hex(),
        },
        "31".repeat(32),
        Some(ack.id.to_hex()),
        n,
        n + 500,
    )
    .unwrap();
    db.post_registry_event(
        c,
        &origin,
        owner.public_key(),
        &serde_json::to_vec(&revoke).unwrap(),
        false,
        &[0; 32],
    )
    .await
    .unwrap();
    q.nonce = "23".repeat(32);
    assert!(
        db.read_host_deployment(c, host.public_key(), Some(&g), false, &q)
            .await
            .is_err(),
        "revocation precedes subsequent permits"
    );
    q.view = "operation".into();
    assert!(db
        .read_host_deployment(c, owner.public_key(), None, false, &q)
        .await
        .is_err());
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM paired_agent_placements WHERE community_id=$1")
            .bind(c.as_uuid())
            .fetch_one(db.pool())
            .await
            .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
#[ignore = "requires Postgres"]
async fn migration_schema_deployment_82_preserves_registration_and_matches_desired() {
    let url = crate::test_support::database_url();
    let pool = sqlx::PgPool::connect(&url).await.unwrap();
    crate::migration::run_migrations_through(&pool, 81)
        .await
        .unwrap();
    let db = Db::from_pool(pool.clone());
    let name = format!("upgrade-{}.test", uuid::Uuid::new_v4());
    let c = db.ensure_configured_community(&name).await.unwrap().id;
    let origin = format!("https://{name}");
    let owner = Keys::generate();
    let host = Keys::generate();
    let (scope, ack) = enroll(&db, c, &owner, &host, &origin).await;
    // Explicit out-of-band expansion: serving registration does not run DDL.
    crate::migration::run_migrations(&pool).await.unwrap();
    let g = bytes(&scope.generation).unwrap();
    let session = db
        .registry_session(c, host.public_key().as_bytes(), &g, false)
        .await
        .unwrap();
    assert_eq!(session.owner, Some(owner.public_key().to_bytes()));
    db.post_registry_event(
        c,
        &origin,
        host.public_key(),
        &serde_json::to_vec(&ack).unwrap(),
        false,
        &[0; 32],
    )
    .await
    .unwrap();
    let (other, _) = enroll(&db, c, &owner, &Keys::generate(), &origin).await;
    assert_ne!(other.generation, scope.generation);
    let scratch = crate::test_support::ScratchDb::create(&url, "deployment_desired").await;
    let desired = scratch.connect().await;
    for ddl in [
        include_str!("../../../../../schema/tables/public/paired_agent_operations.sql"),
        include_str!("../../../../../schema/tables/public/paired_agent_placements.sql"),
        include_str!("../../../../../schema/tables/public/paired_host_deployment_status.sql"),
    ] {
        sqlx::raw_sql(ddl).execute(&desired).await.unwrap();
    }
    for table in [
        "paired_agent_operations",
        "paired_agent_placements",
        "paired_host_deployment_status",
    ] {
        async fn shape(pool: &sqlx::PgPool, table: &str) -> Vec<String> {
            sqlx::query_scalar("SELECT column_name||':'||data_type||':'||is_nullable||':'||coalesce(column_default,'') FROM information_schema.columns WHERE table_schema='public' AND table_name=$1 UNION ALL SELECT indexdef FROM pg_indexes WHERE schemaname='public' AND tablename=$1 ORDER BY 1")
                .bind(table).fetch_all(pool).await.unwrap()
        }
        assert_eq!(
            shape(&pool, table).await,
            shape(&desired, table).await,
            "{table}"
        );
    }
}
