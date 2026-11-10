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
            authority,
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
            scope: scope.clone(),
            agent: agent.public_key(),
            runtime_id: config.runtime_id.clone(),
            catalog_revision: config.catalog_revision.clone(),
            operation_id: intent.protected.authority.operation_id.clone(),
            revision: intent.protected.authority.expected_revision + 1,
            action_id: "de".repeat(32),
            observed_at: now().unwrap(),
            expires_at: now().unwrap() + 120,
            login: LoginState::Succeeded,
            login_at: Some(now().unwrap()),
            models: Models::Available {
                catalog: catalog.clone(),
            },
        }),
    };
    let mut initial_body = body_for(&first);
    let initial_observation = initial_body.observation.as_mut().unwrap();
    initial_observation.login = LoginState::NotObserved;
    initial_observation.login_at = None;
    initial_observation.models = Models::Unavailable {
        reason: d::observation::UnavailableReason::NetworkDisabled,
    };
    let receipt = wire::result_body(&host, &initial_body, now().unwrap()).unwrap();
    let current = serde_json::json!({"event":first.event,"result":serde_json::to_string(&receipt).unwrap(),"state":"needs-login","unit_id":"ef".repeat(32),"revision":1});
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let responses = std::sync::Arc::new(std::sync::Mutex::new(current));
    let served = responses.clone();
    let relay_key = relay.public_key();
    let peer = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut socket = tokio_tungstenite::accept_async(socket).await.unwrap();
        while let Some(Ok(frame)) = socket.next().await {
            let Ok(text) = frame.to_text() else {
                continue;
            };
            let msg: Value = serde_json::from_str(text).unwrap();
            if msg[0] == "REQ" {
                let query: wire::Query =
                    serde_json::from_value(msg[2]["host_deployment"].clone()).unwrap();
                let reply = wire::Reply {
                    query: query.clone(),
                    expires_at: now().unwrap() + wire::PERMIT_SECONDS,
                    data: served.lock().unwrap().clone(),
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
    drop(management);
    peer.abort();
    let _ = peer.await;
}
