use super::postgres_tests::*;
use super::*;
use buzz_core::paired_host::{build_ack, build_claim, verify_claim, Offer};
use nostr::{Keys, Timestamp};
use tower::ServiceExt;

#[tokio::test]
#[ignore = "requires Postgres"]
async fn insecure_public_origin_hides_descriptor_and_route() {
    let mut f = Fixture::new().await;
    let mut state = (*f.state).clone();
    Arc::make_mut(&mut state.config).relay_url = format!("ws://{}", f.host);
    f.state = Arc::new(state);
    let req = Request::builder()
        .uri("/info")
        .header("host", &f.host)
        .body(Body::empty())
        .unwrap();
    let response = crate::router::build_router(f.state.clone())
        .oneshot(req)
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), 1 << 20).await.unwrap();
    assert_eq!(status, StatusCode::OK);
    let info: Value = serde_json::from_slice(&body).unwrap();
    assert!(info.get("paired_hosts_v1").is_none());
    assert_eq!(
        f.call(&Keys::generate(), "/hosts", None).await.0,
        StatusCode::NOT_FOUND
    );
}

// Seed a busy one-minute window just below its boundary, without a wall
// clock dependency on running 600 round trips within the same Redis TTL.
async fn near_capacity(f: &Fixture, signer: &Keys) {
    let mut conn = f.state.redis_pool.get().await.unwrap();
    let key = format!(
        "buzz:{}:paired-host-signer:normal:{}",
        f.community,
        signer.public_key().to_hex()
    );
    let _:i64=redis::cmd("EVAL").arg("for i=1,599 do redis.call('SADD',KEYS[1],i) end; return redis.call('EXPIRE',KEYS[1],60)").arg(1).arg(key).query_async(&mut *conn).await.unwrap();
}
#[tokio::test]
#[ignore = "requires Postgres"]
async fn shared_proxy_abuse_preserves_independent_principal_and_unpair_admission() {
    let f = Fixture::new().await;
    let attacker = Keys::generate();
    let owner = Keys::generate();
    let host = Keys::generate();
    f.member(&attacker).await;
    f.member(&owner).await;
    // Every request uses exactly the same socket peer. No forwarding header.
    for _ in 0..601 {
        assert_eq!(
            f.send("/hosts", None, "Nostr eA==").await.0,
            StatusCode::UNAUTHORIZED
        );
    }
    let socket_bucket = mac(&f.state, &["IP", &f.community.to_string(), "127.0.0.1"])
        .unwrap()
        .finalize()
        .into_bytes();
    let key = format!(
        "buzz:{}:paired-host-ingress:{}",
        f.community,
        hex::encode(socket_bucket)
    );
    let mut conn = f.state.redis_pool.get().await.unwrap();
    let count: Option<i64> = redis::cmd("GET")
        .arg(&key)
        .query_async(&mut *conn)
        .await
        .unwrap();
    assert!(
        count.is_none(),
        "invalid proofs spent the shared socket allowance"
    );
    let _: () = redis::cmd("SET")
        .arg(key)
        .arg(599)
        .arg("EX")
        .arg(60)
        .query_async(&mut *conn)
        .await
        .unwrap();
    drop(conn);
    near_capacity(&f, &attacker).await;
    assert_eq!(f.call(&attacker, "/hosts", None).await.0, StatusCode::OK);
    assert_eq!(
        f.call(&attacker, "/hosts", None).await.0,
        StatusCode::TOO_MANY_REQUESTS
    );
    assert_eq!(
        f.call(&Keys::generate(), "/hosts", None).await.0,
        StatusCode::TOO_MANY_REQUESTS
    );
    assert_eq!(f.call(&owner, "/hosts", None).await.0, StatusCode::OK);
    let n = Timestamp::now().as_secs();
    let c = format!("https://{}", f.host);
    let (offer, proof) = Offer::issue(c.clone(), host.public_key(), n).unwrap();
    let g = &offer.generation;
    let (claim, _) = build_claim(&owner, &offer, &proof, n).unwrap();
    let path = format!("/offers/{g}");
    assert_eq!(
        f.call(&owner, &path, Some(&wrap(&claim, None, offer.expires_at)))
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        f.call(&host, &path, None).await.1["messages"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let verified = verify_claim(
        &serde_json::to_vec(&claim).unwrap(),
        &host,
        &offer,
        None,
        &proof.digest(),
        n,
    )
    .unwrap();
    let ack = build_ack(&host, &verified, n).unwrap();
    assert_eq!(
        f.call(
            &host,
            &format!("{path}/ack"),
            Some(&wrap(&ack, Some(claim.id), offer.expires_at))
        )
        .await
        .0,
        StatusCode::OK
    );
    // Both participants have already spent ordinary slots; filling their own
    // buckets must still permit owner-first and host-final closing messages.
    near_capacity(&f, &host).await;
    near_capacity(&f, &owner).await;
    let binding = format!("/bindings/{g}");
    for k in [&host, &owner] {
        assert_eq!(
            f.call(k, &binding, None).await.0,
            StatusCode::TOO_MANY_REQUESTS
        );
    }
    let msg = format!("{binding}/messages");
    for (from, to, op) in [(&owner, &host, "unpair"), (&host, &owner, "unpaired")] {
        let packet = wrap(&private_event(from, to, &c, g, op), Some(ack.id), n + 600);
        assert_eq!(f.call(from, &msg, Some(&packet)).await.0, StatusCode::OK);
    }
}
