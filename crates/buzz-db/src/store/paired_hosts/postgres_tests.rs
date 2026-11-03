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
