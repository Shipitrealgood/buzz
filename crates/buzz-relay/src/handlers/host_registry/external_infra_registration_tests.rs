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

    include!("registration_executable_tests.rs");

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
        let current = recovered.recover(&mut b, &owner).await.unwrap();
        assert_eq!(current, vec![s2.clone()]);
        assert_eq!(b.view(&f.origin(), &owner, &s1).unwrap().state, "unpaired");
        assert!(b
            .pending_unpair(&f.origin(), &owner, &s1)
            .unwrap()
            .is_none());
        assert!(b
            .next_deliveries(&f.origin(), &owner)
            .unwrap()
            .iter()
            .all(|(scope, _, _)| scope != &s1));

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
        net.publish(&owner, unpair.clone()).await.unwrap();
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
        assert!(through_proxy.publish(&owner, restrictive).await.is_err());
        // Exact durable retries retain their separate recovery allowance.
        through_proxy.publish(&owner, unpair).await.unwrap();
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
        let http_scope = buzz_core::paired_message::Scope {
            owner: outsider.public_key(),
            generation: "ce".repeat(32),
            ..scope.clone()
        };
        let http_claim = wire::seal(
            &outsider,
            &http_scope,
            wire::Body::PairClaim {
                proof: "ab".repeat(32),
            },
            "cd".repeat(32),
            None,
            now(),
            now() + 600,
        )
        .unwrap();
        wire::verify(
            &serde_json::to_vec(&http_claim).unwrap(),
            &f.origin(),
            outsider.public_key(),
        )
        .unwrap();
        for (path, filter, unsupported) in [
            ("/query", json!([{"ids":[ack.id.to_hex()]}]), false),
            ("/count", json!([{"ids":[ack.id.to_hex()]}]), false),
            ("/events", serde_json::to_value(&http_claim).unwrap(), true),
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
                std::fs::remove_file(&ready).unwrap();
                let replacement=Command::new(std::env::current_exe().unwrap()).args(["handlers::host_registry::external_infra_registration_tests::external_infra_tests::external_infra_process_tests::child_root_relay","--exact","--ignored","--nocapture"])
                    .env("BUZZ_3720_CHILD_HOST",&f.host).env("BUZZ_3720_CHILD_ADDR",addr.to_string()).env("BUZZ_3720_CHILD_RELAY_KEY",f.state.relay_keypair.secret_key().to_secret_hex()).env("BUZZ_3720_CHILD_READY",&ready).env_remove("BUZZ_3720_CRASH_PHASE").stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap();
                let _replacement = Child(replacement);
                marker(&ready).await;
                let mut recovered = NostrWsConnection::connect_to_addr(
                    &format!("wss://{}", f.host),
                    &addr.to_string(),
                )
                .await
                .unwrap();
                h.authenticate(&mut recovered, &scope.generation)
                    .await
                    .unwrap();
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
        let held = f
            .state
            .handler_semaphore
            .clone()
            .acquire_many_owned(f.state.config.max_concurrent_handlers as u32)
            .await
            .unwrap();
        assert!(a
            .page(&owner, registry::query("hosts", None, None).unwrap())
            .await
            .is_err());
        assert!(h
            .page(
                &mut hw,
                f.state.relay_keypair.public_key(),
                registry::query("binding", Some(&scope.generation), None).unwrap()
            )
            .await
            .is_ok());
        drop(held);
        println!("TLS_DNS two_tenants=true host_registration_and_reconcile=true ports=32000,34443 global_queue_held=true reserved_binding_via_proxy=true");
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
    fn test_connection(f: &Fixture, o: &Keys) -> crate::connection::tests::TestConn {
        let ctx = buzz_auth::AuthContext {
            pubkey: o.public_key(),
            scopes: vec![],
            channel_ids: None,
            auth_method: buzz_auth::AuthMethod::Nip42,
            agent_owner_pubkey: None,
        };
        let mut tc = crate::connection::tests::test_conn_in(
            AuthState::Authenticated(ctx),
            None,
            f.community,
        );
        Arc::get_mut(&mut tc.conn).unwrap().tenant =
            buzz_core::TenantContext::resolved(f.community, f.host.clone());
        tc
    }
    fn request_frame(o: &Keys, sub: &str, q: &wire::Query) -> serde_json::Value {
        json!(["REQ",sub,{"kinds":[43103],"#p":[o.public_key().to_hex()],"host_registry":q}])
    }
    async fn owner_barrier(c: &ConnectionState, sub: &str, previous: Option<u64>) -> u64 {
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if let Some(owner) = c.subscriptions.lock().await.get(sub).copied() {
                    if Some(owner) != previous {
                        return owner;
                    }
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap()
    }
    #[tokio::test]
    #[ignore = "requires Postgres, Redis and isolated Docker network"]
    async fn round1_old_errors_and_timeouts_preserve_new_subscription_owner() {
        for timeout in [false, true] {
            let f = Fixture::new().await;
            let o = Keys::generate();
            f.member(&o).await;
            let mut tc = test_connection(&f, &o);
            let mut tx = if timeout {
                None
            } else {
                let mut tx = f.state.db.pool().begin().await.unwrap();
                sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('invite-admission:' || $1::text,0))")
                .bind(f.community.as_uuid()).execute(&mut *tx).await.unwrap();
                Some(tx)
            };
            let q = registry::query("binding", Some(&"ab".repeat(32)), None).unwrap();
            let old = if timeout {
                let pause = Arc::new(crate::handlers::close::test_seam::Pause::default());
                let (c, s, raw) = (tc.conn.clone(), f.state.clone(), raw_filter(&o, &q));
                let task = tokio::spawn(test_seam::READ_PAUSE.scope(pause.clone(), async move {
                    request("same", &raw, &c, &s).await
                }));
                tokio::time::timeout(Duration::from_secs(2), pause.reached.notified())
                    .await
                    .unwrap();
                Some(task)
            } else {
                dispatch(
                    &request_frame(&o, "same", &q).to_string(),
                    tc.conn.clone(),
                    f.state.clone(),
                )
                .await;
                None
            };
            let first = owner_barrier(&tc.conn, "same", None).await;
            let q = registry::query("live", None, None).unwrap();
            dispatch(
                &request_frame(&o, "same", &q).to_string(),
                tc.conn.clone(),
                f.state.clone(),
            )
            .await;
            let second = owner_barrier(&tc.conn, "same", Some(first)).await;
            if let Some(tx) = tx.take() {
                tx.commit().await.unwrap();
            }
            if let Some(task) = old {
                tokio::time::timeout(Duration::from_secs(6), task)
                    .await
                    .unwrap()
                    .unwrap()
                    .unwrap();
            }
            let deadline = tokio::time::Instant::now() + Duration::from_secs(1);
            let mut eose = 0;
            while let Ok(Some(frame)) = tokio::time::timeout_at(deadline, tc.send_rx.recv()).await {
                if let axum::extract::ws::Message::Text(raw) = frame {
                    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
                    assert_ne!(v[0], "CLOSED", "late old terminal frame: {v}");
                    if v[0] == "EOSE" {
                        eose += 1;
                    }
                }
            }
            assert_eq!(eose, 1);
            assert_eq!(
                tc.conn.subscriptions.lock().await.get("same"),
                Some(&second)
            );
            crate::handlers::close::handle_close("same".into(), tc.conn.clone(), f.state.clone())
                .await;
            assert_eq!(
                f.state
                    .pubsub
                    .topic_refcount(&tc.conn.tenant, buzz_pubsub::EventTopic::Global)
                    .await,
                0
            );
            assert!(f
                .state
                .sub_registry
                .get_filters(tc.conn.conn_id, "same")
                .is_none());
            tc.conn.cancel.cancel();
            crate::handlers::close::release_connection_subscriptions(&tc.conn, &f.state).await;
            println!("ROUND1_LIFECYCLE old_timeout={timeout} replacement_retained=true no_late_frames=true topics=0");
            f.stop().await;
        }
    }
    #[tokio::test]
    #[ignore = "requires Postgres, Redis and isolated Docker network"]
    async fn round1_setup_serializes_close_replacement_and_connection_cleanup() {
        for action in ["close", "replace", "cancel"] {
            let f = Fixture::new().await;
            let o = Keys::generate();
            f.member(&o).await;
            let mut tc = test_connection(&f, &o);
            let pause = Arc::new(crate::handlers::close::test_seam::Pause::default());
            let q = registry::query("live", None, None).unwrap();
            let raw = request_frame(&o, "setup", &q)[2].to_string();
            let (c, s) = (tc.conn.clone(), f.state.clone());
            let setup = tokio::spawn(test_seam::SETUP_PAUSE.scope(pause.clone(), async move {
                request("setup", &raw, &c, &s).await
            }));
            tokio::time::timeout(Duration::from_secs(2), pause.reached.notified())
                .await
                .unwrap();
            let (c, s) = (tc.conn.clone(), f.state.clone());
            let transition = tokio::spawn(async move {
                match action {
                    "close" => crate::handlers::close::handle_close("setup".into(), c, s).await,
                    "replace" => {
                        request(
                            "setup",
                            &raw_filter(&o, &registry::query("live", None, None).unwrap()),
                            &c,
                            &s,
                        )
                        .await
                        .unwrap();
                    }
                    _ => {
                        c.cancel.cancel();
                        crate::handlers::close::release_connection_subscriptions(&c, &s).await;
                    }
                }
            });
            tokio::task::yield_now().await;
            assert!(
                !transition.is_finished(),
                "transition crossed incomplete lifecycle"
            );
            pause.resume.notify_one();
            setup.await.unwrap().unwrap();
            transition.await.unwrap();
            if action == "replace" {
                assert_eq!(tc.conn.subscriptions.lock().await.len(), 1);
                assert_eq!(
                    f.state
                        .pubsub
                        .topic_refcount(&tc.conn.tenant, buzz_pubsub::EventTopic::Global)
                        .await,
                    1
                );
                crate::handlers::close::handle_close(
                    "setup".into(),
                    tc.conn.clone(),
                    f.state.clone(),
                )
                .await;
            }
            assert!(tc.conn.subscriptions.lock().await.is_empty());
            assert!(f
                .state
                .sub_registry
                .get_filters(tc.conn.conn_id, "setup")
                .is_none());
            assert_eq!(
                f.state
                    .pubsub
                    .topic_refcount(&tc.conn.tenant, buzz_pubsub::EventTopic::Global)
                    .await,
                0
            );
            if action != "cancel" {
                let mut closed = false;
                let deadline = tokio::time::Instant::now() + Duration::from_millis(80);
                while let Ok(Some(axum::extract::ws::Message::Text(raw))) =
                    tokio::time::timeout_at(deadline, tc.send_rx.recv()).await
                {
                    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
                    assert!(!closed, "late old frame after CLOSED: {v}");
                    closed = v[0] == "CLOSED";
                }
                assert!(closed);
            }
            tc.conn.cancel.cancel();
            crate::handlers::close::release_connection_subscriptions(&tc.conn, &f.state).await;
            println!("ROUND1_SETUP action={action} map_index_topic_clean=true");
            f.stop().await;
        }
    }
    fn raw_filter(o: &Keys, q: &wire::Query) -> String {
        request_frame(o, "unused", q)[2].to_string()
    }
    #[tokio::test]
    #[ignore = "requires Postgres, Redis and isolated Docker network"]
    async fn round1_reconcile_refreshes_clock_and_keeps_timely_saved_ack_retry() {
        fn expired() -> anyhow::Result<RegistrationClock> {
            let mut c = RegistrationClock::read()?;
            c.wall += 601;
            c.millis += 601000;
            Ok(c)
        }
        fn rollback() -> anyhow::Result<RegistrationClock> {
            let mut c = RegistrationClock::read()?;
            c.wall -= 1000;
            Ok(c)
        }
        fn reboot() -> anyhow::Result<RegistrationClock> {
            let mut c = RegistrationClock::read()?;
            c.boot.push_str("-next-boot");
            Ok(c)
        }
        for (name, source) in [
            (
                "expired",
                expired as fn() -> anyhow::Result<RegistrationClock>,
            ),
            ("rollback", rollback),
            ("reboot", reboot),
            ("saved-retry", expired),
        ] {
            let f = Fixture::new().await;
            let o = Keys::generate();
            f.member(&o).await;
            let t = tempfile::tempdir().unwrap();
            let mut d = Device::at(&t.path().join("owner.sqlite")).unwrap();
            let (_hd, mut h) = host();
            let mut start = clock();
            let inv = h.offer(f.origin(), &start).unwrap();
            let (s, claim) = d.pair_registry(&f.origin(), &o, &inv).unwrap();
            let mut net = f.owner(&o).await;
            net.publish(&o, claim.clone()).await.unwrap();
            let saved = if name == "saved-retry" {
                h.claim(&s.generation, &claim, &start).unwrap();
                Some(h.pending().unwrap()[0].clone())
            } else {
                None
            };
            let mut ws = f.host_socket(&h, &s.generation).await;
            if let Some(ack) = &saved {
                // Commit while timely but keep the durable ack pending locally.
                assert!(ws.send_event(ack.clone()).await.unwrap().accepted);
                let q = wire::seal(
                    &o,
                    &s,
                    wire::Body::StatusQuery {
                        ack_id: ack.id.to_hex(),
                        nonce: "ab".repeat(32),
                    },
                    "cd".repeat(32),
                    Some(ack.id.to_hex()),
                    now(),
                    now() + 60,
                )
                .unwrap();
                net.publish(&o, q).await.unwrap();
            }
            start.source = Some(source);
            let result = h
                .reconcile(
                    &mut ws,
                    f.state.relay_keypair.public_key(),
                    &s.generation,
                    "clock",
                    &start,
                )
                .await;
            if let Some(ack) = saved {
                result.unwrap();
                assert_eq!(f.binding_count(s.host).await, 1);
                assert!(ws.send_event(ack).await.unwrap().accepted);
                let b = net
                    .page(
                        &o,
                        registry::query("binding", Some(&s.generation), None).unwrap(),
                    )
                    .await
                    .unwrap();
                assert!(
                    b.data["latest_status_id"].is_null(),
                    "expired readiness query consumed using pre-await time"
                );
            } else {
                assert!(result.is_err(), "{name}");
                assert_eq!(h.state(&s.generation).unwrap(), "spent");
                assert_eq!(f.binding_count(s.host).await, 0);
                assert!(h.pending().unwrap().is_empty());
            }
            println!("ROUND1_CLOCK case={name} correct=true");
            ws.disconnect().await.unwrap();
            net.disconnect().await.unwrap();
            f.stop().await;
        }
    }
    #[tokio::test]
    #[ignore = "requires Postgres, Redis and isolated Docker network"]
    async fn round1_pruning_resets_fresh_mailbox_and_rejects_stale_expired_cursors() {
        let f = Fixture::new().await;
        let o = Keys::generate();
        f.member(&o).await;
        let t = tempfile::tempdir().unwrap();
        let mut d = Device::at(&t.path().join("owner.sqlite")).unwrap();
        let (_hd, mut h) = host();
        let mut net = f.owner(&o).await;
        let (s, mut ws, claim, ack) = pair(&f, &o, &mut d, &mut h, &mut net).await;
        let first = d.registry_status_query(&f.origin(), &o, &s).unwrap();
        net.publish(&o, first.clone()).await.unwrap();
        sqlx::query("UPDATE paired_host_events SET received_at=now()-interval '6 minutes' WHERE community_id=$1 AND event_id=$2")
            .bind(f.community.as_uuid()).bind(first.id.as_bytes().as_slice()).execute(f.state.db.pool()).await.unwrap();
        let fresh = d.registry_status_query(&f.origin(), &o, &s).unwrap();
        net.publish(&o, fresh.clone()).await.unwrap();
        let floor:i64=sqlx::query_scalar("SELECT retention_floor FROM paired_host_bindings WHERE community_id=$1 AND generation=$2").bind(f.community.as_uuid()).bind(wire::bytes(&s.generation).unwrap().as_slice()).fetch_one(f.state.db.pool()).await.unwrap();
        assert!(floor > 0);
        let tc = test_connection(&f, &o);
        let q = registry::query("messages", Some(&s.generation), None).unwrap();
        let high: i64 = sqlx::query_scalar(
            "SELECT max(seq) FROM paired_host_events WHERE community_id=$1 AND generation=$2",
        )
        .bind(f.community.as_uuid())
        .bind(wire::bytes(&s.generation).unwrap().as_slice())
        .fetch_one(f.state.db.pool())
        .await
        .unwrap();
        for (kind, position, expires) in [
            ("pruned", "0", now() + 60),
            ("expired", &high.to_string(), now() - 1),
        ] {
            let parts = scope(&tc.conn, &o.public_key().to_hex(), &q);
            let parts: Vec<_> = parts.iter().map(String::as_str).collect();
            let token = crate::api::paired_hosts::seal_cursor(
                &f.state,
                &parts,
                &serde_json::to_string(&Cursor {
                    position: position.into(),
                    high,
                    expires,
                })
                .unwrap(),
                160,
            )
            .unwrap();
            let mut old = q.clone();
            old.cursor = Some(token);
            assert!(net.page(&o, old).await.is_err(), "{kind}");
        }
        h.reconcile(
            &mut ws,
            f.state.relay_keypair.public_key(),
            &s.generation,
            "retained",
            &clock(),
        )
        .await
        .unwrap();
        let b = net
            .page(
                &o,
                registry::query("binding", Some(&s.generation), None).unwrap(),
            )
            .await
            .unwrap();
        let status_id = b.data["latest_status_id"].as_str().unwrap();
        let e = net
            .page(
                &o,
                registry::query("evidence", Some(&s.generation), Some(status_id)).unwrap(),
            )
            .await
            .unwrap();
        let e: Event = serde_json::from_value(e.data["event"].clone()).unwrap();
        d.accept_registry_status(&f.origin(), &o, &s, &e).unwrap();
        for original in [claim, ack] {
            let p = net
                .page(
                    &o,
                    registry::query("evidence", Some(&s.generation), Some(&original.id.to_hex()))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(p.data["event"]["id"], original.id.to_hex());
        }
        let list = h
            .page(&mut ws, f.state.relay_keypair.public_key(), q)
            .await
            .unwrap();
        assert!(list.data["events"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["id"] != first.id.to_hex()));
        println!("ROUND1_PRUNING actual_floor={floor} fresh_reconcile_nonce=true consent_retained=true stale_and_expired_closed=true");
        ws.disconnect().await.unwrap();
        net.disconnect().await.unwrap();
        f.stop().await;
    }
    #[tokio::test]
    #[ignore = "requires Postgres, Redis and isolated Docker network"]
    async fn round1_offline_tombstone_restart_retry_and_fresh_generation() {
        let f = Fixture::new().await;
        let o = Keys::generate();
        f.member(&o).await;
        let t = tempfile::tempdir().unwrap();
        let mut d = Device::at(&t.path().join("owner.sqlite")).unwrap();
        let (hd, mut h) = host();
        let mut net = f.owner(&o).await;
        let (s, ws, _claim, ack) = pair(&f, &o, &mut d, &mut h, &mut net).await;
        ws.disconnect().await.unwrap();
        let unpair = d.unpair_registry(&f.origin(), &o, &s).unwrap();
        net.publish(&o, unpair).await.unwrap();
        let mut ws = f.host_socket(&h, &s.generation).await;
        let b = h
            .page(
                &mut ws,
                f.state.relay_keypair.public_key(),
                registry::query("binding", Some(&s.generation), None).unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(b.data["state"], "fenced");
        ws.disconnect().await.unwrap();
        h.revoke(&s.generation, &clock()).unwrap();
        let tomb = h.pending().unwrap()[0].clone();
        assert_eq!(h.state(&s.generation).unwrap(), "dead");
        drop(h);
        let mut h = Host::at(hd.path(), false).unwrap();
        assert_eq!(h.pending().unwrap()[0], tomb);
        let mut old = f.host_socket(&h, &s.generation).await;
        let first = old.send_event(tomb.clone()).await.unwrap();
        if !first.accepted {
            let writer = f
                .state
                .db
                .post_registry_event(
                    f.community,
                    &f.origin(),
                    s.host,
                    &serde_json::to_vec(&tomb).unwrap(),
                    false,
                    &[0; 32],
                )
                .await;
            panic!("first tomb denied: {first:?}, exact writer={writer:?}");
        }
        // Simulate loss of the first OK at the durable consumer: no delivered
        // call, reconnect and resend exactly the persisted tombstone.
        old.disconnect().await.unwrap();
        let mut old = f.host_socket(&h, &s.generation).await;
        assert!(
            old.send_event(tomb.clone()).await.unwrap().accepted,
            "exact tomb retry after reconnect"
        );
        h.delivered(&tomb.id.to_hex()).unwrap();
        assert!(h.pending().unwrap().is_empty());
        let inv = h.offer(f.origin(), &clock()).unwrap();
        let (next, claim) = d.pair_registry(&f.origin(), &o, &inv).unwrap();
        assert_ne!(s.generation, next.generation);
        assert_eq!(s.host, next.host);
        net.publish(&o, claim.clone()).await.unwrap();
        h.claim(&next.generation, &claim, &clock()).unwrap();
        let new_ack = h.pending().unwrap()[0].clone();
        assert!(!old.send_event(new_ack.clone()).await.unwrap().accepted);
        let mut new = f.host_socket(&h, &next.generation).await;
        assert!(new.send_event(new_ack.clone()).await.unwrap().accepted);
        h.delivered(&new_ack.id.to_hex()).unwrap();
        // Exact immutable history may return OK; it must grant no authority.
        old.send_event(ack).await.unwrap();
        let terminal = net
            .page(
                &o,
                registry::query("binding", Some(&s.generation), None).unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(terminal.data["state"], "dead");
        let query = wire::seal(
            &o,
            &s,
            wire::Body::StatusQuery {
                ack_id: wire::verify(&serde_json::to_vec(&tomb).unwrap(), &f.origin(), s.host)
                    .unwrap()
                    .1
                    .parent
                    .unwrap(),
                nonce: "aa".repeat(32),
            },
            "bb".repeat(32),
            wire::verify(&serde_json::to_vec(&tomb).unwrap(), &f.origin(), s.host)
                .unwrap()
                .1
                .parent,
            now(),
            now() + 60,
        )
        .unwrap();
        assert!(net.publish(&o, query).await.is_err());
        assert_eq!(f.binding_count(s.host).await, 1);
        assert!(h.pending().unwrap().is_empty());
        // A synthetic legitimate host signer supplies an altered ack against
        // the same DEAD writer state; the real host facade never re-signs one.
        let controlled = Keys::generate();
        let (dead_scope, mut controlled_ws, old_ack) =
            signed_pair(&f, &o, &controlled, &mut net, "ad".repeat(32)).await;
        let tomb = wire::seal(
            &controlled,
            &dead_scope,
            wire::Body::Unpaired {
                ack_id: old_ack.id.to_hex(),
            },
            "ae".repeat(32),
            Some(old_ack.id.to_hex()),
            now(),
            now() + 86400,
        )
        .unwrap();
        assert!(controlled_ws.send_event(tomb).await.unwrap().accepted);
        let header = wire::verify(
            &serde_json::to_vec(&old_ack).unwrap(),
            &f.origin(),
            controlled.public_key(),
        )
        .unwrap()
        .1;
        let altered = wire::seal(
            &controlled,
            &dead_scope,
            wire::Body::PairAck {
                claim_id: header.parent.clone().unwrap(),
            },
            "af".repeat(32),
            header.parent,
            now(),
            header.expires_at,
        )
        .unwrap();
        assert!(!controlled_ws.send_event(altered).await.unwrap().accepted);
        assert_eq!(f.binding_count(controlled.public_key()).await, 0);
        controlled_ws.disconnect().await.unwrap();
        println!("ROUND1_OFFLINE dead_persisted_before_send=true exact_restart_retry=true fresh_generation=true old_ack_cannot_resurrect=true altered_legitimate_ack_denied=true live_owners=1");
        old.disconnect().await.unwrap();
        new.disconnect().await.unwrap();
        net.disconnect().await.unwrap();
        f.stop().await;
    }
    fn corrupt_payload(
        e: &Event,
        signer: &Keys,
        recipient: PublicKey,
        edit: impl FnOnce(&mut serde_json::Value),
    ) -> Event {
        use nostr::nips::nip44::{self, Version};
        let plain = nip44::decrypt(signer.secret_key(), &recipient, &e.content).unwrap();
        let mut p: serde_json::Value = serde_json::from_str(&plain).unwrap();
        edit(&mut p);
        let encrypted = nip44::encrypt(
            signer.secret_key(),
            &recipient,
            serde_json::to_vec(&p).unwrap(),
            Version::V2,
        )
        .unwrap();
        EventBuilder::new(e.kind, encrypted)
            .tags(e.tags.clone())
            .custom_created_at(e.created_at)
            .sign_with_keys(signer)
            .unwrap()
    }
    #[tokio::test]
    #[ignore = "requires Postgres, Redis and isolated Docker network"]
    async fn round1_legitimate_signer_contradictions_have_no_consumer_effect() {
        let f = Fixture::new().await;
        let o = Keys::generate();
        f.member(&o).await;
        let t = tempfile::tempdir().unwrap();
        let mut d = Device::at(&t.path().join("owner.sqlite")).unwrap();
        let (_hd, mut h) = host();
        let inv = h.offer(f.origin(), &clock()).unwrap();
        let (s, claim) = d.pair_registry(&f.origin(), &o, &inv).unwrap();
        for field in [
            "v",
            "community",
            "owner",
            "host",
            "generation",
            "op",
            "operation_id",
            "parent",
            "expires_at",
            "body-type",
        ] {
            let bad = corrupt_payload(&claim, &o, s.host, |p| match field {
                "v" => p["header"][field] = 3.into(),
                "expires_at" => p["header"][field] = (now() + 1000).into(),
                "community" => p["header"][field] = "https://wrong.test".into(),
                "op" => p["header"][field] = "unpair".into(),
                "body-type" => p["body"] = json!({"type":"unpair","ack_id":"ab".repeat(32)}),
                _ => p["header"][field] = "ab".repeat(32).into(),
            });
            bad.verify().unwrap();
            wire::verify(
                &serde_json::to_vec(&bad).unwrap(),
                &f.origin(),
                o.public_key(),
            )
            .unwrap();
            assert!(h.claim(&s.generation, &bad, &clock()).is_err(), "{field}");
            assert_eq!(h.state(&s.generation).unwrap(), "offered");
            assert!(h.pending().unwrap().is_empty());
            println!("ROUND1_CONTRADICTION field={field} outer_signature=true consumer_rejected=true durable_ack=false");
        }
        // Owner consumer: both the relay projection and cryptographically valid
        // peer evidence must agree, even if an untrusted relay supplies new IDs.
        let h1 = Keys::generate();
        let h2 = Keys::generate();
        let ss = buzz_core::paired_message::Scope {
            community: f.origin(),
            generation: "dd".repeat(32),
            owner: o.public_key(),
            host: h1.public_key(),
        };
        let claim = wire::seal(
            &o,
            &ss,
            wire::Body::PairClaim {
                proof: "ab".repeat(32),
            },
            "01".repeat(32),
            None,
            now(),
            now() + 600,
        )
        .unwrap();
        let ack = wire::seal(
            &h1,
            &ss,
            wire::Body::PairAck {
                claim_id: claim.id.to_hex(),
            },
            "02".repeat(32),
            Some(claim.id.to_hex()),
            now(),
            now() + 600,
        )
        .unwrap();
        let binding = registry::RegistryBinding {
            generation: ss.generation.clone(),
            owner: ss.owner.to_hex(),
            host: ss.host.to_hex(),
            state: "active".into(),
            claim_id: claim.id.to_hex(),
            ack_id: ack.id.to_hex(),
            tombstone_id: None,
            latest_status_id: None,
        };
        for variant in [
            "inner-parent",
            "body-claim",
            "wrong-host",
            "recipient",
            "tenant",
        ] {
            let bad = if variant == "wrong-host" {
                EventBuilder::new(ack.kind, ack.content.clone())
                    .tags(ack.tags.clone())
                    .custom_created_at(ack.created_at)
                    .sign_with_keys(&h2)
                    .unwrap()
            } else if matches!(variant, "recipient" | "tenant") {
                let tags: Vec<_> = ack
                    .tags
                    .iter()
                    .map(|t| {
                        if t.as_slice()[0]
                            == if variant == "recipient" {
                                "p"
                            } else {
                                "community"
                            }
                        {
                            Tag::parse([
                                t.as_slice()[0].as_str(),
                                if variant == "recipient" {
                                    h2.public_key().to_hex()
                                } else {
                                    "https://other.test".into()
                                }
                                .as_str(),
                            ])
                            .unwrap()
                        } else {
                            t.clone()
                        }
                    })
                    .collect();
                EventBuilder::new(ack.kind, ack.content.clone())
                    .tags(tags)
                    .custom_created_at(ack.created_at)
                    .sign_with_keys(&h1)
                    .unwrap()
            } else {
                corrupt_payload(&ack, &h1, o.public_key(), |p| {
                    if variant == "inner-parent" {
                        p["header"]["parent"] = "ff".repeat(32).into()
                    } else {
                        p["body"]["claim_id"] = "ff".repeat(32).into()
                    }
                })
            };
            bad.verify().unwrap();
            let mut b = binding.clone();
            b.ack_id = bad.id.to_hex();
            assert!(
                d.restore_registry(&f.origin(), &o, &b, &claim, &bad, None)
                    .is_err(),
                "{variant}"
            );
            assert!(d
                .hosts(&f.origin(), &o)
                .unwrap()
                .iter()
                .all(|v| v.scope.generation != ss.generation));
        }
        d.restore_registry(&f.origin(), &o, &binding, &claim, &ack, None)
            .unwrap();
        let q = d.registry_status_query(&f.origin(), &o, &ss).unwrap();
        let (_, wire::Body::StatusQuery { nonce, .. }) = wire::open(&h1, &ss, &q, now()).unwrap()
        else {
            panic!()
        };
        let status = wire::seal(
            &h1,
            &ss,
            wire::Body::Status {
                ack_id: ack.id.to_hex(),
                nonce,
                sequence: 1,
                state: "active".into(),
                label: "valid".into(),
            },
            "03".repeat(32),
            Some(q.id.to_hex()),
            now(),
            now() + 60,
        )
        .unwrap();
        for field in ["ack_id", "nonce"] {
            let bad = corrupt_payload(&status, &h1, o.public_key(), |p| {
                p["body"][field] = "ee".repeat(32).into()
            });
            bad.verify().unwrap();
            assert!(d
                .accept_registry_status(&f.origin(), &o, &ss, &bad)
                .is_err());
        }
        d.accept_registry_status(&f.origin(), &o, &ss, &status)
            .unwrap();
        println!("ROUND1_CONSUMER wrong_host_parent_ack_nonce_recipient_tenant_rejected=true valid_followup=true");
        f.stop().await;
    }
    #[tokio::test]
    #[ignore = "requires Postgres, Redis and isolated Docker network"]
    async fn round1_wire_exchange_is_complete_and_correlated() {
        use futures_util::{SinkExt, StreamExt};
        use tokio_tungstenite::tungstenite::{client::IntoClientRequest, Message};
        let f = Fixture::new().await;
        let o = Keys::generate();
        f.member(&o).await;
        let tcp = tokio::net::TcpStream::connect(f.addr).await.unwrap();
        let (mut ws, _) = tokio_tungstenite::client_async(
            format!("wss://{}", f.host).into_client_request().unwrap(),
            tcp,
        )
        .await
        .unwrap();
        macro_rules! recv {()=>{{loop{let frame=tokio::time::timeout(Duration::from_secs(5),ws.next()).await.unwrap().unwrap().unwrap();if let Message::Text(raw)=frame{let v:serde_json::Value=serde_json::from_str(&raw).unwrap();println!("WIRE_TRACE {}",json!({"direction":"relay-to-client","frame":v}));break v;}}}};}
        macro_rules! send {($v:expr)=>{{let v=$v;println!("WIRE_TRACE {}",json!({"direction":"client-to-relay","frame":v}));ws.send(Message::Text(v.to_string().into())).await.unwrap();}};}
        let challenge = recv!();
        assert_eq!(challenge[0], "AUTH");
        let auth = EventBuilder::auth(
            challenge[1].as_str().unwrap(),
            nostr::RelayUrl::parse(&format!("wss://{}", f.host)).unwrap(),
        )
        .sign_with_keys(&o)
        .unwrap();
        send!(json!(["AUTH", auth]));
        let ok = recv!();
        assert_eq!(ok, json!(["OK", auth.id.to_hex(), true, ""]));
        let h = Keys::generate();
        let s = buzz_core::paired_message::Scope {
            community: f.origin(),
            generation: "de".repeat(32),
            owner: o.public_key(),
            host: h.public_key(),
        };
        let e = wire::seal(
            &o,
            &s,
            wire::Body::PairClaim {
                proof: "ab".repeat(32),
            },
            "bc".repeat(32),
            None,
            now(),
            now() + 600,
        )
        .unwrap();
        send!(json!(["EVENT", e]));
        let ok = recv!();
        assert_eq!(ok[0], "OK");
        assert_eq!(ok[1], e.id.to_hex());
        assert_eq!(ok[2], true);
        let q = registry::query("hosts", None, None).unwrap();
        send!(request_frame(&o, "wire", &q));
        let p = recv!();
        assert_eq!(p[0], "EVENT");
        assert_eq!(p[1], "wire");
        let e: Event = serde_json::from_value(p[2].clone()).unwrap();
        wire::open_reply(&o, f.state.relay_keypair.public_key(), &f.origin(), &q, &e).unwrap();
        assert_eq!(recv!(), json!(["EOSE", "wire"]));
        send!(json!(["CLOSE", "wire"]));
        assert_eq!(recv!(), json!(["CLOSED", "wire", ""]));
        ws.close(None).await.unwrap();
        f.stop().await;
    }
    #[tokio::test]
    #[ignore = "requires Postgres, Redis and isolated Docker network"]
    async fn round1_valid_oa_and_channel_guest_cannot_gain_registration() {
        let mut f = Fixture::new().await;
        let mut enabled = (*f.state).clone();
        Arc::make_mut(&mut enabled.config).allow_nip_oa_auth = true;
        f.state = Arc::new(enabled);
        let (addr, server) = serve(f.state.clone()).await;
        f.addr = addr;
        f.tasks.push(server);
        let owner = Keys::generate();
        let agent = Keys::generate();
        let guest = Keys::generate();
        let host = Keys::generate();
        f.member(&owner).await;
        let tag = buzz_sdk::nip_oa::parse_auth_tag(
            &buzz_sdk::nip_oa::compute_auth_tag(&owner, &agent.public_key(), "").unwrap(),
        )
        .unwrap();
        let mut oa = f.socket().await;
        oa.authenticate(&agent, Some(&tag)).await.unwrap();
        let profile = EventBuilder::new(Kind::Metadata, "{}")
            .tags([tag.clone()])
            .sign_with_keys(&agent)
            .unwrap();
        assert!(
            oa.send_event(profile).await.unwrap().accepted,
            "valid OA ordinary baseline"
        );
        let s = buzz_core::paired_message::Scope {
            community: f.origin(),
            generation: "fa".repeat(32),
            owner: agent.public_key(),
            host: host.public_key(),
        };
        let claim = wire::seal(
            &agent,
            &s,
            wire::Body::PairClaim {
                proof: "ab".repeat(32),
            },
            "01".repeat(32),
            None,
            now(),
            now() + 600,
        )
        .unwrap();
        assert!(!oa.send_event(claim.clone()).await.unwrap().accepted);
        expect_denied(
            &mut oa,
            request_frame(&agent, "oa", &registry::query("hosts", None, None).unwrap()),
        )
        .await;
        assert_eq!(f.binding_count(host.public_key()).await, 0);
        // A direct member's independently established rights remain valid.
        f.member(&agent).await;
        let mut direct = f.owner(&agent).await;
        direct.publish(&agent, claim).await.unwrap();
        let mut mixed = f.socket().await;
        let ClientReply::Auth { challenge } =
            mixed.next_event(Duration::from_secs(5)).await.unwrap()
        else {
            panic!()
        };
        let auth = EventBuilder::auth(
            challenge,
            nostr::RelayUrl::parse(&format!("wss://{}", f.host)).unwrap(),
        )
        .tags([
            tag,
            Tag::parse(["host-registry", "2", &s.generation]).unwrap(),
        ])
        .sign_with_keys(&agent)
        .unwrap();
        mixed.send_raw(&json!(["AUTH", auth])).await.unwrap();
        assert!(
            matches!(mixed.next_event(Duration::from_secs(5)).await.unwrap(),ClientReply::Ok(ok) if !ok.accepted)
        );
        // Guest is a supported channel role, not a root bearer/token identity.
        let channel = f
            .state
            .db
            .create_channel(
                f.community,
                "guest-proof",
                buzz_core::channel::ChannelType::Stream,
                buzz_core::channel::ChannelVisibility::Open,
                None,
                owner.public_key().as_bytes(),
                None,
            )
            .await
            .unwrap();
        f.state
            .db
            .add_member(
                f.community,
                channel.id,
                guest.public_key().as_bytes(),
                buzz_core::channel::MemberRole::Guest,
                None,
            )
            .await
            .unwrap();
        let mut open = (*f.state).clone();
        Arc::make_mut(&mut open.config).require_relay_membership = false;
        let (addr, t) = serve(Arc::new(open)).await;
        f.tasks.push(t);
        let mut ws =
            NostrWsConnection::connect_to_addr(&format!("wss://{}", f.host), &addr.to_string())
                .await
                .unwrap();
        ws.authenticate(&guest, None).await.unwrap();
        ws.send_raw(&json!(["REQ","guest-read",{"#h":[channel.id.to_string()]}]))
            .await
            .unwrap();
        loop {
            match ws.next_event(Duration::from_secs(5)).await.unwrap() {
                ClientReply::Eose { .. } => break,
                ClientReply::Closed { message, .. } => {
                    panic!("guest baseline read denied {message}")
                }
                _ => {}
            }
        }
        let gs = buzz_core::paired_message::Scope {
            owner: guest.public_key(),
            generation: "fb".repeat(32),
            ..s.clone()
        };
        let gc = wire::seal(
            &guest,
            &gs,
            wire::Body::PairClaim {
                proof: "ab".repeat(32),
            },
            "02".repeat(32),
            None,
            now(),
            now() + 600,
        )
        .unwrap();
        assert!(!ws.send_event(gc).await.unwrap().accepted);
        expect_denied(
            &mut ws,
            request_frame(
                &guest,
                "guest-registry",
                &registry::query("hosts", None, None).unwrap(),
            ),
        )
        .await;
        let mut enforcing = (*f.state).clone();
        Arc::make_mut(&mut enforcing.config).nip_fi.mode = buzz_auth::NipFiMode::Enforce;
        assert!(!enforcing.config.serves_host_registry());
        let (addr, t) = serve(Arc::new(enforcing)).await;
        f.tasks.push(t);
        assert!(
            NostrWsConnection::connect_to_addr(&format!("wss://{}", f.host), &addr.to_string())
                .await
                .is_err(),
            "enforcing FI root refuses missing token at actual upgrade"
        );
        println!("ROUND1_CREDENTIALS valid_oa_ordinary=true oa_registration=false direct_member_owner=true channel_guest_read=true guest_registration=false no_guest_token_root_route=true enforcing_fi_upgrade_refused=true");
        oa.disconnect().await.unwrap();
        direct.disconnect().await.unwrap();
        mixed.disconnect().await.unwrap();
        ws.disconnect().await.unwrap();
        f.stop().await;
    }
    #[tokio::test]
    #[ignore = "requires Postgres, Redis and isolated Docker network"]
    async fn round1_database_failure_retains_cache_and_lost_notice_recovers_on_timer() {
        let f = Fixture::new().await;
        let o = Keys::generate();
        f.member(&o).await;
        let t = tempfile::tempdir().unwrap();
        let mut d = Device::at(&t.path().join("owner.sqlite")).unwrap();
        let (_hd, mut h) = host();
        let mut net = f.owner(&o).await;
        let (s, mut hw, _, _) = pair(&f, &o, &mut d, &mut h, &mut net).await;
        net.subscribe(&o).await.unwrap();
        let cached = serde_json::to_value(d.hosts(&f.origin(), &o).unwrap()).unwrap();
        let mut raw = f.socket().await;
        raw.authenticate(&o, None).await.unwrap();
        // Only this fresh task database; restore before asserting the outcome.
        sqlx::query("ALTER TABLE paired_host_bindings RENAME TO paired_host_bindings_fault3720")
            .execute(f.state.db.pool())
            .await
            .unwrap();
        raw.send_raw(&request_frame(
            &o,
            "db-unavailable",
            &registry::query("binding", Some(&s.generation), None).unwrap(),
        ))
        .await
        .unwrap();
        let terminal = raw.next_event(Duration::from_secs(2)).await;
        let late = raw.next_event(Duration::from_millis(80)).await;
        let unavailable = net.recover(&mut d, &o).await;
        let after = serde_json::to_value(d.hosts(&f.origin(), &o).unwrap()).unwrap();
        sqlx::query("ALTER TABLE paired_host_bindings_fault3720 RENAME TO paired_host_bindings")
            .execute(f.state.db.pool())
            .await
            .unwrap();
        assert!(
            matches!(terminal, Ok(ClientReply::Closed { .. })),
            "DB failure sent success/EOSE: {terminal:?}"
        );
        assert!(
            matches!(late, Err(buzz_ws_client::WsClientError::Timeout)),
            "late success after DB failure: {late:?}"
        );
        raw.disconnect().await.unwrap();
        assert!(
            unavailable.is_err(),
            "DB loss must not become complete empty list/EOSE"
        );
        assert_eq!(cached, after);
        assert_eq!(net.recover(&mut d, &o).await.unwrap().len(), 1);
        let q = d.registry_status_query(&f.origin(), &o, &s).unwrap();
        net.publish(&o, q.clone()).await.unwrap();
        h.receive(&s.generation, &q, "lost notification label", &clock())
            .unwrap();
        let status = h.pending().unwrap()[0].clone();
        net.subscribe(&o).await.unwrap(); // establish high-water barrier after query
        test_seam::DROP_NOTICE.lock().unwrap().insert(status.id);
        assert!(hw.send_event(status.clone()).await.unwrap().accepted);
        h.delivered(&status.id.to_hex()).unwrap();
        assert!(
            !test_seam::DROP_NOTICE.lock().unwrap().contains(&status.id),
            "fault must be consumed after commit"
        );
        let start = tokio::time::Instant::now();
        net.wait_for_refresh(&o, Duration::from_secs(1))
            .await
            .unwrap();
        assert!(
            start.elapsed() >= Duration::from_millis(900),
            "unexpected notification delivered"
        );
        net.recover(&mut d, &o).await.unwrap();
        let cards = serde_json::to_value(d.hosts(&f.origin(), &o).unwrap()).unwrap();
        assert!(cards.to_string().contains("lost notification label"));
        println!("ROUND1_DEPENDENCY snapshot_failed_closed=true cache_retained=true db_recovered=true postcommit_notice_dropped=true periodic_resync_label_recovered=true");
        hw.disconnect().await.unwrap();
        net.disconnect().await.unwrap();
        f.stop().await;
    }
    #[tokio::test]
    #[ignore = "requires Postgres, Redis and isolated Docker network"]
    async fn round1_held_work_slots_and_encrypted_boundaries_are_bounded() {
        let f = Fixture::new().await;
        let o = Keys::generate();
        f.member(&o).await;
        let h = Keys::generate();
        let mut net = f.owner(&o).await;
        let (s, hw, ack) = signed_pair(&f, &o, &h, &mut net, "fc".repeat(32)).await;
        let mut tc = test_connection(&f, &o);
        let held = tc
            .conn
            .registry
            .ordinary
            .clone()
            .acquire_many_owned(3)
            .await
            .unwrap();
        let held_global = f
            .state
            .handler_semaphore
            .clone()
            .acquire_many_owned(f.state.config.max_concurrent_handlers as u32)
            .await
            .unwrap();
        dispatch(
            &request_frame(&o, "busy", &registry::query("hosts", None, None).unwrap()).to_string(),
            tc.conn.clone(),
            f.state.clone(),
        )
        .await;
        let frame = tokio::time::timeout(Duration::from_secs(2), tc.send_rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(frame,axum::extract::ws::Message::Text(raw) if raw.contains("CLOSED")));
        dispatch(
            &request_frame(
                &o,
                "recover",
                &registry::query("binding", Some(&s.generation), None).unwrap(),
            )
            .to_string(),
            tc.conn.clone(),
            f.state.clone(),
        )
        .await;
        let mut recovered = false;
        for _ in 0..2 {
            let frame = tokio::time::timeout(Duration::from_secs(2), tc.send_rx.recv())
                .await
                .unwrap()
                .unwrap();
            if let axum::extract::ws::Message::Text(raw) = frame {
                let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
                assert_ne!(v[0], "CLOSED");
                if v[0] == "EOSE" {
                    recovered = true;
                }
            }
        }
        assert!(recovered);
        // Real socket is also blocked by the global ordinary queue; restrictive
        // binding uses the independently reserved connection slot.
        assert!(net
            .page(&o, registry::query("hosts", None, None).unwrap())
            .await
            .is_err());
        assert!(net
            .page(
                &o,
                registry::query("binding", Some(&s.generation), None).unwrap()
            )
            .await
            .is_ok());
        drop(held_global);
        drop(held);
        let valid = wire::seal(
            &o,
            &s,
            wire::Body::StatusQuery {
                ack_id: ack.id.to_hex(),
                nonce: "ab".repeat(32),
            },
            "dc".repeat(32),
            Some(ack.id.to_hex()),
            now(),
            now() + 60,
        )
        .unwrap();
        let empty = EventBuilder::new(valid.kind, "")
            .tags(valid.tags.clone())
            .custom_created_at(valid.created_at)
            .sign_with_keys(&o)
            .unwrap();
        let overhead = serde_json::to_vec(&empty).unwrap().len();
        for target in [wire::MAX_EVENT, wire::MAX_EVENT + 4] {
            use base64::Engine as _;
            let tags: Vec<_> = valid
                .tags
                .iter()
                .map(|t| {
                    if t.as_slice()[0] == "i" {
                        Tag::parse(["i", &format!("{target:064x}")]).unwrap()
                    } else {
                        t.clone()
                    }
                })
                .collect();
            let mut envelope = vec![0; ((target - overhead) / 4) * 3];
            envelope[0] = 2;
            let content = base64::engine::general_purpose::STANDARD.encode(envelope);
            let bad = EventBuilder::new(valid.kind, content)
                .tags(tags)
                .custom_created_at(valid.created_at)
                .sign_with_keys(&o)
                .unwrap();
            let size = serde_json::to_vec(&bad).unwrap().len();
            assert!(size <= target && target - size < 4);
            assert_eq!(
                net.publish(&o, bad.clone()).await.is_ok(),
                size <= wire::MAX_EVENT
            );
            assert!(wire::open(&h, &s, &bad, now()).is_err());
            println!(
                "ROUND1_EVENT_EDGE serialized_bytes={size} bound={} accepted={}",
                wire::MAX_EVENT,
                size <= wire::MAX_EVENT
            );
        }
        // Exact encrypted payload limit with a valid JSON body plus whitespace,
        // then one byte over; ordinary relay cannot decrypt peer payloads.
        use nostr::nips::nip44::{self, Version};
        let plain = nip44::decrypt(o.secret_key(), &s.host, &valid.content).unwrap();
        for size in [wire::MAX_PAYLOAD, wire::MAX_PAYLOAD + 1] {
            let mut bytes = plain.as_bytes().to_vec();
            bytes.resize(size, b' ');
            let content = nip44::encrypt(o.secret_key(), &s.host, &bytes, Version::V2).unwrap();
            let e = EventBuilder::new(valid.kind, content)
                .tags(valid.tags.clone())
                .custom_created_at(valid.created_at)
                .sign_with_keys(&o)
                .unwrap();
            e.verify().unwrap();
            assert!(serde_json::to_vec(&e).unwrap().len() <= wire::MAX_EVENT);
            assert_eq!(
                wire::open(&h, &s, &e, now()).is_ok(),
                size == wire::MAX_PAYLOAD
            );
        }
        let content = nip44::encrypt(o.secret_key(), &s.host, b"{malformed", Version::V2).unwrap();
        let malformed_json = EventBuilder::new(valid.kind, content)
            .tags(valid.tags.clone())
            .custom_created_at(valid.created_at)
            .sign_with_keys(&o)
            .unwrap();
        assert!(wire::open(&h, &s, &malformed_json, now()).is_err());
        let malformed = EventBuilder::new(valid.kind, "not-nip44")
            .tags(valid.tags.clone())
            .custom_created_at(valid.created_at)
            .sign_with_keys(&o)
            .unwrap();
        assert!(wire::open(&h, &s, &malformed, now()).is_err());
        let q = registry::query("hosts", None, None).unwrap();
        let mut reply = wire::Reply {
            v: 2,
            community: f.origin(),
            recipient: o.public_key().to_hex(),
            op: "registry-page".into(),
            query: q.clone(),
            high_water: "ab".repeat(32),
            data: json!(""),
            next_cursor: None,
            complete: true,
        };
        let overhead = serde_json::to_vec(&reply).unwrap().len();
        reply.data = json!("x".repeat(wire::MAX_PAGE - overhead));
        let event =
            wire::reply(&f.state.relay_keypair, o.public_key(), reply.clone(), now()).unwrap();
        assert!(serde_json::to_vec(&event).unwrap().len() + 300 < wire::MAX_FRAME);
        wire::open_reply(
            &o,
            f.state.relay_keypair.public_key(),
            &f.origin(),
            &q,
            &event,
        )
        .unwrap();
        reply.data = json!("x".repeat(wire::MAX_PAGE - overhead + 1));
        assert!(wire::reply(&f.state.relay_keypair, o.public_key(), reply, now()).is_err());
        net.publish(&o, valid).await.unwrap();
        assert!(net
            .page(
                &o,
                registry::query("binding", Some(&s.generation), None).unwrap()
            )
            .await
            .is_ok());
        println!("ROUND1_BOUNDS ordinary_slots_held=3 global_queue_held=true reserved_recovery=true encrypted_page_exact_limit=true oversize_and_malformed_denied=true valid_followup=true");
        tc.conn.cancel.cancel();
        crate::handlers::close::release_connection_subscriptions(&tc.conn, &f.state).await;
        hw.disconnect().await.unwrap();
        net.disconnect().await.unwrap();
        f.stop().await;
    }
}
