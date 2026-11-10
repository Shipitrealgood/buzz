mod external_infra_deployment_tests {
    use super::*;
    #[tokio::test]
    #[ignore = "requires Postgres, Redis and isolated Docker network"]
    async fn deployment_denied_same_id_retires_holder_and_topic() {
        for case in [
            "disabled",
            "malformed",
            "multifilter",
            "ordinary-capacity",
            "global-capacity",
        ] {
            let f = Fixture::with_deployment(case != "disabled").await;
            let owner = Keys::generate();
            f.member(&owner).await;
            let mut tc = test_connection(&f, &owner);
            let q = registry::query("live", None, None).unwrap();
            dispatch(
                &request_frame(&owner, "same", &q).to_string(),
                tc.conn.clone(),
                f.state.clone(),
            )
            .await;
            let previous = owner_barrier(&tc.conn, "same", None).await;
            loop {
                let frame = tokio::time::timeout(Duration::from_secs(3), tc.send_rx.recv())
                    .await
                    .unwrap()
                    .unwrap();
                if let axum::extract::ws::Message::Text(raw) = frame {
                    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
                    assert_ne!(v[0], "CLOSED");
                    if v[0] == "EOSE" {
                        break;
                    }
                }
            }
            assert_eq!(
                f.state
                    .pubsub
                    .topic_refcount(&tc.conn.tenant, buzz_pubsub::EventTopic::Global)
                    .await,
                1
            );
            let ordinary = (case == "ordinary-capacity").then(|| {
                tc.conn
                    .registry
                    .ordinary
                    .clone()
                    .try_acquire_many_owned(tc.conn.registry.ordinary.available_permits() as u32)
                    .unwrap()
            });
            let global = (case == "global-capacity").then(|| {
                f.state
                    .handler_semaphore
                    .clone()
                    .try_acquire_many_owned(f.state.handler_semaphore.available_permits() as u32)
                    .unwrap()
            });
            let mut frame = json!(["REQ", "same", {"kinds":[43104], "host_deployment":{}}]);
            if case == "multifilter" {
                frame.as_array_mut().unwrap().push(json!({}));
            }
            assert!(
                crate::handlers::host_deployment::dispatch(
                    &frame.to_string(),
                    tc.conn.clone(),
                    f.state.clone()
                )
                .await
            );
            let frame = tokio::time::timeout(Duration::from_secs(3), tc.send_rx.recv())
                .await
                .unwrap()
                .unwrap();
            let axum::extract::ws::Message::Text(raw) = frame else {
                panic!("missing CLOSED")
            };
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&raw).unwrap()[0],
                "CLOSED"
            );
            assert!(tc.conn.subscriptions.lock().await.is_empty());
            assert!(f
                .state
                .sub_registry
                .get_filters(tc.conn.conn_id, "same")
                .is_none());
            assert_eq!(
                f.state
                    .pubsub
                    .topic_refcount(&tc.conn.tenant, buzz_pubsub::EventTopic::Global)
                    .await,
                0
            );
            // The displaced live worker cannot enqueue a late frame after CLOSED.
            let late = EventBuilder::new(nostr::Kind::TextNote, "late holder")
                .sign_with_keys(&owner)
                .unwrap();
            assert!(
                !crate::handlers::close::send_if_owner(
                    "same",
                    previous,
                    crate::protocol::RelayMessage::event("same", &late),
                    &tc.conn
                )
                .await
            );
            assert!(
                tokio::time::timeout(Duration::from_millis(50), tc.send_rx.recv())
                    .await
                    .is_err()
            );
            drop((ordinary, global));
            tc.conn.cancel.cancel();
            crate::handlers::close::release_connection_subscriptions(&tc.conn, &f.state).await;
            f.stop().await;
        }
    }
    // Included in the existing external_infra_tests fixture; no alternate relay.
    struct ExecutionInstallation {
        root: tempfile::TempDir,
        helper: buzz_hostd::deployment::Helper,
        catalog: buzz_hostd::Catalog,
        revision: String,
        effects: Arc<std::sync::Mutex<(usize, bool, bool)>>,
        requests: Arc<std::sync::Mutex<Vec<String>>>,
        task: JoinHandle<()>,
    }
    impl ExecutionInstallation {
        async fn new(
            staging: &std::path::Path,
            scope: &buzz_core::paired_message::Scope,
            relay: PublicKey,
        ) -> Self {
            use sha2::{Digest, Sha256};
            use std::os::unix::fs::MetadataExt;
            use std::os::unix::process::ExitStatusExt;
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let root = tempfile::tempdir().unwrap();
            let private = root.path().join("private");
            std::fs::create_dir(&private).unwrap();
            std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o700)).unwrap();
            let template = root.path().join("buzz-paired-agent@.service");
            let catalog_path = root.path().join("catalog.json");
            let exe = buzz_hostd::Executable {
                path: "/usr/bin/true".into(),
                sha256: hex::encode(Sha256::digest(std::fs::read("/usr/bin/true").unwrap())),
            };
            let catalog = buzz_hostd::Catalog {
                community: Some(scope.community.clone()),
                source: None,
                label: "Disposable proof".into(),
                capacity: 8,
                relay_key: relay.to_hex(),
                runtimes: [(
                    "acp/codex".into(),
                    buzz_hostd::Runtime {
                        harness: exe.clone(),
                        adapter: exe,
                        args: vec![],
                        credential_profile: "none".into(),
                    },
                )]
                .into(),
            };
            std::fs::write(&catalog_path, serde_json::to_vec(&catalog).unwrap()).unwrap();
            let (catalog, revision) = buzz_hostd::Catalog::read(&catalog_path).unwrap();
            let socket = root.path().join("activate.sock");
            let listener = tokio::net::UnixListener::bind(&socket).unwrap();
            let helper = buzz_hostd::deployment::Helper::at(staging, &socket).unwrap();
            assert!(buzz_hostd::deployment::Helper::installed(staging).is_err());
            let layout = buzz_hostd::activation::Layout {
                staging: staging.into(),
                private,
                template,
                catalog: catalog_path,
                root_uid: std::fs::metadata(staging).unwrap().uid(),
            };
            let effects = Arc::new(std::sync::Mutex::new((0, false, false)));
            let state = effects.clone();
            let requests = Arc::new(std::sync::Mutex::new(Vec::new()));
            let received = requests.clone();
            let task = tokio::spawn(async move {
                loop {
                    let (mut socket, _) = listener.accept().await.unwrap();
                    let mut request = Vec::new();
                    (&mut socket)
                        .take(74)
                        .read_to_end(&mut request)
                        .await
                        .unwrap();
                    let request = String::from_utf8(request).unwrap();
                    received.lock().unwrap().push(request.clone());
                    let reply = buzz_hostd::activation::apply(
                        &layout,
                        layout.root_uid,
                        &request,
                        &mut |args, _unit| {
                            let mut state = state.lock().unwrap();
                            state.0 += 1;
                            let mut ok = true;
                            let mut stdout = String::new();
                            match args[0] {
                                "show" => {
                                    stdout = format!(
                                "ActiveState={}\nUnitFileState={}\nFragmentPath={}\nDropInPaths=\n",
                                if state.1 { "active" } else { "inactive" },
                                if state.1 { "enabled" } else { "disabled" },
                                layout.template.display()
                            )
                                }
                                "disable" => {
                                    ok = !state.2;
                                    if ok {
                                        state.1 = false;
                                    }
                                }
                                "enable" => state.1 = true,
                                "is-active" | "is-enabled" => ok = state.1,
                                _ => panic!("unexpected fixed operation"),
                            }
                            Ok(std::process::Output {
                                status: std::process::ExitStatus::from_raw(if ok {
                                    0
                                } else {
                                    256
                                }),
                                stdout: stdout.into_bytes(),
                                stderr: vec![],
                            })
                        },
                    )
                    .unwrap_or_else(|_| "failed\n".into());
                    socket.write_all(reply.as_bytes()).await.unwrap();
                }
            });
            Self {
                root,
                helper,
                catalog,
                revision,
                effects,
                requests,
                task,
            }
        }
        async fn stop(self) {
            self.task.abort();
            let _ = self.task.await;
        }
    }
    fn deployment_query(
        scope: &buzz_core::paired_message::Scope,
        view: &str,
        operation: Option<&str>,
    ) -> buzz_core::host_deployment::wire::Query {
        buzz_core::host_deployment::wire::Query {
            v: 1,
            scope: scope.clone(),
            view: view.into(),
            nonce: Keys::generate().secret_key().to_secret_hex(),
            operation_id: operation.map(str::to_string),
        }
    }
    #[tokio::test]
    #[ignore = "requires Postgres, Redis and isolated Docker network"]
    async fn deployment_owner_host_helper_restart_cleanup_journey() {
        use buzz_core::host_deployment::{self as d, wire};
        use nostr::ToBech32;
        let f = Fixture::with_deployment(true).await;
        let owner = Keys::generate();
        f.member(&owner).await;
        let journal = tempfile::tempdir().unwrap();
        let path = journal.path().join("owner.sqlite");
        let mut device = Device::at(&path).unwrap();
        let mut registration = f.owner(&owner).await;
        let (host_dir, mut host) = host();
        let (scope, mut host_ws, _, _) =
            pair(&f, &owner, &mut device, &mut host, &mut registration).await;
        let installation =
            ExecutionInstallation::new(host_dir.path(), &scope, f.state.relay_keypair.public_key())
                .await;
        host.reconcile_deployments(
            &mut host_ws,
            f.state.relay_keypair.public_key(),
            &scope.generation,
            &installation.catalog,
            &installation.revision,
            &installation.helper,
        )
        .await
        .unwrap();
        let mut ws = f.socket().await;
        ws.authenticate(&owner, None).await.unwrap();
        let mut management = buzz_paired_owner::deployment::Management::from_authenticated(
            true,
            &f.origin(),
            owner.public_key(),
            f.state.relay_keypair.public_key(),
            ws,
        )
        .unwrap();
        let reply = management
            .query(
                &|| Ok(owner.clone()),
                &deployment_query(&scope, "status", None),
            )
            .await
            .unwrap();
        let status: Event = serde_json::from_str(reply.data["event"].as_str().unwrap()).unwrap();
        let status =
            wire::open_status(&owner, &status, &scope, nostr::Timestamp::now().as_secs()).unwrap();
        // Private reads require the exact owner, and generic filters cannot opt in.
        let stranger = Keys::generate();
        f.member(&stranger).await;
        let mut stranger_ws = f.socket().await;
        stranger_ws.authenticate(&stranger, None).await.unwrap();
        let q = deployment_query(&scope, "status", None);
        expect_denied(&mut stranger_ws,json!(["REQ","private",{"kinds":[43104],"#p":[stranger.public_key().to_hex()],"host_deployment":q}])).await;
        expect_denied(&mut stranger_ws, json!(["REQ","generic",{"kinds":[43104]}])).await;
        expect_denied(&mut stranger_ws, json!(["COUNT","count",{"kinds":[43104]}])).await;
        let mut generic = f.socket().await;
        generic.authenticate(&owner, None).await.unwrap();
        generic.send_raw(&json!(["REQ", "all", {}])).await.unwrap();
        loop {
            match generic.next_event(Duration::from_secs(5)).await.unwrap() {
                ClientReply::Event { event, .. } => assert_ne!(event.kind.as_u16(), 43104),
                ClientReply::Eose { .. } | ClientReply::Closed { .. } => break,
                _ => {}
            }
        }
        let mut live = f.socket().await;
        live.authenticate(&owner, None).await.unwrap();
        let live_q = deployment_query(&scope, "live", None);
        live.send_raw(&json!(["REQ","private-live",{"kinds":[43104],"#p":[owner.public_key().to_hex()],"host_deployment":live_q}])).await.unwrap();
        let first = live.next_event(Duration::from_secs(5)).await.unwrap();
        let ClientReply::Event { event, .. } = first else {
            panic!("missing private live snapshot")
        };
        wire::open_reply(
            &owner,
            f.state.relay_keypair.public_key(),
            &live_q,
            &event,
            nostr::Timestamp::now().as_secs(),
        )
        .unwrap();
        assert!(matches!(
            live.next_event(Duration::from_secs(5)).await.unwrap(),
            ClientReply::Eose { .. }
        ));
        let agent = Keys::generate();
        let operation = "be".repeat(32);
        let authority = d::Authority {
            v: 1,
            scope: scope.clone(),
            agent: agent.public_key(),
            action: d::Action::Deploy,
            operation_id: operation.clone(),
            expected_revision: 0,
            expires_at: nostr::Timestamp::now().as_secs() + 500,
            content_sha256: String::new(),
        };
        let command = d::Command::Deploy {
            config: Box::new(d::Deploy {
                catalog_revision: installation.revision.clone(),
                runtime_id: "acp/codex".into(),
                name: "Disposable agent".into(),
                policy: buzz_core::paired_message::Policy {
                    respond_to: "owner-only".into(),
                    respond_to_allowlist: vec![],
                    channel_add_policy: "owner_only".into(),
                },
                model: d::ModelSelection::PendingLogin {},
                provider: None,
                system_prompt: Some("Proof prompt".into()),
                effort: None,
                parallelism: 1,
            }),
            identity: buzz_core::private_managed_agent::PrivateIdentity {
                private_key_nsec: agent.secret_key().to_bech32().unwrap(),
                auth_tag: Some(
                    buzz_sdk::nip_oa::compute_auth_tag(&owner, &agent.public_key(), "").unwrap(),
                ),
            },
        };
        let snapshot =
            json!({"record":{"pubkey":agent.public_key().to_hex(),"name":"Disposable agent"}});
        let intent = device
            .prepare_deployment(
                &f.origin(),
                &owner,
                authority.clone(),
                &command,
                &status,
                snapshot.clone(),
            )
            .unwrap();
        let exact = intent.event.clone();
        drop(device);
        // Crash before main save: recover exact candidate and original identity.
        let mut device = Device::at(&path).unwrap();
        let recovered = device
            .deployment_intent(&f.origin(), &owner, &operation)
            .unwrap()
            .unwrap();
        assert_eq!(
            device
                .restore_materialization(&f.origin(), &owner, &recovered)
                .unwrap(),
            snapshot
        );
        assert!(
            wire::restore_command(
                &owner,
                &recovered.protected.authority,
                &recovered.protected.ciphertext
            )
            .unwrap()
                == command
        );
        assert!(management
            .dispatch(&mut device, &|| Ok(owner.clone()), &operation)
            .await
            .is_err());
        device
            .deployment_advance(&f.origin(), &owner, &operation, 1)
            .unwrap();
        device
            .deployment_advance(&f.origin(), &owner, &operation, 2)
            .unwrap();
        management
            .dispatch(&mut device, &|| Ok(owner.clone()), &operation)
            .await
            .unwrap();
        management
            .dispatch(&mut device, &|| Ok(owner.clone()), &operation)
            .await
            .unwrap();
        assert_eq!(
            device
                .deployment_intent(&f.origin(), &owner, &operation)
                .unwrap()
                .unwrap()
                .event,
            exact
        );
        host.reconcile_deployments(
            &mut host_ws,
            f.state.relay_keypair.public_key(),
            &scope.generation,
            &installation.catalog,
            &installation.revision,
            &installation.helper,
        )
        .await
        .unwrap();
        assert!(
            matches!(
                generic.next_event(Duration::from_millis(150)).await,
                Err(buzz_ws_client::WsClientError::Timeout)
            ),
            "generic fanout leaked private management"
        );
        let unit = buzz_hostd::unit_id(&scope, &operation);
        let cred = installation
            .root
            .path()
            .join("private")
            .join(format!("{unit}.cred"));
        let original = std::fs::read(&cred).unwrap();
        let prepared: serde_json::Value = serde_json::from_slice(&original).unwrap();
        assert_eq!(
            prepared["instance"]["secret"],
            agent.secret_key().to_secret_hex()
        );
        assert!(!installation.effects.lock().unwrap().1);
        let row = management
            .query(
                &|| Ok(owner.clone()),
                &deployment_query(&scope, "operation", Some(&operation)),
            )
            .await
            .unwrap();
        let result: Event = serde_json::from_str(row.data["result"].as_str().unwrap()).unwrap();
        assert_eq!(
            device
                .deployment_result(&f.origin(), &owner, &operation, &result)
                .unwrap()
                .outcome,
            d::Outcome::NeedsLogin
        );
        // A lost host receipt is represented by the relay still having pending work:
        // the durable host result must be retransmitted without another helper effect.
        sqlx::query("UPDATE paired_agent_operations SET state='pending',result_json=NULL WHERE community_id=$1 AND operation_id=$2").bind(f.community.as_uuid()).bind(&operation).execute(f.state.db.pool()).await.unwrap();
        // Lost owner reply and daemon restart retain a single key, unit and launch file.
        drop(host);
        let mut host = Host::at(host_dir.path(), false).unwrap();
        let before = installation.effects.lock().unwrap().0;
        host.reconcile_deployments(
            &mut host_ws,
            f.state.relay_keypair.public_key(),
            &scope.generation,
            &installation.catalog,
            &installation.revision,
            &installation.helper,
        )
        .await
        .unwrap();
        assert_eq!(installation.effects.lock().unwrap().0, before);
        assert_eq!(std::fs::read(&cred).unwrap(), original);
        // Queued work expiring before local admission is durably failed once; it
        // cannot starve later operations or change the already prepared identity.
        let mut owner_ws = f.socket().await;
        owner_ws.authenticate(&owner, None).await.unwrap();
        let mut expired = authority.clone();
        expired.operation_id = "bd".repeat(32);
        expired.expected_revision = 1;
        expired.expires_at = nostr::Timestamp::now().as_secs() + 1;
        let (_, event) =
            wire::seal_command(&owner, expired, &command, nostr::Timestamp::now().as_secs())
                .unwrap();
        assert!(owner_ws.send_event(event).await.unwrap().accepted);
        tokio::time::sleep(Duration::from_secs(2)).await;
        let before = installation.effects.lock().unwrap().0;
        for _ in 0..2 {
            host.reconcile_deployments(
                &mut host_ws,
                f.state.relay_keypair.public_key(),
                &scope.generation,
                &installation.catalog,
                &installation.revision,
                &installation.helper,
            )
            .await
            .unwrap();
        }
        assert_eq!(installation.effects.lock().unwrap().0, before);
        assert_eq!(std::fs::read(&cred).unwrap(), original);
        let expired_row = management
            .query(
                &|| Ok(owner.clone()),
                &deployment_query(&scope, "operation", Some(&"bd".repeat(32))),
            )
            .await
            .unwrap();
        assert_eq!(expired_row.data["state"], "failed");
        // A later selected-model update reuses the first unit and imported identity.
        let mut update = authority.clone();
        update.operation_id = "bf".repeat(32);
        update.expected_revision = 2;
        let mut updated: d::Command =
            serde_json::from_value(serde_json::to_value(&command).unwrap()).unwrap();
        if let d::Command::Deploy { config, .. } = &mut updated {
            config.model = d::ModelSelection::Selected {
                id: "fixture-model".into(),
            };
        }
        let (_, event) =
            wire::seal_command(&owner, update, &updated, nostr::Timestamp::now().as_secs())
                .unwrap();
        assert!(owner_ws.send_event(event.clone()).await.unwrap().accepted);
        host.reconcile_deployments(
            &mut host_ws,
            f.state.relay_keypair.public_key(),
            &scope.generation,
            &installation.catalog,
            &installation.revision,
            &installation.helper,
        )
        .await
        .unwrap();
        assert!(installation.effects.lock().unwrap().1);
        let updated_file: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&cred).unwrap()).unwrap();
        assert_eq!(
            updated_file["instance"]["secret"],
            prepared["instance"]["secret"]
        );
        assert_eq!(
            updated_file["instance"]["unit_id"],
            prepared["instance"]["unit_id"]
        );
        let effects = installation.effects.lock().unwrap().0;
        assert!(owner_ws.send_event(event).await.unwrap().accepted);
        host.reconcile_deployments(
            &mut host_ws,
            f.state.relay_keypair.public_key(),
            &scope.generation,
            &installation.catalog,
            &installation.revision,
            &installation.helper,
        )
        .await
        .unwrap();
        assert_eq!(installation.effects.lock().unwrap().0, effects);
        // Current authority unavailable denies work without killing an admitted unit.
        let effects = installation.effects.lock().unwrap().0;
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
        assert!(host
            .reconcile_deployments(
                &mut host_ws,
                f.state.relay_keypair.public_key(),
                &scope.generation,
                &installation.catalog,
                &installation.revision,
                &installation.helper
            )
            .await
            .is_err());
        assert_eq!(installation.effects.lock().unwrap().0, effects);
        assert!(installation.effects.lock().unwrap().1);
        f.member(&owner).await;
        // Forty retained failed cleanups precede useful work: exercise the actual
        // relay page (32), host pass (8), helper and restart cursor together.
        for n in 1..=40 {
            let mut queued = authority.clone();
            queued.agent = Keys::generate().public_key();
            queued.operation_id = format!("e{n:063x}");
            queued.content_sha256 = intent.protected.authority.content_sha256.clone();
            let queued_event = wire::sign_command(
                &owner,
                &queued,
                &intent.protected.ciphertext,
                nostr::Timestamp::now().as_secs(),
            )
            .unwrap();
            f.state
                .db
                .post_host_deployment(
                    f.community,
                    &f.origin(),
                    owner.public_key(),
                    &serde_json::to_vec(&queued_event).unwrap(),
                    false,
                )
                .await
                .unwrap();
            queued.action = d::Action::Cleanup;
            queued.expected_revision = 1;
            let (mut a, encrypted) = wire::seal_command(
                &owner,
                queued,
                &d::Command::Cleanup {},
                nostr::Timestamp::now().as_secs(),
            )
            .unwrap();
            a.operation_id = format!("{n:064x}"); // first anchor permits opaque IDs
            let event = wire::sign_command(
                &owner,
                &a,
                encrypted.content.as_bytes(),
                encrypted.created_at.as_secs(),
            )
            .unwrap();
            f.state
                .db
                .post_host_deployment(
                    f.community,
                    &f.origin(),
                    owner.public_key(),
                    &serde_json::to_vec(&event).unwrap(),
                    false,
                )
                .await
                .unwrap();
        }
        let mut cleanup = authority.clone();
        cleanup.action = d::Action::Cleanup;
        cleanup.expected_revision = 3;
        cleanup.expires_at = nostr::Timestamp::now().as_secs() + 1;
        let (mut expired_cleanup, encrypted) = wire::seal_command(
            &owner,
            cleanup.clone(),
            &d::Command::Cleanup {},
            nostr::Timestamp::now().as_secs(),
        )
        .unwrap();
        expired_cleanup.operation_id = "ff".repeat(32);
        let anchor = wire::sign_command(
            &owner,
            &expired_cleanup,
            encrypted.content.as_bytes(),
            encrypted.created_at.as_secs(),
        )
        .unwrap();
        assert!(owner_ws.send_event(anchor).await.unwrap().accepted);
        // Compare the exact admitted transport serialization, not a different
        // local serde field ordering of the same signed event.
        let anchor_raw: String = sqlx::query_scalar("SELECT event_json FROM paired_agent_operations WHERE community_id=$1 AND operation_id=$2")
            .bind(f.community.as_uuid()).bind(&expired_cleanup.operation_id).fetch_one(f.state.db.pool()).await.unwrap();
        tokio::time::sleep(Duration::from_secs(2)).await;
        let initial_requests = installation.requests.lock().unwrap().len();
        for pass in 0..6 {
            host.reconcile_deployments(
                &mut host_ws,
                f.state.relay_keypair.public_key(),
                &scope.generation,
                &installation.catalog,
                &installation.revision,
                &installation.helper,
            )
            .await
            .unwrap();
            if pass == 0 {
                let attempts = installation.requests.lock().unwrap().len() - initial_requests;
                assert!((7..=8).contains(&attempts), "finite first pass");
                drop(host);
                host = Host::at(host_dir.path(), false).unwrap();
                assert_eq!(host.public_key().unwrap(), scope.host);
            }
        }
        let attempted: std::collections::BTreeSet<_> = installation.requests.lock().unwrap()
            [initial_requests..]
            .iter()
            .cloned()
            .collect();
        assert_eq!(
            attempted.len(),
            40,
            "all retained failures beyond page32 were visited after reopen"
        );
        let expired_row = management
            .query(
                &|| Ok(owner.clone()),
                &deployment_query(&scope, "operation", Some(&expired_cleanup.operation_id)),
            )
            .await
            .unwrap();
        assert_eq!(expired_row.data["state"], "failed");
        assert_eq!(expired_row.data["event"], anchor_raw);
        assert!(
            !installation
                .requests
                .lock()
                .unwrap()
                .contains(&format!("cleanup {unit}")),
            "expired-before-receipt cleanup has no helper effect"
        );
        assert!(
            installation.effects.lock().unwrap().1,
            "expiry did not stop the running agent"
        );
        // Fresh current authorization recovers the fenced placement; previous
        // bytes, original unit and imported key remain unchanged.
        cleanup.expected_revision = 4;
        cleanup.expires_at = nostr::Timestamp::now().as_secs() + 500;
        let (_, event) = wire::seal_command(
            &owner,
            cleanup,
            &d::Command::Cleanup {},
            nostr::Timestamp::now().as_secs(),
        )
        .unwrap();
        assert!(owner_ws.send_event(event).await.unwrap().accepted);
        installation.effects.lock().unwrap().2 = true;
        let fence = installation
            .root
            .path()
            .join("private")
            .join(format!("{unit}.closed"));
        for _ in 0..6 {
            host.reconcile_deployments(
                &mut host_ws,
                f.state.relay_keypair.public_key(),
                &scope.generation,
                &installation.catalog,
                &installation.revision,
                &installation.helper,
            )
            .await
            .unwrap();
            if fence.exists() {
                break;
            }
        }
        assert!(cred.exists());
        assert!(
            fence.exists(),
            "fresh cleanup progressed beyond retained failures"
        );
        let before = installation.requests.lock().unwrap().len();
        for _ in 0..6 {
            host.cleanup_deployments(&scope.generation, &installation.helper)
                .await
                .unwrap();
        }
        let offline: std::collections::BTreeSet<_> = installation.requests.lock().unwrap()
            [before..]
            .iter()
            .cloned()
            .collect();
        assert_eq!(
            offline.len(),
            41,
            "offline cleanup rotates past eight failures too"
        );
        host.revoke(&scope.generation, &clock()).unwrap();
        host.flush_pending(&mut host_ws, &scope.generation)
            .await
            .unwrap();
        let mut closed = false;
        for _ in 0..3 {
            match live.next_event(Duration::from_secs(7)).await.unwrap() {
                ClientReply::Closed {
                    subscription_id, ..
                } if subscription_id == "private-live" => {
                    closed = true;
                    break;
                }
                ClientReply::Event { event, .. } => {
                    wire::open_reply(
                        &owner,
                        f.state.relay_keypair.public_key(),
                        &live_q,
                        &event,
                        nostr::Timestamp::now().as_secs(),
                    )
                    .unwrap();
                }
                _ => panic!("unexpected private live response"),
            }
        }
        assert!(closed, "revoked live view must close");
        assert!(management
            .query(
                &|| Ok(owner.clone()),
                &deployment_query(&scope, "operation", Some(&operation))
            )
            .await
            .is_err());

        installation.effects.lock().unwrap().2 = false;
        for _ in 0..6 {
            host.retry_deployment_cleanup(&f.origin(), &installation.helper)
                .await
                .unwrap();
        }
        assert!(!cred.exists());
        assert!(!host_dir.path().join(format!("{unit}.cred")).exists());
        let before = installation
            .requests
            .lock()
            .unwrap()
            .iter()
            .filter(|r| **r == format!("cleanup {unit}"))
            .count();
        host.retry_deployment_cleanup(&f.origin(), &installation.helper)
            .await
            .unwrap();
        assert_eq!(
            installation
                .requests
                .lock()
                .unwrap()
                .iter()
                .filter(|r| **r == format!("cleanup {unit}"))
                .count(),
            before
        );
        assert!(host_dir.path().join("host.sqlite").exists());
        drop(generic);
        drop(live);
        drop(stranger_ws);
        drop(host_ws);
        drop(owner_ws);
        drop(management);
        drop(registration);
        installation.stop().await;
        f.stop().await;
    }

    #[tokio::test]
    #[ignore = "requires Postgres, Redis and isolated Docker network"]
    async fn deployment_default_off_admission_and_capability_are_independent_of_registration() {
        use axum::{
            body::{to_bytes, Body},
            http::Request,
        };
        use buzz_core::host_deployment::{wire, Action, Authority};
        use sha2::{Digest, Sha256};
        use tower::ServiceExt;
        for enabled in [false, true] {
            let f = Fixture::with_deployment(enabled).await;
            let owner = Keys::generate();
            f.member(&owner).await;
            let journal = tempfile::tempdir().unwrap();
            let mut device = Device::at(&journal.path().join("owner.sqlite")).unwrap();
            let (host_dir, mut host) = host();
            let mut registration = f.owner(&owner).await;
            let (scope, mut host_ws, _, _) =
                pair(&f, &owner, &mut device, &mut host, &mut registration).await;
            let response = crate::router::build_router(f.state.clone())
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
            let metadata: serde_json::Value =
                serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap())
                    .unwrap();
            Registry::capability(&metadata).unwrap();
            assert_eq!(metadata.get("host_deployment").is_some(), enabled);
            // Relay intentionally cannot inspect encrypted config; the host validates it.
            let content = nostr::nips::nip44::encrypt(
                owner.secret_key(),
                &scope.host,
                b"disposable opaque relay admission probe",
                nostr::nips::nip44::Version::V2,
            )
            .unwrap();
            let a = Authority {
                v: 1,
                scope: scope.clone(),
                agent: Keys::generate().public_key(),
                action: Action::Deploy,
                operation_id: "de".repeat(32),
                expected_revision: 0,
                expires_at: nostr::Timestamp::now().as_secs() + 500,
                content_sha256: hex::encode(Sha256::digest(content.as_bytes())),
            };
            let event = wire::sign_command(
                &owner,
                &a,
                content.as_bytes(),
                nostr::Timestamp::now().as_secs(),
            )
            .unwrap();
            let mut ws = f.socket().await;
            ws.authenticate(&owner, None).await.unwrap();
            assert_eq!(ws.send_event(event).await.unwrap().accepted, enabled);
            if enabled {
                let installation = ExecutionInstallation::new(
                    host_dir.path(),
                    &scope,
                    f.state.relay_keypair.public_key(),
                )
                .await;
                host.reconcile_deployments(
                    &mut host_ws,
                    f.state.relay_keypair.public_key(),
                    &scope.generation,
                    &installation.catalog,
                    &installation.revision,
                    &installation.helper,
                )
                .await
                .unwrap();
                assert_eq!(
                    installation.effects.lock().unwrap().0,
                    0,
                    "invalid inner command must never reach helper"
                );
                let state:String=sqlx::query_scalar("SELECT state FROM paired_agent_operations WHERE community_id=$1 AND operation_id=$2").bind(f.community.as_uuid()).bind(&a.operation_id).fetch_one(f.state.db.pool()).await.unwrap();
                assert_eq!(state, "failed");
                installation.stop().await;
            }
            drop(ws);
            drop(host_ws);
            drop(registration);
            f.stop().await;
        }
    }
}
