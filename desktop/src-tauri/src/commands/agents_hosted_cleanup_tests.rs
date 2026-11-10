//! Extends the isolated native fixture; only the authenticated socket peer is fake.
use super::*;
use futures_util::{SinkExt, StreamExt};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

pub(super) async fn exercise<'a, R: tauri::Runtime>(
    app: &AppHandle<R>,
    state: &'a AppState,
    context: &mut Create<'a>,
    host: &Keys,
    path: &std::path::Path,
    retention: &std::path::Path,
) {
    let scope = context.scope.clone();
    let operation = context.operation.clone();
    let owner = context.session.keys().unwrap();
    let agent = context.intent().unwrap().protected.authority.agent;
    let before = serde_json::to_value(load_managed_agents(app).unwrap()).unwrap();
    let relay = Keys::generate();
    let relay_key = relay.public_key();
    let replies = Arc::new(Mutex::new(json!({})));
    let denied = Arc::new(AtomicBool::new(false));
    let switch = Arc::new(AtomicBool::new(false));
    let server_replies = replies.clone();
    let server_denied = denied.clone();
    let server_switch = switch.clone();
    let server_app = app.clone();
    let server_community = scope.community.clone();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let peer = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut socket = tokio_tungstenite::accept_async(socket).await.unwrap();
        while let Some(Ok(frame)) = socket.next().await {
            let Ok(text) = frame.to_text() else {
                continue;
            };
            let msg: serde_json::Value = serde_json::from_str(text).unwrap();
            if msg[0] == "EVENT" {
                let raw = serde_json::to_string(&msg[1]).unwrap();
                let event: nostr::Event = serde_json::from_value(msg[1].clone()).unwrap();
                let (_, wire::Header::Command { authority }) =
                    wire::verify(raw.as_bytes(), &server_community, event.pubkey).unwrap()
                else {
                    panic!("command");
                };
                {
                    let mut replies = server_replies.lock().unwrap();
                    for row in replies.as_object_mut().unwrap().values_mut() {
                        row["revision"] = (authority.expected_revision + 1).into();
                    }
                    replies[&authority.operation_id] = json!({"event":raw,"state":"cleanup-pending","revision":authority.expected_revision+1,"unit_id":"ef".repeat(32)});
                }
                socket
                    .send(tokio_tungstenite::tungstenite::Message::Text(
                        json!(["OK", msg[1]["id"], true, ""]).to_string().into(),
                    ))
                    .await
                    .unwrap();
            }
            if msg[0] != "REQ" {
                continue;
            }
            if server_switch.swap(false, Ordering::SeqCst) {
                server_app
                    .state::<AppState>()
                    .workspace_apply_generation
                    .fetch_add(1, Ordering::AcqRel);
            }
            if server_denied.load(Ordering::SeqCst) {
                socket
                    .send(tokio_tungstenite::tungstenite::Message::Text(
                        json!(["CLOSED", msg[1], "restricted: fixture denied"])
                            .to_string()
                            .into(),
                    ))
                    .await
                    .unwrap();
                continue;
            }
            let query: wire::Query =
                serde_json::from_value(msg[2]["host_deployment"].clone()).unwrap();
            let data = server_replies
                .lock()
                .unwrap()
                .get(query.operation_id.as_deref().unwrap())
                .cloned()
                .unwrap_or_else(|| json!({"state":"missing-or-superseded"}));
            let now = nostr::Timestamp::now().as_secs();
            let reply = wire::Reply {
                query: query.clone(),
                expires_at: now + wire::PERMIT_SECONDS,
                data,
            };
            let event = wire::reply(&relay, query.scope.owner, &reply, now).unwrap();
            socket
                .send(tokio_tungstenite::tungstenite::Message::Text(
                    json!(["EVENT", msg[1], event]).to_string().into(),
                ))
                .await
                .unwrap();
            socket
                .send(tokio_tungstenite::tungstenite::Message::Text(
                    json!(["EOSE", msg[1]]).to_string().into(),
                ))
                .await
                .unwrap();
        }
    });
    let ws = buzz_ws_client_pkg::NostrWsConnection::connect(&format!("ws://{addr}"))
        .await
        .unwrap();
    let mut management =
        Management::from_authenticated(true, &scope.community, owner.public_key(), relay_key, ws)
            .unwrap();
    let run = |result: Result<HostedSetup, String>| result.unwrap();
    // The shared fixture has an unadmitted, unexpired follow-on Deploy. Its
    // competing-admission refusal preserves the original identity and recovery.
    let error = cleanup::coordinate(
        app,
        state,
        &context.session,
        &mut context.device,
        &mut management,
        &scope,
        &operation,
        false,
    )
    .await
    .unwrap_err();
    assert_eq!(error, "competing setup admission unresolved");
    assert_eq!(
        serde_json::to_value(load_managed_agents(app).unwrap()).unwrap(),
        before
    );
    for intent in context
        .device
        .deployment_intents(&scope.community, &owner)
        .unwrap()
    {
        replies.lock().unwrap()[&intent.protected.authority.operation_id] =
            json!({"event":intent.event,"state":"failed","revision":2,"unit_id":"ef".repeat(32)});
    }
    switch.store(true, Ordering::SeqCst);
    assert!(cleanup::coordinate(
        app,
        state,
        &context.session,
        &mut context.device,
        &mut management,
        &scope,
        &operation,
        false
    )
    .await
    .is_err());
    assert_eq!(
        serde_json::to_value(load_managed_agents(app).unwrap()).unwrap(),
        before
    );
    context.session = Session::capture(state, &scope.community, &scope.owner.to_hex())
        .await
        .unwrap();
    // Runtime inventory disappears, but restrictive Cleanup still works.
    context.status.inventory = None;
    let pending = run(cleanup::coordinate(
        app,
        state,
        &context.session,
        &mut context.device,
        &mut management,
        &scope,
        &operation,
        true,
    )
    .await);
    assert_eq!(pending.state, "cleanup-pending");
    assert_eq!(
        serde_json::to_value(load_managed_agents(app).unwrap()).unwrap(),
        before
    );
    let intent = context
        .device
        .deployment_intent(&scope.community, &owner, &pending.operation)
        .unwrap()
        .unwrap();
    for row in replies
        .lock()
        .unwrap()
        .as_object_mut()
        .unwrap()
        .values_mut()
    {
        row["revision"] = 3.into();
    }
    replies.lock().unwrap()[&pending.operation] = json!({"event":intent.event,"state":"cleanup-pending","revision":3,"unit_id":"ef".repeat(32)});
    denied.store(true, Ordering::SeqCst);
    assert!(cleanup::coordinate(
        app,
        state,
        &context.session,
        &mut context.device,
        &mut management,
        &scope,
        &operation,
        false
    )
    .await
    .is_err());
    denied.store(false, Ordering::SeqCst);
    let receipt = wire::result(
        host,
        &d::DeploymentResult {
            request: intent.protected.authority.clone(),
            outcome: d::Outcome::Cleaned,
            applied_revision: Some(3),
        },
        nostr::Timestamp::now().as_secs(),
    )
    .unwrap();
    replies.lock().unwrap()[&pending.operation] = json!({"event":intent.event,"state":"cleaned","revision":3,
        "unit_id":"ef".repeat(32),"result":serde_json::to_string(&receipt).unwrap()});
    let conn = rusqlite::Connection::open(retention).unwrap();
    conn.execute_batch("CREATE TRIGGER cleanup_fixture_failure BEFORE INSERT ON persona_events WHEN NEW.kind=9035 BEGIN SELECT RAISE(ABORT,'fixture finalization failure'); END;").unwrap();
    let failed = run(cleanup::coordinate(
        app,
        state,
        &context.session,
        &mut context.device,
        &mut management,
        &scope,
        &operation,
        false,
    )
    .await);
    assert_eq!(failed.state, "cleanup-local-pending");
    assert!(failed
        .error
        .unwrap()
        .contains("fixture finalization failure"));
    assert!(load_managed_agents(app).unwrap().is_empty());
    assert_eq!(
        context
            .device
            .deployment_intent(&scope.community, &owner, &pending.operation)
            .unwrap()
            .unwrap()
            .phase,
        3
    );
    assert!(context.recover(app, state).await.is_err());
    assert!(load_managed_agents(app).unwrap().is_empty());
    conn.execute_batch("DROP TRIGGER cleanup_fixture_failure")
        .unwrap();
    context.device = Device::at(path).unwrap();
    // H-2: fresh proof precedes a finite local commit. Contention with either
    // existing writer boundary must leave phase 3 retryable before any effect.
    assert!(
        management
            .cleanup_state(
                &mut context.device,
                &|| context.session.keys(),
                &pending.operation
            )
            .await
            .unwrap()
            .1
    );
    for identity in [false, true] {
        let workspace_guard = (!identity).then(|| state.workspace_apply_lock.try_lock().unwrap());
        let identity_guard = identity.then(|| state.identity_mutation.lock().unwrap());
        assert!(cleanup::finish(
            app,
            state,
            &context.session,
            &mut context.device,
            &intent,
            None,
            || panic!("contended finalization must have no effects")
        )
        .is_err());
        assert_eq!(
            context
                .device
                .deployment_intent(&scope.community, &owner, &pending.operation)
                .unwrap()
                .unwrap()
                .phase,
            3
        );
        drop(identity_guard);
        drop(workspace_guard);
    }
    let switched_relay = "wss://cleanup-switched.example";
    let switched_owner = Keys::generate();
    let import_dir = tempfile::tempdir().unwrap();
    std::thread::scope(|threads| {
        let (workspace_start, workspace_started) = std::sync::mpsc::channel();
        let (identity_start, identity_started) = std::sync::mpsc::channel();
        let (tested, attempts) = std::sync::mpsc::channel();
        let workspace_tested = tested.clone();
        let phase = || {
            Device::at(path)
                .unwrap()
                .deployment_intent(&scope.community, &owner, &pending.operation)
                .unwrap()
                .unwrap()
                .phase
        };
        let workspace_writer = threads.spawn(move || {
            workspace_started
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap();
            assert!(
                state.workspace_apply_lock.try_lock().is_err(),
                "workspace writer entered during finalization"
            );
            workspace_tested.send(()).unwrap();
            let _guard = state.workspace_apply_lock.blocking_lock();
            assert_eq!(phase(), 4, "writer entered before completion mark");
            // Same owner is sufficient to reproduce the review's community race.
            state
                .workspace_apply_generation
                .fetch_add(1, Ordering::AcqRel);
            *state.relay_url_override.lock().unwrap() = Some(switched_relay.into());
        });
        let imported_owner = switched_owner.clone();
        let import_path = import_dir.path();
        let identity_writer = threads.spawn(move || {
            identity_started
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap();
            assert!(
                state.identity_mutation.try_lock().is_err(),
                "identity import entered during finalization"
            );
            tested.send(()).unwrap();
            let _guard = state.identity_mutation.lock().unwrap();
            assert_eq!(phase(), 4, "identity import entered before completion mark");
            crate::commands::identity::commit_imported_identity(
                state,
                import_path,
                imported_owner,
                |_| Ok(crate::app_state::IdentityStorage::Ephemeral),
            )
            .unwrap();
        });
        cleanup::finish(
            app,
            state,
            &context.session,
            &mut context.device,
            &intent,
            None,
            || {
                // This is the shared finalizer's existing before-remove boundary,
                // inside the real Session commit and managed-agent store guards.
                assert!(state.managed_agents_store_lock.try_lock().is_err());
                assert_eq!(phase(), 3);
                workspace_start.send(()).unwrap();
                identity_start.send(()).unwrap();
                for _ in 0..2 {
                    attempts
                        .recv_timeout(std::time::Duration::from_secs(5))
                        .unwrap();
                }
                Ok(())
            },
        )
        .unwrap();
        workspace_writer.join().unwrap();
        identity_writer.join().unwrap();
    });
    use crate::managed_agents::retention::{
        get_retained_event, scoped_retention_db_path, tombstone_retention_d_tag,
    };
    use buzz_core_pkg::kind::{KIND_IA_ARCHIVE_REQUEST, KIND_MANAGED_AGENT};
    assert!(get_retained_event(
        &conn,
        5,
        &owner.public_key().to_hex(),
        &tombstone_retention_d_tag(KIND_MANAGED_AGENT, &agent.to_hex())
    )
    .unwrap()
    .is_some());
    assert!(get_retained_event(
        &conn,
        KIND_IA_ARCHIVE_REQUEST,
        &owner.public_key().to_hex(),
        &agent.to_hex()
    )
    .unwrap()
    .is_some());
    let original_relay = scope.community.replacen("https://", "wss://", 1);
    for (relay, signer) in [
        (switched_relay, owner.public_key()),
        (original_relay.as_str(), switched_owner.public_key()),
        (switched_relay, switched_owner.public_key()),
    ] {
        assert!(
            !scoped_retention_db_path(
                &managed_agents_base_dir(app).unwrap(),
                relay,
                &signer.to_hex()
            )
            .exists(),
            "Cleanup leaked into switched retention scope"
        );
    }
    assert!(cleanup::finish(
        app,
        state,
        &context.session,
        &mut context.device,
        &intent,
        None,
        || panic!("stale session must not finalize")
    )
    .is_err());
    {
        let _workspace = state.workspace_apply_lock.try_lock().unwrap();
        let _identity = state.identity_mutation.lock().unwrap();
        *state.relay_url_override.lock().unwrap() = Some(original_relay);
        *state.keys.lock().unwrap() = owner.clone();
    }
    context.session = Session::capture(state, &scope.community, &scope.owner.to_hex())
        .await
        .unwrap();
    let done = run(cleanup::coordinate(
        app,
        state,
        &context.session,
        &mut context.device,
        &mut management,
        &scope,
        &operation,
        false,
    )
    .await);
    assert_eq!(done.state, "deleted");
    assert_eq!(done.phase, 4);
    let public = serde_json::to_string(&done).unwrap();
    for secret in [
        "private_key_nsec",
        "auth_tag",
        "ciphertext",
        "materialization",
        "cleanup_unit",
    ] {
        assert!(!public.contains(secret));
    }
    let retained = cleanup::retained(
        &context.device,
        &owner,
        &context
            .device
            .deployment_intent(&scope.community, &owner, &pending.operation)
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(retained.pubkey, agent.to_hex());
    assert!(context.recover(app, state).await.is_err());
    drop(management);
    peer.abort();
    let _ = peer.await;
}
