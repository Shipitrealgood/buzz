use super::*;
use buzz_core::{
    host_deployment::{self as d, wire},
    paired_message::{Policy, Scope},
    private_managed_agent::PrivateIdentity,
};
use nostr::ToBech32;

#[test]
fn protected_main_snapshot_same_key_expiry_tamper_and_terminal_receipt() {
    let owner = Keys::generate();
    let host = Keys::generate();
    let agent = Keys::generate();
    let scope = Scope {
        community: "https://owner.test".into(),
        owner: owner.public_key(),
        host: host.public_key(),
        generation: "ab".repeat(32),
    };
    let authority = d::Authority {
        v: 1,
        scope: scope.clone(),
        agent: agent.public_key(),
        action: d::Action::Deploy,
        operation_id: "bc".repeat(32),
        expected_revision: 0,
        expires_at: now().unwrap() + 2,
        content_sha256: String::new(),
    };
    let status = d::Status {
        inventory: None,
        scope: scope.clone(),
        availability: d::Availability::Current,
        capability: d::Capability::Supported {
            harnesses: Default::default(),
            catalog_revision: "cd".repeat(32),
            runtimes: vec!["acp/codex".into()],
        },
        expires_at: now().unwrap() + 120,
    };
    let secret = agent.secret_key().to_bech32().unwrap();
    let config = d::Deploy {
        catalog_revision: "cd".repeat(32),
        runtime_id: "acp/codex".into(),
        name: "Retained".into(),
        policy: Policy {
            respond_to: "owner-only".into(),
            respond_to_allowlist: vec![],
            channel_add_policy: "owner_only".into(),
        },
        model: d::ModelSelection::PendingLogin {},
        provider: None,
        system_prompt: None,
        effort: None,
        parallelism: 1,
    };
    let command = d::Command::Deploy {
        config: Box::new(config),
        identity: PrivateIdentity {
            private_key_nsec: secret.clone(),
            auth_tag: Some(
                buzz_sdk::nip_oa::compute_auth_tag(&owner, &agent.public_key(), "").unwrap(),
            ),
        },
    };
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("owner.sqlite");
    let mut device = Device::at(&path).unwrap();
    let snapshot = serde_json::json!({"record":{"pubkey":agent.public_key().to_hex(),"name":"Retained"},"about":"main publication input"});
    let intent = device
        .prepare_deployment(
            &scope.community,
            &owner,
            authority.clone(),
            &command,
            &status,
            snapshot.clone(),
        )
        .unwrap();
    let retry = device
        .prepare_deployment(
            &scope.community,
            &owner,
            authority.clone(),
            &command,
            &status,
            snapshot.clone(),
        )
        .unwrap();
    assert_eq!(intent.event, retry.event);
    assert_eq!(intent.materialization, retry.materialization);
    assert!(!serde_json::to_string(&intent).unwrap().contains(&secret));
    assert!(device
        .prepare_deployment(
            &scope.community,
            &owner,
            authority.clone(),
            &command,
            &status,
            serde_json::json!({"record":{"private_key_nsec":secret}})
        )
        .is_err());
    assert!(device
        .restore_materialization("https://wrong.test", &owner, &intent)
        .is_err());
    assert!(device
        .restore_materialization(&scope.community, &host, &intent)
        .is_err());
    let mut bad: Intent = serde_json::from_value(serde_json::to_value(&intent).unwrap()).unwrap();
    bad.materialization.push('a');
    assert!(device
        .restore_materialization(&scope.community, &owner, &bad)
        .is_err());
    let mut bad: Intent = serde_json::from_value(serde_json::to_value(&intent).unwrap()).unwrap();
    bad.protected.authority.agent = Keys::generate().public_key();
    assert!(device
        .restore_materialization(&scope.community, &owner, &bad)
        .is_err());
    assert!(device
        .deployment_advance(&scope.community, &owner, &authority.operation_id, 2)
        .is_err());
    device
        .deployment_advance(&scope.community, &owner, &authority.operation_id, 1)
        .unwrap();
    assert!(device
        .deployment_advance(&scope.community, &owner, &authority.operation_id, 0)
        .is_err());
    drop(device);
    std::thread::sleep(std::time::Duration::from_secs(3));
    let mut device = Device::at(&path).unwrap();
    let recovered = device
        .deployment_intent(&scope.community, &owner, &authority.operation_id)
        .unwrap()
        .unwrap();
    assert_eq!(
        device
            .restore_materialization(&scope.community, &owner, &recovered)
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
    assert!(recovered.protected.authority.expires_at < now().unwrap());
    assert_eq!(recovered.event, intent.event);
    let result = d::DeploymentResult {
        request: recovered.protected.authority.clone(),
        outcome: d::Outcome::NeedsLogin,
        applied_revision: Some(1),
    };
    let event = wire::result(&host, &result, now().unwrap()).unwrap();
    device
        .deployment_result(&scope.community, &owner, &authority.operation_id, &event)
        .unwrap();
    let changed = wire::result(
        &host,
        &d::DeploymentResult {
            outcome: d::Outcome::Failed,
            applied_revision: None,
            ..result
        },
        now().unwrap(),
    )
    .unwrap();
    assert!(device
        .deployment_result(&scope.community, &owner, &authority.operation_id, &changed)
        .is_err());
}

/// The real Management transport consumes a nonce-bound encrypted relay reply.
/// Only the socket peer is a fixture; Device and continuation are production.
#[tokio::test]
async fn deployment_current_revision_refresh_launch_and_exact_retry() {
    use d::observation::{LoginState, Models, Observation, ResultBody};
    use futures_util::{SinkExt, StreamExt};
    let owner = Keys::generate();
    let host = Keys::generate();
    let relay = Keys::generate();
    let agent = Keys::generate();
    let scope = Scope {
        community: "https://owner.test".into(),
        owner: owner.public_key(),
        host: host.public_key(),
        generation: "ab".repeat(32),
    };
    let status = d::Status {
        inventory: None,
        scope: scope.clone(),
        availability: d::Availability::Current,
        capability: d::Capability::Supported {
            catalog_revision: "cd".repeat(32),
            runtimes: vec!["native/grok".into()],
            harnesses: [("native/grok".into(), "grok".into())].into(),
        },
        expires_at: now().unwrap() + 120,
    };
    let config = d::Deploy {
        catalog_revision: "cd".repeat(32),
        runtime_id: "native/grok".into(),
        name: "New isolated agent".into(),
        policy: Policy {
            respond_to: "owner-only".into(),
            respond_to_allowlist: vec![],
            channel_add_policy: "owner_only".into(),
        },
        model: d::ModelSelection::PendingLogin {},
        provider: None,
        system_prompt: None,
        effort: None,
        parallelism: 1,
    };
    let command = d::Command::Deploy {
        config: Box::new(config.clone()),
        identity: PrivateIdentity {
            private_key_nsec: agent.secret_key().to_bech32().unwrap(),
            auth_tag: Some(
                buzz_sdk::nip_oa::compute_auth_tag(&owner, &agent.public_key(), "").unwrap(),
            ),
        },
    };
    let authority = d::Authority {
        v: 1,
        scope: scope.clone(),
        agent: agent.public_key(),
        action: d::Action::Deploy,
        operation_id: "bc".repeat(32),
        expected_revision: 0,
        expires_at: now().unwrap() + 120,
        content_sha256: String::new(),
    };
    let snapshot = serde_json::json!({"record":{"pubkey":agent.public_key().to_hex(),"persona_id":"one-definition","model":null}});
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("owner.sqlite");
    let mut device = Device::at(&path).unwrap();
    let first = device
        .prepare_deployment(
            &scope.community,
            &owner,
            authority.clone(),
            &command,
            &status,
            snapshot.clone(),
        )
        .unwrap();
    let second_path = path.clone();
    let second_owner = owner.clone();
    let second_authority = first.protected.authority.clone();
    let second_command = wire::restore_command(
        &owner,
        &first.protected.authority,
        &first.protected.ciphertext,
    )
    .unwrap();
    let second_status = status.clone();
    let second_snapshot = snapshot.clone();
    let concurrent = std::thread::spawn(move || {
        Device::at(&second_path)
            .unwrap()
            .prepare_deployment(
                &second_status.scope.community,
                &second_owner,
                second_authority,
                &second_command,
                &second_status,
                second_snapshot,
            )
            .unwrap()
            .event
    });
    let repeated = device
        .prepare_deployment(
            &scope.community,
            &owner,
            first.protected.authority.clone(),
            &command,
            &status,
            snapshot.clone(),
        )
        .unwrap();
    assert_eq!(repeated.event, concurrent.join().unwrap());
    for phase in 1..=2 {
        device
            .deployment_advance(
                &scope.community,
                &owner,
                &first.protected.authority.operation_id,
                phase,
            )
            .unwrap();
    }
    drop(device); // Lost reply/reopened process: identity remains in the protected journal.
    let mut device = Device::at(&path).unwrap();
    assert_eq!(
        device
            .deployment_intents(&scope.community, &owner)
            .unwrap()
            .len(),
        1
    );
    assert!(device
        .deployment_intents("https://another.test", &owner)
        .unwrap()
        .is_empty());
    let catalog = buzz_core::agent_models::AgentModelsResponse {
        agent_name: "Grok".into(),
        agent_version: "fixture".into(),
        supports_switching: true,
        models: vec![buzz_core::agent_models::AgentModelInfo {
            id: "opaque/model:1".into(),
            name: Some("Fixture model".into()),
            description: None,
        }],
        agent_default_model: None,
        selected_model: None,
    };
    let body_for = |intent: &Intent| ResultBody {
        result: d::DeploymentResult {
            request: intent.protected.authority.clone(),
            outcome: d::Outcome::NeedsLogin,
            applied_revision: Some(intent.protected.authority.expected_revision + 1),
        },
        observation: Some(Observation {
            readiness: Some(buzz_core::harness::Readiness::LoggedIn),
            availability: Some(buzz_core::harness::AcpAvailabilityStatus::Available),
            scope: scope.clone(),
            agent: agent.public_key(),
            runtime_id: config.runtime_id.clone(),
            catalog_revision: config.catalog_revision.clone(),
            operation_id: intent.protected.authority.operation_id.clone(),
            revision: intent.protected.authority.expected_revision + 1,
            action_id: "de".repeat(32),
            observed_at: now().unwrap(),
            expires_at: now().unwrap() + 120,
            login: LoginState::NotObserved,
            login_at: None,
            models: Models::Available {
                catalog: catalog.clone(),
            },
        }),
    };
    let mut initial_body = body_for(&first);
    let initial_observation = initial_body.observation.as_mut().unwrap();
    initial_observation.readiness = Some(buzz_core::harness::Readiness::LoggedOut);
    initial_observation.login = LoginState::Succeeded;
    initial_observation.login_at = Some(now().unwrap());
    let receipt = wire::result_body(&host, &initial_body, now().unwrap()).unwrap();
    let current = serde_json::json!({"event":first.event,"result":serde_json::to_string(&receipt).unwrap(),"state":"needs-login","unit_id":"ef".repeat(32),"revision":1});
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let responses = std::sync::Arc::new(std::sync::Mutex::new(current));
    let served = responses.clone();
    let sent = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let published = sent.clone();
    let deny_send = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let denied_send = deny_send.clone();
    let relay_key = relay.public_key();
    let peer = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut socket = tokio_tungstenite::accept_async(socket).await.unwrap();
        while let Some(Ok(frame)) = socket.next().await {
            let Ok(text) = frame.to_text() else {
                continue;
            };
            let msg: Value = serde_json::from_str(text).unwrap();
            if msg[0] == "EVENT" {
                // Match the real relay's exact event storage, including ordering.
                let parts: Vec<&serde_json::value::RawValue> = serde_json::from_str(text).unwrap();
                published.lock().unwrap().push(parts[1].get().to_owned());
                socket
                    .send(tokio_tungstenite::tungstenite::Message::Text(
                        serde_json::json!([
                            "OK",
                            msg[1]["id"],
                            !denied_send.load(std::sync::atomic::Ordering::SeqCst),
                            "fixture admission"
                        ])
                        .to_string()
                        .into(),
                    ))
                    .await
                    .unwrap();
            }
            if msg[0] == "REQ" {
                if served.lock().unwrap().get("denied") == Some(&Value::Bool(true)) {
                    socket
                        .send(tokio_tungstenite::tungstenite::Message::Text(
                            serde_json::json!([
                                "CLOSED",
                                msg[1],
                                "SECRET_SENTINEL: private denial"
                            ])
                            .to_string()
                            .into(),
                        ))
                        .await
                        .unwrap();
                    continue;
                }
                let query: wire::Query =
                    serde_json::from_value(msg[2]["host_deployment"].clone()).unwrap();
                let mode = served.lock().unwrap()["reply_mode"]
                    .as_str()
                    .unwrap_or("")
                    .to_owned();
                let reply = wire::Reply {
                    query: query.clone(),
                    expires_at: if mode == "invalid" {
                        0
                    } else {
                        now().unwrap() + wire::PERMIT_SECONDS
                    },
                    data: {
                        let data = served.lock().unwrap().clone();
                        if let Some(by_operation) = data.get("by_operation") {
                            by_operation[query.operation_id.as_deref().unwrap()].clone()
                        } else {
                            data
                        }
                    },
                };
                let event = wire::reply(&relay, query.scope.owner, &reply, now().unwrap()).unwrap();
                for _ in 0..match mode.as_str() {
                    "missing" => 0,
                    "duplicate" => 2,
                    _ => 1,
                } {
                    socket
                        .send(tokio_tungstenite::tungstenite::Message::Text(
                            serde_json::json!(["EVENT", msg[1], event])
                                .to_string()
                                .into(),
                        ))
                        .await
                        .unwrap();
                }
                socket
                    .send(tokio_tungstenite::tungstenite::Message::Text(
                        serde_json::json!(["EOSE", msg[1]]).to_string().into(),
                    ))
                    .await
                    .unwrap();
            }
        }
    });
    let ws = buzz_ws_client::NostrWsConnection::connect(&format!("ws://{addr}"))
        .await
        .unwrap();
    let mut management =
        Management::from_authenticated(true, &scope.community, owner.public_key(), relay_key, ws)
            .unwrap();
    let signer = || Ok(owner.clone());
    let admitted = responses.lock().unwrap().clone();
    let missing = serde_json::json!({"state":"missing-or-superseded"});
    *responses.lock().unwrap() = missing.clone();
    assert!(management
        .dispatch(&mut device, &signer, &authority.operation_id)
        .await
        .is_err());
    assert_eq!(
        device
            .deployment_intent(&scope.community, &owner, &authority.operation_id)
            .unwrap()
            .unwrap()
            .phase,
        2
    );
    deny_send.store(false, std::sync::atomic::Ordering::SeqCst);
    management
        .dispatch(&mut device, &signer, &authority.operation_id)
        .await
        .unwrap();
    management
        .dispatch(&mut device, &signer, &authority.operation_id)
        .await
        .unwrap();
    assert_eq!(sent.lock().unwrap().len(), 3);
    for event in sent.lock().unwrap().iter() {
        assert_eq!(*event, first.event);
    }
    for (fixture, category) in [
        (serde_json::json!({"denied":true}), "read-closed"),
        (serde_json::json!({"reply_mode":"invalid"}), "reply-invalid"),
        (
            serde_json::json!({"reply_mode":"duplicate"}),
            "reply-duplicate",
        ),
        (serde_json::json!({"reply_mode":"missing"}), "reply-missing"),
    ] {
        *responses.lock().unwrap() = fixture;
        let error = management
            .operation(&mut device, &signer, &authority.operation_id)
            .await
            .err()
            .unwrap();
        assert_eq!(operation_error_category(&error), category);
        assert!(!error.to_string().contains("SECRET_SENTINEL"));
    }
    use buzz_ws_client::WsClientError;
    for (error, category) in [
        (
            anyhow::anyhow!("SECRET_SENTINEL: local path or credential"),
            "operation-read-failed",
        ),
        (
            WsClientError::UnexpectedMessage("SECRET_SENTINEL".into()).into(),
            "operation-read-failed",
        ),
        (
            WsClientError::WebSocket(tokio_tungstenite::tungstenite::Error::Io(
                std::io::Error::other("SECRET_SENTINEL"),
            ))
            .into(),
            "operation-read-failed",
        ),
        (WsClientError::Timeout.into(), "read-timeout"),
        (WsClientError::ConnectionClosed.into(), "read-closed"),
    ] {
        assert_eq!(
            operation_error_category(&error.context("SECRET_SENTINEL: context")),
            category
        );
    }
    *responses.lock().unwrap() = admitted;
    let read = management
        .operation(
            &mut device,
            &signer,
            &first.protected.authority.operation_id,
        )
        .await
        .unwrap();
    assert!(read.current && read.admitted);
    assert_eq!(read.unit.as_deref(), Some("ef".repeat(32).as_str()));
    let mut stale_login_snapshot = snapshot.clone();
    stale_login_snapshot["record"]["model"] = "opaque/model:1".into();
    let refused = management
        .continue_deployment(
            &mut device,
            &signer,
            &first.protected.authority.operation_id,
            &status,
            d::ModelSelection::Selected {
                id: "opaque/model:1".into(),
            },
            stale_login_snapshot,
        )
        .await
        .err()
        .unwrap();
    assert!(refused.to_string().contains("current host readiness"));
    let refreshed = management
        .continue_deployment(
            &mut device,
            &signer,
            &first.protected.authority.operation_id,
            &status,
            d::ModelSelection::PendingLogin {},
            snapshot.clone(),
        )
        .await
        .unwrap();
    assert_eq!(refreshed.protected.authority.expected_revision, 1);
    assert_eq!(refreshed.protected.authority.agent, agent.public_key());
    let refresh_receipt = wire::result_body(&host, &body_for(&refreshed), now().unwrap()).unwrap();
    *responses.lock().unwrap() = serde_json::json!({"event":refreshed.event,"result":serde_json::to_string(&refresh_receipt).unwrap(),"state":"needs-login","unit_id":"ef".repeat(32),"revision":2});
    let mut selected_snapshot = snapshot.clone();
    selected_snapshot["record"]["model"] = "opaque/model:1".into();
    for model in ["disappeared-model", ""] {
        let mut invalid_snapshot = snapshot.clone();
        invalid_snapshot["record"]["model"] = model.into();
        assert!(management
            .continue_deployment(
                &mut device,
                &signer,
                &refreshed.protected.authority.operation_id,
                &status,
                d::ModelSelection::Selected { id: model.into() },
                invalid_snapshot
            )
            .await
            .is_err());
    }
    for availability in [
        d::Availability::Stale,
        d::Availability::Revoked,
        d::Availability::Unavailable,
    ] {
        let mut unavailable = status.clone();
        unavailable.availability = availability;
        assert!(management
            .continue_deployment(
                &mut device,
                &signer,
                &refreshed.protected.authority.operation_id,
                &unavailable,
                d::ModelSelection::PendingLogin {},
                snapshot.clone()
            )
            .await
            .is_err());
    }
    let selected = management
        .continue_deployment(
            &mut device,
            &signer,
            &refreshed.protected.authority.operation_id,
            &status,
            d::ModelSelection::Selected {
                id: "opaque/model:1".into(),
            },
            selected_snapshot.clone(),
        )
        .await
        .unwrap();
    assert_eq!(selected.protected.authority.expected_revision, 2);
    assert_eq!(
        device
            .restore_materialization(&scope.community, &owner, &selected)
            .unwrap(),
        selected_snapshot
    );
    let failed = wire::result(
        &host,
        &d::DeploymentResult {
            request: selected.protected.authority.clone(),
            outcome: d::Outcome::Failed,
            applied_revision: None,
        },
        now().unwrap(),
    )
    .unwrap();
    let failed_data = serde_json::json!({"event":selected.event,"result":serde_json::to_string(&failed).unwrap(),"state":"failed","unit_id":"ef".repeat(32),"revision":3});
    *responses.lock().unwrap() = failed_data.clone();
    let read = management
        .operation(
            &mut device,
            &signer,
            &selected.protected.authority.operation_id,
        )
        .await
        .unwrap();
    assert!(read.current);
    assert_eq!(read.revision, Some(3));
    let next = management
        .continue_deployment(
            &mut device,
            &signer,
            &selected.protected.authority.operation_id,
            &status,
            d::ModelSelection::PendingLogin {},
            selected_snapshot,
        )
        .await
        .unwrap();
    assert_eq!(next.protected.authority.expected_revision, 3);
    let replay = device
        .deployment_intent(
            &scope.community,
            &owner,
            &selected.protected.authority.operation_id,
        )
        .unwrap()
        .unwrap();
    assert_eq!(replay.event, selected.event);
    assert_eq!(
        replay.protected.authority.agent,
        first.protected.authority.agent
    );
    let mut advanced = failed_data;
    advanced["revision"] = 4.into();
    *responses.lock().unwrap() = advanced;
    assert!(
        !management
            .operation(
                &mut device,
                &signer,
                &selected.protected.authority.operation_id
            )
            .await
            .unwrap()
            .current
    );
    assert!(management
        .observation(
            &mut device,
            &signer,
            &selected.protected.authority.operation_id
        )
        .await
        .is_err());
    let mut corrupt = device
        .deployment_intent(
            &scope.community,
            &owner,
            &next.protected.authority.operation_id,
        )
        .unwrap()
        .unwrap();
    corrupt.materialization.push('a');
    assert!(device
        .restore_materialization(&scope.community, &owner, &corrupt)
        .is_err());
    assert!(management
        .operation(
            &mut device,
            &|| Ok(Keys::generate()),
            &selected.protected.authority.operation_id
        )
        .await
        .is_err());
    let serialized = serde_json::to_string(&read).unwrap();
    assert!(!serialized.contains("nsec1"));
    assert!(!serialized.contains("auth_tag"));
    // Expired nonzero PendingLogin and Selected attempts preserve their
    // protected snapshot/identity. Only an exact current predecessor proves
    // where a fresh preparation may continue; local revision alone cannot.
    for selected_model in [false, true] {
        for scenario in [
            "unknown",
            "newer",
            "lost-pending",
            "lost-terminal",
            "absent",
        ] {
            let dir = tempfile::tempdir().unwrap();
            let mut journal = Device::at(&dir.path().join("follow-on.sqlite")).unwrap();
            let root = seed_intent(
                &mut journal,
                &owner,
                authority.clone(),
                &command,
                snapshot.clone(),
            );
            let root_receipt = wire::result_body(&host, &body_for(&root), now().unwrap()).unwrap();
            let mut attempt_authority = authority.clone();
            attempt_authority.operation_id = "da".repeat(32);
            attempt_authority.expected_revision = 1;
            attempt_authority.expires_at = now().unwrap() - 1;
            let d::Command::Deploy {
                mut config,
                identity,
            } = wire::restore_command(
                &owner,
                &root.protected.authority,
                &root.protected.ciphertext,
            )
            .unwrap()
            else {
                panic!("fixture deploy")
            };
            let mut retained_snapshot = snapshot.clone();
            if selected_model {
                config.model = d::ModelSelection::Selected {
                    id: "opaque/model:1".into(),
                };
                retained_snapshot["record"]["model"] = "opaque/model:1".into();
            }
            let attempt = seed_intent(
                &mut journal,
                &owner,
                attempt_authority,
                &d::Command::Deploy { config, identity },
                retained_snapshot.clone(),
            );
            let predecessor = if scenario == "unknown" {
                missing.clone()
            } else {
                serde_json::json!({"event":root.event,"result":serde_json::to_string(&root_receipt).unwrap(),"state":"needs-login","unit_id":"ef".repeat(32),"revision":if scenario=="newer" {2} else {1}})
            };
            let attempted = match scenario {
                "lost-pending" => {
                    serde_json::json!({"event":attempt.event,"state":"pending","unit_id":"ef".repeat(32),"revision":2})
                }
                "lost-terminal" => {
                    let receipt =
                        wire::result_body(&host, &body_for(&attempt), now().unwrap()).unwrap();
                    serde_json::json!({"event":attempt.event,"result":serde_json::to_string(&receipt).unwrap(),"state":"needs-login","unit_id":"ef".repeat(32),"revision":2})
                }
                _ => missing.clone(),
            };
            *responses.lock().unwrap() = serde_json::json!({"by_operation":{
                (root.protected.authority.operation_id.clone()):predecessor,
                (attempt.protected.authority.operation_id.clone()):attempted
            }});
            let recovered = management
                .continue_deployment(
                    &mut journal,
                    &signer,
                    &attempt.protected.authority.operation_id,
                    &status,
                    d::ModelSelection::PendingLogin {},
                    retained_snapshot.clone(),
                )
                .await;
            if matches!(scenario, "unknown" | "newer" | "lost-pending") {
                assert!(recovered.is_err(), "{scenario}");
                assert_eq!(
                    journal
                        .deployment_intents(&scope.community, &owner)
                        .unwrap()
                        .len(),
                    2
                );
                if scenario == "lost-pending" {
                    assert_eq!(
                        journal
                            .deployment_intent(
                                &scope.community,
                                &owner,
                                &attempt.protected.authority.operation_id
                            )
                            .unwrap()
                            .unwrap()
                            .phase,
                        3
                    );
                }
            } else {
                let recovered = recovered.unwrap();
                let revision = if scenario == "lost-terminal" { 2 } else { 1 };
                assert_eq!(recovered.protected.authority.expected_revision, revision);
                assert_eq!(recovered.protected.authority.agent, agent.public_key());
                assert_eq!(
                    journal
                        .restore_materialization(&scope.community, &owner, &recovered)
                        .unwrap(),
                    retained_snapshot
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
                let receipt =
                    wire::result_body(&host, &body_for(&recovered), now().unwrap()).unwrap();
                *responses.lock().unwrap() = serde_json::json!({"event":recovered.event,"result":serde_json::to_string(&receipt).unwrap(),"state":"needs-login","unit_id":"ef".repeat(32),"revision":revision+1});
                let observed = management
                    .operation(
                        &mut journal,
                        &signer,
                        &recovered.protected.authority.operation_id,
                    )
                    .await
                    .unwrap();
                assert_eq!(observed.unit.as_deref(), Some("ef".repeat(32).as_str()));
                assert!(observed.current);
            }
        }
    }
    // Explicit Start shares setup's current-placement/peer checks, but can
    // carry the saved full configuration after an applied predecessor.
    for scenario in ["applied", "pending", "advanced", "identity", "competing"] {
        let dir = tempfile::tempdir().unwrap();
        let mut journal = Device::at(&dir.path().join("start.sqlite")).unwrap();
        let prior = seed_intent(
            &mut journal,
            &owner,
            authority.clone(),
            &command,
            snapshot.clone(),
        );
        let outcome = if scenario == "pending" {
            d::Outcome::Pending
        } else {
            d::Outcome::Applied
        };
        let receipt = wire::result(
            &host,
            &d::DeploymentResult {
                request: prior.protected.authority.clone(),
                outcome,
                applied_revision: if scenario == "pending" { None } else { Some(1) },
            },
            now().unwrap(),
        )
        .unwrap();
        *responses.lock().unwrap() = serde_json::json!({
            "event": prior.event, "result": serde_json::to_string(&receipt).unwrap(),
            "state": if scenario == "pending" { "pending" } else { "applied" },
            "unit_id": "ef".repeat(32), "revision": if scenario == "advanced" { 2 } else { 1 }
        });
        let mut candidate = wire::restore_command(
            &owner,
            &prior.protected.authority,
            &prior.protected.ciphertext,
        )
        .unwrap();
        let d::Command::Deploy { config, identity } = &mut candidate else {
            panic!("fixture deploy")
        };
        config.model = d::ModelSelection::Selected {
            id: "saved/model:2".into(),
        };
        config.system_prompt = Some("Saved instructions".into());
        if scenario == "identity" {
            identity.auth_tag = None;
        }
        let mut saved = snapshot.clone();
        saved["record"]["model"] = "saved/model:2".into();
        saved["record"]["system_prompt"] = "Saved instructions".into();
        assert!(management
            .continue_deployment(
                &mut journal,
                &signer,
                &prior.protected.authority.operation_id,
                &status,
                d::ModelSelection::PendingLogin {},
                saved.clone()
            )
            .await
            .is_err());
        if scenario == "competing" {
            let mut peer_authority = authority.clone();
            peer_authority.operation_id = "ca".repeat(32);
            peer_authority.expected_revision = 1;
            seed_intent(
                &mut journal,
                &owner,
                peer_authority,
                &command,
                snapshot.clone(),
            );
        }
        let started = management
            .start_existing_deployment(
                &mut journal,
                &signer,
                &prior.protected.authority.operation_id,
                &status,
                serde_json::from_value(serde_json::to_value(&candidate).unwrap()).unwrap(),
                saved.clone(),
            )
            .await;
        if scenario != "applied" {
            assert!(started.is_err(), "{scenario}");
            continue;
        }
        let started = started.unwrap();
        assert_eq!(started.protected.authority.expected_revision, 1);
        assert_eq!(started.protected.authority.agent, agent.public_key());
        assert_eq!(started.protected.authority.scope, scope);
        saved["existing_start"] = true.into();
        assert_eq!(
            journal
                .restore_materialization(&scope.community, &owner, &started)
                .unwrap(),
            saved
        );
        assert!(
            wire::restore_command(
                &owner,
                &started.protected.authority,
                &started.protected.ciphertext
            )
            .unwrap()
                == candidate
        );
        for phase in 1..=2 {
            journal
                .deployment_advance(
                    &scope.community,
                    &owner,
                    &started.protected.authority.operation_id,
                    phase,
                )
                .unwrap();
        }
        drop(journal);
        let mut journal = Device::at(&dir.path().join("start.sqlite")).unwrap();
        *responses.lock().unwrap() = missing.clone();
        let before = sent.lock().unwrap().len();
        for _ in 0..2 {
            management
                .dispatch(
                    &mut journal,
                    &signer,
                    &started.protected.authority.operation_id,
                )
                .await
                .unwrap();
        }
        assert_eq!(
            &sent.lock().unwrap()[before..],
            &[started.event.clone(), started.event]
        );
    }
    // Expired commands remain recoverable. Admission, not wall-clock expiry,
    // determines whether a new initial operation is allowed.
    let expired_dir = tempfile::tempdir().unwrap();
    let mut expired_device = Device::at(&expired_dir.path().join("owner.sqlite")).unwrap();
    let mut expired_authority = authority.clone();
    expired_authority.operation_id = "fa".repeat(32);
    expired_authority.expires_at = now().unwrap() - 1;
    let expired = seed_intent(
        &mut expired_device,
        &owner,
        expired_authority,
        &command,
        snapshot.clone(),
    );
    *responses.lock().unwrap() = missing.clone();
    let before = sent.lock().unwrap().len();
    assert!(management
        .dispatch(
            &mut expired_device,
            &signer,
            &expired.protected.authority.operation_id
        )
        .await
        .is_err());
    assert_eq!(sent.lock().unwrap().len(), before);
    let mut again_authority = expired.protected.authority.clone();
    again_authority.operation_id = "fc".repeat(32);
    let again = seed_intent(
        &mut expired_device,
        &owner,
        again_authority,
        &command,
        snapshot.clone(),
    );
    let lost_dir = tempfile::tempdir().unwrap();
    let mut lost_device = Device::at(&lost_dir.path().join("owner.sqlite")).unwrap();
    let lost = seed_intent(
        &mut lost_device,
        &owner,
        expired.protected.authority.clone(),
        &command,
        snapshot.clone(),
    );
    let absent = seed_intent(
        &mut lost_device,
        &owner,
        again.protected.authority.clone(),
        &command,
        snapshot.clone(),
    );
    let lost_receipt = wire::result_body(&host, &body_for(&lost), now().unwrap()).unwrap();
    *responses.lock().unwrap() = serde_json::json!({"by_operation":{
        (absent.protected.authority.operation_id.clone()):missing.clone(),
        (lost.protected.authority.operation_id.clone()):{"event":lost.event,"result":serde_json::to_string(&lost_receipt).unwrap(),"state":"needs-login","unit_id":"ef".repeat(32),"revision":1}
    }});
    assert!(management
        .continue_deployment(
            &mut lost_device,
            &signer,
            &absent.protected.authority.operation_id,
            &status,
            d::ModelSelection::PendingLogin {},
            snapshot.clone()
        )
        .await
        .is_err());
    assert_eq!(
        lost_device
            .deployment_intents(&scope.community, &owner)
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        lost_device
            .deployment_intent(
                &scope.community,
                &owner,
                &lost.protected.authority.operation_id
            )
            .unwrap()
            .unwrap()
            .phase,
        3
    );
    *responses.lock().unwrap() = missing.clone();
    let replacement = management
        .continue_deployment(
            &mut expired_device,
            &signer,
            &again.protected.authority.operation_id,
            &status,
            d::ModelSelection::PendingLogin {},
            snapshot.clone(),
        )
        .await
        .unwrap();
    assert_eq!(replacement.protected.authority.expected_revision, 0);
    assert_eq!(replacement.protected.authority.agent, agent.public_key());
    assert!(
        wire::restore_command(
            &owner,
            &replacement.protected.authority,
            &replacement.protected.ciphertext
        )
        .unwrap()
            == command
    );
    // A distinct reopened journal whose initial operation was already admitted
    // reconciles its terminal receipt without trying to resend expired bytes.
    let admitted_dir = tempfile::tempdir().unwrap();
    let mut admitted_device = Device::at(&admitted_dir.path().join("owner.sqlite")).unwrap();
    let expired = seed_intent(
        &mut admitted_device,
        &owner,
        expired.protected.authority.clone(),
        &command,
        snapshot.clone(),
    );
    let mut expired_body = body_for(&expired);
    let observation = expired_body.observation.as_mut().unwrap();
    observation.observed_at = now().unwrap() - 60;
    observation.login = LoginState::Succeeded;
    observation.login_at = Some(now().unwrap() - 60);
    observation.expires_at = now().unwrap() - 1;
    let receipt = wire::result_body(&host, &expired_body, now().unwrap() - 60).unwrap();
    *responses.lock().unwrap() = serde_json::json!({"event":expired.event,"result":serde_json::to_string(&receipt).unwrap(),"state":"needs-login","unit_id":"ef".repeat(32),"revision":1});
    management
        .dispatch(
            &mut admitted_device,
            &signer,
            &expired.protected.authority.operation_id,
        )
        .await
        .unwrap();
    assert_eq!(sent.lock().unwrap().len(), before);
    assert_eq!(
        admitted_device
            .deployment_intent(
                &scope.community,
                &owner,
                &expired.protected.authority.operation_id
            )
            .unwrap()
            .unwrap()
            .phase,
        3
    );
    assert!(management
        .observation(
            &mut admitted_device,
            &signer,
            &expired.protected.authority.operation_id
        )
        .await
        .is_err());
    *responses.lock().unwrap() = missing;
    assert!(management
        .continue_deployment(
            &mut admitted_device,
            &signer,
            &expired.protected.authority.operation_id,
            &status,
            d::ModelSelection::PendingLogin {},
            snapshot.clone()
        )
        .await
        .is_err());
    // Signer changes during the read cannot authorize a phase/result mutation.
    *responses.lock().unwrap() = serde_json::json!({"state":"missing-or-superseded"});
    let calls = std::cell::Cell::new(0);
    let changed = || {
        let n = calls.get();
        calls.set(n + 1);
        Ok(if n < 2 {
            owner.clone()
        } else {
            Keys::generate()
        })
    };
    assert!(management
        .operation(
            &mut admitted_device,
            &changed,
            &expired.protected.authority.operation_id
        )
        .await
        .is_err());
    // Fill the existing bounded table only in this disposable fixture; refusal
    // keeps every prior row rather than evicting recovery evidence.
    let tx = expired_device.db.transaction().unwrap();
    for index in 3..1024 {
        tx.execute(
            "INSERT INTO deployment_intents VALUES(?,?,?,?)",
            params![
                scope.community,
                owner.public_key().to_hex(),
                format!("{index:064x}"),
                serde_json::to_string(&expired).unwrap()
            ],
        )
        .unwrap();
    }
    tx.commit().unwrap();
    let mut extra = authority.clone();
    extra.operation_id = "fb".repeat(32);
    assert!(expired_device
        .prepare_deployment(&scope.community, &owner, extra, &command, &status, snapshot)
        .err()
        .unwrap()
        .to_string()
        .contains("capacity"));
    let count: i64 = expired_device
        .db
        .query_row("SELECT count(*) FROM deployment_intents", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1024);
    // Two main candidates mint different keys for the same new operation.
    // The journal transaction chooses one before either can save a main record.
    let race_dir = tempfile::tempdir().unwrap();
    let race_path = race_dir.path().join("race.sqlite");
    Device::at(&race_path).unwrap().deployment_schema().unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let workers:Vec<_>=(0..2).map(|_| {
        let path=race_path.clone(); let owner=owner.clone(); let status=status.clone();
        let mut authority=authority.clone(); let config=config.clone(); let barrier=barrier.clone();
        std::thread::spawn(move || {
            let candidate=Keys::generate(); authority.agent=candidate.public_key();
            let command=d::Command::Deploy {config:Box::new(config),identity:PrivateIdentity {
                private_key_nsec:candidate.secret_key().to_bech32().unwrap(),
                auth_tag:Some(buzz_sdk::nip_oa::compute_auth_tag(&owner,&candidate.public_key(),"").unwrap())}};
            let snapshot=serde_json::json!({"record":{"pubkey":candidate.public_key().to_hex(),"model":null}});
            let mut device=Device::at(&path).unwrap(); barrier.wait();
            device.prepare_deployment(&status.scope.community,&owner,authority,&command,&status,snapshot).map(|i|i.protected.authority.agent)
        })
    }).collect();
    let outcomes: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect();
    assert_eq!(outcomes.iter().filter(|result| result.is_ok()).count(), 1);
    let winner = Device::at(&race_path)
        .unwrap()
        .deployment_intents(&scope.community, &owner)
        .unwrap();
    assert_eq!(winner.len(), 1);
    assert_eq!(winner[0].phase, 0);
    assert_eq!(
        winner[0].protected.authority.agent,
        *outcomes
            .iter()
            .find_map(|result| result.as_ref().ok())
            .unwrap()
    );
    cleanup_tests::exercise(
        &mut management,
        &responses,
        &owner,
        &host,
        &first,
        &command,
        &status,
    )
    .await;
    drop(management);
    peer.abort();
    let _ = peer.await;
}

use cleanup_tests::seed_intent;

#[path = "deployment/cleanup_tests.rs"]
mod cleanup_tests;

fn paired_scope(device: &mut Device, owner: &Keys, host: &Keys) -> Scope {
    use buzz_core::paired_host;
    let community = "https://owner.test";
    let t = now().unwrap();
    let (offer, proof) =
        paired_host::Offer::issue(community.to_string(), host.public_key(), t).unwrap();
    let json = serde_json::json!({
        "v": 1,
        "community": community,
        "generation": offer.generation,
        "host": offer.host.to_hex(),
        "expires_at": offer.expires_at,
        "proof": *proof.expose_hex(),
    })
    .to_string();
    let (paired_scope, claim) = device.pair(community, owner, &json).unwrap();
    let verified = paired_host::verify_claim(
        claim.event.get().as_bytes(),
        host,
        &offer,
        Some(owner.public_key()),
        &proof.digest(),
        t,
    )
    .unwrap();
    let ack = crate::Packet {
        event: serde_json::value::to_raw_value(
            &paired_host::build_ack(host, &verified, t).unwrap(),
        )
        .unwrap(),
        route: buzz_core::paired_message::Route {
            r: None,
            prev: Some(id(&claim).unwrap()),
            expires_at: offer.expires_at,
        },
    };
    let binding = crate::Binding {
        generation: paired_scope.generation.clone(),
        owner: owner.public_key().to_hex(),
        host: host.public_key().to_hex(),
        state: "active".into(),
        claim,
        ack,
        inventory: None,
        tombstone: None,
        high_water: None,
    };
    device.restore(community, owner, &binding).unwrap();
    paired_scope
}

#[test]
fn reservation_replay_mismatch_desired_profile_and_tombstone_fencing() {
    let owner = Keys::generate();
    let host = Keys::generate();
    let dir = tempfile::tempdir().unwrap();
    let mut device = Device::at(&dir.path().join("owner.sqlite")).unwrap();
    let scope = paired_scope(&mut device, &owner, &host);
    let other_scope = paired_scope(&mut device, &owner, &Keys::generate());
    let cfg = ReservationConfig {
        instructions: Some("be concise".into()),
        effort: None,
        parallelism: 1,
        respond_to: "owner-only".into(),
        respond_to_allowlist: vec![],
        channel_add_policy: "owner_only".into(),
    };
    let res1 = device
        .reserve_agent(
            &scope.community,
            &owner,
            ReserveAgent {
                scope: &scope,
                runtime_id: "acp/claude",
                name: "  Alpha  ",
                picture: Some("https://example.test/a.png".into()),
                config: cfg.clone(),
            },
        )
        .unwrap();
    assert_eq!(res1.state, ReservationState::Reserved);
    assert_eq!(res1.pending_name.as_deref(), Some("Alpha"));
    assert!(res1.agent_nsec.is_some());

    // Replay with updated pre-attestation config preserves agent key and original picture.
    let mut cfg2 = cfg.clone();
    cfg2.instructions = Some("updated before prepare".into());
    let res_replay = device
        .reserve_agent(
            &scope.community,
            &owner,
            ReserveAgent {
                scope: &scope,
                runtime_id: "acp/claude",
                name: "Alpha",
                picture: Some("https://example.test/other.png".into()),
                config: cfg2.clone(),
            },
        )
        .unwrap();
    assert_eq!(res_replay.agent, res1.agent);
    assert_eq!(
        res_replay.picture.as_deref(),
        Some("https://example.test/a.png")
    );
    assert_eq!(
        res_replay.config.instructions.as_deref(),
        Some("updated before prepare")
    );

    // Host or harness mismatch for same pending_name must be refused.
    assert!(device
        .reserve_agent(
            &scope.community,
            &owner,
            ReserveAgent {
                scope: &other_scope,
                runtime_id: "acp/claude",
                name: "Alpha",
                picture: None,
                config: cfg2.clone(),
            },
        )
        .is_err());
    assert!(device
        .reserve_agent(
            &scope.community,
            &owner,
            ReserveAgent {
                scope: &scope,
                runtime_id: "acp/codex",
                name: "Alpha",
                picture: None,
                config: cfg2.clone(),
            },
        )
        .is_err());

    // Attestation & Prepare freeze non-effort config.
    let auth_tag: Vec<String> =
        serde_json::from_str(&buzz_sdk::nip_oa::compute_auth_tag(&owner, &res1.agent, "").unwrap())
            .unwrap();
    device
        .bind_reservation_attestation(&scope.community, &owner, &res1.agent, auth_tag.clone())
        .unwrap();
    // Second valid randomized-aux_rand NIP-OA signature for same (owner, agent) retains original nip_oa.
    let secp = nostr::secp256k1::Secp256k1::new();
    let kp = nostr::secp256k1::Keypair::from_secret_key(&secp, owner.secret_key());
    let preimage = format!("nostr:agent-auth:{}:", res1.agent.to_hex());
    let digest = <sha2::Sha256 as sha2::Digest>::digest(preimage.as_bytes());
    let msg = nostr::secp256k1::Message::from_digest(digest.into());
    let sig2 = secp.sign_schnorr_with_aux_rand(&msg, &kp, &[0x42; 32]);
    let auth_tag_2 = vec![
        "auth".into(),
        owner.public_key().to_hex(),
        String::new(),
        sig2.to_string(),
    ];
    assert_ne!(auth_tag_2, auth_tag);
    let rebound = device
        .bind_reservation_attestation(&scope.community, &owner, &res1.agent, auth_tag_2)
        .unwrap();
    assert_eq!(rebound.nip_oa.as_ref(), Some(&auth_tag));

    let (status, command, authority, snapshot) = reservation_deploy(&rebound);
    let op_id = authority.operation_id.clone();
    // Save the authentic pre-retention row to reproduce an older journal whose
    // command was committed before its reservation link.
    device
        .db
        .execute_batch("CREATE TEMP TABLE legacy_reservation AS SELECT * FROM agent_reservations")
        .unwrap();
    let first = device
        .prepare_deployment(
            &scope.community,
            &owner,
            authority.clone(),
            &command,
            &status,
            snapshot.clone(),
        )
        .unwrap();
    device
        .db
        .execute_batch("INSERT OR REPLACE INTO agent_reservations SELECT * FROM legacy_reservation")
        .unwrap();
    let mut legacy_competitor = authority.clone();
    legacy_competitor.operation_id = "dc".repeat(32);
    let conflict = device
        .prepare_deployment(
            &scope.community,
            &owner,
            legacy_competitor,
            &command,
            &status,
            snapshot.clone(),
        )
        .err()
        .unwrap();
    assert!(conflict.to_string().contains("earlier deployment"));
    let unchanged = device
        .reservation(&scope.community, &owner, &res1.agent)
        .unwrap()
        .unwrap();
    assert_eq!(unchanged.state, ReservationState::Reserved);
    assert!(unchanged.operation_id.is_none());
    let repaired = device
        .prepare_deployment(
            &scope.community,
            &owner,
            authority.clone(),
            &command,
            &status,
            snapshot.clone(),
        )
        .unwrap();
    assert_eq!(repaired.event, first.event);
    device
        .mark_reservation_prepared(&scope.community, &owner, &res1.agent, &op_id)
        .unwrap();
    assert!(device
        .reserve_agent(
            &scope.community,
            &owner,
            ReserveAgent {
                scope: &scope,
                runtime_id: "acp/claude",
                name: "Alpha",
                picture: None,
                config: cfg.clone(), // differs from cfg2 instructions
            },
        )
        .is_err());

    // Active clears pending_name, allowing a new reservation with the same name later.
    device
        .mark_reservation_active(
            &scope.community,
            &owner,
            &res1.agent,
            &op_id,
            Some(Some("high".into())),
        )
        .unwrap();
    let res_active = device
        .reservation(&scope.community, &owner, &res1.agent)
        .unwrap()
        .unwrap();
    assert_eq!(res_active.state, ReservationState::Active);
    assert!(res_active.pending_name.is_none());
    assert_eq!(res_active.config.effort.as_deref(), Some("high"));

    let res_second = device
        .reserve_agent(
            &scope.community,
            &owner,
            ReserveAgent {
                scope: &scope,
                runtime_id: "acp/claude",
                name: "Alpha",
                picture: None,
                config: cfg.clone(),
            },
        )
        .unwrap();
    assert_ne!(res_second.agent, res1.agent);

    // Desired profile is stored outside Deploy bytes and tracks publication state.
    let updated = device
        .set_desired_profile(
            &scope.community,
            &owner,
            &res1.agent,
            "Alpha Renamed",
            Some("https://example.test/b.png".into()),
        )
        .unwrap();
    assert!(!updated.profile_synced);
    assert_eq!(
        updated.desired_profile.as_ref().unwrap().name,
        "Alpha Renamed"
    );
    let published = device
        .mark_profile_published(&scope.community, &owner, &res1.agent, 1_700_000_000)
        .unwrap();
    assert!(published.profile_synced);
    assert_eq!(published.last_profile_created_at, Some(1_700_000_000));

    // Retiring / Retired clears agent_nsec + nip_oa and fences Deploy.
    device
        .begin_retire_reservation(&scope.community, &owner, &res1.agent)
        .unwrap();
    let dummy_auth = d::Authority {
        v: 1,
        scope: scope.clone(),
        agent: res1.agent,
        action: d::Action::Deploy,
        operation_id: "22".repeat(32),
        expected_revision: 0,
        expires_at: now().unwrap() + 60,
        content_sha256: String::new(),
    };
    assert!(device
        .deployment_open(&scope.community, &owner, &dummy_auth)
        .is_err());
    let retired = device
        .finish_retire_reservation(&scope.community, &owner, &res1.agent, "cleaned", None)
        .unwrap();
    assert_eq!(retired.state, ReservationState::Retired);
    assert!(retired.agent_nsec.is_none());
    assert!(retired.nip_oa.is_none());
    assert!(retired.pending_name.is_none());
    assert_eq!(retired.retired_via.as_deref(), Some("cleaned"));
    assert!(device
        .deployment_open(&scope.community, &owner, &dummy_auth)
        .is_err());
    assert!(device
        .set_desired_profile(&scope.community, &owner, &res1.agent, "Late", None)
        .is_err());
}

fn reservation_deploy(r: &Reservation) -> (d::Status, d::Command, d::Authority, Value) {
    let status = d::Status {
        inventory: None,
        scope: r.scope.clone(),
        availability: d::Availability::Current,
        capability: d::Capability::Supported {
            catalog_revision: "cd".repeat(32),
            runtimes: vec![r.runtime_id.clone()],
            harnesses: [(r.runtime_id.clone(), "claude".into())].into(),
        },
        expires_at: now().unwrap() + 120,
    };
    let config = d::Deploy {
        catalog_revision: "cd".repeat(32),
        runtime_id: r.runtime_id.clone(),
        name: r.name_scope.clone(),
        policy: Policy {
            respond_to: "owner-only".into(),
            respond_to_allowlist: vec![],
            channel_add_policy: "owner_only".into(),
        },
        model: d::ModelSelection::PendingLogin {},
        provider: None,
        system_prompt: r.config.instructions.clone(),
        effort: None,
        parallelism: 1,
    };
    let command = d::Command::Deploy {
        config: Box::new(config),
        identity: PrivateIdentity {
            private_key_nsec: r.agent_nsec.as_ref().unwrap().as_str().into(),
            auth_tag: Some(serde_json::to_string(r.nip_oa.as_ref().unwrap()).unwrap()),
        },
    };
    let authority = d::Authority {
        v: 1,
        scope: r.scope.clone(),
        agent: r.agent,
        action: d::Action::Deploy,
        operation_id: "bc".repeat(32),
        expected_revision: 0,
        expires_at: now().unwrap() + 120,
        content_sha256: String::new(),
    };
    let snapshot = serde_json::json!({
        "record": {
            "pubkey": r.agent.to_hex(),
            "model": null,
            "effort": null
        }
    });
    (status, command, authority, snapshot)
}

#[tokio::test]
async fn continue_deployment_with_effort_updates_effort_field() {
    use d::observation::{LoginState, Models, Observation, ResultBody};
    use futures_util::{SinkExt, StreamExt};
    let owner = Keys::generate();
    let host = Keys::generate();
    let relay = Keys::generate();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("effort.sqlite");
    let mut device = Device::at(&path).unwrap();
    let scope = paired_scope(&mut device, &owner, &host);
    let reservation = device
        .reserve_agent(
            &scope.community,
            &owner,
            ReserveAgent {
                scope: &scope,
                runtime_id: "acp/claude",
                name: "Effort Agent",
                picture: None,
                config: ReservationConfig {
                    instructions: None,
                    effort: None,
                    parallelism: 1,
                    respond_to: "owner-only".into(),
                    respond_to_allowlist: vec![],
                    channel_add_policy: "owner_only".into(),
                },
            },
        )
        .unwrap();
    let agent = reservation.agent;
    let attestation = buzz_sdk::nip_oa::compute_auth_tag(&owner, &agent, "").unwrap();
    let reservation = device
        .bind_reservation_attestation(
            &scope.community,
            &owner,
            &agent,
            serde_json::from_str(&attestation).unwrap(),
        )
        .unwrap();
    let (status, command, authority, snapshot) = reservation_deploy(&reservation);
    let d::Command::Deploy { ref config, .. } = command else {
        panic!("expected deployment");
    };
    let first = device
        .prepare_deployment(
            &scope.community,
            &owner,
            authority.clone(),
            &command,
            &status,
            snapshot.clone(),
        )
        .unwrap();
    let reopen = |device: Device, expected: &Intent, count: usize| {
        drop(device);
        let device = Device::at(&path).unwrap();
        let reservation = device
            .reservation(&scope.community, &owner, &agent)
            .unwrap()
            .unwrap();
        assert_eq!(reservation.state, ReservationState::Prepared);
        assert_eq!(
            reservation.operation_id.as_deref(),
            Some(expected.protected.authority.operation_id.as_str())
        );
        let retained = device
            .deployment_intent(
                &scope.community,
                &owner,
                reservation.operation_id.as_deref().unwrap(),
            )
            .unwrap()
            .unwrap();
        assert_eq!(retained.phase, 0);
        assert_eq!(retained.protected.ciphertext, expected.protected.ciphertext);
        assert_eq!(retained.event, expected.event);
        assert_eq!(
            device
                .deployment_intents(&scope.community, &owner)
                .unwrap()
                .len(),
            count
        );
        device
    };
    device = reopen(device, &first, 1);
    let mut competitor = authority.clone();
    competitor.operation_id = "dc".repeat(32);
    assert!(device
        .prepare_deployment(
            &scope.community,
            &owner,
            competitor,
            &command,
            &status,
            snapshot.clone()
        )
        .is_err());
    let catalog = buzz_core::agent_models::AgentModelsResponse {
        agent_name: "Claude".into(),
        agent_version: "1.0".into(),
        supports_switching: true,
        models: vec![buzz_core::agent_models::AgentModelInfo {
            id: "claude-sonnet".into(),
            name: Some("Claude Sonnet".into()),
            description: None,
        }],
        agent_default_model: None,
        selected_model: None,
    };
    let body = ResultBody {
        result: d::DeploymentResult {
            request: first.protected.authority.clone(),
            outcome: d::Outcome::NeedsLogin,
            applied_revision: Some(1),
        },
        observation: Some(Observation {
            readiness: Some(buzz_core::harness::Readiness::LoggedIn),
            availability: Some(buzz_core::harness::AcpAvailabilityStatus::Available),
            scope: scope.clone(),
            agent,
            runtime_id: config.runtime_id.clone(),
            catalog_revision: config.catalog_revision.clone(),
            operation_id: first.protected.authority.operation_id.clone(),
            revision: 1,
            action_id: "de".repeat(32),
            observed_at: now().unwrap(),
            expires_at: now().unwrap() + 120,
            login: LoginState::NotObserved,
            login_at: None,
            models: Models::Available { catalog },
        }),
    };
    let receipt = wire::result_body(&host, &body, now().unwrap()).unwrap();
    let served = serde_json::json!({
        "event": first.event,
        "result": serde_json::to_string(&receipt).unwrap(),
        "state": "needs-login",
        "unit_id": "ef".repeat(32),
        "revision": 1
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let relay_srv = relay.clone();
    let first_id = first.protected.authority.operation_id.clone();
    let host_key = host.clone();
    let owner_key = owner.public_key();
    let peer = tokio::spawn(async move {
        let mut sent = std::collections::BTreeMap::new();
        let (socket, _) = listener.accept().await.unwrap();
        let mut socket = tokio_tungstenite::accept_async(socket).await.unwrap();
        while let Some(Ok(frame)) = socket.next().await {
            let Ok(text) = frame.to_text() else {
                continue;
            };
            let msg: Value = serde_json::from_str(text).unwrap();
            if msg[0] == "EVENT" {
                let ev: Event = serde_json::from_value(msg[1].clone()).unwrap();
                let raw = serde_json::to_string(&ev).unwrap();
                let (_, wire::Header::Command { authority }) =
                    wire::verify(raw.as_bytes(), "https://owner.test", owner_key).unwrap()
                else {
                    panic!("expected deployment");
                };
                wire::open_command(&host_key, &ev, &authority).unwrap();
                assert_eq!(authority.agent, agent);
                let entry = if authority.operation_id == first_id {
                    assert_eq!(served["event"], raw);
                    served.clone()
                } else {
                    assert_eq!(authority.expected_revision, 1);
                    let receipt = wire::result(
                        &host_key,
                        &d::DeploymentResult {
                            request: authority.clone(),
                            applied_revision: Some(2),
                            outcome: d::Outcome::Applied,
                        },
                        now().unwrap(),
                    )
                    .unwrap();
                    serde_json::json!({"event":raw, "result":serde_json::to_string(&receipt).unwrap(), "state":"applied","unit_id":"ef".repeat(32),"revision":2})
                };
                assert!(
                    sent.insert(authority.operation_id, entry).is_none(),
                    "duplicate host effect"
                );
                socket
                    .send(
                        serde_json::json!(["OK", ev.id, true, ""])
                            .to_string()
                            .into(),
                    )
                    .await
                    .unwrap();
            } else if msg[0] == "REQ" {
                let query: wire::Query =
                    serde_json::from_value(msg[2]["host_deployment"].clone()).unwrap();
                let reply = wire::Reply {
                    query: query.clone(),
                    expires_at: now().unwrap() + wire::PERMIT_SECONDS,
                    data: sent
                        .get(query.operation_id.as_ref().unwrap())
                        .cloned()
                        .unwrap_or_else(|| serde_json::json!({"state":"missing-or-superseded"})),
                };
                let event =
                    wire::reply(&relay_srv, query.scope.owner, &reply, now().unwrap()).unwrap();
                socket
                    .send(tokio_tungstenite::tungstenite::Message::Text(
                        serde_json::json!(["EVENT", msg[1], event])
                            .to_string()
                            .into(),
                    ))
                    .await
                    .unwrap();
                socket
                    .send(tokio_tungstenite::tungstenite::Message::Text(
                        serde_json::json!(["EOSE", msg[1]]).to_string().into(),
                    ))
                    .await
                    .unwrap();
            }
        }
        sent
    });
    let ws = buzz_ws_client::NostrWsConnection::connect(&format!("ws://{addr}"))
        .await
        .unwrap();
    let mut management = Management::from_authenticated(
        true,
        &scope.community,
        owner.public_key(),
        relay.public_key(),
        ws,
    )
    .unwrap();
    let signer = || Ok(owner.clone());
    let dispatch = async |device: &mut Device, management: &mut Management, intent: &Intent| {
        let id = &intent.protected.authority.operation_id;
        for phase in 1..=2 {
            device
                .deployment_advance(&scope.community, &owner, id, phase)
                .unwrap();
        }
        management.dispatch(device, &signer, id).await.unwrap();
    };
    dispatch(&mut device, &mut management, &first).await;
    let mut selected_snapshot = snapshot.clone();
    selected_snapshot["record"]["model"] = "claude-sonnet".into();
    selected_snapshot["record"]["effort"] = "high".into();
    let next = management
        .continue_deployment_with_effort(
            &mut device,
            &signer,
            ContinueDeploy {
                previous: &first.protected.authority.operation_id,
                status: &status,
                model: d::ModelSelection::Selected {
                    id: "claude-sonnet".into(),
                },
                effort: Some(Some("high".into())),
                snapshot: selected_snapshot,
            },
        )
        .await
        .unwrap();
    device = reopen(device, &next, 2);
    dispatch(&mut device, &mut management, &next).await;
    let applied = management
        .operation(&mut device, &signer, &next.protected.authority.operation_id)
        .await
        .unwrap();
    assert!(applied.admitted && applied.current && applied.state == "applied");
    let d::Command::Deploy {
        config: updated_cfg,
        ..
    } = wire::restore_command(
        &owner,
        &next.protected.authority,
        &next.protected.ciphertext,
    )
    .unwrap()
    else {
        panic!("expected deploy");
    };
    assert_eq!(updated_cfg.effort.as_deref(), Some("high"));
    assert_eq!(
        updated_cfg.model,
        d::ModelSelection::Selected {
            id: "claude-sonnet".into()
        }
    );
    drop(management);
    let sent = peer.await.unwrap();
    assert_eq!(sent.len(), 2);
    assert_eq!(
        sent[&next.protected.authority.operation_id]["event"],
        next.event
    );
}
