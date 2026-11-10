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
    let sent = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Value>::new()));
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
                published.lock().unwrap().push(msg[1].clone());
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
                            serde_json::json!(["CLOSED", msg[1], "restricted: read denied"])
                                .to_string()
                                .into(),
                        ))
                        .await
                        .unwrap();
                    continue;
                }
                let query: wire::Query =
                    serde_json::from_value(msg[2]["host_deployment"].clone()).unwrap();
                let reply = wire::Reply {
                    query: query.clone(),
                    expires_at: now().unwrap() + wire::PERMIT_SECONDS,
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
        assert_eq!(*event, serde_json::from_str::<Value>(&first.event).unwrap());
    }
    *responses.lock().unwrap() = serde_json::json!({"denied":true});
    assert!(management
        .operation(&mut device, &signer, &authority.operation_id)
        .await
        .is_err());
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
