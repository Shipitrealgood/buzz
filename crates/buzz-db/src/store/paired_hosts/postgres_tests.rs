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
    let read_as = |reader: nostr::PublicKey, scoped: bool, view: &str, event: Option<String>| {
        let db = &db;
        let generation = scope.generation.clone();
        let view = view.to_owned();
        let ceiling = scoped.then(|| w::bytes(&generation).unwrap());
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
                reader.as_bytes(),
                false,
                RegistryRead {
                    query: &q,
                    after: None,
                    snapshot: None,
                    host_generation: ceiling.as_ref(),
                },
            )
            .await
        }
    };
    // Advance the stored deadline of an earlier offer without sleeping for its expiry.
    let earlier = w::seal(
        &owner,
        &scope,
        Body::PairClaim {
            proof: "cc".repeat(32),
        },
        "00".repeat(32),
        None,
        now,
        now + 600,
    )
    .unwrap();
    post(earlier.clone()).await.unwrap();
    sqlx::query("UPDATE paired_host_events SET expires_at = floor(extract(epoch FROM now()))::bigint - 1 WHERE community_id=$1 AND event_id=$2")
        .bind(t.as_uuid()).bind(earlier.id.as_bytes().as_slice()).execute(&db.pool).await.unwrap();
    let session = db
        .registry_session(
            t,
            host.public_key().as_bytes(),
            &w::bytes(&scope.generation).unwrap(),
            false,
        )
        .await
        .unwrap();
    assert!(session.ack_retry_only);
    assert!(read_as(host.public_key(), true, "offers", None)
        .await
        .is_err());
    post(claim.clone()).await.unwrap();
    // Membership does not turn the host role into an owner role, even before selection.
    member(&db, t, host.public_key().as_bytes()).await;
    for (reader, scoped, count) in [
        (owner.public_key(), false, 2),
        (host.public_key(), true, 2),
        (host.public_key(), false, 0),
    ] {
        let offers = read_as(reader, scoped, "offers", None).await.unwrap();
        assert_eq!(offers.data["events"].as_array().unwrap().len(), count);
        assert_eq!(offers.high_water > 0, count > 0);
        let evidence = read_as(reader, scoped, "evidence", Some(claim.id.to_hex())).await;
        assert_eq!(evidence.is_ok(), count > 0);
    }
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
    let read = |view: &str, event| read_as(owner.public_key(), false, view, event);
    assert!(read_as(host.public_key(), true, "binding", None)
        .await
        .is_ok());
    assert!(read_as(host.public_key(), false, "binding", None)
        .await
        .is_err());
    assert!(read_as(
        host.public_key(),
        false,
        "evidence",
        Some(response.id.to_hex())
    )
    .await
    .is_err());
    assert!(read("evidence", Some(response.id.to_hex())).await.is_ok());
    assert!(read("binding", None).await.unwrap().data["latest_status_id"].is_null());
    let rows: i64 =
        sqlx::query_scalar("SELECT count(*) FROM paired_host_events WHERE community_id=$1")
            .bind(t.as_uuid())
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(rows, 5, "duplicate request/reply adds no transcript row");
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
    // A removed owner can no longer read. The scoped host can still reconcile the
    // restrictive binding state without seeing its former owner's payload history.
    sqlx::query("DELETE FROM relay_members WHERE community_id=$1 AND pubkey=$2")
        .bind(t.as_uuid())
        .bind(owner.public_key().to_hex())
        .execute(db.pool())
        .await
        .unwrap();
    assert!(read("binding", None).await.is_err());
    let recovery = read_as(host.public_key(), true, "binding", None)
        .await
        .unwrap();
    assert_eq!(
        recovery.data,
        json!({"generation":scope.generation,"state":"fenced","recovery_only":true})
    );
    assert!(
        read_as(host.public_key(), true, "evidence", Some(ack.id.to_hex()))
            .await
            .is_err()
    );
}
