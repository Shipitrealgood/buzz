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
