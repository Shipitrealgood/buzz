use super::*;
use buzz_core::paired_host::{build_ack, build_claim, verify_claim, Offer};
use nostr::{Event, EventBuilder, Keys, Kind, Tag, Timestamp};
use sha2::Digest;
use tower::ServiceExt;

pub(super) struct Fixture {
    pub(super) state: Arc<AppState>,
    pub(super) host: String,
    pub(super) community: buzz_core::CommunityId,
}
impl Fixture {
    pub(super) async fn new() -> Self {
        let f = crate::api::bridge::postgres_tests::bridge_handler_test_state()
            .await
            .expect("disposable DB/Redis");
        let mut s = (*f).clone();
        s.nip98_replay = Arc::new(buzz_pubsub::RedisNip98ReplayGuard::new(
            s.redis_pool.clone(),
        ));
        let c = Arc::make_mut(&mut s.config);
        c.paired_hosts_enabled = true;
        c.require_relay_membership = true;
        let host = format!("pair-{}.test", uuid::Uuid::new_v4().simple());
        let community = s.db.ensure_configured_community(&host).await.unwrap().id;
        Self {
            state: Arc::new(s),
            host,
            community,
        }
    }
    pub(super) async fn member(&self, keys: &Keys) {
        let _ = self
            .state
            .db
            .add_relay_member(
                self.community,
                &keys.public_key().to_hex(),
                "member",
                None,
                &self.state.relay_keypair,
            )
            .await
            .unwrap();
    }
    fn proof(&self, k: &Keys, path: &str, body: Option<&[u8]>) -> String {
        self.sign_url(k, &format!("https://{}{PREFIX}{path}", self.host), body)
    }
    fn sign_url(&self, k: &Keys, url: &str, body: Option<&[u8]>) -> String {
        let mut tags = vec![
            Tag::parse(["u", url]).unwrap(),
            Tag::parse(["method", if body.is_some() { "POST" } else { "GET" }]).unwrap(),
            Tag::parse([
                "nonce",
                &hex::encode(Sha256::digest(uuid::Uuid::new_v4().as_bytes())),
            ])
            .unwrap(),
        ];
        if let Some(data) = body {
            tags.push(Tag::parse(["payload", &hex::encode(Sha256::digest(data))]).unwrap());
        }
        let event = EventBuilder::new(Kind::HttpAuth, "")
            .tags(tags)
            .sign_with_keys(k)
            .unwrap();
        format!(
            "Nostr {}",
            STANDARD.encode(serde_json::to_vec(&event).unwrap())
        )
    }
    pub(super) async fn send(
        &self,
        path: &str,
        body: Option<Vec<u8>>,
        proof: &str,
    ) -> (StatusCode, Value) {
        let request = Request::builder()
            .method(if body.is_some() { "POST" } else { "GET" })
            .uri(format!("{PREFIX}{path}"))
            .header("host", &self.host)
            .header("authorization", proof)
            .extension(ConnectInfo(
                "127.0.0.1:34237".parse::<SocketAddr>().unwrap(),
            ))
            .body(Body::from(body.unwrap_or_default()))
            .unwrap();
        let response = crate::router::build_router(self.state.clone())
            .oneshot(request)
            .await
            .unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), 8 << 20).await.unwrap();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }
    async fn ordinary_rights(&self, host: &Keys, claim: nostr::EventId, open: bool) {
        use futures_util::{SinkExt, StreamExt};
        use tokio_tungstenite::{
            connect_async,
            tungstenite::{client::IntoClientRequest, Message},
        };
        let mut state = (*self.state).clone();
        Arc::make_mut(&mut state.config).require_relay_membership = !open;
        let router = crate::router::build_router(Arc::new(state));
        let cases = [
            ("/query", json!([{"ids":[claim.to_hex()]}])),
            ("/query", json!([{"kinds":[43102]}])),
            ("/count", json!([{"kinds":[43102]}])),
        ];
        for (path, filter) in cases {
            let bytes = serde_json::to_vec(&filter).unwrap();
            let proof = self.sign_url(host, &format!("https://{}{path}", self.host), Some(&bytes));
            let req = Request::builder()
                .method("POST")
                .uri(path)
                .header("host", &self.host)
                .header("authorization", proof)
                .header("content-type", "application/json")
                .body(Body::from(bytes))
                .unwrap();
            let response = router.clone().oneshot(req).await.unwrap();
            if open {
                assert_eq!(response.status(), StatusCode::OK);
                let data: Value =
                    serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap())
                        .unwrap();
                if path == "/count" {
                    assert_eq!(data["count"], 0)
                } else {
                    assert_eq!(data, json!([]))
                }
            } else {
                assert_eq!(response.status(), StatusCode::FORBIDDEN);
            }
        }
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(
                listener,
                router.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
        });
        let mut req = format!("ws://{addr}/").into_client_request().unwrap();
        req.headers_mut().insert("host", self.host.parse().unwrap());
        let (mut client, _) = connect_async(req).await.unwrap();
        let Message::Text(raw) = client.next().await.unwrap().unwrap() else {
            panic!("expected challenge")
        };
        let challenge: Value = serde_json::from_str(&raw).unwrap();
        let event = EventBuilder::auth(
            challenge[1].as_str().unwrap(),
            nostr::RelayUrl::parse(&format!("wss://{}", self.host)).unwrap(),
        )
        .sign_with_keys(host)
        .unwrap();
        client
            .send(Message::Text(json!(["AUTH", event]).to_string().into()))
            .await
            .unwrap();
        let ok = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                match client.next().await.unwrap().unwrap() {
                    Message::Text(raw) => {
                        let frame: Value = serde_json::from_str(&raw).unwrap();
                        if frame[0] == "OK" {
                            break frame;
                        }
                    }
                    Message::Ping(_) => {}
                    other => panic!("expected auth decision: {other:?}"),
                }
            }
        })
        .await
        .unwrap();
        assert_eq!(ok[2], open, "unexpected general WS authorization");
        drop(client);
        server.abort();
        let _ = server.await;
    }
    pub(super) async fn call(
        &self,
        k: &Keys,
        path: &str,
        body: Option<&Value>,
    ) -> (StatusCode, Value) {
        let bytes = body.map(|b| serde_json::to_vec(b).unwrap());
        let auth = self.proof(k, path, bytes.as_deref());
        self.send(path, bytes, &auth).await
    }
}
pub(super) fn wrap(event: &Event, prev: Option<nostr::EventId>, expiry: u64) -> Value {
    json!({"event":event,"route":{"r":null,"prev":prev.map(|id|id.to_hex()),"expires_at":expiry}})
}
pub(super) fn private_event(from: &Keys, to: &Keys, c: &str, g: &str, op: &str) -> Event {
    let cipher = nostr::nips::nip44::encrypt(
        from.secret_key(),
        &to.public_key(),
        r#"{"test":"disposable"}"#,
        nostr::nips::nip44::Version::V2,
    )
    .unwrap();
    EventBuilder::new(Kind::Custom(43102), cipher)
        .tags([
            Tag::parse(["p", &to.public_key().to_hex()]).unwrap(),
            Tag::parse(["community", c]).unwrap(),
            Tag::parse(["g", g]).unwrap(),
            Tag::parse(["op", op]).unwrap(),
        ])
        .sign_with_keys(from)
        .unwrap()
}
#[tokio::test]
#[ignore = "requires Postgres"]
async fn paired_enrollment_real_router_and_revocation() {
    let f = Fixture::new().await;
    let owner = Keys::generate();
    let rival = Keys::generate();
    let host = Keys::generate();
    let c = format!("https://{}", f.host);
    let now = Timestamp::now().as_secs();
    let (offer, proof) = Offer::issue(c.clone(), host.public_key(), now).unwrap();
    let g = &offer.generation;
    let path = format!("/offers/{g}");
    let ack_path = format!("{path}/ack");
    let binding = format!("/bindings/{g}");
    let messages = format!("{binding}/messages");
    let (claim, _) = build_claim(&owner, &offer, &proof, now).unwrap();
    let claim_packet = wrap(&claim, None, offer.expires_at);
    let verified = verify_claim(
        &serde_json::to_vec(&claim).unwrap(),
        &host,
        &offer,
        None,
        &proof.digest(),
        now,
    )
    .unwrap();
    let ack = build_ack(&host, &verified, now).unwrap();
    let ack_packet = wrap(&ack, Some(claim.id), offer.expires_at);
    assert_eq!(
        f.call(&owner, &path, Some(&claim_packet)).await.0,
        StatusCode::NOT_FOUND
    );
    f.member(&owner).await;
    f.member(&rival).await;
    for open in [false, true] {
        f.ordinary_rights(&host, claim.id, open).await;
    }
    let bytes = serde_json::to_vec(&claim_packet).unwrap();
    let auth = f.proof(&owner, &path, Some(&bytes));
    let mut altered = bytes.clone();
    altered.push(b' ');
    assert_eq!(
        f.send(&path, Some(altered), &auth).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        f.send(&path, Some(bytes.clone()), &auth).await.0,
        StatusCode::OK
    );
    assert_eq!(
        f.send(&path, Some(bytes), &auth).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        f.call(&owner, &path, Some(&claim_packet)).await.0,
        StatusCode::OK
    );
    assert_eq!(
        f.call(&host, &path, None).await.1["messages"],
        json!([claim_packet])
    );
    assert_eq!(f.call(&rival, &path, None).await.1["messages"], json!([]));
    let (rival_claim, _) = build_claim(&rival, &offer, &proof, now).unwrap();
    assert_eq!(
        f.call(
            &rival,
            &path,
            Some(&wrap(&rival_claim, None, offer.expires_at))
        )
        .await
        .0,
        StatusCode::OK
    );
    let mut moved = ack_packet.clone();
    moved["route"]["prev"] = json!(rival_claim.id.to_hex());
    assert_eq!(
        f.call(&host, &ack_path, Some(&moved)).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        f.call(&host, &ack_path, Some(&ack_packet)).await.0,
        StatusCode::OK
    );
    assert_eq!(
        f.call(&host, &ack_path, Some(&ack_packet)).await.0,
        StatusCode::OK
    );
    assert_eq!(
        f.call(&host, &ack_path, Some(&moved)).await.0,
        StatusCode::NOT_FOUND
    );
    for open in [false, true] {
        f.ordinary_rights(&host, claim.id, open).await;
    }
    let list = f.call(&owner, "/hosts", None).await;
    assert_eq!(list.1["hosts"][0]["ack"], ack_packet);
    assert_eq!(f.call(&rival, "/hosts", None).await.1["hosts"], json!([]));
    assert_eq!(f.call(&host, "/hosts", None).await.0, StatusCode::NOT_FOUND);
    let fresh = f.call(&owner, &binding, None).await.1;
    let cursor = fresh["high_water"].as_str().unwrap();
    let page = format!("{messages}?after={cursor}");
    assert_eq!(f.call(&owner, &page, None).await.1["messages"], json!([]));
    assert_eq!(f.call(&host, &page, None).await.0, StatusCode::NOT_FOUND);
    let unpair = private_event(&owner, &host, &c, g, "unpair");
    let req = wrap(&unpair, Some(ack.id), now + 86400);
    assert_eq!(
        f.call(&owner, &messages, Some(&req)).await.0,
        StatusCode::OK
    );
    assert_eq!(f.call(&owner, &binding, None).await.1["state"], "fenced");
    let _ = f
        .state
        .db
        .remove_relay_member(
            f.community,
            &owner.public_key().to_hex(),
            &f.state.relay_keypair,
        )
        .await
        .unwrap();
    assert_eq!(f.call(&host, &binding, None).await.0, StatusCode::NOT_FOUND);
    let tomb = private_event(&host, &owner, &c, g, "unpaired");
    let terminal = wrap(&tomb, Some(ack.id), now + 86400);
    assert_eq!(
        f.call(&host, &messages, Some(&terminal)).await.0,
        StatusCode::OK
    );
    assert_eq!(
        f.call(&host, &messages, Some(&terminal)).await.0,
        StatusCode::OK
    );
    f.member(&owner).await;
    assert_eq!(
        f.call(&owner, &binding, None).await.1["tombstone"],
        terminal
    );
    assert_eq!(f.call(&owner, "/hosts", None).await.1["hosts"], json!([]));
    assert_eq!(
        f.call(&owner, "/hosts?include=history", None).await.1["hosts"][0]["state"],
        "dead"
    );
}

#[tokio::test]
#[ignore = "requires Postgres"]
async fn removed_owner_host_reenroll_cannot_replay_old_tombstone_into_new_grant() {
    let f = Fixture::new().await;
    let first = Keys::generate();
    let next = Keys::generate();
    let host = Keys::generate();
    let foreign = Keys::generate();
    let origin = format!("https://{}", f.host);
    let mut prior: Option<(String, Value)> = None;
    for owner in [&first, &next] {
        f.member(owner).await;
        let n = Timestamp::now().as_secs();
        let (offer, proof) = Offer::issue(origin.clone(), host.public_key(), n).unwrap();
        let g = &offer.generation;
        let (claim, _) = build_claim(owner, &offer, &proof, n).unwrap();
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
        let path = format!("/offers/{g}");
        let binding = format!("/bindings/{g}");
        let msg = format!("{binding}/messages");
        assert_eq!(
            f.call(owner, &path, Some(&wrap(&claim, None, offer.expires_at)))
                .await
                .0,
            StatusCode::OK
        );
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
        if let Some((old, tombstone)) = &prior {
            assert_ne!(f.call(&host, &msg, Some(tombstone)).await.0, StatusCode::OK);
            assert_eq!(f.call(&host, old, Some(tombstone)).await.0, StatusCode::OK);
            assert_eq!(f.call(owner, &binding, None).await.1["state"], "active");
            assert_eq!(
                f.call(&first, &binding, None).await.0,
                StatusCode::NOT_FOUND
            );
        } else {
            let _ = f
                .state
                .db
                .remove_relay_member(
                    f.community,
                    &owner.public_key().to_hex(),
                    &f.state.relay_keypair,
                )
                .await
                .unwrap();
            let tomb = wrap(
                &private_event(&host, owner, &origin, g, "unpaired"),
                Some(ack.id),
                n + 600,
            );
            let forged = wrap(
                &private_event(&foreign, owner, &origin, g, "unpaired"),
                Some(ack.id),
                n + 600,
            );
            assert_eq!(
                f.call(&foreign, &msg, Some(&forged)).await.0,
                StatusCode::NOT_FOUND
            );
            f.state
                .db
                .ban_community_member(
                    f.community,
                    host.public_key().as_bytes(),
                    next.public_key().as_bytes(),
                    None,
                    None,
                )
                .await
                .unwrap();
            assert_eq!(
                f.call(&host, &msg, Some(&tomb)).await.0,
                StatusCode::NOT_FOUND
            );
            f.state
                .db
                .unban_community_member(
                    f.community,
                    host.public_key().as_bytes(),
                    next.public_key().as_bytes(),
                )
                .await
                .unwrap();
            assert_eq!(f.call(&host, &msg, Some(&tomb)).await.0, StatusCode::OK);
            prior = Some((msg, tomb));
        }
    }
}
