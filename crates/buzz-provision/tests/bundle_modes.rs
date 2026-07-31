//! Bundle-mode schema tests: the `mode` field's back-compat default (every
//! pre-mode manifest is a sealed B bundle — refused at load) and the
//! "manifest-only" structural rules enforced by `bundle::load` +
//! `bundle::validate`.

use std::fs;

use buzz_provision::bundle::{self, BundleError, BundleMode, Manifest, ManifestConfig};
use nostr::Keys;
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
    // Simulate a pre-mode manifest (sealed era): no mode field at all.
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
fn manifest_only_bundle_is_valid() {
    let owner = Keys::generate();
    let agent = Keys::generate();
    let chan = Uuid::new_v4();
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("manifest.json"),
        serde_json::to_string(&manifest(BundleMode::A, &owner, &agent, vec![chan])).unwrap(),
    )
    .unwrap();

    let loaded = bundle::load(tmp.path()).unwrap();
    let problems = bundle::validate(&loaded);
    assert!(
        problems.is_empty(),
        "manifest-only bundle with channels must validate: {problems:?}"
    );
}

#[test]
fn sealed_b_mode_bundle_is_refused_at_load() {
    let owner = Keys::generate();
    let agent = Keys::generate();
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("manifest.json"),
        serde_json::to_string(&manifest(BundleMode::B, &owner, &agent, vec![])).unwrap(),
    )
    .unwrap();

    let err = bundle::load(tmp.path()).unwrap_err();
    assert!(
        matches!(err, BundleError::SealedModeRemoved { .. }),
        "sealed-era bundles must be refused with the removal message, got: {err}"
    );
    assert!(err.to_string().contains("buzz-mint attest"));
}

#[test]
fn stray_sealed_json_is_flagged() {
    let owner = Keys::generate();
    let agent = Keys::generate();
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("manifest.json"),
        serde_json::to_string(&manifest(BundleMode::A, &owner, &agent, vec![])).unwrap(),
    )
    .unwrap();
    fs::write(tmp.path().join("sealed.json"), "not even json").unwrap();

    // Load must not try to parse it — it records the ceremony violation
    // instead of erroring on garbage content.
    let loaded = bundle::load(tmp.path()).unwrap();
    let problems = bundle::validate(&loaded);
    assert!(
        problems.iter().any(|p| p.contains("sealed.json")),
        "stray sealed.json must be a ceremony violation: {problems:?}"
    );
}

#[test]
fn stray_events_dir_is_flagged() {
    let owner = Keys::generate();
    let agent = Keys::generate();
    let chan = Uuid::new_v4();
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("manifest.json"),
        serde_json::to_string(&manifest(BundleMode::A, &owner, &agent, vec![chan])).unwrap(),
    )
    .unwrap();
    fs::create_dir_all(tmp.path().join("events")).unwrap();

    let loaded = bundle::load(tmp.path()).unwrap();
    let problems = bundle::validate(&loaded);
    assert!(
        problems.iter().any(|p| p.contains("events/")),
        "stray events dir must be a ceremony violation: {problems:?}"
    );
}
