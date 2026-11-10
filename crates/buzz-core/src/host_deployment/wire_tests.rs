use super::{wire::*, *};
use nostr::{Event, Keys};

fn input() -> (Keys, Keys, Authority) {
    let owner = Keys::generate();
    let host = Keys::generate();
    let authority = Authority {
        v: 1,
        scope: Scope {
            community: "https://fixture.test".into(),
            owner: owner.public_key(),
            host: host.public_key(),
            generation: "aa".repeat(32),
        },
        agent: Keys::generate().public_key(),
        action: Action::Cleanup,
        operation_id: "bb".repeat(32),
        expected_revision: 1,
        expires_at: 200,
        content_sha256: String::new(),
    };
    (owner, host, authority)
}
#[test]
fn exact_signed_encrypted_command_and_native_restore() {
    let (owner, host, a) = input();
    let (a, e) = seal_command(&owner, a, &Command::Cleanup {}, 100).unwrap();
    let raw = serde_json::to_vec(&e).unwrap();
    assert!(verify(&raw, &a.scope.community, owner.public_key()).is_ok());
    assert!(matches!(
        open_command(&host, &e, &a).unwrap(),
        Command::Cleanup {}
    ));
    assert!(matches!(
        restore_command(&owner, &a, e.content.as_bytes()).unwrap(),
        Command::Cleanup {}
    ));
    assert!(open_command(&owner, &e, &a).is_err());
    assert!(restore_command(&host, &a, e.content.as_bytes()).is_err());
    assert_eq!(
        sign_command(&owner, &a, e.content.as_bytes(), 100)
            .unwrap()
            .id,
        e.id
    );
    assert!(verify(&raw, "https://other.test", owner.public_key()).is_err());
    assert!(verify(&raw, &a.scope.community, host.public_key()).is_err());
}
#[test]
fn altered_signature_ciphertext_and_duplicate_wire_fields_fail() {
    let (owner, _, a) = input();
    let (_, e) = seal_command(&owner, a.clone(), &Command::Cleanup {}, 100).unwrap();
    let raw = serde_json::to_string(&e).unwrap();
    for field in ["id", "sig", "content", "pubkey"] {
        let mut v = serde_json::to_value(&e).unwrap();
        let s = v[field].as_str().unwrap();
        v[field] = format!("{}{}", if &s[..1] == "a" { "b" } else { "a" }, &s[1..]).into();
        assert!(
            verify(
                &serde_json::to_vec(&v).unwrap(),
                &a.scope.community,
                owner.public_key()
            )
            .is_err(),
            "{field}"
        );
    }
    let dup = raw.replacen('{', "{\"kind\":43104,", 1);
    assert!(verify(dup.as_bytes(), &a.scope.community, owner.public_key()).is_err());
    assert!(sign_command(&owner, &a, b"changed", 100).is_err());
}
#[test]
fn result_binding_and_permit_challenge_are_not_interchangeable() {
    let (owner, host, mut a) = input();
    let (signed, _) = seal_command(&owner, a.clone(), &Command::Cleanup {}, 100).unwrap();
    a = signed;
    let r = DeploymentResult {
        request: a.clone(),
        outcome: Outcome::Cleaned,
        applied_revision: Some(2),
    };
    let e = result(&host, &r, 110).unwrap();
    assert_eq!(open_result(&owner, &e, &a).unwrap(), r);
    a.operation_id = "cc".repeat(32);
    assert!(open_result(&owner, &e, &a).is_err());
    let relay = Keys::generate();
    let q = Query {
        v: 1,
        scope: a.scope.clone(),
        view: "permit".into(),
        nonce: "dd".repeat(32),
        operation_id: Some(a.operation_id.clone()),
    };
    let response = Reply {
        query: q.clone(),
        expires_at: 120,
        data: serde_json::json!({"authority":a,"unit_id":"bb".repeat(32),"revision":2}),
    };
    let permit = reply(&relay, host.public_key(), &response, 110).unwrap();
    assert!(open_reply(&host, relay.public_key(), &q, &permit, 110).is_ok());
    assert!(open_reply(&host, relay.public_key(), &q, &permit, 120).is_err());
    let mut foreign = q.clone();
    foreign.nonce = "ee".repeat(32);
    assert!(open_reply(&host, relay.public_key(), &foreign, &permit, 110).is_err());
    assert!(open_reply(&owner, relay.public_key(), &q, &permit, 110).is_err());
    assert!(open_reply(&host, owner.public_key(), &q, &permit, 110).is_err());
}
#[test]
fn status_is_host_signed_scoped_and_expires() {
    let (owner, host, a) = input();
    let s = Status {
        scope: a.scope,
        availability: Availability::Current,
        capability: Capability::Supported {
            harnesses: Default::default(),
            catalog_revision: "ff".repeat(32),
            runtimes: vec![],
        },
        expires_at: 120,
    };
    let e = status(&host, &s, 100).unwrap();
    assert_eq!(open_status(&owner, &e, &s.scope, 100).unwrap(), s);
    assert!(open_status(&owner, &e, &s.scope, 120).is_err());
    assert!(status(&owner, &s, 100).is_err());
    let mut other = s.scope.clone();
    other.generation = "ab".repeat(32);
    assert!(open_status(&owner, &e, &other, 100).is_err());
    let mut tampered: Event = e;
    tampered.content.push('a');
    assert!(open_status(&owner, &tampered, &s.scope, 100).is_err());
}

#[test]
fn bounded_operation_with_receipt_fits_encrypted_query_and_rejects_padded_results() {
    let (owner, host, mut a) = input();
    let content = "A".repeat(29 * 1024);
    use sha2::{Digest, Sha256};
    a.content_sha256 = hex::encode(Sha256::digest(content.as_bytes()));
    let command = sign_command(&owner, &a, content.as_bytes(), 100).unwrap();
    let receipt = result(
        &host,
        &DeploymentResult {
            request: a.clone(),
            outcome: Outcome::Cleaned,
            applied_revision: Some(2),
        },
        110,
    )
    .unwrap();
    let q = Query {
        v: 1,
        scope: a.scope.clone(),
        view: "operation".into(),
        nonce: "ef".repeat(32),
        operation_id: Some(a.operation_id.clone()),
    };
    let r = Reply {
        query: q.clone(),
        expires_at: 120,
        data: serde_json::json!({"event":serde_json::to_string(&command).unwrap(),"result":serde_json::to_string(&receipt).unwrap(),"unit_id":a.operation_id,"revision":2}),
    };
    let relay = Keys::generate();
    let envelope = reply(&relay, owner.public_key(), &r, 110).unwrap();
    assert!(serde_json::to_vec(&envelope).unwrap().len() < MAX_FRAME);
    assert_eq!(
        open_reply(&owner, relay.public_key(), &q, &envelope, 110)
            .unwrap()
            .data,
        r.data
    );
    let padded = nostr::EventBuilder::new(receipt.kind, "A".repeat(4097))
        .tags(receipt.tags.iter().cloned())
        .custom_created_at(receipt.created_at)
        .sign_with_keys(&host)
        .unwrap();
    assert!(verify(
        &serde_json::to_vec(&padded).unwrap(),
        &a.scope.community,
        host.public_key()
    )
    .is_err());
}

#[test]
fn cleanup_namespace_cannot_be_reused_for_deploy_or_foreign_cleanup() {
    let (owner, _, a) = input();
    let (a, e) = seal_command(&owner, a, &Command::Cleanup {}, 100).unwrap();
    assert!(a.operation_id.starts_with(CLEANUP_OPERATION_PREFIX));
    assert_eq!(a.operation_id.len(), 64);
    for case in ["deploy", "foreign-agent", "revision", "expiry", "created"] {
        let mut changed = a.clone();
        let mut created = 100;
        match case {
            "deploy" => changed.action = Action::Deploy,
            "foreign-agent" => changed.agent = Keys::generate().public_key(),
            "revision" => changed.expected_revision += 1,
            "expiry" => changed.expires_at += 1,
            "created" => created += 1,
            _ => unreachable!(),
        }
        let header = serde_json::to_string(&Header::Command { authority: changed }).unwrap();
        let forged = nostr::EventBuilder::new(e.kind, e.content.clone())
            .tags([
                nostr::Tag::parse(["deployment", &header]).unwrap(),
                nostr::Tag::parse(["p", &a.scope.host.to_hex()]).unwrap(),
            ])
            .custom_created_at(nostr::Timestamp::from(created))
            .sign_with_keys(&owner)
            .unwrap();
        forged.verify().unwrap();
        assert!(
            verify(
                &serde_json::to_vec(&forged).unwrap(),
                &a.scope.community,
                owner.public_key()
            )
            .is_err(),
            "{case}"
        );
    }
    let mut normal = a.clone();
    normal.operation_id = "ab".repeat(32);
    for action in [Action::Deploy, Action::Cleanup] {
        normal.action = action;
        assert!(sign_command(&owner, &normal, e.content.as_bytes(), 100).is_ok());
    }
}

#[test]
fn private_observation_is_bound_bounded_historical_and_legacy_compatible() {
    use super::observation::*;
    let (owner, host, mut a) = input();
    a.action = Action::Deploy;
    a.content_sha256 = "ab".repeat(32);
    let result = DeploymentResult {
        request: a.clone(),
        outcome: Outcome::NeedsLogin,
        applied_revision: Some(2),
    };
    let o = Observation {
        scope: a.scope.clone(),
        agent: a.agent,
        runtime_id: "native/grok".into(),
        catalog_revision: "cd".repeat(32),
        operation_id: a.operation_id.clone(),
        revision: 2,
        action_id: "ef".repeat(32),
        observed_at: 110,
        expires_at: 230,
        login: LoginState::NotObserved,
        login_at: None,
        models: Models::Available {
            catalog: crate::agent_models::normalize_agent_models(
                &serde_json::json!({
                    "agent":{"name":"synthetic","version":"1"},
                    "stable":{"configOptions":[{"category":"model","currentValue":"private-model-sentinel","options":[{"value":"private-model-sentinel","name":"Private model"}]}]}
                }),
                None,
            ),
        },
    };
    let body = ResultBody {
        result: result.clone(),
        observation: Some(o.clone()),
    };
    let event = result_body(&host, &body, 110).unwrap();
    assert!(event.content.len() <= 4096);
    assert!(!serde_json::to_string(&event)
        .unwrap()
        .contains("private-model-sentinel"));
    assert_eq!(open_result_body(&owner, &event, &a).unwrap(), body);
    assert_eq!(open_result(&owner, &event, &a).unwrap(), result);
    assert!(open_result_body(&host, &event, &a).is_err());
    let legacy = super::wire::result(&host, &result, 110).unwrap();
    assert_eq!(
        open_result_body(&owner, &legacy, &a).unwrap().observation,
        None
    );
    let config = Deploy {
        catalog_revision: o.catalog_revision.clone(),
        runtime_id: o.runtime_id.clone(),
        name: "test".into(),
        policy: Policy {
            respond_to: "owner-only".into(),
            respond_to_allowlist: vec![],
            channel_add_policy: "owner_only".into(),
        },
        model: ModelSelection::PendingLogin {},
        provider: None,
        system_prompt: None,
        effort: None,
        parallelism: 1,
    };
    assert!(o.current(&result, &config, 110).is_ok());
    assert!(o.current(&result, &config, 230).is_err());
    assert!(o.current(&result, &config, 109).is_err());
    for field in [
        "scope",
        "agent",
        "operation",
        "revision",
        "expiry",
        "login",
        "oversize",
    ] {
        let mut wrong = o.clone();
        match field {
            "scope" => wrong.scope.generation = "01".repeat(32),
            "agent" => wrong.agent = Keys::generate().public_key(),
            "operation" => wrong.operation_id = "02".repeat(32),
            "revision" => wrong.revision += 1,
            "expiry" => wrong.expires_at += 1,
            "login" => wrong.login = LoginState::Succeeded,
            "oversize" => {
                if let Models::Available { catalog } = &mut wrong.models {
                    catalog.models = (0..64)
                        .map(|i| crate::agent_models::AgentModelInfo {
                            id: format!("{i}{}", "x".repeat(100)),
                            name: None,
                            description: None,
                        })
                        .collect();
                }
            }
            _ => unreachable!(),
        }
        assert!(
            result_body(
                &host,
                &ResultBody {
                    result: result.clone(),
                    observation: Some(wrong)
                },
                110
            )
            .is_err(),
            "{field}"
        );
    }
    let mut wrong_config = config;
    wrong_config.catalog_revision = "03".repeat(32);
    assert!(o.current(&result, &wrong_config, 110).is_err());
    let wrong_header = DeploymentResult {
        outcome: Outcome::Applied,
        ..result.clone()
    };
    let forged = nostr::EventBuilder::new(event.kind, event.content.clone())
        .tags([
            nostr::Tag::parse([
                "deployment",
                &serde_json::to_string(&Header::Result {
                    result: wrong_header,
                })
                .unwrap(),
            ])
            .unwrap(),
            nostr::Tag::parse(["p", &owner.public_key().to_hex()]).unwrap(),
        ])
        .custom_created_at(event.created_at)
        .sign_with_keys(&host)
        .unwrap();
    assert!(open_result_body(&owner, &forged, &a).is_err());
}
