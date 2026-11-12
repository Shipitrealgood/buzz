use super::*;

async fn setup() -> (Db, CommunityId) {
    let pool = sqlx::PgPool::connect(&crate::test_support::database_url())
        .await
        .unwrap();
    let db = Db::from_pool(pool);
    let t = db
        .ensure_configured_community(&format!("private-{}.test", uuid::Uuid::new_v4()))
        .await
        .unwrap()
        .id;
    (db, t)
}
async fn member(db: &Db, t: CommunityId, key: &[u8]) {
    let _ = db
        .add_relay_member(
            t,
            &hex::encode(key),
            "member",
            None,
            &nostr::Keys::generate(),
        )
        .await
        .unwrap();
}
fn packet(g: u8, o: u8, h: u8, ack: bool) -> Packet {
    let n = chrono::Utc::now().timestamp();
    let id = if ack { g + 1 } else { g };
    Packet {
        id: [id; 32],
        generation: [g; 32],
        owner: [o; 32],
        host: [h; 32],
        op: if ack { "pair-ack" } else { "pair-claim" }.into(),
        prev: ack.then_some([g; 32]),
        request: None,
        created: n,
        expiry: n + 500,
        event: serde_json::value::to_raw_value(&json!({"created_at":n})).unwrap(),
        route: json!({"expires_at":n+500}),
    }
}
#[tokio::test]
#[ignore = "requires Postgres"]
async fn pairing_killed_ack_transaction_retains_only_offer_and_retry_converges() {
    let (db, t) = setup().await;
    let claim = packet(10, 13, 14, false);
    let mut ack = packet(10, 13, 14, true);
    ack.expiry = claim.expiry;
    member(&db, t, &claim.owner).await;
    db.post_paired_packet(t, false, &claim, &[5; 32])
        .await
        .unwrap();
    let mut blocker = db.pool().begin().await.unwrap();
    sqlx::query("LOCK TABLE paired_host_bindings IN SHARE MODE")
        .execute(&mut *blocker)
        .await
        .unwrap();
    let locker: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    let writer = db.clone();
    let job = tokio::spawn(async move {
        let outcome = writer.post_paired_packet(t, false, &ack, &[5; 32]).await;
        (outcome, ack)
    });
    // SELECT succeeded and the event INSERT has executed. The exact connection
    // is now trying to insert the binding in the same uncommitted transaction.
    let pid=tokio::time::timeout(std::time::Duration::from_secs(2),async{
        loop{
            let p:Option<i32>=sqlx::query_scalar(
                r#"SELECT pid FROM pg_stat_activity WHERE datname = current_database()
                   AND $1 = ANY(pg_blocking_pids(pid)) AND query LIKE 'INSERT INTO paired_host_bindings%'
                   LIMIT 1"#,
            ).bind(locker).fetch_optional(&mut *blocker).await.unwrap();
            if let Some(pid)=p{break pid}
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            sqlx::query("SELECT pg_stat_clear_snapshot()").execute(&mut *blocker).await.unwrap();
        }
    }).await.unwrap();
    let killed: bool = sqlx::query_scalar("SELECT pg_terminate_backend($1)")
        .bind(pid)
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    assert!(killed);
    blocker.rollback().await.unwrap();
    let (outcome, ack) = job.await.unwrap();
    assert!(outcome.is_err());
    let rows: i64 =
        sqlx::query_scalar("SELECT count(*) FROM paired_host_events WHERE community_id=$1")
            .bind(t.as_uuid())
            .fetch_one(db.pool())
            .await
            .unwrap();
    assert_eq!(rows, 1);
    let rows: i64 =
        sqlx::query_scalar("SELECT count(*) FROM paired_host_bindings WHERE community_id=$1")
            .bind(t.as_uuid())
            .fetch_one(db.pool())
            .await
            .unwrap();
    assert_eq!(rows, 0);
    db.post_paired_packet(t, false, &ack, &[5; 32])
        .await
        .unwrap();
    db.post_paired_packet(t, false, &ack, &[5; 32])
        .await
        .unwrap();
    let (events, bindings): (i64, i64) = sqlx::query_as(
        r#"SELECT (SELECT count(*) FROM paired_host_events
           WHERE community_id = $1), (SELECT count(*) FROM paired_host_bindings
           WHERE community_id = $1)"#,
    )
    .bind(t.as_uuid())
    .fetch_one(db.pool())
    .await
    .unwrap();
    assert_eq!((events, bindings), (2, 1));
}
#[tokio::test]
#[ignore = "requires Postgres"]
async fn competing_bindings_and_private_tenant_reads() {
    let (db, t) = setup().await;
    let other = db
        .ensure_configured_community(&format!("other-{}.test", uuid::Uuid::new_v4()))
        .await
        .unwrap()
        .id;
    let mut a = packet(10, 13, 14, false);
    let wire = format!("{{\n    \"created_at\" : {}\n}}", a.created);
    a.event = serde_json::value::RawValue::from_string(wire.clone()).unwrap();
    let b = packet(20, 15, 14, false);
    for c in [t, other] {
        for owner in [&a.owner, &b.owner] {
            member(&db, c, owner).await;
        }
    }
    db.post_paired_packet(t, false, &a, &[0; 32]).await.unwrap();
    db.post_paired_packet(t, false, &b, &[0; 32]).await.unwrap();
    let recovered = db
        .read_paired(t, &a.host, false, Read::Offers(a.generation))
        .await
        .unwrap();
    assert!(
        recovered.data["messages"].get().contains(&wire),
        "private mailbox rewrote signed JSON"
    );
    let mut a_ack = packet(10, 13, 14, true);
    a_ack.expiry = a.expiry;
    let mut b_ack = packet(20, 15, 14, true);
    b_ack.expiry = b.expiry;
    let (first, second) = tokio::join!(
        db.post_paired_packet(t, false, &a_ack, &[0; 32]),
        db.post_paired_packet(t, false, &b_ack, &[0; 32])
    );
    assert_ne!(first.is_ok(), second.is_ok());
    let winner = if first.is_ok() { &a } else { &b };
    let loser = if first.is_ok() { &b } else { &a };
    assert!(db
        .read_paired(t, &loser.owner, false, Read::Binding(winner.generation))
        .await
        .is_err());
    assert!(db
        .read_paired(
            other,
            &winner.owner,
            false,
            Read::Binding(winner.generation)
        )
        .await
        .is_err());
    let rows: i64 =
        sqlx::query_scalar("SELECT count(*) FROM events WHERE community_id=$1 AND kind=43102")
            .bind(t.as_uuid())
            .fetch_one(db.pool())
            .await
            .unwrap();
    assert_eq!(rows, 0);
}

#[tokio::test]
#[ignore = "requires Postgres"]
async fn one_member_cannot_fill_another_owners_target_host() {
    let (db, t) = setup().await;
    let host = 75;
    let bad = 11;
    let good = 12;
    for owner in [bad, good] {
        member(&db, t, &[owner; 32]).await;
    }
    for g in 1..=16 {
        let claim = packet(g, bad, host, false);
        db.post_paired_packet(t, false, &claim, &[0; 32])
            .await
            .unwrap();
        // Exact retransmission returns its saved decision without charging.
        db.post_paired_packet(t, false, &claim, &[0; 32])
            .await
            .unwrap();
    }
    let denied = db
        .post_paired_packet(t, false, &packet(17, bad, host, false), &[0; 32])
        .await;
    assert!(matches!(denied, Err(DbError::AccessDenied(_))));
    let claim = packet(70, good, host, false);
    db.post_paired_packet(t, false, &claim, &[0; 32])
        .await
        .unwrap();
    let mut ack = packet(70, good, host, true);
    ack.expiry = claim.expiry;
    db.post_paired_packet(t, false, &ack, &[0; 32])
        .await
        .unwrap();
    let binding = db
        .read_paired(t, &[good; 32], false, Read::Binding(claim.generation))
        .await
        .unwrap();
    assert_eq!(binding.data["state"].get(), r#""active""#);
}

#[tokio::test]
#[ignore = "requires Postgres"]
async fn registry_models_admission_and_revoked_reads_have_no_agent_effects() {
    use buzz_core::{
        host_registry::{self as w, Body},
        paired_message::Scope,
    };
    use nostr::Keys;
    let (db, t) = setup().await;
    let owner = Keys::generate();
    let host = Keys::generate();
    member(&db, t, owner.public_key().as_bytes()).await;
    let scope = Scope {
        community: "https://models.test".into(),
        owner: owner.public_key(),
        host: host.public_key(),
        generation: "aa".repeat(32),
    };
    let now = nostr::Timestamp::now().as_secs();
    let claim = w::seal(
        &owner,
        &scope,
        Body::PairClaim {
            proof: "bb".repeat(32),
        },
        "01".repeat(32),
        None,
        now,
        now + 600,
    )
    .unwrap();
    let ack = w::seal(
        &host,
        &scope,
        Body::PairAck {
            claim_id: claim.id.to_hex(),
        },
        "02".repeat(32),
        Some(claim.id.to_hex()),
        now,
        now + 600,
    )
    .unwrap();
    let post = |e: nostr::Event| {
        let db = &db;
        let origin = scope.community.clone();
        async move {
            db.post_registry_event(
                t,
                &origin,
                e.pubkey,
                &serde_json::to_vec(&e).unwrap(),
                false,
                &[0; 32],
            )
            .await
        }
    };
    post(claim.clone()).await.unwrap();
    post(ack.clone()).await.unwrap();
    let ordinary_before: i64 =
        sqlx::query_scalar("SELECT count(*) FROM events WHERE community_id=$1")
            .bind(t.as_uuid())
            .fetch_one(&db.pool)
            .await
            .unwrap();
    let request = w::ModelsQuery {
        ack_id: ack.id.to_hex(),
        nonce: "cc".repeat(32),
        runtime_id: "main/claude".into(),
        catalog_revision: "dd".repeat(32),
    };
    let query = w::seal(
        &owner,
        &scope,
        Body::ModelsQuery {
            request: request.clone(),
        },
        "03".repeat(32),
        Some(ack.id.to_hex()),
        now,
        now + 60,
    )
    .unwrap();
    post(query.clone()).await.unwrap();
    post(query.clone()).await.unwrap();
    let body = Body::Models {
        result: w::HostModels {
            request,
            observed_at: now,
            models: buzz_core::host_deployment::observation::Models::Empty,
            readiness: buzz_core::harness::Readiness::LoggedIn,
            availability: buzz_core::harness::AcpAvailabilityStatus::Available,
        },
    };
    let response = w::seal(
        &host,
        &scope,
        body.clone(),
        "04".repeat(32),
        Some(query.id.to_hex()),
        now,
        now + 60,
    )
    .unwrap();
    post(response.clone()).await.unwrap();
    post(response.clone()).await.unwrap();
    for (prev, expiry) in [(ack.id.to_hex(), now + 60), (query.id.to_hex(), now + 61)] {
        let invalid = w::seal(
            &host,
            &scope,
            body.clone(),
            "05".repeat(32),
            Some(prev),
            now,
            expiry,
        )
        .unwrap();
        assert!(post(invalid).await.is_err());
    }
    for field in ["owner", "host", "generation"] {
        let mut other = scope.clone();
        match field {
            "owner" => other.owner = Keys::generate().public_key(),
            "host" => other.host = Keys::generate().public_key(),
            _ => other.generation = "ee".repeat(32),
        }
        let bad = w::seal(
            &owner,
            &other,
            Body::ModelsQuery {
                request: match &body {
                    Body::Models { result } => result.request.clone(),
                    _ => unreachable!(),
                },
            },
            "06".repeat(32),
            Some(ack.id.to_hex()),
            now,
            now + 60,
        );
        assert!(bad.is_err() || post(bad.unwrap()).await.is_err());
    }
    let read = |view: &str, event: Option<String>| {
        let db = &db;
        let generation = scope.generation.clone();
        let view = view.to_owned();
        let reader = owner.public_key().to_bytes();
        async move {
            let q = w::Query {
                v: 2,
                view,
                nonce: "ff".repeat(32),
                cursor: None,
                generation: Some(generation),
                event_id: event,
            };
            db.read_registry(
                t,
                &reader,
                false,
                RegistryRead {
                    query: &q,
                    after: None,
                    snapshot: None,
                    host_generation: None,
                },
            )
            .await
        }
    };
    assert!(read("evidence", Some(response.id.to_hex())).await.is_ok());
    assert!(read("binding", None).await.unwrap().data["latest_status_id"].is_null());
    let rows: i64 =
        sqlx::query_scalar("SELECT count(*) FROM paired_host_events WHERE community_id=$1")
            .bind(t.as_uuid())
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(rows, 4, "duplicate request/reply adds no transcript row");
    let counts:(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM paired_agent_operations WHERE community_id=$1),(SELECT count(*) FROM paired_agent_placements WHERE community_id=$1)").bind(t.as_uuid()).fetch_one(&db.pool).await.unwrap();
    assert_eq!(
        counts,
        (0, 0),
        "discovery creates no agent operation or placement"
    );
    let ordinary_after: i64 =
        sqlx::query_scalar("SELECT count(*) FROM events WHERE community_id=$1")
            .bind(t.as_uuid())
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(
        ordinary_before, ordinary_after,
        "no profile or identity event"
    );
    let unpair = w::seal(
        &owner,
        &scope,
        Body::Unpair {
            ack_id: ack.id.to_hex(),
        },
        "07".repeat(32),
        Some(ack.id.to_hex()),
        now,
        now + 60,
    )
    .unwrap();
    post(unpair).await.unwrap();
    assert!(post(query).await.is_err());
    assert!(post(response.clone()).await.is_err());
    assert!(read("evidence", Some(response.id.to_hex())).await.is_err());
    assert!(
        read("evidence", Some(ack.id.to_hex())).await.is_ok(),
        "historical enrollment recovery remains"
    );
    let messages = read("messages", None).await.unwrap();
    assert!(messages.data["events"]
        .as_array()
        .unwrap()
        .iter()
        .all(|e| !matches!(e["op"].as_str(), Some("models" | "models-query"))));
}
