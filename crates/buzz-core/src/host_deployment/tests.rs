use super::*;
use crate::host_registry::object;
use nostr::Keys;

fn scope() -> Scope {
    Scope {
        community: "https://fixture.test".into(),
        owner: Keys::generate().public_key(),
        host: Keys::generate().public_key(),
        generation: "aa".repeat(32),
    }
}
fn authority(scope: &Scope) -> Authority {
    Authority {
        v: 1,
        scope: scope.clone(),
        agent: Keys::generate().public_key(),
        action: Action::Deploy,
        operation_id: "bb".repeat(32),
        expected_revision: 0,
        expires_at: 200,
        content_sha256: hex::encode(Sha256::digest(b"encrypted")),
    }
}
#[test]
fn authority_roundtrip_and_each_binding_rejects_mismatch() {
    let s = scope();
    let a = authority(&s);
    let raw = serde_json::to_vec(&a).unwrap();
    let decoded: Authority = object(&raw, 4096).unwrap();
    assert_eq!(decoded, a);
    assert!(decoded.validate(&s, 0, 100, b"encrypted").is_ok());
    for foreign in [
        Scope {
            community: "https://other.test".into(),
            ..s.clone()
        },
        Scope {
            owner: Keys::generate().public_key(),
            ..s.clone()
        },
        Scope {
            host: Keys::generate().public_key(),
            ..s.clone()
        },
        Scope {
            generation: "cc".repeat(32),
            ..s.clone()
        },
    ] {
        assert!(a.validate(&foreign, 0, 100, b"encrypted").is_err());
    }
    for (rev, now, bytes) in [
        (1, 100, b"encrypted".as_slice()),
        (0, 200, b"encrypted"),
        (0, 100, b"changed"),
        (0, 100, b""),
    ] {
        assert!(a.validate(&s, rev, now, bytes).is_err());
    }
    for bad in [
        Authority { v: 2, ..a.clone() },
        Authority {
            operation_id: "AA".repeat(32),
            ..a.clone()
        },
        Authority {
            agent: s.owner,
            ..a.clone()
        },
        Authority {
            agent: s.host,
            ..a.clone()
        },
        Authority {
            expires_at: 701,
            ..a.clone()
        },
    ] {
        assert!(bad.validate(&s, 0, 100, b"encrypted").is_err());
    }
    let duplicate = String::from_utf8(raw)
        .unwrap()
        .replacen('{', "{\"v\":1,", 1);
    assert!(object::<Authority>(duplicate.as_bytes(), 4096).is_err());
}
#[test]
fn registration_unsupported_empty_stale_revoked_never_permit_deploy() {
    let s = scope();
    let supported = Capability::Supported {
        harnesses: Default::default(),
        catalog_revision: "dd".repeat(32),
        runtimes: vec!["fixture/acp".into()],
    };
    for capability in [
        Capability::RegistrationOnly {},
        Capability::Unsupported {},
        Capability::Supported {
            harnesses: Default::default(),
            catalog_revision: "dd".repeat(32),
            runtimes: vec![],
        },
        supported.clone(),
    ] {
        for availability in [
            Availability::Unknown,
            Availability::Unavailable,
            Availability::Stale,
            Availability::Revoked,
            Availability::Current,
        ] {
            let status = Status {
                inventory: None,
                scope: s.clone(),
                capability: capability.clone(),
                availability: availability.clone(),
                expires_at: 200,
            };
            let expected = capability == supported && availability == Availability::Current;
            assert_eq!(
                status
                    .permits(&s, &"dd".repeat(32), "fixture/acp", 100)
                    .is_ok(),
                expected
            );
            assert!(status
                .permits(&s, &"dd".repeat(32), "fixture/acp", 200)
                .is_err());
            assert!(status
                .permits(&s, &"ee".repeat(32), "fixture/acp", 100)
                .is_err());
            assert!(status
                .permits(&scope(), &"dd".repeat(32), "fixture/acp", 100)
                .is_err());
        }
    }
    for runtimes in [
        vec!["x".into(), "x".into()],
        vec!["../bin".into()],
        vec!["x".into(); 65],
    ] {
        assert!(Capability::Supported {
            harnesses: Default::default(),
            catalog_revision: "dd".repeat(32),
            runtimes
        }
        .validate()
        .is_err());
    }
}
#[test]
fn closed_config_destination_intent_and_cleanup_results() {
    let s = scope();
    let a = authority(&s);
    let config = deploy();
    assert!(config.validate().is_ok());
    let mut raw = serde_json::to_value(&config).unwrap();
    raw["command"] = "/bin/sh".into();
    assert!(serde_json::from_value::<Deploy>(raw).is_err());
    for bad in [
        Deploy {
            parallelism: 0,
            ..config.clone()
        },
        Deploy {
            runtime_id: "../shell".into(),
            ..config.clone()
        },
        Deploy {
            model: ModelSelection::Selected { id: "".into() },
            ..config.clone()
        },
    ] {
        assert!(bad.validate().is_err());
    }
    let destination = Destination::RegisteredHost { scope: s.clone() };
    assert_eq!(
        serde_json::from_slice::<Destination>(&serde_json::to_vec(&destination).unwrap()).unwrap(),
        destination
    );
    for raw in [
        r#"{"action":"login"}"#,
        r#"{"action":"cleanup","command":"rm"}"#,
    ] {
        assert!(serde_json::from_str::<Command>(raw).is_err());
    }
    let intent = ProtectedCreateIntent {
        authority: a.clone(),
        ciphertext: b"encrypted".to_vec(),
    };
    assert!(intent.validate(&s, 100).is_ok());
    let roundtrip: ProtectedCreateIntent =
        serde_json::from_slice(&serde_json::to_vec(&intent).unwrap()).unwrap();
    assert!(roundtrip == intent);
    assert!(ProtectedCreateIntent {
        ciphertext: vec![],
        ..intent
    }
    .validate(&s, 100)
    .is_err());
    for action in [Action::Deploy, Action::Cleanup] {
        let request = Authority {
            action,
            ..a.clone()
        };
        for outcome in [
            Outcome::Pending,
            Outcome::NeedsLogin,
            Outcome::Applied,
            Outcome::CleanupPending,
            Outcome::Cleaned,
            Outcome::Failed,
        ] {
            let applied = matches!(
                outcome,
                Outcome::NeedsLogin | Outcome::Applied | Outcome::Cleaned
            );
            let expected = !matches!(
                (action, &outcome),
                (Action::Deploy, Outcome::CleanupPending | Outcome::Cleaned)
                    | (Action::Cleanup, Outcome::NeedsLogin | Outcome::Applied)
            );
            let result = DeploymentResult {
                request: request.clone(),
                outcome,
                applied_revision: applied.then_some(1),
            };
            assert_eq!(result.validate(&request).is_ok(), expected);
            assert!(result
                .validate(&Authority {
                    operation_id: "ff".repeat(32),
                    ..request.clone()
                })
                .is_err());
        }
    }
}

#[test]
fn encrypted_command_identity_and_action_cannot_be_substituted() {
    let s = scope();
    let key = Keys::generate();
    let a = Authority {
        agent: key.public_key(),
        ..authority(&s)
    };
    use nostr::ToBech32;
    let owner = Keys::generate();
    let a = Authority {
        scope: Scope {
            owner: owner.public_key(),
            ..s
        },
        ..a
    };
    let digest =
        Sha256::digest(format!("nostr:agent-auth:{}:", key.public_key().to_hex()).as_bytes());
    let signature = owner.sign_schnorr(&nostr::secp256k1::Message::from_digest(digest.into()));
    let identity = PrivateIdentity {
        private_key_nsec: key.secret_key().to_bech32().unwrap(),
        auth_tag: Some(
            serde_json::json!([
                "auth",
                owner.public_key().to_hex(),
                "",
                signature.to_string()
            ])
            .to_string(),
        ),
    };
    let command = Command::Deploy {
        config: Box::new(deploy()),
        identity,
    };
    assert!(command.validate(&a).is_ok());
    let encoded = serde_json::to_vec(&command).unwrap();
    assert!(serde_json::from_slice::<Command>(&encoded)
        .unwrap()
        .validate(&a)
        .is_ok());
    assert!(command.validate(&authority(&a.scope)).is_err());
    assert!(command
        .validate(&Authority {
            scope: scope(),
            ..a.clone()
        })
        .is_err());
    assert!(command
        .validate(&Authority {
            action: Action::Cleanup,
            ..a.clone()
        })
        .is_err());
    assert!(Command::Cleanup {}.validate(&a).is_err());
    assert!(Command::Cleanup {}
        .validate(&Authority {
            action: Action::Cleanup,
            ..a
        })
        .is_ok());
}

#[test]
fn empty_variants_remain_closed_and_overflow_cannot_look_applied() {
    for raw in [
        r#"{"state":"registration-only","runtimes":[]}"#,
        r#"{"state":"unsupported","command":"sh"}"#,
    ] {
        assert!(serde_json::from_str::<Capability>(raw).is_err());
    }
    assert!(
        serde_json::from_str::<ModelSelection>(r#"{"state":"pending-login","id":"made-up"}"#)
            .is_err()
    );
    let a = Authority {
        expected_revision: u64::MAX,
        ..authority(&scope())
    };
    assert!(DeploymentResult {
        request: a.clone(),
        outcome: Outcome::Applied,
        applied_revision: None
    }
    .validate(&a)
    .is_err());
}

fn deploy() -> Deploy {
    Deploy {
        catalog_revision: "dd".repeat(32),
        runtime_id: "fixture/acp".into(),
        name: "Fixture".into(),
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
    }
}
