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
