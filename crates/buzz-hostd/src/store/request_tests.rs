use super::*;
#[test]
fn create_commit_and_unpair_in_both_orders() {
    use nostr::{EventBuilder, Kind, Tags, Timestamp};
    use paired_message::{Body, Policy};
    for scenario in [
        "revoke",
        "commit",
        "late",
        "catalog",
        "elapsed",
        "inventory",
        "renew",
        "registration-conflict",
        "membership-denied",
    ] {
        let unpair_first = scenario == "revoke";
        let (dir, mut db, clock) = fresh();
        let owner = Keys::generate();
        let (o, proof) = offer(&mut db, &clock);
        let cp = claim(&o, &proof, &owner, clock.wall);
        db.claim(&o.generation, &cp, &clock).unwrap();
        let ack_id: String =
            db.0.query_row("SELECT ack_id FROM grants", [], |r| r.get(0))
                .unwrap();
        let scope = Scope {
            community: o.community.clone(),
            generation: o.generation.clone(),
            owner: owner.public_key(),
            host: o.host,
        };

        let mut cat = Catalog {
            community: Some(o.community.clone()),
            source: None,
            label: "Shared label".into(),
            capacity: 2,
            relay_key: owner.public_key().to_hex(),
            runtimes: [("local/goose".into(), test_runtime())].into(),
        };
        let path = dir.0.join("catalog.json");
        fs::write(&path, serde_json::to_vec(&cat).unwrap()).unwrap();
        let (disk, revision) = Catalog::read(&path).unwrap();
        cat = disk;
        let send = |db: &mut Store, body: Body, ttl: u64| {
            let pkt =
                paired_message::seal(&owner, &scope, body, clock.wall, clock.wall + ttl).unwrap();
            db.receive(&scope.generation, &pkt, &cat, &revision, &clock)
                .unwrap();
            pkt
        };
        let response = |db: &Store, op: &str| {
            let s: String =
                db.0.query_row(
                    "SELECT packet FROM sent WHERE op=? ORDER BY rowid DESC LIMIT 1",
                    [op],
                    |r| r.get(0),
                )
                .unwrap();
            paired_message::open(
                &owner,
                &scope,
                &serde_json::from_str(&s).unwrap(),
                clock.wall,
            )
            .unwrap()
        };
        send(
            &mut db,
            Body::InventoryQuery {
                pair_ack_id: ack_id,
                nonce: random().unwrap(),
            },
            60,
        );
        let catalog_id = response(&db, "inventory").id;
        let policy = Policy {
            respond_to: "owner-only".into(),
            respond_to_allowlist: vec![],
            channel_add_policy: "owner_only".into(),
        };
        let r = random().unwrap();
        let body = Body::Create {
            request_id: r.clone(),
            catalog_id,
            runtime: "local/goose".into(),
            name: "test agent".into(),
            policy,
            channels: if scenario == "membership-denied" {
                vec!["ab78c298-5810-47aa-928b-2707ef1d2639".into()]
            } else {
                vec![]
            },
        };
        if scenario == "inventory" {
            let pkt =
                paired_message::seal(&owner, &scope, body, clock.wall, clock.wall + 900).unwrap();
            let mut t = clock.clone();
            t.wall += 61;
            t.millis += 61000;
            db.receive(&scope.generation, &pkt, &cat, &revision, &t)
                .unwrap();
            let raw: String =
                db.0.query_row("SELECT packet FROM sent WHERE op='result'", [], |r| {
                    r.get(0)
                })
                .unwrap();
            let msg =
                paired_message::open(&owner, &scope, &serde_json::from_str(&raw).unwrap(), t.wall)
                    .unwrap();
            assert!(
                matches!(msg.body,Body::Result{state,steps,..} if state=="failed" && steps==["catalog-changed"])
            );
            continue;
        }
        let create = send(&mut db, body.clone(), 900);
        let conflict =
            paired_message::seal(&owner, &scope, body, clock.wall, clock.wall + 900).unwrap();
        assert!(db
            .receive(&scope.generation, &conflict, &cat, &revision, &clock)
            .is_err());
        let challenge = response(&db, "challenge");
        let Body::Challenge { nonce, .. } = challenge.body else {
            panic!("expected challenge");
        };
        let intent = packet_id(&create).unwrap();
        let confirm = send(
            &mut db,
            Body::Confirm {
                request_id: r.clone(),
                intent_id: intent.clone(),
                challenge_id: challenge.id,
                phase: "generate".into(),
                nonce,
            },
            60,
        );
        drop(db);
        let mut db = Store::load(&dir.0, false).unwrap();
        db.receive(&scope.generation, &confirm, &cat, &revision, &clock)
            .unwrap();
        let offer = response(&db, "key-offer");
        let Body::KeyOffer { agent, nonce, .. } = offer.body else {
            panic!("expected offer");
        };
        let a = PublicKey::from_hex(&agent).unwrap();
        let oa: [String; 4] =
            serde_json::from_str(&buzz_sdk::nip_oa::compute_auth_tag(&owner, &a, "").unwrap())
                .unwrap();
        let reg = EventBuilder::new(
            Kind::Custom(30177),
            r#"{"name":"test agent","parallelism":1,"respond_to":"owner-only"}"#,
        )
        .tags(Tags::parse([["d", &agent], ["runtime", "local/goose"]]).unwrap())
        .custom_created_at(Timestamp::from(clock.wall))
        .sign_with_keys(&owner)
        .unwrap();
        let creds = |id: String| Body::Credentials {
            request_id: r.clone(),
            intent_id: intent.clone(),
            commit_id: id,
            agent: agent.clone(),
            oa: oa.clone(),
            registration: reg.clone(),
        };
        let bad = paired_message::seal(
            &owner,
            &scope,
            creds("ff".repeat(32)),
            clock.wall,
            clock.wall + 86400,
        )
        .unwrap();
        assert!(db
            .receive(&scope.generation, &bad, &cat, &revision, &clock)
            .is_err());
        if unpair_first {
            db.revoke(&scope.generation, &clock).unwrap();
        }
        if scenario == "renew" {
            let mut now = clock.clone();
            now.wall += 61;
            now.millis += 61000;
            crate::request::tick(&mut db, &scope.generation, &now, &revision, true).unwrap();
            let raw: String =
                db.0.query_row(
                    "SELECT packet FROM sent WHERE op='key-offer' ORDER BY rowid DESC LIMIT 1",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            let fresh = paired_message::open(
                &owner,
                &scope,
                &serde_json::from_str(&raw).unwrap(),
                now.wall,
            )
            .unwrap();
            assert_ne!(fresh.id, offer.id);
            assert!(
                matches!(fresh.body,Body::KeyOffer{agent:ref a,nonce:ref n,..} if a==&agent && n!=&nonce)
            );
            let old = paired_message::seal(
                &owner,
                &scope,
                Body::Activate {
                    request_id: r.clone(),
                    intent_id: intent.clone(),
                    offer_id: offer.id,
                    agent,
                    nonce,
                },
                clock.wall,
                clock.wall + 60,
            )
            .unwrap();
            db.receive(&scope.generation, &old, &cat, &revision, &now)
                .unwrap();
            crate::request::tick(&mut db, &scope.generation, &now, &revision, true).unwrap();
            assert_eq!(
                db.0.query_row("SELECT COUNT(*) FROM sent WHERE op='result'", [], |row| row
                    .get::<_, u32>(0))
                    .unwrap(),
                1
            );
            continue;
        }
        if scenario == "late" {
            let act = paired_message::seal(
                &owner,
                &scope,
                Body::Activate {
                    request_id: r.clone(),
                    intent_id: intent.clone(),
                    offer_id: offer.id,
                    agent,
                    nonce,
                },
                clock.wall,
                clock.wall + 60,
            )
            .unwrap();
            let mut late = clock.clone();
            late.wall += 61;
            late.millis += 61000;
            db.receive(&scope.generation, &act, &cat, &revision, &late)
                .unwrap();
            db.receive(&scope.generation, &create, &cat, &revision, &late)
                .unwrap();
            assert_eq!(
                db.0.query_row("SELECT COUNT(*) FROM sent WHERE op='key-offer'", [], |r| {
                    r.get::<_, u32>(0)
                })
                .unwrap(),
                1
            );
            assert_eq!(
                db.0.query_row("SELECT COUNT(*) FROM sent WHERE op='result'", [], |r| r
                    .get::<_, u32>(0))
                    .unwrap(),
                1
            );
            continue;
        }
        if matches!(scenario, "catalog" | "elapsed") {
            if scenario == "catalog" {
                let mut changed: serde_json::Value =
                    serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
                changed["runtimes"] =
                    serde_json::json!({"other/goose":changed["runtimes"]["local/goose"]});
                fs::write(&path, serde_json::to_vec(&changed).unwrap()).unwrap();
            }
            let act = paired_message::seal(
                &owner,
                &scope,
                Body::Activate {
                    request_id: r.clone(),
                    intent_id: intent.clone(),
                    offer_id: offer.id,
                    agent,
                    nonce,
                },
                clock.wall,
                clock.wall + 60,
            )
            .unwrap();
            let mut t = clock.clone();
            if scenario == "elapsed" {
                t.source = Some(|| {
                    Ok(Clock {
                        source: None,
                        wall: 1061,
                        boot: "boot-A".into(),
                        millis: 65000,
                    })
                });
            }
            db.receive(&scope.generation, &act, &cat, &revision, &t)
                .unwrap();
            assert_eq!(
                db.0.query_row("SELECT COUNT(*) FROM sent WHERE op='committed'", [], |r| {
                    r.get::<_, u32>(0)
                })
                .unwrap(),
                0
            );
            assert_eq!(
                db.0.query_row("SELECT COUNT(*) FROM sent WHERE op='result'", [], |r| r
                    .get::<_, u32>(0))
                    .unwrap(),
                1
            );
            continue;
        }
        let act = send(
            &mut db,
            Body::Activate {
                request_id: r.clone(),
                intent_id: intent.clone(),
                offer_id: offer.id,
                agent: agent.clone(),
                nonce,
            },
            60,
        );
        if unpair_first {
            assert!(
                matches!(response(&db,"result").body,Body::Result{state,commit_id:None,..} if state=="revoked-before-commit")
            );
            continue;
        }
        if test_runtime().harness.verify().is_err() {
            // Some namespace runners map root ownership to overflowuid. The production
            // trust check must refuse execution there. Native Linux covers commit below.
            assert!(
                matches!(response(&db,"result").body,Body::Result{state,steps,..} if state=="failed" && steps==["catalog-changed"])
            );
            continue;
        }
        let commit = response(&db, "committed");
        assert!(
            matches!(&commit.body,Body::Committed{activate_id,agent:pubkey,..} if activate_id==&packet_id(&act).unwrap() && pubkey==&agent)
        );
        db.revoke(&scope.generation, &clock).unwrap();
        drop(db);
        let mut db = Store::load(&dir.0, false).unwrap();
        send(&mut db, creds(commit.id), 86400);
        let n: u32 =
            db.0.query_row("SELECT COUNT(*) FROM onboarding", [], |r| r.get(0))
                .unwrap();
        assert_eq!(n, 1);
        db.receive(&scope.generation, &act, &cat, &revision, &clock)
            .unwrap();
        assert_eq!(
            db.0.query_row("SELECT COUNT(*) FROM sent WHERE op='committed'", [], |r| {
                r.get::<_, u32>(0)
            })
            .unwrap(),
            1
        );
        if matches!(scenario, "registration-conflict" | "membership-denied") {
            let (p, d, reg): (String, String, String) =
                db.0.query_row(
                    "SELECT profile,directory,registration FROM onboarding",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .unwrap();
            let profile: serde_json::Value = serde_json::from_str(&p).unwrap();
            let directory: serde_json::Value = serde_json::from_str(&d).unwrap();
            let reg: nostr::Event = serde_json::from_str(&reg).unwrap();
            let registration = if scenario == "registration-conflict" {
                EventBuilder::new(reg.kind, "{}")
                    .tags(reg.tags)
                    .custom_created_at(Timestamp::from(clock.wall + 1))
                    .sign_with_keys(&owner)
                    .unwrap()
            } else {
                reg
            };
            let net = crate::transport::Http::fixture(Box::new(move |url, body| {
                if url.ends_with("/events") {
                    return Ok(serde_json::json!({"accepted":true}));
                }
                let filters: serde_json::Value = serde_json::from_str(body.unwrap())?;
                assert_eq!(filters[0]["consistency"], "strong");
                match filters[0]["kinds"][0].as_u64().unwrap() {
                    0 => Ok(serde_json::json!([profile])),
                    10100 => Ok(serde_json::json!([directory])),
                    30177 => Ok(serde_json::json!([registration])),
                    39002 => Err(crate::transport::HttpStatus(403).into()),
                    _ => panic!("unexpected publication"),
                }
            }));
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(crate::onboard::reconcile(
                    &mut db,
                    &scope.generation,
                    &r,
                    &net,
                    &cat,
                    &revision,
                ))
                .unwrap();
            let terminal: String =
                db.0.query_row("SELECT packet FROM sent WHERE op='result'", [], |row| {
                    row.get(0)
                })
                .unwrap();
            let msg = paired_message::open(
                &owner,
                &scope,
                &serde_json::from_str(&terminal).unwrap(),
                Clock::read().unwrap().wall,
            )
            .unwrap();
            let expected = if scenario == "registration-conflict" {
                "profile-conflict"
            } else {
                "membership-denied:ab78c298-5810-47aa-928b-2707ef1d2639"
            };
            assert!(
                matches!(msg.body,Body::Result{state,steps,..} if state=="failed" && steps.last().unwrap()==expected)
            );
            continue;
        }
        // Replay a crash after enable but before the daemon recorded unit-active.
        let tx = db.tx().unwrap();
        let mut q = crate::request::Request::load(&tx, &scope.generation, &r)
            .unwrap()
            .unwrap();
        q.start_attempted = true;
        q.update(&tx, &scope.generation).unwrap();
        tx.commit().unwrap();
        let socket = dir.0.join("helper.sock");
        let listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
        db.2 = Some(socket);
        let unit_id = buzz_hostd::unit_id(&scope, &r);
        let helper = std::thread::spawn(move || {
            use std::io::{Read, Write};
            for attempt in 0..2 {
                let (mut sock, _) = listener.accept().unwrap();
                let mut cmd = String::new();
                sock.read_to_string(&mut cmd).unwrap();
                assert_eq!(cmd, format!("abort {unit_id}"));
                if attempt == 1 {
                    sock.write_all(b"inactive\n").unwrap();
                }
            }
        });
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let net = crate::transport::Http::fixture(Box::new(|_, _| {
            panic!("failed create must not publish or start")
        }));
        assert!(rt
            .block_on(crate::onboard::reconcile(
                &mut db,
                &scope.generation,
                &r,
                &net,
                &cat,
                "removed"
            ))
            .is_err());
        let tx = db.tx().unwrap();
        let pending = crate::request::Request::load(&tx, &scope.generation, &r)
            .unwrap()
            .unwrap();
        drop(tx);
        assert_eq!(pending.failure.as_deref(), Some("catalog-changed"));
        assert!(pending.terminal.is_none());
        rt.block_on(crate::onboard::reconcile(
            &mut db,
            &scope.generation,
            &r,
            &net,
            &cat,
            &revision,
        ))
        .unwrap();
        helper.join().unwrap();
        let terminal: String =
            db.0.query_row("SELECT packet FROM sent WHERE op='result'", [], |row| {
                row.get(0)
            })
            .unwrap();
        let msg = paired_message::open(
            &owner,
            &scope,
            &serde_json::from_str(&terminal).unwrap(),
            Clock::read().unwrap().wall,
        )
        .unwrap();
        assert!(
            matches!(msg.body,Body::Result{state,steps,..} if state=="failed" && steps==["catalog-changed"])
        );
    }
}
