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
    // No admitted placement preserves the original identity and recovery.
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
    .unwrap_err()
    .contains("Admission"));
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
