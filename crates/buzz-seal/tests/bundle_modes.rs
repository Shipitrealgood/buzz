//! Bundle-mode schema tests: the `mode` field's back-compat default (every
//! pre-mode manifest is a sealed B bundle) and the A-mode "manifest-only"
//! structural rules enforced by `bundle::load` + `bundle::validate`.

use std::fs;

use buzz_seal::bundle::{self, BundleMode, Manifest, ManifestConfig, SealedPresence};
use nostr::{JsonUtil, Keys};
use uuid::Uuid;

fn manifest(mode: BundleMode, owner: &Keys, agent: &Keys, channels: Vec<Uuid>) -> Manifest {
    let agent_pubkey = agent.public_key();
    let attestation = buzz_sdk::nip_oa::compute_auth_tag(owner, &agent_pubkey, "").unwrap();
    Manifest {
        v: 1,
        name: "modes".to_owned(),
        agent_pubkey,
        owner_pubkey: owner.public_key(),
        host_pubkey: Keys::generate().public_key(),
        attestation,
        created_at: nostr::Timestamp::now().as_secs(),
        mode,
        config: ManifestConfig {
            respond_to: "owner-only".to_owned(),
            channels,
            harness_cmd: None,
            relay_url: None,
        },
    }
}

#[test]
fn missing_mode_field_deserializes_as_b() {
    let owner = Keys::generate();
    let agent = Keys::generate();
    let mut json: serde_json::Value =
        serde_json::to_value(manifest(BundleMode::B, &owner, &agent, vec![])).unwrap();
    // Simulate a pre-mode manifest (T3/T4 era): no mode field at all.
    json.as_object_mut().unwrap().remove("mode");
    let parsed: Manifest = serde_json::from_value(json).unwrap();
    assert_eq!(
        parsed.mode,
        BundleMode::B,
        "pre-mode manifests are sealed bundles"
    );
}

#[test]
fn mode_round_trips_as_lowercase_letter() {
    let owner = Keys::generate();
    let agent = Keys::generate();
    let json = serde_json::to_value(manifest(BundleMode::A, &owner, &agent, vec![])).unwrap();
    assert_eq!(json["mode"], "a");
    let json = serde_json::to_value(manifest(BundleMode::B, &owner, &agent, vec![])).unwrap();
    assert_eq!(json["mode"], "b");
}

#[test]
fn a_mode_manifest_only_bundle_is_valid_even_without_channel_events() {
    let owner = Keys::generate();
    let agent = Keys::generate();
    let chan = Uuid::new_v4();
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("manifest.json"),
        serde_json::to_string(&manifest(BundleMode::A, &owner, &agent, vec![chan])).unwrap(),
    )
    .unwrap();

    // SealedPresence::Required must not apply to A-mode (there is no seal).
    let loaded = bundle::load(tmp.path(), SealedPresence::Required).unwrap();
    assert!(loaded.envelope.is_none());
    let problems = bundle::validate(&loaded);
    assert!(
        problems.is_empty(),
        "manifest-only A bundle with channels must validate: {problems:?}"
    );
}

#[test]
fn a_mode_bundle_with_sealed_json_is_flagged() {
    let owner = Keys::generate();
    let agent = Keys::generate();
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("manifest.json"),
        serde_json::to_string(&manifest(BundleMode::A, &owner, &agent, vec![])).unwrap(),
    )
    .unwrap();
    fs::write(tmp.path().join("sealed.json"), "not even json").unwrap();

    // Load must not try to parse it (A-mode never seals) — it records the
    // mode violation instead of erroring on garbage content.
    let loaded = bundle::load(tmp.path(), SealedPresence::Optional).unwrap();
    let problems = bundle::validate(&loaded);
    assert!(
        problems.iter().any(|p| p.contains("A-mode")),
        "stray sealed.json must be a mode violation: {problems:?}"
    );
}

#[test]
fn a_mode_bundle_with_events_is_flagged() {
    let owner = Keys::generate();
    let agent = Keys::generate();
    let chan = Uuid::new_v4();
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("manifest.json"),
        serde_json::to_string(&manifest(BundleMode::A, &owner, &agent, vec![chan])).unwrap(),
    )
    .unwrap();
    let event = buzz_sdk::builders::build_add_member(
        chan,
        &agent.public_key().to_hex(),
        Some(buzz_sdk::MemberRole::Bot),
    )
    .unwrap()
    .sign_with_keys(&owner)
    .unwrap();
    fs::create_dir_all(tmp.path().join("events")).unwrap();
    fs::write(
        tmp.path().join("events").join(format!("9000-{chan}.json")),
        event.as_json(),
    )
    .unwrap();

    let loaded = bundle::load(tmp.path(), SealedPresence::Optional).unwrap();
    let problems = bundle::validate(&loaded);
    assert!(
        problems
            .iter()
            .any(|p| p.contains("A-mode") && p.contains("9000")),
        "stray events must be a mode violation: {problems:?}"
    );
}

#[test]
fn b_mode_still_requires_one_event_per_manifest_channel() {
    let owner = Keys::generate();
    let agent = Keys::generate();
    let chan = Uuid::new_v4();
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("manifest.json"),
        serde_json::to_string(&manifest(BundleMode::B, &owner, &agent, vec![chan])).unwrap(),
    )
    .unwrap();

    let loaded = bundle::load(tmp.path(), SealedPresence::Optional).unwrap();
    let problems = bundle::validate(&loaded);
    assert!(
        problems
            .iter()
            .any(|p| p.contains(&format!("missing events/9000-{chan}.json"))),
        "B-mode regression: per-channel event requirement must hold: {problems:?}"
    );
}
