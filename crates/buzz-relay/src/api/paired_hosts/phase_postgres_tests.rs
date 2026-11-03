use super::postgres_tests::*;
use super::*;
use nostr::{Event, Keys, Timestamp};

async fn send(
    f: &Fixture,
    from: &Keys,
    to: &Keys,
    g: &str,
    op: &str,
    r: Option<&str>,
    prev: nostr::EventId,
    expiry: u64,
) -> (Event, StatusCode) {
    let event = private_event(from, to, &format!("https://{}", f.host), g, op);
    let mut packet = wrap(&event, Some(prev), expiry);
    packet["route"]["r"] = json!(r);
    let (status, _) = f
        .call(from, &format!("/bindings/{g}/messages"), Some(&packet))
        .await;
    (event, status)
}
#[tokio::test]
#[ignore = "requires Postgres"]
async fn two_intents_fence_parent_correlation_and_resync() {
    let f = Fixture::new().await;
    let o = Keys::generate();
    let h = Keys::generate();
    f.member(&o).await;
    let c = format!("https://{}", f.host);
    let n = Timestamp::now().as_secs();
    let (offer, proof) = buzz_core::paired_host::Offer::issue(c, h.public_key(), n).unwrap();
    let g = &offer.generation;
    let (claim, _) = buzz_core::paired_host::build_claim(&o, &offer, &proof, n).unwrap();
    let verified = buzz_core::paired_host::verify_claim(
        &serde_json::to_vec(&claim).unwrap(),
        &h,
        &offer,
        None,
        &proof.digest(),
        n,
    )
    .unwrap();
    let ack = buzz_core::paired_host::build_ack(&h, &verified, n).unwrap();
    assert_eq!(
        f.call(
            &o,
            &format!("/offers/{g}"),
            Some(&wrap(&claim, None, offer.expires_at))
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        f.call(
            &h,
            &format!("/offers/{g}/ack"),
            Some(&wrap(&ack, Some(claim.id), offer.expires_at))
        )
        .await
        .0,
        StatusCode::OK
    );
    let (query, s) = send(&f, &o, &h, g, "inventory-query", None, ack.id, n + 60).await;
    assert_eq!(s, StatusCode::OK);
    let (inventory, s) = send(&f, &h, &o, g, "inventory", None, query.id, n + 60).await;
    assert_eq!(s, StatusCode::OK);
    let one = "01".repeat(32);
    let two = "02".repeat(32);
    let (i1, s) = send(&f, &o, &h, g, "create", Some(&one), inventory.id, n + 900).await;
    assert_eq!(s, StatusCode::OK);
    let (i2, s) = send(&f, &o, &h, g, "create", Some(&two), inventory.id, n + 900).await;
    assert_eq!(s, StatusCode::OK);
    assert_ne!(
        send(&f, &o, &h, g, "create", Some(&one), inventory.id, n + 900)
            .await
            .1,
        StatusCode::OK
    );
    assert_ne!(
        send(&f, &h, &o, g, "challenge", Some(&two), i1.id, n + 60)
            .await
            .1,
        StatusCode::OK
    );
    let (old, s) = send(&f, &h, &o, g, "challenge", Some(&one), i1.id, n + 60).await;
    assert_eq!(s, StatusCode::OK);
    let (ch, s) = send(&f, &h, &o, g, "challenge", Some(&one), i1.id, n + 60).await;
    assert_eq!(s, StatusCode::OK);
    assert_ne!(
        send(&f, &o, &h, g, "confirm", Some(&one), old.id, n + 60)
            .await
            .1,
        StatusCode::OK
    );
    let (co, s) = send(&f, &o, &h, g, "confirm", Some(&one), ch.id, n + 60).await;
    assert_eq!(s, StatusCode::OK);
    let (off, s) = send(&f, &h, &o, g, "key-offer", Some(&one), co.id, n + 60).await;
    assert_eq!(s, StatusCode::OK);
    let (act, s) = send(&f, &o, &h, g, "activate", Some(&one), off.id, n + 60).await;
    assert_eq!(s, StatusCode::OK);
    assert_ne!(
        send(&f, &o, &h, g, "credentials", Some(&one), act.id, n + 80000)
            .await
            .1,
        StatusCode::OK
    );
    for idx in 3..=8 {
        let r = format!("{idx:064x}");
        assert_eq!(
            send(&f, &o, &h, g, "create", Some(&r), inventory.id, n + 900)
                .await
                .1,
            StatusCode::OK
        );
    }
    assert_ne!(
        send(
            &f,
            &o,
            &h,
            g,
            "create",
            Some(&"09".repeat(32)),
            inventory.id,
            n + 900
        )
        .await
        .1,
        StatusCode::OK
    );
    let (_, s) = send(&f, &h, &o, g, "unpaired", None, ack.id, n + 600).await;
    assert_eq!(s, StatusCode::OK);
    assert_ne!(
        send(&f, &h, &o, g, "challenge", Some(&two), i2.id, n + 60)
            .await
            .1,
        StatusCode::OK
    );
    assert_ne!(
        send(&f, &h, &o, g, "committed", Some(&two), act.id, n + 80000)
            .await
            .1,
        StatusCode::OK
    );
    let (committed, s) = send(&f, &h, &o, g, "committed", Some(&one), act.id, n + 80000).await;
    assert_eq!(s, StatusCode::OK);
    let (_, s) = send(
        &f,
        &o,
        &h,
        g,
        "credentials",
        Some(&one),
        committed.id,
        n + 80000,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let (done, s) = send(&f, &h, &o, g, "result", Some(&one), committed.id, n + 80000).await;
    assert_eq!(s, StatusCode::OK);
    let (_, s) = send(&f, &h, &o, g, "result", Some(&two), i2.id, n + 80000).await;
    assert_eq!(s, StatusCode::OK);
    assert_ne!(
        send(&f, &h, &o, g, "result", Some(&one), i1.id, n + 80000)
            .await
            .1,
        StatusCode::OK
    );
    let requests = format!("/bindings/{g}/requests");
    assert_eq!(
        f.call(&o, &requests, None).await.1["requests"]
            .as_array()
            .unwrap()
            .len(),
        8
    );
    let (_, detail) = f.call(&h, &format!("{requests}/{one}"), None).await;
    assert_eq!(detail["terminal_id"], done.id.to_hex());
    assert_eq!(detail["unresolved"], false);
    assert!(detail["messages"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v["event"]["id"] == act.id.to_hex()));
    let page = f.call(&h, &format!("/bindings/{g}/messages"), None).await.1;
    assert!(!page["messages"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v["event"]["id"] == i2.id.to_hex()));
    let (query, s) = send(&f, &o, &h, g, "status-query", Some(&one), i1.id, n + 60).await;
    assert_eq!(s, StatusCode::OK);
    let (status, s) = send(&f, &h, &o, g, "status", Some(&one), query.id, n + 60).await;
    assert_eq!(s, StatusCode::OK);
    // Move only the transient status into the past. Its referenced intent,
    // terminal and enrollment evidence must remain reachable after pruning.
    sqlx::query("UPDATE paired_host_events SET received_at=now()-interval '8 days' WHERE community_id=$1 AND event_id=$2")
        .bind(f.community.as_uuid()).bind(status.id.as_bytes().as_slice()).execute(f.state.db.pool()).await.unwrap();
    let (_, s) = send(&f, &o, &h, g, "status-query", Some(&one), i1.id, n + 60).await;
    assert_eq!(s, StatusCode::OK);
    let old = page["after"].as_str().unwrap();
    let pruned = f
        .call(&h, &format!("/bindings/{g}/messages?after={old}"), None)
        .await;
    assert_eq!(
        (pruned.0, pruned.1["code"].as_str()),
        (StatusCode::GONE, Some("cursor_pruned"))
    );
    let (s, recovered) = f.call(&h, &format!("{requests}/{one}"), None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(recovered["terminal_id"], done.id.to_hex());
    let snap = f.call(&h, &format!("/bindings/{g}"), None).await.1;
    let catchup = f
        .call(
            &h,
            &format!(
                "/bindings/{g}/messages?after={}",
                snap["high_water"].as_str().unwrap()
            ),
            None,
        )
        .await;
    assert_eq!(catchup.0, StatusCode::OK);
}
