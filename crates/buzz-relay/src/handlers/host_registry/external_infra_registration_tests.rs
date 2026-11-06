//! Actual root WS integration in the canonical inline external-infra namespace.
use super::*;
mod external_infra_tests {
    //! Actual root WS, host.sqlite and independent native Device journals.
    use super::*;
    use buzz_hostd::registration::{Host, RegistrationClock};
    use buzz_paired_owner::{
        registry::{self, Registry},
        Device,
    };
    use buzz_ws_client::{NostrWsConnection, RelayMessage as ClientReply};
    use nostr::{Keys, PublicKey};
    use std::{net::SocketAddr, os::unix::fs::PermissionsExt, time::Duration};
    use tokio::task::JoinHandle;

    struct Fixture {
        state: Arc<AppState>,
        host: String,
        community: buzz_core::CommunityId,
        addr: SocketAddr,
        tasks: Vec<JoinHandle<()>>,
    }
    impl Fixture {
        async fn new() -> Self {
            let url = crate::test_support::database_url();
            let parsed = url::Url::parse(&url).unwrap();
            assert_eq!(
                std::env::var("BUZZ_3720_ISOLATED_FIXTURE").as_deref(),
                Ok("1"),
                "requires network-confined task fixture"
            );
            assert_eq!(parsed.host_str(), Some("127.0.0.1"));
            assert_eq!(parsed.username(), "buzz3720");
            assert!(parsed.path().starts_with("/buzz_3720_fixture_"));
            let mut state =
                (*crate::api::bridge::postgres_tests::bridge_handler_test_state_on(&url)
                    .await
                    .expect("fresh desired-state DB and Redis"))
                .clone();
            let c = Arc::make_mut(&mut state.config);
            c.host_registry_enabled = true;
            c.require_relay_membership = true;
            let host = format!("registry-{}.test", uuid::Uuid::new_v4().simple());
            let community = state
                .db
                .ensure_configured_community(&host)
                .await
                .unwrap()
                .id;
            let state = Arc::new(state);
            let (addr, server) = serve(state.clone()).await;
            let pubsub = state.pubsub.clone();
            let subscriber = tokio::spawn(async move { pubsub.run_subscriber().await });
            Self {
                state,
                host,
                community,
                addr,
                tasks: vec![server, subscriber],
            }
        }
        fn origin(&self) -> String {
            format!("https://{}", self.host)
        }
        async fn socket(&self) -> NostrWsConnection {
            NostrWsConnection::connect_to_addr(
                &format!("wss://{}", self.host),
                &self.addr.to_string(),
            )
            .await
            .unwrap()
        }
        async fn owner(&self, k: &Keys) -> Registry {
            let mut ws = self.socket().await;
            ws.authenticate(k, None).await.unwrap();
            Registry::from_authenticated(
                &self.origin(),
                k.public_key(),
                self.state.relay_keypair.public_key(),
                ws,
            )
            .unwrap()
        }
        async fn member(&self, k: &Keys) {
            let _ = self
                .state
                .db
                .add_relay_member(
                    self.community,
                    &k.public_key().to_hex(),
                    "member",
                    None,
                    &self.state.relay_keypair,
                )
                .await
                .unwrap();
        }
        async fn host_socket(&self, h: &Host, g: &str) -> NostrWsConnection {
            let mut ws = self.socket().await;
            h.authenticate(&mut ws, g).await.unwrap();
            ws
        }
        async fn stop(self) {
            for t in &self.tasks {
                t.abort();
            }
            for t in self.tasks {
                let _ = t.await;
            }
            self.state.db.pool().close().await;
        }
        async fn binding_count(&self, h: PublicKey) -> i64 {
            sqlx::query_scalar("SELECT count(*) FROM paired_host_bindings WHERE community_id=$1 AND host_key=$2 AND state<>'dead'").bind(self.community.as_uuid()).bind(h.as_bytes().as_slice()).fetch_one(self.state.db.pool()).await.unwrap()
        }
    }
    async fn serve(state: Arc<AppState>) -> (SocketAddr, JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let router = crate::router::build_router(state);
        let task = tokio::spawn(async move {
            axum::serve(
                listener,
                router.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .unwrap()
        });
        (addr, task)
    }
    fn host() -> (tempfile::TempDir, Host) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let h = Host::at(dir.path(), true).unwrap();
        (dir, h)
    }
    fn record(event: &Event) {
        println!("PROOF_FRAME {}", json!(["EVENT", event]));
    }
    fn clock() -> RegistrationClock {
        RegistrationClock::read().unwrap()
    }
    async fn expect_denied(ws: &mut NostrWsConnection, frame: serde_json::Value) {
        ws.send_raw(&frame).await.unwrap();
        match ws.next_event(Duration::from_secs(5)).await.unwrap() {
            ClientReply::Ok(ok) => assert!(!ok.accepted),
            ClientReply::Closed { .. } | ClientReply::Notice { .. } => {}
            r => panic!("unexpected admission response: {r:?}"),
        }
    }
    async fn pair(
        f: &Fixture,
        owner: &Keys,
        d: &mut Device,
        h: &mut Host,
        net: &mut Registry,
    ) -> (
        buzz_core::paired_message::Scope,
        NostrWsConnection,
        Event,
        Event,
    ) {
        let invitation = h.offer(f.origin(), &clock()).unwrap();
        let (scope, claim) = d.pair_registry(&f.origin(), owner, &invitation).unwrap();
        record(&claim);
        net.publish(owner, claim.clone()).await.unwrap();
        let mut ws = f.host_socket(h, &scope.generation).await;
        h.claim(&scope.generation, &claim, &clock()).unwrap();
        let ack = h.pending().unwrap().remove(0);
        record(&ack);
        assert!(ws.send_event(ack.clone()).await.unwrap().accepted);
        h.delivered(&ack.id.to_hex()).unwrap();
        let b = net
            .page(
                owner,
                registry::query("binding", Some(&scope.generation), None).unwrap(),
            )
            .await
            .unwrap();
        d.restore_registry(
            &f.origin(),
            owner,
            &serde_json::from_value(b.data).unwrap(),
            &claim,
            &ack,
            None,
        )
        .unwrap();
        (scope, ws, claim, ack)
    }
    #[tokio::test]
    #[ignore = "requires Postgres, Redis and isolated Docker network"]
    async fn registration_journey_two_journals_and_revocation() {
        let f = Fixture::new().await;
        let owner = Keys::generate();
        f.member(&owner).await;
        let journals = tempfile::tempdir().unwrap();
        let mut a = Device::at(&journals.path().join("a.sqlite")).unwrap();
        let mut b = Device::at(&journals.path().join("b.sqlite")).unwrap();
        let mut net = f.owner(&owner).await;
        net.subscribe(&owner).await.unwrap();
        let (_d1, mut h1) = host();
        let (_d2, mut h2) = host();
        let (s1, mut w1, c1, a1) = pair(&f, &owner, &mut a, &mut h1, &mut net).await;
        let (s2, mut w2, _, _) = pair(&f, &owner, &mut a, &mut h2, &mut net).await;
        assert_ne!(s1.host, s2.host);
        assert_ne!(s1.generation, s2.generation);
        assert_eq!(f.binding_count(s1.host).await, 1);
        for (s, h, w) in [(&s1, &mut h1, &mut w1), (&s2, &mut h2, &mut w2)] {
            let q = a.registry_status_query(&f.origin(), &owner, s).unwrap();
            record(&q);
            net.publish(&owner, q.clone()).await.unwrap();
            h.receive(&s.generation, &q, "same label", &clock())
                .unwrap();
            let status = h.pending().unwrap().remove(0);
            record(&status);
            assert!(w.send_event(status.clone()).await.unwrap().accepted);
            h.delivered(&status.id.to_hex()).unwrap();
            a.accept_registry_status(&f.origin(), &owner, s, &status)
                .unwrap();
        }
        // Independent empty journal recovers from relay evidence, no copied proof/cache.
        let mut recovered = f.owner(&owner).await;
        let scopes = recovered.recover(&mut b, &owner).await.unwrap();
        assert_eq!(scopes.len(), 2);
        let views = b.hosts(&f.origin(), &owner).unwrap();
        assert!(views
            .iter()
            .all(|v| v.label.as_deref() == Some("same label") && !v.fresh));
        assert!(views.iter().any(|v| v.scope == s1) && views.iter().any(|v| v.scope == s2));
        let q = b.registry_status_query(&f.origin(), &owner, &s1).unwrap();
        record(&q);
        recovered.publish(&owner, q.clone()).await.unwrap();
        h1.receive(&s1.generation, &q, "same label", &clock())
            .unwrap();
        let status = h1.pending().unwrap().remove(0);
        record(&status);
        assert!(w1.send_event(status.clone()).await.unwrap().accepted);
        b.accept_registry_status(&f.origin(), &owner, &s1, &status)
            .unwrap();
        assert!(b.view(&f.origin(), &owner, &s1).unwrap().fresh);
        assert!(!b.view(&f.origin(), &owner, &s2).unwrap().fresh);
        // Exact retry survives reopening both journals, yielding no second binding.
        assert!(w1.send_event(a1.clone()).await.unwrap().accepted);
        let unpair = b.unpair_registry(&f.origin(), &owner, &s1).unwrap();
        record(&unpair);
        recovered.publish(&owner, unpair.clone()).await.unwrap();
        assert!(w1.send_event(status.clone()).await.unwrap().accepted); // historical exact replay is evidence only
        let delayed = wire::seal(
            &owner,
            &s1,
            wire::Body::StatusQuery {
                ack_id: a1.id.to_hex(),
                nonce: "ea".repeat(32),
            },
            "eb".repeat(32),
            Some(a1.id.to_hex()),
            now(),
            now() + 60,
        )
        .unwrap();
        assert!(recovered.publish(&owner, delayed).await.is_err());

        let state: String = sqlx::query_scalar(
            "SELECT state FROM paired_host_bindings WHERE community_id=$1 AND generation=$2",
        )
        .bind(f.community.as_uuid())
        .bind(wire::bytes(&s1.generation).unwrap().as_slice())
        .fetch_one(f.state.db.pool())
        .await
        .unwrap();
        assert_eq!(state, "fenced");
        h1.receive(&s1.generation, &unpair, "same label", &clock())
            .unwrap();
        assert_eq!(h1.state(&s1.generation).unwrap(), "dead");
        let tomb = h1
            .pending()
            .unwrap()
            .into_iter()
            .find(|e| e.tags.iter().any(|t| t.as_slice() == ["op", "unpaired"]))
            .unwrap();
        record(&tomb);
        assert!(w1.send_event(tomb).await.unwrap().accepted);
        assert_eq!(f.binding_count(s1.host).await, 0);
        // Generic tables contain neither peer evidence nor relay-generated pages.
        let public: i64 =
            sqlx::query_scalar("SELECT count(*) FROM events WHERE community_id=$1 AND kind=43103")
                .bind(f.community.as_uuid())
                .fetch_one(f.state.db.pool())
                .await
                .unwrap();
        assert_eq!(public, 0);
        assert!(wire::verify(
            &serde_json::to_vec(&c1).unwrap(),
            &f.origin(),
            owner.public_key()
        )
        .is_ok());
        recovered.disconnect().await.unwrap();
        net.disconnect().await.unwrap();
        w1.disconnect().await.unwrap();
        w2.disconnect().await.unwrap();
        f.stop().await;
    }
    #[tokio::test]
    #[ignore = "requires Postgres, Redis and isolated Docker network"]
    async fn host_scope_denies_every_other_message_and_bootstrap() {
        let f = Fixture::new().await;
        let owner = Keys::generate();
        f.member(&owner).await;
        let (_dir, mut h) = host();
        let invitation = h.offer(f.origin(), &clock()).unwrap();
        let inv: serde_json::Value = serde_json::from_str(&invitation).unwrap();
        let g = inv["generation"].as_str().unwrap();
        let mut early = f.socket().await;
        assert!(h.authenticate(&mut early, g).await.is_err());
        early.disconnect().await.unwrap();
        let journal = tempfile::tempdir().unwrap();
        let mut device = Device::at(&journal.path().join("owner.sqlite")).unwrap();
        let (_, claim) = device
            .pair_registry(&f.origin(), &owner, &invitation)
            .unwrap();
        let mut owner_ws = f.owner(&owner).await;
        owner_ws.publish(&owner, claim).await.unwrap();
        let mut ws = f.host_socket(&h, g).await;
        for frame in [
            json!(["REQ", "normal", {}]),
            json!(["REQ","search",{"search":"host"}]),
            json!(["COUNT", "count", {}]),
            json!(["NEG-OPEN", "n", {}, ""]),
            json!(["UNKNOWN"]),
            json!(["REQ","sync",{"artifact":"current","kinds":[30818]}]),
        ] {
            expect_denied(&mut ws, frame).await;
        }
        // A valid host-authored normal event must still be refused (not bad signature).
        let profile = EventBuilder::new(Kind::Metadata, "{}")
            .sign_with_keys(&Keys::generate())
            .unwrap();
        expect_denied(&mut ws, json!(["EVENT", profile])).await;
        for table in ["relay_members", "pubkey_allowlist"] {
            let sql = if table == "relay_members" {
                "SELECT count(*) FROM relay_members WHERE community_id=$1 AND pubkey=$2"
            } else {
                "SELECT count(*) FROM pubkey_allowlist WHERE community_id=$1 AND pubkey=decode($2,'hex')"
            };
            let n: i64 = sqlx::query_scalar(sql)
                .bind(f.community.as_uuid())
                .bind(h.public_key().unwrap().to_hex())
                .fetch_one(f.state.db.pool())
                .await
                .unwrap();
            assert_eq!(n, 0);
        }
        let users: i64 =
            sqlx::query_scalar("SELECT count(*) FROM users WHERE community_id=$1 AND pubkey=$2")
                .bind(f.community.as_uuid())
                .bind(h.public_key().unwrap().as_bytes().as_slice())
                .fetch_one(f.state.db.pool())
                .await
                .unwrap();
        assert_eq!(users, 0);
        ws.disconnect().await.unwrap();
        owner_ws.disconnect().await.unwrap();
        f.stop().await;
    }

    #[tokio::test]
    #[ignore = "requires Postgres, Redis and isolated Docker network"]
    async fn owner_membership_wrong_parties_and_tenants() {
        let mut f = Fixture::new().await;
        let a = Keys::generate();
        let b = Keys::generate();
        let outsider = Keys::generate();
        f.member(&a).await;
        f.member(&b).await;
        for role in ["owner", "admin"] {
            let key = Keys::generate();
            let _ = f
                .state
                .db
                .add_relay_member(
                    f.community,
                    &key.public_key().to_hex(),
                    role,
                    None,
                    &f.state.relay_keypair,
                )
                .await
                .unwrap();
            let mut role_net = f.owner(&key).await;
            let target = Keys::generate();
            let scope = buzz_core::paired_message::Scope {
                community: f.origin(),
                generation: hex::encode(Sha256::digest(uuid::Uuid::new_v4().as_bytes())),
                owner: key.public_key(),
                host: target.public_key(),
            };
            let claim = wire::seal(
                &key,
                &scope,
                wire::Body::PairClaim {
                    proof: "ab".repeat(32),
                },
                "cd".repeat(32),
                None,
                now(),
                now() + 600,
            )
            .unwrap();
            role_net.publish(&key, claim).await.unwrap();
            role_net.disconnect().await.unwrap();
        }
        let temp = tempfile::tempdir().unwrap();
        let mut device = Device::at(&temp.path().join("owner.sqlite")).unwrap();
        let (_dir, mut h) = host();
        let mut net = f.owner(&a).await;
        let (scope, ws, claim, _) = pair(&f, &a, &mut device, &mut h, &mut net).await;
        let mut other = f.owner(&b).await;
        assert!(other
            .page(
                &b,
                registry::query("binding", Some(&scope.generation), None).unwrap()
            )
            .await
            .is_err());
        assert!(other
            .evidence(&b, &scope.generation, &claim.id.to_hex())
            .await
            .is_err());
        let stolen = buzz_core::paired_message::Scope {
            owner: b.public_key(),
            ..scope.clone()
        };
        let fake = wire::seal(
            &b,
            &stolen,
            wire::Body::Unpair {
                ack_id: "ab".repeat(32),
            },
            "cd".repeat(32),
            Some("ab".repeat(32)),
            now(),
            now() + 60,
        )
        .unwrap();
        assert!(other.publish(&b, fake).await.is_err());
        // Same key, separate bound tenant: no cross-tenant read or mutation.
        let second = f
            .state
            .db
            .ensure_configured_community("registry-second.test")
            .await
            .unwrap()
            .id;
        let _ = f
            .state
            .db
            .add_relay_member(
                second,
                &a.public_key().to_hex(),
                "member",
                None,
                &f.state.relay_keypair,
            )
            .await
            .unwrap();
        let mut second_ws =
            NostrWsConnection::connect_to_addr("wss://registry-second.test", &f.addr.to_string())
                .await
                .unwrap();
        second_ws.authenticate(&a, None).await.unwrap();
        let mut second_net = Registry::from_authenticated(
            "https://registry-second.test",
            a.public_key(),
            f.state.relay_keypair.public_key(),
            second_ws,
        )
        .unwrap();
        assert!(second_net
            .page(
                &a,
                registry::query("binding", Some(&scope.generation), None).unwrap()
            )
            .await
            .is_err());
        // Open relay auth does not create direct owner eligibility.
        let mut open = (*f.state).clone();
        Arc::make_mut(&mut open.config).require_relay_membership = false;
        let (addr, server) = serve(Arc::new(open)).await;
        f.tasks.push(server);
        let mut guest =
            NostrWsConnection::connect_to_addr(&format!("wss://{}", f.host), &addr.to_string())
                .await
                .unwrap();
        guest.authenticate(&outsider, None).await.unwrap();
        let (_dir2, mut h2) = host();
        let invitation = h2.offer(f.origin(), &clock()).unwrap();
        let (_, claim) = device
            .pair_registry(&f.origin(), &outsider, &invitation)
            .unwrap();
        assert!(!guest.send_event(claim).await.unwrap().accepted);
        guest.disconnect().await.unwrap();
        second_net.disconnect().await.unwrap();
        other.disconnect().await.unwrap();
        net.disconnect().await.unwrap();
        ws.disconnect().await.unwrap();
        f.stop().await;
    }
    #[tokio::test]
    #[ignore = "requires Postgres, Redis and isolated Docker network"]
    async fn parent_generation_and_operation_identity_are_enforced() {
        let f = Fixture::new().await;
        let o = Keys::generate();
        f.member(&o).await;
        let temp = tempfile::tempdir().unwrap();
        let mut d = Device::at(&temp.path().join("a.sqlite")).unwrap();
        let (_dir, mut h) = host();
        let mut net = f.owner(&o).await;
        let (s, ws, claim, ack) = pair(&f, &o, &mut d, &mut h, &mut net).await;
        let bogus = "11".repeat(32);
        let wrong = wire::seal(
            &o,
            &s,
            wire::Body::StatusQuery {
                ack_id: bogus.clone(),
                nonce: "22".repeat(32),
            },
            "33".repeat(32),
            Some(bogus),
            now(),
            now() + 60,
        )
        .unwrap();
        assert!(net.publish(&o, wrong).await.is_err());
        let mut foreign = s.clone();
        foreign.generation = "44".repeat(32);
        let wrong = wire::seal(
            &o,
            &foreign,
            wire::Body::StatusQuery {
                ack_id: ack.id.to_hex(),
                nonce: "55".repeat(32),
            },
            "66".repeat(32),
            Some(ack.id.to_hex()),
            now(),
            now() + 60,
        )
        .unwrap();
        assert!(net.publish(&o, wrong).await.is_err());
        let q = d.registry_status_query(&f.origin(), &o, &s).unwrap();
        net.publish(&o, q.clone()).await.unwrap();
        let (_, header) = wire::verify(
            &serde_json::to_vec(&q).unwrap(),
            &f.origin(),
            o.public_key(),
        )
        .unwrap();
        let conflict = wire::seal(
            &o,
            &s,
            wire::Body::StatusQuery {
                ack_id: ack.id.to_hex(),
                nonce: "77".repeat(32),
            },
            header.operation_id,
            Some(ack.id.to_hex()),
            q.created_at.as_secs(),
            header.expires_at,
        )
        .unwrap();
        assert!(net.publish(&o, conflict).await.is_err());
        net.publish(&o, q).await.unwrap();
        // Duplicate route tags signed by the correct owner remain invalid.
        let mut tags = claim.tags.clone().to_vec();
        tags.push(Tag::parse(["g", &s.generation]).unwrap());
        let duplicate = EventBuilder::new(claim.kind, claim.content.clone())
            .tags(tags)
            .custom_created_at(claim.created_at)
            .sign_with_keys(&o)
            .unwrap();
        let mut raw = f.socket().await;
        raw.authenticate(&o, None).await.unwrap();
        assert!(!raw.send_event(duplicate).await.unwrap().accepted);
        raw.disconnect().await.unwrap();
        net.disconnect().await.unwrap();
        ws.disconnect().await.unwrap();
        f.stop().await;
    }
    #[tokio::test]
    #[ignore = "requires Postgres, Redis and isolated Docker network"]
    async fn unique_live_binding_survives_competing_signed_host_acks() {
        let f = Fixture::new().await;
        let a = Keys::generate();
        let b = Keys::generate();
        let h = Keys::generate();
        f.member(&a).await;
        f.member(&b).await;
        let mut an = f.owner(&a).await;
        let mut bn = f.owner(&b).await;
        let mut host_sockets = Vec::new();
        let mut acks = Vec::new();
        for (owner, net, g) in [
            (&a, &mut an, "ab".repeat(32)),
            (&b, &mut bn, "cd".repeat(32)),
        ] {
            let s = buzz_core::paired_message::Scope {
                community: f.origin(),
                generation: g.clone(),
                owner: owner.public_key(),
                host: h.public_key(),
            };
            let claim = wire::seal(
                owner,
                &s,
                wire::Body::PairClaim {
                    proof: "ef".repeat(32),
                },
                hex::encode(Sha256::digest(g.as_bytes())),
                None,
                now(),
                now() + 600,
            )
            .unwrap();
            net.publish(owner, claim.clone()).await.unwrap();
            let mut ws = f.socket().await;
            ws.authenticate(&h, Some(&Tag::parse(["host-registry", "2", &g]).unwrap()))
                .await
                .unwrap();
            let ack = wire::seal(
                &h,
                &s,
                wire::Body::PairAck {
                    claim_id: claim.id.to_hex(),
                },
                hex::encode(Sha256::digest(claim.id.as_bytes())),
                Some(claim.id.to_hex()),
                now(),
                now() + 600,
            )
            .unwrap();
            host_sockets.push(ws);
            acks.push(ack);
        }
        let mut second = host_sockets.pop().unwrap();
        let mut first = host_sockets.pop().unwrap();
        let ack2 = acks.pop().unwrap();
        let ack1 = acks.pop().unwrap();
        let (a, b) = tokio::join!(first.send_event(ack1), second.send_event(ack2));
        assert_eq!(
            usize::from(a.unwrap().accepted) + usize::from(b.unwrap().accepted),
            1
        );
        assert_eq!(f.binding_count(h.public_key()).await, 1);
        first.disconnect().await.unwrap();
        second.disconnect().await.unwrap();
        an.disconnect().await.unwrap();
        bn.disconnect().await.unwrap();
        f.stop().await;
    }
    #[tokio::test]
    #[ignore = "requires Postgres, Redis and isolated Docker network"]
    async fn live_delivery_reauthorizes_on_second_relay_node() {
        let mut f = Fixture::new().await;
        let owner = Keys::generate();
        f.member(&owner).await;
        // A genuinely separate AppState/connection manager/pubsub subscriber uses
        // the same fresh DB/Redis and relay identity.
        let url = crate::test_support::database_url();
        let mut second = (*crate::api::bridge::postgres_tests::bridge_handler_test_state_on(&url)
            .await
            .unwrap())
        .clone();
        second.config = f.state.config.clone();
        second.relay_keypair = f.state.relay_keypair.clone();
        let second = Arc::new(second);
        let (addr, server) = serve(second.clone()).await;
        let pubsub = second.pubsub.clone();
        f.tasks.push(server);
        f.tasks
            .push(tokio::spawn(async move { pubsub.run_subscriber().await }));
        let mut ws =
            NostrWsConnection::connect_to_addr(&format!("wss://{}", f.host), &addr.to_string())
                .await
                .unwrap();
        ws.authenticate(&owner, None).await.unwrap();
        let live = registry::query("live", None, None).unwrap();
        ws.send_raw(&json!(["REQ","live",{"kinds":[43103],"#p":[owner.public_key().to_hex()],"host_registry":live}])).await.unwrap();
        loop {
            if matches!(
                ws.next_event(Duration::from_secs(5)).await.unwrap(),
                ClientReply::Eose { .. }
            ) {
                break;
            }
        }
        let mut local = f.socket().await;
        local.authenticate(&owner, None).await.unwrap();
        local.send_raw(&json!(["REQ","live",{"kinds":[43103],"#p":[owner.public_key().to_hex()],"host_registry":live}])).await.unwrap();
        loop {
            if matches!(
                local.next_event(Duration::from_secs(5)).await.unwrap(),
                ClientReply::Eose { .. }
            ) {
                break;
            }
        }
        let temp = tempfile::tempdir().unwrap();
        let mut d = Device::at(&temp.path().join("a.sqlite")).unwrap();
        let (_dir, mut h) = host();
        let mut net = f.owner(&owner).await;
        let (s, hw, _, _) = pair(&f, &owner, &mut d, &mut h, &mut net).await;
        let expected_high: i64 = sqlx::query_scalar(
            "SELECT max(seq) FROM paired_host_events WHERE community_id=$1 AND generation=$2",
        )
        .bind(f.community.as_uuid())
        .bind(wire::bytes(&s.generation).unwrap().as_slice())
        .fetch_one(f.state.db.pool())
        .await
        .unwrap();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            if let ClientReply::Event { event, .. } = ws
                .next_event(deadline.saturating_duration_since(tokio::time::Instant::now()))
                .await
                .unwrap()
            {
                let reply = wire::open_reply(
                    &owner,
                    f.state.relay_keypair.public_key(),
                    &f.origin(),
                    &live,
                    &event,
                )
                .unwrap();
                if reply.op == "registry-change"
                    && reply.high_water
                        == high_token(
                            &f.state,
                            f.community,
                            owner.public_key(),
                            &live,
                            expected_high,
                        )
                        .unwrap()
                {
                    break;
                }
            }
        }
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            if let ClientReply::Event { event, .. } = local
                .next_event(deadline.saturating_duration_since(tokio::time::Instant::now()))
                .await
                .unwrap()
            {
                let reply = wire::open_reply(
                    &owner,
                    f.state.relay_keypair.public_key(),
                    &f.origin(),
                    &live,
                    &event,
                )
                .unwrap();
                if reply.op == "registry-change"
                    && reply.high_water
                        == high_token(
                            &f.state,
                            f.community,
                            owner.public_key(),
                            &live,
                            expected_high,
                        )
                        .unwrap()
                {
                    break;
                }
            }
        }
        println!("LIVE_BARRIER both_nodes_high_water={expected_high}");
        sqlx::query("DELETE FROM relay_members WHERE community_id=$1 AND pubkey=$2")
            .bind(f.community.as_uuid())
            .bind(owner.public_key().to_hex())
            .execute(f.state.db.pool())
            .await
            .unwrap();
        // A direct DB removal deliberately leaves the stale second-node socket open.
        // Host restrictive completion publishes an invalidation after the removal.
        h.revoke(&s.generation, &clock()).unwrap();
        let tomb = h
            .pending()
            .unwrap()
            .into_iter()
            .find(|e| e.tags.iter().any(|t| t.as_slice() == ["op", "unpaired"]))
            .unwrap();
        record(&tomb);
        println!("LIVE_POST_REMOVAL new_event_id={}", tomb.id);
        let mut hw = hw;
        assert!(hw.send_event(tomb).await.unwrap().accepted);
        match ws.next_event(Duration::from_secs(5)).await.unwrap() {
            ClientReply::Closed { .. } => {}
            r => panic!("stale subscription leaked after removal: {r:?}"),
        }
        match local.next_event(Duration::from_secs(5)).await.unwrap() {
            ClientReply::Closed { .. } => {}
            r => panic!("same-node stale subscription leaked after removal: {r:?}"),
        }
        local.disconnect().await.unwrap();
        ws.disconnect().await.unwrap();
        hw.disconnect().await.unwrap();
        net.disconnect().await.unwrap();
        f.stop().await;
    }

    async fn signed_pair(
        f: &Fixture,
        o: &Keys,
        h: &Keys,
        net: &mut Registry,
        g: String,
    ) -> (buzz_core::paired_message::Scope, NostrWsConnection, Event) {
        let scope = buzz_core::paired_message::Scope {
            community: f.origin(),
            generation: g,
            owner: o.public_key(),
            host: h.public_key(),
        };
        let expiry = now() + 600;
        let claim = wire::seal(
            o,
            &scope,
            wire::Body::PairClaim {
                proof: "ab".repeat(32),
            },
            hex::encode(Sha256::digest(uuid::Uuid::new_v4().as_bytes())),
            None,
            now(),
            expiry,
        )
        .unwrap();
        net.publish(o, claim.clone()).await.unwrap();
        let mut ws = f.socket().await;
        ws.authenticate(
            h,
            Some(&Tag::parse(["host-registry", "2", &scope.generation]).unwrap()),
        )
        .await
        .unwrap();
        let ack = wire::seal(
            h,
            &scope,
            wire::Body::PairAck {
                claim_id: claim.id.to_hex(),
            },
            hex::encode(Sha256::digest(claim.id.as_bytes())),
            Some(claim.id.to_hex()),
            now(),
            expiry,
        )
        .unwrap();
        assert!(ws.send_event(ack.clone()).await.unwrap().accepted);
        (scope, ws, ack)
    }
    #[tokio::test]
    #[ignore = "requires Postgres, Redis and isolated Docker network"]
    async fn pagination_cursor_races_and_saturation_are_bounded() {
        let f = Fixture::new().await;
        let owner = Keys::generate();
        f.member(&owner).await;
        let mut net = f.owner(&owner).await;
        let mut sockets = Vec::new();
        let mut target = None;
        for n in 1..=17 {
            let h = Keys::generate();
            let (scope, ws, ack) = signed_pair(&f, &owner, &h, &mut net, format!("{n:064x}")).await;
            sockets.push(ws);
            if n == 1 {
                target = Some((scope, ack));
            }
        }
        let mut q = registry::query("hosts", None, None).unwrap();
        let page = net.page(&owner, q.clone()).await.unwrap();
        assert!(!page.complete);
        assert_eq!(page.data["hosts"].as_array().unwrap().len(), 16);
        assert!(page.next_cursor.as_ref().unwrap().len() <= 512);
        q.cursor = page.next_cursor.clone();
        let last = net.page(&owner, q.clone()).await.unwrap();
        assert!(last.complete);
        assert_eq!(last.data["hosts"].as_array().unwrap().len(), 1);
        let mut borrowed = q.clone();
        borrowed.nonce = "ef".repeat(32);
        assert!(net.page(&owner, borrowed).await.is_err());
        let (scope, ack) = target.unwrap();
        let e = wire::seal(
            &owner,
            &scope,
            wire::Body::StatusQuery {
                ack_id: ack.id.to_hex(),
                nonce: "cd".repeat(32),
            },
            "ee".repeat(32),
            Some(ack.id.to_hex()),
            now(),
            now() + 60,
        )
        .unwrap();
        net.publish(&owner, e).await.unwrap();
        assert!(net.page(&owner, q).await.is_err());
        // Saturate ordinary principal and shared-source budgets. A restrictive
        // unpair still has its separate reservation and authenticated allowance.
        let mut redis = f.state.redis_pool.get().await.unwrap();
        let _: () = redis::cmd("SET")
            .arg(format!(
                "buzz:{}:host-registry:normal:{}",
                f.community,
                owner.public_key()
            ))
            .arg(600)
            .query_async(&mut *redis)
            .await
            .unwrap();
        let bucket = Sha256::digest(b"127.0.0.1");
        let _: () = redis::cmd("SET")
            .arg(format!(
                "buzz:{}:paired-host-ingress:{}",
                f.community,
                hex::encode(bucket)
            ))
            .arg(600)
            .query_async(&mut *redis)
            .await
            .unwrap();
        assert!(net
            .page(&owner, registry::query("hosts", None, None).unwrap())
            .await
            .is_err());
        let unpair = wire::seal(
            &owner,
            &scope,
            wire::Body::Unpair {
                ack_id: ack.id.to_hex(),
            },
            "ff".repeat(32),
            Some(ack.id.to_hex()),
            now(),
            now() + 86400,
        )
        .unwrap();
        net.publish(&owner, unpair).await.unwrap();
        // An ordinary TCP proxy carries the same root endpoint; no custom edge module.
        let proxy =
            crate::test_support::PausingProxy::spawn("127.0.0.1".into(), f.addr.port()).await;
        let mut proxied = NostrWsConnection::connect_to_addr(
            &format!("wss://{}", f.host),
            &format!("127.0.0.1:{}", proxy.port),
        )
        .await
        .unwrap();
        proxied.authenticate(&owner, None).await.unwrap();
        let mut through_proxy = Registry::from_authenticated(
            &f.origin(),
            owner.public_key(),
            f.state.relay_keypair.public_key(),
            proxied,
        )
        .unwrap();
        let restrictive = wire::seal(
            &owner,
            &scope,
            wire::Body::Unpair {
                ack_id: ack.id.to_hex(),
            },
            "dd".repeat(32),
            Some(ack.id.to_hex()),
            now(),
            now() + 86400,
        )
        .unwrap();
        through_proxy.publish(&owner, restrictive).await.unwrap();
        through_proxy.disconnect().await.unwrap();
        net.disconnect().await.unwrap();
        for ws in sockets {
            ws.disconnect().await.unwrap();
        }
        f.stop().await;
    }
    #[tokio::test]
    #[ignore = "requires Postgres, Redis and isolated Docker network"]
    async fn disabled_capability_and_private_generic_reads_fail_closed() {
        use axum::{
            body::{to_bytes, Body},
            http::{Request, StatusCode},
        };
        use tower::ServiceExt;
        let mut f = Fixture::new().await;
        let owner = Keys::generate();
        let outsider = Keys::generate();
        f.member(&owner).await;
        f.member(&outsider).await;
        let mut net = f.owner(&owner).await;
        let h = Keys::generate();
        let (scope, hw, ack) = signed_pair(&f, &owner, &h, &mut net, "ab".repeat(32)).await;
        let mut plain = f.socket().await;
        plain.authenticate(&outsider, None).await.unwrap();
        for filter in [
            json!({}),
            json!({"ids":[ack.id.to_hex()]}),
            json!({"kinds":[1,43103]}),
            json!({"search":"host"}),
        ] {
            plain
                .send_raw(&json!(["REQ", "generic", filter]))
                .await
                .unwrap();
            loop {
                match plain.next_event(Duration::from_secs(5)).await.unwrap() {
                    ClientReply::Event { event, .. } => {
                        assert_ne!(u32::from(event.kind.as_u16()), 43103)
                    }
                    ClientReply::Eose { .. } | ClientReply::Closed { .. } => break,
                    _ => {}
                }
            }
        }
        for filter in [json!({}), json!({"ids":[ack.id.to_hex()]})] {
            plain
                .send_raw(&json!(["COUNT", "count", filter]))
                .await
                .unwrap();
            match plain.next_event(Duration::from_secs(5)).await.unwrap() {
                ClientReply::Count { count, .. } => assert_eq!(count, 0),
                ClientReply::Closed { .. } => {}
                r => panic!("unexpected count: {r:?}"),
            }
        }
        let router = crate::router::build_router(f.state.clone());
        use base64::Engine as _;
        for (path, filter, unsupported) in [
            ("/query", json!([{"ids":[ack.id.to_hex()]}]), false),
            ("/count", json!([{"ids":[ack.id.to_hex()]}]), false),
            ("/query", json!([{"kinds":[43103]}]), true),
            ("/count", json!([{"host_registry":{"v":2}}]), true),
        ] {
            let body = serde_json::to_vec(&filter).unwrap();
            let auth = EventBuilder::new(Kind::HttpAuth, "")
                .tags([
                    Tag::parse(["u", &format!("{}{path}", f.origin())]).unwrap(),
                    Tag::parse(["method", "POST"]).unwrap(),
                    Tag::parse(["payload", &hex::encode(Sha256::digest(&body))]).unwrap(),
                    Tag::parse(["nonce", &uuid::Uuid::new_v4().to_string()]).unwrap(),
                ])
                .sign_with_keys(&outsider)
                .unwrap();
            let proof = format!(
                "Nostr {}",
                base64::engine::general_purpose::STANDARD
                    .encode(serde_json::to_vec(&auth).unwrap())
            );
            let response = router
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(path)
                        .header("host", &f.host)
                        .header("authorization", proof)
                        .header("content-type", "application/json")
                        .body(Body::from(body))
                        .unwrap(),
                )
                .await
                .unwrap();
            if unsupported {
                assert_eq!(response.status(), StatusCode::BAD_REQUEST);
            } else {
                assert_eq!(response.status(), StatusCode::OK);
                let value: serde_json::Value =
                    serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap())
                        .unwrap();
                if path == "/query" {
                    assert_eq!(value, json!([]));
                } else {
                    assert_eq!(value["count"], 0);
                }
            }
        }
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header("host", &f.host)
                    .header("accept", "application/nostr+json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let info: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap();
        assert_eq!(
            Registry::capability(&info).unwrap(),
            f.state.relay_keypair.public_key()
        );
        for m in [
            json!({}),
            json!({"paired_hosts_v1":true}),
            json!({"host_registry":{"versions":[3],"kind":43103,"admission":"host-scoped","reads":"req-v2"}}),
        ] {
            assert!(Registry::capability(&m).is_err());
        }
        let mut disabled = (*f.state).clone();
        Arc::make_mut(&mut disabled.config).host_registry_enabled = false;
        assert!(!disabled.config.serves_host_registry());
        let (addr, server) = serve(Arc::new(disabled)).await;
        f.tasks.push(server);
        let mut old =
            NostrWsConnection::connect_to_addr(&format!("wss://{}", f.host), &addr.to_string())
                .await
                .unwrap();
        old.authenticate(&owner, None).await.unwrap();
        let q = wire::seal(
            &owner,
            &scope,
            wire::Body::StatusQuery {
                ack_id: ack.id.to_hex(),
                nonce: "11".repeat(32),
            },
            "22".repeat(32),
            Some(ack.id.to_hex()),
            now(),
            now() + 60,
        )
        .unwrap();
        assert!(!old.send_event(q).await.unwrap().accepted);
        old.disconnect().await.unwrap();
        plain.disconnect().await.unwrap();
        hw.disconnect().await.unwrap();
        net.disconnect().await.unwrap();
        f.stop().await;
    }

    mod external_infra_process_tests {
        use super::*;
        // Internal child of process_crash_before_and_after_commit_retries_exact_ack.
        // This helper is excluded from PostgreSQL automatic test discovery.
        #[tokio::test]
        #[ignore = "internal isolated proof subprocess"]
        async fn child_root_relay() {
            let Ok(host) = std::env::var("BUZZ_3720_CHILD_HOST") else {
                return;
            };
            assert_eq!(
                std::env::var("BUZZ_3720_ISOLATED_FIXTURE").as_deref(),
                Ok("1")
            );
            let url = crate::test_support::database_url();
            let mut state =
                (*crate::api::bridge::postgres_tests::bridge_handler_test_state_on(&url)
                    .await
                    .unwrap())
                .clone();
            let config = Arc::make_mut(&mut state.config);
            config.host_registry_enabled = true;
            config.require_relay_membership = true;
            state.relay_keypair =
                Keys::parse(&std::env::var("BUZZ_3720_CHILD_RELAY_KEY").unwrap()).unwrap();
            state.db.ensure_configured_community(&host).await.unwrap();
            let addr = std::env::var("BUZZ_3720_CHILD_ADDR").unwrap();
            let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
            std::fs::write(std::env::var("BUZZ_3720_CHILD_READY").unwrap(), "ready").unwrap();
            axum::serve(
                listener,
                crate::router::build_router(Arc::new(state))
                    .into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .unwrap();
        }
        struct Child(std::process::Child);
        impl Drop for Child {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
        async fn marker(path: &std::path::Path) {
            let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
            while !path.exists() {
                assert!(
                    tokio::time::Instant::now() < deadline,
                    "child process marker unavailable"
                );
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }
        #[tokio::test]
        #[ignore = "requires Postgres, Redis and isolated Docker network"]
        async fn process_crash_before_and_after_commit_retries_exact_ack() {
            use std::process::{Command, Stdio};
            for phase in ["before", "after"] {
                let f = Fixture::new().await;
                let owner = Keys::generate();
                f.member(&owner).await;
                let tmp = tempfile::tempdir().unwrap();
                let mut d = Device::at(&tmp.path().join("owner.sqlite")).unwrap();
                let (hd, mut h) = host();
                let invitation = h.offer(f.origin(), &clock()).unwrap();
                let (scope, claim) = d.pair_registry(&f.origin(), &owner, &invitation).unwrap();
                let mut net = f.owner(&owner).await;
                net.publish(&owner, claim.clone()).await.unwrap();
                h.claim(&scope.generation, &claim, &clock()).unwrap();
                let ack = h.pending().unwrap().remove(0);
                let reserve = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
                let addr = reserve.local_addr().unwrap();
                drop(reserve);
                let ready = tmp.path().join("child.ready");
                let window = tmp.path().join("commit.window");
                let child=Command::new(std::env::current_exe().unwrap()).args(["handlers::host_registry::external_infra_registration_tests::external_infra_tests::external_infra_process_tests::child_root_relay","--exact","--ignored","--nocapture"])
                .env("BUZZ_3720_CHILD_HOST",&f.host).env("BUZZ_3720_CHILD_ADDR",addr.to_string()).env("BUZZ_3720_CHILD_RELAY_KEY",f.state.relay_keypair.secret_key().to_secret_hex()).env("BUZZ_3720_CHILD_READY",&ready).env("BUZZ_3720_CRASH_MARKER",&window).env("BUZZ_3720_CRASH_PHASE",phase).stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap();
                let mut child = Child(child);
                marker(&ready).await;
                let mut ws = NostrWsConnection::connect_to_addr(
                    &format!("wss://{}", f.host),
                    &addr.to_string(),
                )
                .await
                .unwrap();
                h.authenticate(&mut ws, &scope.generation).await.unwrap();
                ws.send_raw(&json!(["EVENT", ack])).await.unwrap();
                marker(&window).await;
                assert_eq!(
                    f.binding_count(scope.host).await,
                    i64::from(phase == "after")
                );
                child.0.kill().unwrap();
                child.0.wait().unwrap();
                drop(ws);
                drop(h);
                let h = Host::at(hd.path(), false).unwrap();
                assert_eq!(h.pending().unwrap()[0].id, ack.id);
                let mut recovered = f.host_socket(&h, &scope.generation).await;
                assert!(recovered.send_event(ack.clone()).await.unwrap().accepted);
                assert_eq!(f.binding_count(scope.host).await, 1);
                h.delivered(&ack.id.to_hex()).unwrap();
                assert!(h.pending().unwrap().is_empty());
                println!(
                "PROCESS_CRASH phase={phase} committed_before_kill={} exact_ack={} live_bindings=1",
                phase == "after",
                ack.id
            );
                recovered.disconnect().await.unwrap();
                net.disconnect().await.unwrap();
                f.stop().await;
            }
        }
    }

    #[tokio::test]
    #[ignore = "requires Postgres, Redis and isolated Docker network"]
    async fn removed_owner_gets_minimal_recovery_and_banned_host_gets_none() {
        let f = Fixture::new().await;
        let owner = Keys::generate();
        f.member(&owner).await;
        let temp = tempfile::tempdir().unwrap();
        let mut d = Device::at(&temp.path().join("owner.sqlite")).unwrap();
        let (_dir, mut h) = host();
        let mut net = f.owner(&owner).await;
        let (scope, hw, claim, _) = pair(&f, &owner, &mut d, &mut h, &mut net).await;
        hw.disconnect().await.unwrap();
        sqlx::query("DELETE FROM relay_members WHERE community_id=$1 AND pubkey=$2")
            .bind(f.community.as_uuid())
            .bind(owner.public_key().to_hex())
            .execute(f.state.db.pool())
            .await
            .unwrap();
        let mut recovery = f.host_socket(&h, &scope.generation).await;
        let mut redis = f.state.redis_pool.get().await.unwrap();
        let _: () = redis::cmd("SET")
            .arg(format!(
                "buzz:{}:host-registry:normal:{}",
                f.community, scope.host
            ))
            .arg(600)
            .query_async(&mut *redis)
            .await
            .unwrap();

        let binding = h
            .page(
                &mut recovery,
                f.state.relay_keypair.public_key(),
                registry::query("binding", Some(&scope.generation), None).unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            binding.data,
            json!({"generation":scope.generation,"state":"active","recovery_only":true})
        );
        assert!(h
            .page(
                &mut recovery,
                f.state.relay_keypair.public_key(),
                registry::query(
                    "evidence",
                    Some(&scope.generation),
                    Some(&claim.id.to_hex())
                )
                .unwrap()
            )
            .await
            .is_err());
        h.revoke(&scope.generation, &clock()).unwrap();
        let tomb = h.pending().unwrap().remove(0);
        assert!(recovery.send_event(tomb).await.unwrap().accepted);
        f.state
            .db
            .ban_community_member(
                f.community,
                scope.host.as_bytes(),
                owner.public_key().as_bytes(),
                Some("synthetic test ban"),
                None,
            )
            .await
            .unwrap();
        let mut banned = f.socket().await;
        assert!(h
            .authenticate(&mut banned, &scope.generation)
            .await
            .is_err());
        banned.disconnect().await.unwrap();
        recovery.disconnect().await.unwrap();
        net.disconnect().await.unwrap();
        f.stop().await;
    }

    #[tokio::test]
    #[ignore = "requires Postgres, Redis and isolated Docker network"]
    async fn scoped_auth_rejects_wrong_scope_challenge_relay_and_delegation() {
        let f = Fixture::new().await;
        let owner = Keys::generate();
        let h = Keys::generate();
        f.member(&owner).await;
        let mut net = f.owner(&owner).await;
        let generation = "ab".repeat(32);
        let scope = buzz_core::paired_message::Scope {
            community: f.origin(),
            generation: generation.clone(),
            owner: owner.public_key(),
            host: h.public_key(),
        };
        let claim = wire::seal(
            &owner,
            &scope,
            wire::Body::PairClaim {
                proof: "cd".repeat(32),
            },
            "ef".repeat(32),
            None,
            now(),
            now() + 600,
        )
        .unwrap();
        net.publish(&owner, claim).await.unwrap();
        for case in [
            "wrong-generation",
            "wrong-challenge",
            "wrong-relay",
            "duplicate",
            "oa",
            "version",
        ] {
            let mut ws = f.socket().await;
            let ClientReply::Auth { challenge } =
                ws.next_event(Duration::from_secs(5)).await.unwrap()
            else {
                panic!("missing challenge")
            };
            let g = if case == "wrong-generation" {
                "12".repeat(32)
            } else {
                generation.clone()
            };
            let v = if case == "version" { "3" } else { "2" };
            let mut tags = vec![Tag::parse(["host-registry", v, &g]).unwrap()];
            if case == "duplicate" {
                tags.push(Tag::parse(["host-registry", "2", &g]).unwrap());
            }
            if case == "oa" {
                tags.push(
                    Tag::parse([
                        "auth",
                        &owner.public_key().to_hex(),
                        "",
                        "synthetic-invalid-delegation",
                    ])
                    .unwrap(),
                );
            }
            let relay = if case == "wrong-relay" {
                "wss://other.test".to_owned()
            } else {
                format!("wss://{}", f.host)
            };
            let challenge = if case == "wrong-challenge" {
                "old-replayed-challenge"
            } else {
                &challenge
            };
            let e = EventBuilder::auth(challenge, nostr::RelayUrl::parse(&relay).unwrap())
                .tags(tags)
                .sign_with_keys(&h)
                .unwrap();
            ws.send_raw(&json!(["AUTH", e])).await.unwrap();
            match ws.next_event(Duration::from_secs(5)).await.unwrap() {
                ClientReply::Ok(ok) => assert!(!ok.accepted, "case={case}"),
                r => panic!("unexpected scope auth: {r:?}"),
            };
            ws.disconnect().await.unwrap();
        }
        let mut scoped = f.socket().await;
        scoped
            .authenticate(
                &h,
                Some(&Tag::parse(["host-registry", "2", &generation]).unwrap()),
            )
            .await
            .unwrap();
        let profile = EventBuilder::new(Kind::Metadata, "{}")
            .sign_with_keys(&h)
            .unwrap();
        assert!(!scoped.send_event(profile.clone()).await.unwrap().accepted);
        f.member(&h).await;
        assert!(!scoped.send_event(profile).await.unwrap().accepted);
        expect_denied(&mut scoped, json!(["REQ", "still-scoped", {}])).await;
        let mut ordinary = f.socket().await;
        ordinary.authenticate(&h, None).await.unwrap();
        ordinary
            .send_raw(&json!(["REQ","independent",{"ids":["00".repeat(32)]}]))
            .await
            .unwrap();
        loop {
            match ordinary.next_event(Duration::from_secs(5)).await.unwrap() {
                ClientReply::Eose { .. } => break,
                ClientReply::Closed { message, .. } => {
                    panic!("independent member rights denied: {message}")
                }
                _ => {}
            }
        }
        ordinary.disconnect().await.unwrap();
        scoped.disconnect().await.unwrap();
        net.disconnect().await.unwrap();
        f.stop().await;
    }
    #[tokio::test]
    #[ignore = "requires Postgres, Redis and isolated Docker network"]
    async fn timely_ack_retries_after_expiry_with_no_candidate_read_rights() {
        let f = Fixture::new().await;
        let owner = Keys::generate();
        let h = Keys::generate();
        f.member(&owner).await;
        let mut net = f.owner(&owner).await;
        let generation = "ab".repeat(32);
        let scope = buzz_core::paired_message::Scope {
            community: f.origin(),
            generation: generation.clone(),
            owner: owner.public_key(),
            host: h.public_key(),
        };
        let issued = now();
        let expiry = issued + 2;
        let claim = wire::seal(
            &owner,
            &scope,
            wire::Body::PairClaim {
                proof: "cd".repeat(32),
            },
            "ef".repeat(32),
            None,
            issued,
            expiry,
        )
        .unwrap();
        net.publish(&owner, claim.clone()).await.unwrap();
        let ack = wire::seal(
            &h,
            &scope,
            wire::Body::PairAck {
                claim_id: claim.id.to_hex(),
            },
            "12".repeat(32),
            Some(claim.id.to_hex()),
            issued,
            expiry,
        )
        .unwrap();
        tokio::time::sleep(Duration::from_secs(3)).await;
        let mut ws = f.socket().await;
        ws.authenticate(
            &h,
            Some(&Tag::parse(["host-registry", "2", &generation]).unwrap()),
        )
        .await
        .unwrap();
        expect_denied(&mut ws,json!(["REQ","expired",{"kinds":[43103],"#p":[h.public_key().to_hex()],"host_registry":registry::query("offers",Some(&generation),None).unwrap()}])).await;
        assert!(ws.send_event(ack.clone()).await.unwrap().accepted);
        assert!(ws.send_event(ack).await.unwrap().accepted);
        assert_eq!(f.binding_count(h.public_key()).await, 1);
        ws.disconnect().await.unwrap();
        net.disconnect().await.unwrap();
        f.stop().await;
    }

    #[tokio::test]
    #[ignore = "requires Postgres, Redis and isolated Docker network"]
    async fn tls_proxy_two_dns_tenants_preserve_origin_and_private_registration() {
        use rustls::pki_types::pem::PemObject;
        let _ = rustls::crypto::ring::default_provider().install_default();
        let mut f = Fixture::new().await;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:32000")
            .await
            .unwrap();
        let router = crate::router::build_router(f.state.clone());
        f.tasks.push(tokio::spawn(async move {
            axum::serve(
                listener,
                router.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .unwrap()
        }));
        let owner = Keys::generate();
        let mut communities = Vec::new();
        for host in ["registry-tls-a.test:34443", "registry-tls-b.test:34443"] {
            let c = f
                .state
                .db
                .ensure_configured_community(host)
                .await
                .unwrap()
                .id;
            let _ = f
                .state
                .db
                .add_relay_member(
                    c,
                    &owner.public_key().to_hex(),
                    "member",
                    None,
                    &f.state.relay_keypair,
                )
                .await
                .unwrap();
            communities.push(c);
        }
        let pem = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../target/3720-tls/ca.pem"
        ))
        .expect("task-only synthetic CA; run proof script tls");
        let client = reqwest::Client::builder()
            .https_only(true)
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .add_root_certificate(reqwest::Certificate::from_pem(&pem).unwrap())
            .build()
            .unwrap();
        let mut roots = rustls::RootCertStore::empty();
        for cert in rustls::pki_types::CertificateDer::pem_slice_iter(&pem) {
            roots.add(cert.unwrap()).unwrap();
        }
        let config = Arc::new(
            rustls::ClientConfig::builder()
                .with_root_certificates(roots)
                .with_no_client_auth(),
        );
        let mut nets = Vec::new();
        for hostname in ["registry-tls-a.test:34443", "registry-tls-b.test:34443"] {
            let origin = format!("https://{hostname}");
            let metadata: serde_json::Value = client
                .get(&origin)
                .header("Accept", "application/nostr+json")
                .send()
                .await
                .unwrap()
                .error_for_status()
                .unwrap()
                .json()
                .await
                .unwrap();
            let relay = Registry::capability(&metadata).unwrap();
            let mut ws = NostrWsConnection::connect_with_tls_connector(
                &format!("wss://{hostname}"),
                buzz_ws_client::Connector::Rustls(config.clone()),
            )
            .await
            .unwrap();
            ws.authenticate(&owner, None).await.unwrap();
            nets.push(
                Registry::from_authenticated(&origin, owner.public_key(), relay, ws).unwrap(),
            );
        }
        let mut a = nets.remove(0);
        let mut b = nets.remove(0);
        let origin = "https://registry-tls-a.test:34443";
        let temp = tempfile::tempdir().unwrap();
        let mut device = Device::at(&temp.path().join("a.sqlite")).unwrap();
        let (_hd, mut h) = host();
        let invitation = h.offer(origin.into(), &clock()).unwrap();
        let (scope, claim) = device.pair_registry(origin, &owner, &invitation).unwrap();
        a.publish(&owner, claim.clone()).await.unwrap();
        let mut hw = NostrWsConnection::connect_with_tls_connector(
            "wss://registry-tls-a.test:34443",
            buzz_ws_client::Connector::Rustls(config),
        )
        .await
        .unwrap();
        h.authenticate(&mut hw, &scope.generation).await.unwrap();
        h.reconcile(
            &mut hw,
            f.state.relay_keypair.public_key(),
            &scope.generation,
            "tls host",
            &clock(),
        )
        .await
        .unwrap();
        let hosts = a
            .page(&owner, registry::query("hosts", None, None).unwrap())
            .await
            .unwrap();
        assert_eq!(hosts.data["hosts"].as_array().unwrap().len(), 1);
        let hosts = b
            .page(&owner, registry::query("hosts", None, None).unwrap())
            .await
            .unwrap();
        assert_eq!(hosts.data["hosts"], json!([]));
        assert!(b
            .page(
                &owner,
                registry::query("binding", Some(&scope.generation), None).unwrap()
            )
            .await
            .is_err());
        let unpair = wire::seal(
            &owner,
            &scope,
            wire::Body::Unpair {
                ack_id: a
                    .page(
                        &owner,
                        registry::query("binding", Some(&scope.generation), None).unwrap(),
                    )
                    .await
                    .unwrap()
                    .data["ack_id"]
                    .as_str()
                    .unwrap()
                    .into(),
            },
            "12".repeat(32),
            Some(
                a.page(
                    &owner,
                    registry::query("binding", Some(&scope.generation), None).unwrap(),
                )
                .await
                .unwrap()
                .data["ack_id"]
                    .as_str()
                    .unwrap()
                    .into(),
            ),
            now(),
            now() + 86400,
        )
        .unwrap();
        a.publish(&owner, unpair).await.unwrap();
        h.reconcile(
            &mut hw,
            f.state.relay_keypair.public_key(),
            &scope.generation,
            "tls host",
            &clock(),
        )
        .await
        .unwrap();
        assert_eq!(h.state(&scope.generation).unwrap(), "dead");
        println!("TLS_DNS two_tenants=true host_registration_and_reconcile=true ports=32000,34443");
        a.disconnect().await.unwrap();
        b.disconnect().await.unwrap();
        hw.disconnect().await.unwrap();
        f.stop().await;
    }

    #[tokio::test]
    #[ignore = "requires Postgres, Redis and isolated Docker network"]
    async fn retained_timely_ack_has_exact_twenty_four_hour_recovery_boundary() {
        let f = Fixture::new().await;
        let owner = Keys::generate();
        f.member(&owner).await;
        let mut net = f.owner(&owner).await;
        for age in [86390_u64, 86410] {
            let h = Keys::generate();
            let g = format!("{age:064x}");
            let expiry = now() - age;
            let scope = buzz_core::paired_message::Scope {
                community: f.origin(),
                generation: g.clone(),
                owner: owner.public_key(),
                host: h.public_key(),
            };
            let claim = wire::seal(
                &owner,
                &scope,
                wire::Body::PairClaim {
                    proof: "ab".repeat(32),
                },
                "cd".repeat(32),
                None,
                expiry - 600,
                expiry,
            )
            .unwrap();
            // A new first claim after expiry is rejected at the actual root entry.
            assert!(net.publish(&owner, claim.clone()).await.is_err());
            // Seed only the legitimate retained claim that a prior relay accepted
            // while timely. This advances historical DB time without a24h sleep;
            // subsequent admission, ack commit and replay use real root WS code.
            let (_, header) = wire::verify(
                &serde_json::to_vec(&claim).unwrap(),
                &f.origin(),
                owner.public_key(),
            )
            .unwrap();
            sqlx::query("INSERT INTO paired_host_events(community_id,event_id,generation,owner_key,host_key,operation,request_id,prev_id,expires_at,event_json,route_json) VALUES($1,$2,$3,$4,$5,'pair-claim',$6,NULL,$7,$8,$9)")
            .bind(f.community.as_uuid()).bind(claim.id.as_bytes().as_slice()).bind(wire::bytes(&g).unwrap().as_slice()).bind(owner.public_key().as_bytes().as_slice()).bind(h.public_key().as_bytes().as_slice()).bind(wire::bytes(&header.operation_id).unwrap().as_slice()).bind(expiry as i64).bind(serde_json::to_string(&claim).unwrap()).bind(serde_json::to_value(header).unwrap()).execute(f.state.db.pool()).await.unwrap();
            let ack = wire::seal(
                &h,
                &scope,
                wire::Body::PairAck {
                    claim_id: claim.id.to_hex(),
                },
                "ef".repeat(32),
                Some(claim.id.to_hex()),
                expiry - 1,
                expiry,
            )
            .unwrap();
            let tag = Tag::parse(["host-registry", "2", &g]).unwrap();
            let mut ws = f.socket().await;
            if age < 86400 {
                ws.authenticate(&h, Some(&tag)).await.unwrap();
                assert!(ws.send_event(ack.clone()).await.unwrap().accepted);
                assert!(ws.send_event(ack).await.unwrap().accepted);
                assert_eq!(f.binding_count(h.public_key()).await, 1);
            } else {
                assert!(ws.authenticate(&h, Some(&tag)).await.is_err());
                assert_eq!(f.binding_count(h.public_key()).await, 0);
            }
            ws.disconnect().await.unwrap();
        }
        net.disconnect().await.unwrap();
        f.stop().await;
    }
}
