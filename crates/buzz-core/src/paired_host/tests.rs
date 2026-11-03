use super::*;
use nostr::SecretKey;
use serde_json::{json, Value};

const NOW: u64 = 1_800_000_000;
const ORIGIN: &str = "https://buzz.example";

fn key(scalar: u8) -> Keys {
    let mut bytes = [0; 32];
    bytes[31] = scalar;
    Keys::new(SecretKey::from_slice(&bytes).unwrap())
}

fn setup() -> (Keys, Keys, Offer, Proof) {
    let owner = key(1);
    let host = key(2);
    let offer = Offer {
        host: host.public_key(),
        community: ORIGIN.into(),
        generation: "ab".repeat(32),
        expires_at: NOW + 120,
    };
    (
        owner,
        host,
        offer,
        Proof::from_hex(&"cd".repeat(32)).unwrap(),
    )
}

fn raw(e: &Event) -> Vec<u8> {
    serde_json::to_vec(e).unwrap()
}

fn decrypted(e: &Event, recipient: &Keys) -> String {
    nip44::decrypt(recipient.secret_key(), &e.pubkey, &e.content).unwrap()
}

fn signed(keys: &Keys, kind: Kind, time: u64, tags: Tags, content: &str) -> Event {
    EventBuilder::new(kind, content)
        .tags(tags)
        .custom_created_at(Timestamp::from(time))
        .sign_with_keys(keys)
        .unwrap()
}

fn encrypted(sender: &Keys, recipient: PublicKey, plaintext: &str) -> String {
    nip44::encrypt(sender.secret_key(), &recipient, plaintext, Version::V2).unwrap()
}

fn fixture(json: &str, recipient: &Keys) -> Event {
    let vector: Value = serde_json::from_str(json).unwrap();
    let event = parse(&serde_json::to_vec(&vector["event"]).unwrap()).unwrap();
    let plaintext: Value = serde_json::from_str(&decrypted(&event, recipient)).unwrap();
    assert_eq!(plaintext, vector["plaintext"]);
    event
}

#[test]
fn shared_vectors_and_fresh_exchange_bind_one_exact_claim() {
    let (owner, host, offer, proof) = setup();
    let vector = fixture(
        include_str!("../../../../test-fixtures/paired-host/v1/claim.json"),
        &host,
    );
    let ack = fixture(
        include_str!("../../../../test-fixtures/paired-host/v1/ack.json"),
        &owner,
    );
    let verified = verify_claim(&raw(&vector), &host, &offer, None, &proof.digest(), NOW).unwrap();
    assert_eq!(verified.owner(), owner.public_key());
    assert_eq!(verified.id(), vector.id);
    #[derive(Deserialize)]
    struct RestoreCase {
        expected_claim_id: EventId,
        now: u64,
        result: String,
    }
    let cases: Vec<RestoreCase> = serde_json::from_str(include_str!(
        "../../../../test-fixtures/paired-host/v1/restore.json"
    ))
    .unwrap();
    for case in cases {
        let restored = restore_claim(
            &raw(&vector),
            &owner,
            &offer,
            case.expected_claim_id,
            case.now,
        );
        let expected = match case.result.as_str() {
            "fresh" => Acknowledgement::Fresh(ack.id),
            "historical" => Acknowledgement::Historical(ack.id),
            "invalid-claim" => {
                assert_eq!(restored.unwrap_err(), Error::Message);
                continue;
            }
            _ => panic!("unknown fixture outcome"),
        };
        let saved = restored.unwrap();
        assert_eq!(saved.id(), vector.id);
        assert_eq!(
            verify_ack(&raw(&ack), &owner, &saved, case.now).unwrap(),
            expected
        );
    }

    let (fresh, saved) = build_claim(&owner, &offer, &proof, NOW).unwrap();
    let verified = verify_claim(
        &raw(&fresh),
        &host,
        &offer,
        Some(owner.public_key()),
        &proof.digest(),
        NOW,
    )
    .unwrap();
    let receipt = build_ack(&host, &verified, NOW + 1).unwrap();
    assert_eq!(
        verify_ack(&raw(&receipt), &owner, &saved, NOW + 1).unwrap(),
        Acknowledgement::Fresh(receipt.id)
    );
    assert_eq!(
        verify_ack(&raw(&ack), &owner, &saved, NOW + 1),
        Err(Error::Message)
    );

    let (next, next_proof) = Offer::issue(ORIGIN.into(), host.public_key(), NOW).unwrap();
    assert_eq!(next.expires_at, NOW + TTL);
    assert_ne!(next.generation.as_str(), next_proof.expose_hex().as_str());
    assert_eq!(
        verify_claim(&raw(&fresh), &host, &next, None, &next_proof.digest(), NOW).unwrap_err(),
        Error::Message
    );
}

#[test]
fn author_recipient_tags_signature_and_ciphertext_are_all_checked() {
    let (owner, host, offer, proof) = setup();
    let (claim, saved) = build_claim(&owner, &offer, &proof, NOW).unwrap();
    let verified = verify_claim(&raw(&claim), &host, &offer, None, &proof.digest(), NOW).unwrap();
    let ack = build_ack(&host, &verified, NOW).unwrap();
    let foreign = key(3);
    let receive = |raw: &[u8]| {
        verify_claim(
            raw,
            &host,
            &offer,
            Some(owner.public_key()),
            &proof.digest(),
            NOW,
        )
    };
    assert!(verify_claim(&raw(&claim), &foreign, &offer, None, &proof.digest(), NOW).is_err());
    assert!(verify_ack(&raw(&ack), &foreign, &saved, NOW).is_err());
    assert!(build_ack(&foreign, &verified, NOW).is_err());
    // A valid NIP-OA delegation to another signer grants no enrollment authority.
    let delegation = Sha256::digest(format!(
        "nostr:agent-auth:{}:",
        foreign.public_key().to_hex()
    ));
    let attestation =
        owner.sign_schnorr(&nostr::secp256k1::Message::from_digest(delegation.into()));
    let (delegated_claim, _) = build_claim(&foreign, &offer, &proof, NOW).unwrap();
    assert!(receive(&raw(&delegated_claim)).is_err());
    let mut delegated_tags = delegated_claim.tags.clone();
    delegated_tags.push(
        Tag::parse([
            "auth",
            &owner.public_key().to_hex(),
            "",
            &attestation.to_string(),
        ])
        .unwrap(),
    );
    let delegated = signed(
        &foreign,
        KIND,
        NOW,
        delegated_tags,
        &delegated_claim.content,
    );
    assert!(receive(&raw(&delegated)).is_err());
    assert!(verify_claim(&raw(&delegated), &host, &offer, None, &proof.digest(), NOW).is_err());
    let tags: Vec<_> = claim.tags.iter().cloned().collect();
    let mut alternatives = vec![tags[..3].to_vec()];
    for change in [[0, 1], [1, 2], [2, 3]] {
        let mut changed = tags.clone();
        changed.swap(change[0], change[1]);
        alternatives.push(changed);
    }
    for (index, fields) in [
        (0, vec!["p", &foreign.public_key().to_hex()]),
        (1, vec!["community", "https://foreign.example"]),
        (2, vec!["g", &"ce".repeat(32)]),
        (3, vec!["op", "pair-ack"]),
        (3, vec!["op", "pair-claim", "extra"]),
    ] {
        let mut changed = tags.clone();
        changed[index] = Tag::parse(fields).unwrap();
        alternatives.push(changed);
    }
    for extra in [
        tags[0].clone(),
        Tag::parse(["auth", "NIP-OA delegation"]).unwrap(),
    ] {
        let mut changed = tags.clone();
        changed.push(extra);
        alternatives.push(changed);
    }
    for changed in alternatives {
        assert!(receive(&raw(&signed(
            &owner,
            KIND,
            NOW,
            Tags::from_list(changed),
            &claim.content
        )))
        .is_err());
    }
    let bad_cipher = encrypted(&owner, foreign.public_key(), &decrypted(&claim, &host));
    for invalid in [
        signed(&foreign, KIND, NOW, claim.tags.clone(), &claim.content),
        signed(
            &owner,
            Kind::TextNote,
            NOW,
            claim.tags.clone(),
            &claim.content,
        ),
        signed(&owner, KIND, NOW, claim.tags.clone(), &bad_cipher),
        signed(&owner, KIND, NOW, claim.tags.clone(), "not a ciphertext"),
    ] {
        assert!(receive(&raw(&invalid)).is_err());
    }

    let value = serde_json::to_value(&claim).unwrap();
    for (field, replacement) in [
        ("sig", json!("0".repeat(128))),
        ("id", json!("0".repeat(64))),
        ("id", json!(claim.id.to_hex().to_uppercase())),
        ("pubkey", json!(owner.public_key().to_hex().to_uppercase())),
        ("content", json!(ack.content)),
        ("auth", json!(foreign.public_key().to_hex())),
    ] {
        let mut changed = value.clone();
        changed[field] = replacement;
        assert!(receive(&serde_json::to_vec(&changed).unwrap()).is_err());
    }
    let duplicate = serde_json::to_string(&value)
        .unwrap()
        .replacen('{', "{\"kind\":43102,", 1);
    assert!(receive(duplicate.as_bytes()).is_err());
    let array = json!([
        value["id"],
        value["pubkey"],
        value["created_at"],
        value["kind"],
        value["tags"],
        value["content"],
        value["sig"]
    ]);
    assert!(receive(&serde_json::to_vec(&array).unwrap()).is_err());
    assert!(receive(&vec![b' '; MAX_EVENT_BYTES + 1]).is_err());
}

#[test]
fn signed_plaintext_requires_exact_typed_object_and_scope() {
    let (owner, host, offer, proof) = setup();
    let (claim, _) = build_claim(&owner, &offer, &proof, NOW).unwrap();
    let plain = decrypted(&claim, &host);
    let check = |s: &str| {
        let ciphertext = encrypted(&owner, host.public_key(), s);
        let bad = signed(&owner, KIND, NOW, claim.tags.clone(), &ciphertext);
        verify_claim(&raw(&bad), &host, &offer, None, &proof.digest(), NOW).unwrap_err()
    };
    let value: Value = serde_json::from_str(&plain).unwrap();
    for (field, replacement) in [
        ("v", json!("1")),
        ("v", json!(2)),
        ("op", json!("pair-ack")),
        ("community", json!("https://foreign.example")),
        ("host", json!(owner.public_key().to_hex())),
        ("owner", json!(key(3).public_key().to_hex())),
        ("generation", json!(offer.generation.to_uppercase())),
        ("expires_at", json!(offer.expires_at + 1)),
        ("proof", Value::Null),
        ("proof", json!("CD".repeat(32))),
        ("proof", json!("aa")),
        ("claim_id", Value::Null),
        ("claim_id", json!(claim.id.to_hex())),
        ("auth", json!("NIP-OA")),
    ] {
        let mut changed = value.clone();
        changed[field] = replacement;
        assert_eq!(check(&changed.to_string()), Error::Message, "{field}");
    }
    for field in value.as_object().unwrap().keys() {
        let mut changed = value.clone();
        changed.as_object_mut().unwrap().remove(field);
        assert_eq!(check(&changed.to_string()), Error::Message);
        let dup = format!("{{{}:{},{}", json!(field), value[field], &plain[1..]);
        assert_eq!(check(&dup), Error::Message);
    }
    let array = json!([
        1,
        "pair-claim",
        ORIGIN,
        offer.generation,
        owner.public_key().to_hex(),
        offer.host.to_hex(),
        offer.expires_at,
        value["proof"]
    ]);
    for bad in [
        array.to_string(),
        format!("{}{}", " ".repeat(MAX_PLAINTEXT_BYTES), plain),
        "{".into(),
    ] {
        assert_eq!(check(&bad), Error::Message);
    }
    let mut wrong = value;
    wrong["proof"] = json!("ee".repeat(32));
    assert!(matches!(check(&wrong.to_string()), Error::Proof(_)));
    let public = String::from_utf8(raw(&claim)).unwrap();
    let mut logs = format!("{proof:?}");
    for error in [
        Error::Create,
        Error::Enrollment,
        Error::Proof(claim.id),
        Error::Time,
        Error::Message,
    ] {
        logs.push_str(&format!(" {error:?} {error}"));
    }
    assert!(!public.contains(proof.expose_hex().as_str()));
    assert!(!logs.contains(proof.expose_hex().as_str()));
}

#[test]
fn origin_peer_and_freshness_boundaries() {
    let (owner, host, offer, proof) = setup();
    for origin in [
        "http://buzz.example",
        "wss://buzz.example",
        "https://BUZZ.example",
        "https://buzz.example/",
        "https://buzz.example:443",
        "https://me@buzz.example",
        "https://buzz.example?a",
        "https://buzz.example#frag",
        "https://buzz.example.",
        "https://my_host.example",
        "https://127.0.0.1",
        "https://[::1]",
        "https://-bad.example",
    ] {
        assert!(!canonical_origin(origin), "{origin}");
    }
    assert!(canonical_origin("https://buzz.example:9443"));
    let self_offer = Offer {
        host: owner.public_key(),
        ..offer.clone()
    };
    assert!(build_claim(&owner, &self_offer, &proof, NOW).is_err());
    let invalid_host = PublicKey::from_hex(&"ff".repeat(32)).unwrap();
    assert!(Offer::issue(ORIGIN.into(), invalid_host, NOW).is_err());
    let (claim, saved) = build_claim(&owner, &offer, &proof, NOW).unwrap();
    let check_time = |created: u64, now| {
        let event = signed(&owner, KIND, created, claim.tags.clone(), &claim.content);
        verify_claim(&raw(&event), &host, &offer, None, &proof.digest(), now)
    };
    for (created, now) in [
        (offer.expires_at, offer.expires_at - 1),
        (NOW, offer.expires_at),
        (NOW + 31, NOW),
        (offer.expires_at - 631, NOW),
        (offer.expires_at - 630, offer.expires_at - 631),
    ] {
        assert_eq!(check_time(created, now).unwrap_err(), Error::Time);
    }
    assert!(check_time(NOW + 30, NOW).is_ok());
    assert!(check_time(offer.expires_at - 630, offer.expires_at - 630).is_ok());
    let verified =
        verify_claim(&raw(&claim), &host, &offer, None, &proof.digest(), NOW - 30).unwrap();
    let ack = build_ack(&host, &verified, NOW - 30).unwrap();
    assert!(verify_ack(&raw(&ack), &owner, &saved, NOW).is_ok());
    let early = signed(&host, KIND, NOW - 31, ack.tags.clone(), &ack.content);
    assert_eq!(
        verify_ack(&raw(&early), &owner, &saved, NOW),
        Err(Error::Time)
    );
    assert_eq!(
        verify_ack(&raw(&ack), &owner, &saved, offer.expires_at),
        Ok(Acknowledgement::Historical(ack.id))
    );
}

#[test]
fn owner_restores_original_ciphertext_and_classifies_expired_transcript() {
    let (owner, host, offer, proof) = setup();
    let other = key(3);
    let (claim, saved) = build_claim(&owner, &offer, &proof, NOW).unwrap();
    let accepted = verify_claim(
        &raw(&claim),
        &host,
        &offer,
        Some(owner.public_key()),
        &proof.digest(),
        NOW,
    )
    .unwrap();
    let ack = build_ack(&host, &accepted, NOW + 2).unwrap();
    let expected_id = saved.id();
    drop(saved);

    let restore = |event: &Event, signer: &Keys, scope: &Offer, id: EventId, now| {
        restore_claim(&raw(event), signer, scope, id, now)
    };
    let restored = restore(&claim, &owner, &offer, expected_id, NOW + 10).unwrap();
    assert_eq!(
        verify_ack(&raw(&ack), &owner, &restored, NOW + 11),
        Ok(Acknowledgement::Fresh(ack.id))
    );
    let late = offer.expires_at + 86_400;
    let restored = restore(&claim, &owner, &offer, expected_id, late).unwrap();
    assert_eq!(
        verify_ack(&raw(&ack), &owner, &restored, late),
        Ok(Acknowledgement::Historical(ack.id))
    );
    assert_eq!(
        verify_ack(&raw(&ack), &owner, &restored, NOW + 15),
        Err(Error::Time)
    );
    assert!(verify_claim(&raw(&claim), &host, &offer, None, &proof.digest(), late).is_err());
    assert!(build_ack(&host, &accepted, late).is_err());

    let (replacement, _) = build_claim(&owner, &offer, &proof, NOW).unwrap();
    assert!(restore(&replacement, &owner, &offer, expected_id, NOW).is_err());
    assert_eq!(
        verify_ack(
            &raw(&ack),
            &owner,
            &restore(&replacement, &owner, &offer, replacement.id, late).unwrap(),
            late
        ),
        Err(Error::Message)
    );
    assert!(restore(&claim, &other, &offer, expected_id, NOW).is_err());
    assert!(restore(
        &claim,
        &owner,
        &Offer {
            generation: "fa".repeat(32),
            ..offer.clone()
        },
        expected_id,
        NOW
    )
    .is_err());
    assert!(restore(
        &claim,
        &owner,
        &Offer {
            expires_at: offer.expires_at + 1,
            ..offer.clone()
        },
        expected_id,
        NOW
    )
    .is_err());

    // Even when given its ID, an owner-signed impostor with the wrong peer,
    // wrong inner scope, or malformed proof must not create a saved claim.
    let plain = decrypted(&claim, &host);
    let wrong_peer = encrypted(&owner, other.public_key(), &plain);
    let mut inner: Value = serde_json::from_str(&plain).unwrap();
    inner["owner"] = json!(other.public_key().to_hex());
    let wrong_scope = encrypted(&owner, host.public_key(), &inner.to_string());
    inner["owner"] = json!(owner.public_key().to_hex());
    inner["proof"] = json!("invalid");
    let bad_proof = encrypted(&owner, host.public_key(), &inner.to_string());
    for content in [wrong_peer, wrong_scope, bad_proof] {
        let bad = signed(&owner, KIND, NOW, claim.tags.clone(), &content);
        assert!(restore(&bad, &owner, &offer, bad.id, late).is_err());
    }
    let mut bad = claim.clone();
    bad.sig = ack.sig;
    assert!(restore(&bad, &owner, &offer, expected_id, late).is_err());
    let bad_author = signed(&other, KIND, NOW, claim.tags.clone(), &claim.content);
    assert!(restore(&bad_author, &owner, &offer, bad_author.id, late).is_err());
}

#[test]
fn only_scoped_authenticated_wrong_proofs_supply_a_stable_attempt_id() {
    let (owner, host, offer, proof) = setup();
    let wrong = Proof::from_hex(&"ee".repeat(32)).unwrap();
    let mut distinct = std::collections::HashSet::new();
    for _ in 0..5 {
        let (claim, _) = build_claim(&owner, &offer, &wrong, NOW).unwrap();
        for _ in 0..5 {
            let err =
                verify_claim(&raw(&claim), &host, &offer, None, &proof.digest(), NOW).unwrap_err();
            assert_eq!(err, Error::Proof(claim.id));
            let Error::Proof(id) = err else {
                unreachable!()
            };
            distinct.insert(id);
        }
        let mut tampered = claim;
        tampered.content.push('A');
        assert_eq!(
            verify_claim(&raw(&tampered), &host, &offer, None, &proof.digest(), NOW).unwrap_err(),
            Error::Message
        );
    }
    assert_eq!(distinct.len(), 5);
}

#[test]
fn transport_requires_exact_outer_and_http_signer() {
    let (owner, host, offer, proof) = setup();
    let (claim, _) = build_claim(&owner, &offer, &proof, NOW).unwrap();
    let verify =
        |bytes: &[u8], signer| transport::verify_outer(bytes, ORIGIN, &offer.generation, signer);
    assert_eq!(
        verify(&raw(&claim), owner.public_key()).unwrap().1,
        host.public_key()
    );
    assert!(verify(&raw(&claim), host.public_key()).is_err());
    let clear = signed(&owner, KIND, NOW, claim.tags.clone(), "plaintext");
    assert!(verify(&raw(&clear), owner.public_key()).is_err());
    let mut tags = claim.tags.clone();
    tags.push(Tag::parse(["op", "pair-claim"]).unwrap());
    let duplicate = signed(&owner, KIND, NOW, tags, &claim.content);
    assert!(verify(&raw(&duplicate), owner.public_key()).is_err());
}
