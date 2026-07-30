//! The eight test cases from `docs/sealed-mint-bundle-spec.md` §Test spec,
//! implemented against the seal/unseal pair. Provision-side halves (byte-
//! exact credential file, full re-provision idempotence) live in T4; here
//! each case exercises the library boundary that produces/consumes blobs.

use std::collections::BTreeMap;

use buzz_sdk::nip_oa::compute_auth_tag;
use buzz_seal::{
    is_canonical_64_hex, seal, unseal_at, AgentSecretKey, Envelope, InnerPayload, SealError,
    SecretString, UnsealError, UnsealWarning, ENVELOPE_TYPE, STALENESS_WARN_SECS,
};
use nostr::nips::nip44;
use nostr::{Keys, PublicKey};
use zeroize::Zeroizing;

const NOW: u64 = 1_800_000_000;

struct Fixture {
    owner: Keys,
    host: Keys,
    agent: Keys,
    inner: InnerPayload,
    envelope: Envelope,
    agent_sk_hex: Zeroizing<String>,
}

fn fixture_with_api_keys(api_keys: Option<BTreeMap<String, SecretString>>) -> Fixture {
    let owner = Keys::generate();
    let host = Keys::generate();
    let agent = Keys::generate();
    let attestation = compute_auth_tag(&owner, &agent.public_key(), "").unwrap();
    let inner = InnerPayload {
        v: 1,
        agent_sk: AgentSecretKey::from_keys(&agent).unwrap(),
        agent_pubkey: agent.public_key(),
        attestation,
        host_pubkey: host.public_key(),
        created_at: NOW - 60,
        api_keys,
    };
    let envelope = seal(&owner, &host.public_key(), &inner).unwrap();
    let agent_sk_hex = Zeroizing::new(agent.secret_key().to_secret_hex());
    Fixture {
        owner,
        host,
        agent,
        inner,
        envelope,
        agent_sk_hex,
    }
}

fn fixture() -> Fixture {
    let mut api_keys = BTreeMap::new();
    api_keys.insert(
        "FAKE_PROVIDER_KEY".to_owned(),
        SecretString::new("sk-test-a1b2c3d4e5".to_owned()),
    );
    fixture_with_api_keys(Some(api_keys))
}

/// Re-seal arbitrary plaintext into an otherwise well-formed envelope,
/// bypassing `seal()`'s consistency refusals (the adversary's tool).
fn raw_envelope(
    owner: &Keys,
    host_pk: &PublicKey,
    agent_pk: PublicKey,
    plaintext: &str,
) -> Envelope {
    let ciphertext =
        nip44::encrypt(owner.secret_key(), host_pk, plaintext, nip44::Version::V2).unwrap();
    Envelope {
        v: 1,
        bundle_type: ENVELOPE_TYPE.to_owned(),
        owner_pubkey: owner.public_key(),
        host_pubkey: *host_pk,
        agent_pubkey: agent_pk,
        ciphertext,
    }
}

/// Assert no 16-char window of a 64-char secret appears in any surface.
fn assert_no_secret_leak(secret_hex: &str, surfaces: &[&str]) {
    assert_eq!(secret_hex.len(), 64);
    for start in 0..=(64 - 16) {
        let window = &secret_hex[start..start + 16];
        for surface in surfaces {
            assert!(
                !surface.contains(window),
                "secret substring leaked (window at offset {start})"
            );
        }
    }
}

// ── Case 1: round-trip, byte-exact ──────────────────────────────────────────

#[test]
fn case1_round_trip_byte_exact() {
    let f = fixture();
    let pins = [f.owner.public_key()];
    let out = unseal_at(&f.host, &f.envelope, &pins, NOW).expect("round-trip must unseal");

    let sk = out.inner.agent_sk.expose_canonical_hex();
    assert_eq!(sk.len(), 64, "no trailing byte");
    assert!(is_canonical_64_hex(sk.as_bytes()));
    assert_eq!(
        sk,
        f.agent_sk_hex.as_str(),
        "byte-identical to generated sk"
    );

    assert_eq!(out.inner.agent_pubkey, f.agent.public_key());
    assert_eq!(out.inner.host_pubkey, f.host.public_key());
    assert_eq!(out.inner.attestation, f.inner.attestation);
    assert_eq!(out.inner.created_at, f.inner.created_at);
    let keys = out.inner.api_keys.as_ref().expect("api_keys round-trip");
    assert_eq!(
        keys.get("FAKE_PROVIDER_KEY").unwrap().expose(),
        "sk-test-a1b2c3d4e5"
    );
    assert!(out.warnings.is_empty(), "clean bundle warns nothing");
}

// ── Case 2: wrong host key ⇒ step-2 reject, distinct error ──────────────────

#[test]
fn case2_wrong_host_key_rejects_at_step2() {
    let f = fixture();
    let other_host = Keys::generate();
    // The envelope routing hint would already catch it (step 1)…
    let err = unseal_at(&other_host, &f.envelope, &[f.owner.public_key()], NOW).unwrap_err();
    assert!(matches!(err, UnsealError::EnvelopeHostMismatch { .. }));

    // …so also rewrite the (unauthenticated) hint to target the other host:
    // decryption itself must fail on the MAC.
    let mut spliced = f.envelope.clone();
    spliced.host_pubkey = other_host.public_key();
    let err = unseal_at(&other_host, &spliced, &[f.owner.public_key()], NOW).unwrap_err();
    assert!(matches!(err, UnsealError::Decrypt), "got: {err}");
}

// ── Case 3: splice, both directions ─────────────────────────────────────────

#[test]
fn case3a_envelope_owner_swap_rejects_at_step2_mac() {
    let f = fixture();
    let mallory = Keys::generate();
    let mut spliced = f.envelope.clone();
    spliced.owner_pubkey = mallory.public_key();
    let err = unseal_at(&f.host, &spliced, &[mallory.public_key()], NOW).unwrap_err();
    assert!(matches!(err, UnsealError::Decrypt), "got: {err}");
}

#[test]
fn case3b_inner_host_mismatch_rejects_at_step3() {
    // Simulate a re-seal for another host with the same inner: the inner
    // payload still says host B, but the ciphertext/envelope target host A.
    let f = fixture();
    let host_b = Keys::generate();
    let mut inner = f.inner.clone();
    inner.host_pubkey = host_b.public_key();
    let plaintext = serde_json::to_string(&inner).unwrap();
    let env = raw_envelope(
        &f.owner,
        &f.host.public_key(),
        inner.agent_pubkey,
        &plaintext,
    );
    let err = unseal_at(&f.host, &env, &[f.owner.public_key()], NOW).unwrap_err();
    assert!(matches!(err, UnsealError::InnerHostMismatch), "got: {err}");
}

// ── Case 4: attestation for a different agent ⇒ step-4 reject ───────────────

#[test]
fn case4_foreign_agent_attestation_rejects_at_step4() {
    let f = fixture();
    let other_agent = Keys::generate();
    let mut inner = f.inner.clone();
    inner.attestation = compute_auth_tag(&f.owner, &other_agent.public_key(), "").unwrap();
    let plaintext = serde_json::to_string(&inner).unwrap();
    let env = raw_envelope(
        &f.owner,
        &f.host.public_key(),
        inner.agent_pubkey,
        &plaintext,
    );
    let err = unseal_at(&f.host, &env, &[f.owner.public_key()], NOW).unwrap_err();
    assert!(
        matches!(err, UnsealError::AttestationInvalid(_)),
        "got: {err}"
    );

    // seal() must refuse to produce this blob in the first place.
    let err = seal(&f.owner, &f.host.public_key(), &inner).unwrap_err();
    assert!(matches!(err, SealError::AttestationInvalid(_)));
}

/// Step-4's second half: a valid foreign attestation WRAPPED by a different
/// sealer — attester != NIP-44 sender must reject even when the signature
/// itself verifies.
#[test]
fn case4b_wrapped_foreign_attestation_rejects_attester_not_sender() {
    let f = fixture();
    let mallory = Keys::generate();
    // Mallory re-wraps the owner's (public) attestation + a keypair she
    // controls? She cannot know agent_sk — so simulate the strongest variant:
    // she mints her OWN agent but embeds the real owner's attestation for it.
    // She cannot forge that signature, so instead she wraps HER OWN agent with
    // her own attestation but claims the envelope sender is her — consistent —
    // which case 5 catches. The remaining wrap-splice: valid owner attestation
    // inside, but Mallory as the NIP-44 sender.
    let attestation = compute_auth_tag(&f.owner, &f.agent.public_key(), "").unwrap();
    let mut inner = f.inner.clone();
    inner.attestation = attestation;
    let plaintext = serde_json::to_string(&inner).unwrap();
    // Mallory seals it herself (she obtained the plaintext some other way in
    // this hypothetical; the check must still bind sealer == attester).
    let env = raw_envelope(
        &mallory,
        &f.host.public_key(),
        inner.agent_pubkey,
        &plaintext,
    );
    let err = unseal_at(
        &f.host,
        &env,
        &[f.owner.public_key(), mallory.public_key()],
        NOW,
    )
    .unwrap_err();
    assert!(
        matches!(err, UnsealError::AttesterNotSender { .. }),
        "got: {err}"
    );
}

// ── Case 5: self-consistent rogue owner vs pinning ──────────────────────────

#[test]
fn case5_rogue_owner_rejected_unless_explicitly_trusted() {
    let real_owner = Keys::generate();
    let host = Keys::generate();
    let rogue = Keys::generate();
    let rogue_agent = Keys::generate();
    let inner = InnerPayload {
        v: 1,
        agent_sk: AgentSecretKey::from_keys(&rogue_agent).unwrap(),
        agent_pubkey: rogue_agent.public_key(),
        attestation: compute_auth_tag(&rogue, &rogue_agent.public_key(), "").unwrap(),
        host_pubkey: host.public_key(),
        created_at: NOW,
        api_keys: None,
    };
    // Fully self-consistent — seal() happily produces it for the rogue.
    let env = seal(&rogue, &host.public_key(), &inner).unwrap();

    // Pinned to the real owner ⇒ step-5 reject.
    let err = unseal_at(&host, &env, &[real_owner.public_key()], NOW).unwrap_err();
    assert!(
        matches!(err, UnsealError::OwnerNotPinned { .. }),
        "got: {err}"
    );

    // Empty pin list ⇒ fail-closed reject too.
    let err = unseal_at(&host, &env, &[], NOW).unwrap_err();
    assert!(matches!(err, UnsealError::OwnerNotPinned { .. }));

    // Explicit trust extension ⇒ accepted.
    let pins = [real_owner.public_key(), rogue.public_key()];
    assert!(unseal_at(&host, &env, &pins, NOW).is_ok());
}

// ── Case 6: malformed agent_sk at seal AND unseal ────────────────────────────

#[test]
fn case6_malformed_sk_rejected_at_seal_time_by_type() {
    let good = "f5c79eab2f44284ff278780c34aab621aafb149be4d7dba8f9625bfd99419733";
    // Canonical passes.
    assert!(AgentSecretKey::from_canonical_hex(good).is_ok());
    // Uppercase.
    let upper = good.to_uppercase();
    assert!(matches!(
        AgentSecretKey::from_canonical_hex(&upper).unwrap_err(),
        SealError::NonCanonicalSecretKey { len: 64 }
    ));
    // 65 chars.
    let long = format!("{good}0");
    assert!(AgentSecretKey::from_canonical_hex(&long).is_err());
    // Trailing newline / CRLF (the T2-preflight M5 class).
    assert!(AgentSecretKey::from_canonical_hex(&format!("{good}\n")).is_err());
    assert!(AgentSecretKey::from_canonical_hex(&format!("{good}\r\n")).is_err());
    // nsec bech32.
    assert!(AgentSecretKey::from_canonical_hex(
        "nsec1vl029mgpspedva04g90vltkh6fvh240zqtv9k0t9af8935ke9laqsnlfe5"
    )
    .is_err());
    // All-zero (not a valid scalar) — canonical shape, invalid curve point.
    let zeros = "0".repeat(64);
    assert!(matches!(
        AgentSecretKey::from_canonical_hex(&zeros).unwrap_err(),
        SealError::InvalidSecretKey
    ));
}

#[test]
fn case6_malformed_sk_rejected_at_unseal_defense_in_depth() {
    let f = fixture();
    for bad_sk in [
        "F5C79EAB2F44284FF278780C34AAB621AAFB149BE4D7DBA8F9625BFD99419733".to_owned(),
        format!("{}0", f.agent_sk_hex.as_str()),
        format!("{}\\n", f.agent_sk_hex.as_str()), // JSON-escaped newline inside the string
        "nsec1vl029mgpspedva04g90vltkh6fvh240zqtv9k0t9af8935ke9laqsnlfe5".to_owned(),
    ] {
        // Hand-build the plaintext so the malformed sk actually ships.
        let plaintext = format!(
            r#"{{"v":1,"agent_sk":"{bad_sk}","agent_pubkey":"{}","attestation":{},"host_pubkey":"{}","created_at":{}}}"#,
            f.agent.public_key().to_hex(),
            serde_json::to_string(&f.inner.attestation).unwrap(),
            f.host.public_key().to_hex(),
            NOW,
        );
        let env = raw_envelope(
            &f.owner,
            &f.host.public_key(),
            f.agent.public_key(),
            &plaintext,
        );
        let err = unseal_at(&f.host, &env, &[f.owner.public_key()], NOW).unwrap_err();
        assert!(matches!(err, UnsealError::InnerParse { .. }), "got: {err}");
        // The malformed secret must not be echoed by the parse error.
        let rendered = err.to_string();
        assert!(
            !rendered.contains(&bad_sk[..16]),
            "parse error echoed secret material"
        );
    }
}

// ── Case 7: idempotent re-parse ──────────────────────────────────────────────

#[test]
fn case7_unseal_twice_converges() {
    let f = fixture();
    let pins = [f.owner.public_key()];
    let a = unseal_at(&f.host, &f.envelope, &pins, NOW).unwrap();
    let b = unseal_at(&f.host, &f.envelope, &pins, NOW).unwrap();
    assert_eq!(a.inner.agent_sk, b.inner.agent_sk);
    assert_eq!(a.inner.agent_pubkey, b.inner.agent_pubkey);
    assert_eq!(a.inner.attestation, b.inner.attestation);
    assert_eq!(a.inner.host_pubkey, b.inner.host_pubkey);
    assert_eq!(a.inner.created_at, b.inner.created_at);
    assert_eq!(a.inner.api_keys, b.inner.api_keys);
    assert_eq!(a.warnings, b.warnings);

    // Envelope JSON round-trip re-parses to the same unseal result.
    let json = serde_json::to_string(&f.envelope).unwrap();
    let reparsed: Envelope = serde_json::from_str(&json).unwrap();
    let c = unseal_at(&f.host, &reparsed, &pins, NOW).unwrap();
    assert_eq!(a.inner.agent_sk, c.inner.agent_sk);
}

// ── Case 8: no secret in Debug/Display/errors/serialized-envelope ───────────

#[test]
fn case8_no_secret_on_any_formatting_surface() {
    let f = fixture();
    let sk = f.agent_sk_hex.as_str();
    let api_key_value = "sk-test-a1b2c3d4e5";

    // Redacted Debug on the secret newtypes.
    let dbg_sk = format!("{:?}", f.inner.agent_sk);
    assert_eq!(dbg_sk, "AgentSecretKey(<redacted>)");
    let dbg_api = format!(
        "{:?}",
        f.inner
            .api_keys
            .as_ref()
            .unwrap()
            .get("FAKE_PROVIDER_KEY")
            .unwrap()
    );
    assert_eq!(dbg_api, "SecretString(<redacted>)");

    // The public envelope (Debug + serialized JSON) carries no secret.
    let mut surfaces: Vec<String> = vec![
        format!("{:?}", f.envelope),
        serde_json::to_string(&f.envelope).unwrap(),
        dbg_sk,
        dbg_api,
    ];

    // Every reachable error variant's Display.
    let other_host = Keys::generate();
    let mallory = Keys::generate();
    let mut wrong_type = f.envelope.clone();
    wrong_type.bundle_type = "bogus".into();
    let mut wrong_v = f.envelope.clone();
    wrong_v.v = 9;
    let mut owner_swap = f.envelope.clone();
    owner_swap.owner_pubkey = mallory.public_key();
    let errs: Vec<String> = [
        unseal_at(&f.host, &wrong_v, &[f.owner.public_key()], NOW).unwrap_err(),
        unseal_at(&f.host, &wrong_type, &[f.owner.public_key()], NOW).unwrap_err(),
        unseal_at(&other_host, &f.envelope, &[f.owner.public_key()], NOW).unwrap_err(),
        unseal_at(&f.host, &owner_swap, &[f.owner.public_key()], NOW).unwrap_err(),
        unseal_at(&f.host, &f.envelope, &[], NOW).unwrap_err(),
    ]
    .iter()
    .map(|e| format!("{e} / {e:?}"))
    .collect();
    surfaces.extend(errs);

    // Warning Display surfaces.
    let mut hint_swap = f.envelope.clone();
    hint_swap.agent_pubkey = mallory.public_key();
    let out = unseal_at(&f.host, &hint_swap, &[f.owner.public_key()], NOW).unwrap();
    assert!(matches!(
        out.warnings[0],
        UnsealWarning::EnvelopeAgentPubkeyMismatch { .. }
    ));
    let stale = unseal_at(
        &f.host,
        &f.envelope,
        &[f.owner.public_key()],
        NOW + STALENESS_WARN_SECS + 120,
    )
    .unwrap();
    assert!(matches!(stale.warnings[0], UnsealWarning::Stale { .. }));
    for w in out.warnings.iter().chain(stale.warnings.iter()) {
        surfaces.push(format!("{w} / {w:?}"));
    }

    let refs: Vec<&str> = surfaces.iter().map(String::as_str).collect();
    assert_no_secret_leak(sk, &refs);
    assert_no_secret_leak(f.owner.secret_key().to_secret_hex().as_str(), &refs);
    for s in &refs {
        assert!(!s.contains(api_key_value), "api key leaked");
    }
}

// ── Envelope hint mismatch: inner wins, agent named by inner ────────────────

#[test]
fn envelope_agent_hint_mismatch_warns_and_inner_wins() {
    let f = fixture_with_api_keys(None);
    let mut hint_swap = f.envelope.clone();
    hint_swap.agent_pubkey = Keys::generate().public_key();
    let out = unseal_at(&f.host, &hint_swap, &[f.owner.public_key()], NOW).unwrap();
    assert_eq!(out.inner.agent_pubkey, f.agent.public_key(), "inner wins");
    assert_eq!(out.warnings.len(), 1);
}

// ── seal() refusals beyond case 4 ────────────────────────────────────────────

#[test]
fn seal_refuses_host_and_pubkey_inconsistency() {
    let f = fixture_with_api_keys(None);
    // Host mismatch between arg and inner.
    let other_host = Keys::generate();
    let err = seal(&f.owner, &other_host.public_key(), &f.inner).unwrap_err();
    assert!(matches!(err, SealError::HostMismatch));

    // agent_pubkey not derived from agent_sk.
    let mut inner = f.inner.clone();
    inner.agent_pubkey = Keys::generate().public_key();
    let err = seal(&f.owner, &f.host.public_key(), &inner).unwrap_err();
    assert!(matches!(err, SealError::AgentPubkeyMismatch));

    // Sealer is not the attester.
    let mallory = Keys::generate();
    let err = seal(&mallory, &f.host.public_key(), &f.inner).unwrap_err();
    assert!(matches!(err, SealError::AttesterMismatch));

    // Wrong version.
    let mut inner = f.inner.clone();
    inner.v = 2;
    let err = seal(&f.owner, &f.host.public_key(), &inner).unwrap_err();
    assert!(matches!(err, SealError::Version { got: 2 }));
}
