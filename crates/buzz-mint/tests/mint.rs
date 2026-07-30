//! End-to-end tests for `buzz-mint`, run against the real binary with
//! throwaway owner/host keypairs inside tempdirs. Follows the
//! buzz-provision capture-test pattern: every stdout/stderr surface is
//! scanned for secret substrings.

use std::fs;
use std::io::Write as _;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use nostr::{Event, JsonUtil, Keys, PublicKey};
use serde_json::Value;
use zeroize::Zeroizing;

const CHAN_A: &str = "0b5fbf34-7cbe-4a10-9d3f-2f6a30b8a001";
const CHAN_B: &str = "b7f3a6e2-11d4-4f5e-8a3c-9c0d1e2f3a02";

struct Owner {
    keys: Keys,
    key_file: PathBuf,
}

/// Write a throwaway owner key file with the given mode.
fn owner_key_file(dir: &Path, mode: u32, trailing_newline: bool) -> Owner {
    let keys = Keys::generate();
    let path = dir.join("owner.key");
    let hex = Zeroizing::new(keys.secret_key().to_secret_hex());
    let content = if trailing_newline {
        format!("{}\n", hex.as_str())
    } else {
        hex.as_str().to_owned()
    };
    fs::write(&path, content).unwrap();
    #[cfg(unix)]
    fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
    #[cfg(not(unix))]
    let _ = mode;
    Owner {
        keys,
        key_file: path,
    }
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_buzz-mint"))
        .args(args)
        .output()
        .expect("failed to spawn buzz-mint")
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[cfg(unix)]
fn mode_of(path: &Path) -> u32 {
    fs::metadata(path).unwrap().permissions().mode() & 0o7777
}

/// Assert no 16-char window of a 64-char secret appears in any surface.
fn assert_no_secret_leak(secret_hex: &str, surfaces: &[&str]) {
    assert_eq!(secret_hex.len(), 64);
    for start in 0..=(64 - 16) {
        let window = &secret_hex[start..start + 16];
        for surface in surfaces {
            assert!(
                !surface.contains(window),
                "secret substring leaked into output (window at offset {start})"
            );
        }
    }
}

/// Mint a full bundle with two channels + a sealed api key; return
/// (owner, host, bundle_dir, stdout, stderr).
fn mint_full(tmp: &Path) -> (Owner, Keys, PathBuf, String, String) {
    let owner = owner_key_file(tmp, 0o600, true);
    let host = Keys::generate();
    let out = run(&[
        "new",
        "--name",
        "teacup",
        "--host",
        &host.public_key().to_hex(),
        "--channel",
        CHAN_A,
        "--channel",
        CHAN_B,
        "--respond-to",
        "anyone",
        "--harness-cmd",
        "sprig acp --harness claude-code",
        "--seal-env",
        "FAKE_PROVIDER_KEY=sk-test-e2e-000111222333",
        "--out",
        tmp.to_str().unwrap(),
        "--owner-key-file",
        owner.key_file.to_str().unwrap(),
    ]);
    let so = stdout(&out);
    let se = stderr(&out);
    assert!(out.status.success(), "mint failed: {se}");
    (owner, host, tmp.join("teacup-mint"), so, se)
}

#[test]
fn owner_key_file_must_not_be_group_world_accessible() {
    let tmp = tempfile::tempdir().unwrap();
    let owner = owner_key_file(tmp.path(), 0o644, false);
    let host = Keys::generate();
    let out = run(&[
        "new",
        "--name",
        "leaky",
        "--host",
        &host.public_key().to_hex(),
        "--out",
        tmp.path().to_str().unwrap(),
        "--owner-key-file",
        owner.key_file.to_str().unwrap(),
    ]);
    assert!(!out.status.success(), "0644 key file must be refused");
    let se = stderr(&out);
    assert!(
        se.contains("group/world accessible") && se.contains("0644"),
        "stderr should explain the mode refusal: {se}"
    );
    assert!(
        !tmp.path().join("leaky-mint").exists(),
        "no bundle on refusal"
    );
    let owner_sk = Zeroizing::new(owner.keys.secret_key().to_secret_hex());
    assert_no_secret_leak(&owner_sk, &[&stdout(&out), &se]);
}

#[test]
fn full_mint_bundle_structure_consistency_and_no_secret_output() {
    let tmp = tempfile::tempdir().unwrap();
    let (owner, host, bundle, so, se) = mint_full(tmp.path());

    // ── Structure + modes.
    assert!(bundle.is_dir());
    #[cfg(unix)]
    {
        assert_eq!(mode_of(&bundle), 0o700, "bundle dir mode");
        assert_eq!(mode_of(&bundle.join("sealed.json")), 0o600, "sealed mode");
        assert_eq!(mode_of(&bundle.join("events")), 0o700, "events dir mode");
    }

    // ── Manifest fields.
    let manifest: Value =
        serde_json::from_str(&fs::read_to_string(bundle.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["v"], 1);
    assert_eq!(manifest["name"], "teacup");
    assert_eq!(manifest["owner_pubkey"], owner.keys.public_key().to_hex());
    assert_eq!(manifest["host_pubkey"], host.public_key().to_hex());
    assert_eq!(manifest["config"]["respond_to"], "anyone");
    assert_eq!(
        manifest["config"]["harness_cmd"],
        "sprig acp --harness claude-code"
    );
    assert_eq!(manifest["config"]["channels"].as_array().unwrap().len(), 2);

    // ── Envelope agrees with manifest; unseal proves the sealed payload.
    let envelope: buzz_seal::Envelope =
        serde_json::from_str(&fs::read_to_string(bundle.join("sealed.json")).unwrap()).unwrap();
    assert_eq!(
        envelope.agent_pubkey.to_hex(),
        manifest["agent_pubkey"].as_str().unwrap(),
        "manifest/envelope agent_pubkey agreement"
    );
    assert_eq!(envelope.owner_pubkey, owner.keys.public_key());
    assert_eq!(envelope.host_pubkey, host.public_key());
    let unsealed = buzz_seal::unseal(&host, &envelope, &[owner.keys.public_key()])
        .expect("host must be able to unseal the bundle");
    assert!(unsealed.warnings.is_empty());
    let agent_sk = Zeroizing::new(unsealed.inner.agent_sk.expose_canonical_hex().to_owned());
    assert_eq!(agent_sk.len(), 64);
    assert_eq!(
        unsealed.inner.agent_pubkey.to_hex(),
        manifest["agent_pubkey"].as_str().unwrap()
    );
    assert_eq!(
        unsealed
            .inner
            .api_keys
            .as_ref()
            .unwrap()
            .get("FAKE_PROVIDER_KEY")
            .unwrap()
            .expose(),
        "sk-test-e2e-000111222333"
    );
    // Attestation in manifest == attestation sealed inside.
    assert_eq!(
        manifest["attestation"].as_str().unwrap(),
        unsealed.inner.attestation
    );

    // ── 9000 events: shape, signature, signer.
    let agent_pk = PublicKey::from_hex(manifest["agent_pubkey"].as_str().unwrap()).unwrap();
    for chan in [CHAN_A, CHAN_B] {
        let path = bundle.join("events").join(format!("9000-{chan}.json"));
        let event = Event::from_json(fs::read_to_string(&path).unwrap()).unwrap();
        event.verify().expect("valid id + signature");
        assert_eq!(event.kind.as_u16(), 9000);
        assert_eq!(event.pubkey, owner.keys.public_key(), "signed by owner");
        let tag = |name: &str| -> Vec<String> {
            event
                .tags
                .iter()
                .map(|t| t.as_slice().to_vec())
                .find(|s| s.first().map(String::as_str) == Some(name))
                .unwrap_or_default()
        };
        assert_eq!(tag("h"), vec!["h".to_string(), chan.to_string()]);
        assert_eq!(tag("p"), vec!["p".to_string(), agent_pk.to_hex()]);
        assert_eq!(tag("role"), vec!["role".to_string(), "bot".to_string()]);
    }

    // ── stdout announces pubkeys + bundle, never secrets.
    assert!(so.contains(&agent_pk.to_hex()));
    assert!(so.contains("npub"));
    assert!(so.contains("teacup-mint"));

    // ── No secret on ANY surface: process output, manifest, event files.
    let manifest_raw = fs::read_to_string(bundle.join("manifest.json")).unwrap();
    let sealed_raw = fs::read_to_string(bundle.join("sealed.json")).unwrap();
    let ev_a =
        fs::read_to_string(bundle.join("events").join(format!("9000-{CHAN_A}.json"))).unwrap();
    let ev_b =
        fs::read_to_string(bundle.join("events").join(format!("9000-{CHAN_B}.json"))).unwrap();
    let owner_sk = Zeroizing::new(owner.keys.secret_key().to_secret_hex());
    let surfaces = [
        so.as_str(),
        se.as_str(),
        manifest_raw.as_str(),
        sealed_raw.as_str(),
        ev_a.as_str(),
        ev_b.as_str(),
    ];
    assert_no_secret_leak(&agent_sk, &surfaces);
    assert_no_secret_leak(&owner_sk, &surfaces);
    for s in surfaces {
        assert!(
            !s.contains("sk-test-e2e-000111222333"),
            "sealed api key appeared in a plaintext surface"
        );
    }
}

#[test]
fn stdin_pipe_owner_key_works_and_never_echoes() {
    let tmp = tempfile::tempdir().unwrap();
    let owner = Keys::generate();
    let host = Keys::generate();
    let mut child = Command::new(env!("CARGO_BIN_EXE_buzz-mint"))
        .args([
            "new",
            "--name",
            "pipestream",
            "--host",
            &host.public_key().to_hex(),
            "--out",
            tmp.path().to_str().unwrap(),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let hex = Zeroizing::new(owner.secret_key().to_secret_hex());
        let mut stdin = child.stdin.take().unwrap();
        writeln!(stdin, "{}", hex.as_str()).unwrap();
    }
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "stderr: {}", stderr(&out));

    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(tmp.path().join("pipestream-mint").join("manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["owner_pubkey"], owner.public_key().to_hex());
    assert_eq!(
        manifest["config"]["respond_to"], "owner-only",
        "default respond_to"
    );
    assert!(manifest["config"].get("harness_cmd").is_none());
    let owner_sk = Zeroizing::new(owner.secret_key().to_secret_hex());
    assert_no_secret_leak(&owner_sk, &[&stdout(&out), &stderr(&out)]);
}

#[test]
fn refuses_existing_bundle_dir_and_bad_host_key() {
    let tmp = tempfile::tempdir().unwrap();
    let owner = owner_key_file(tmp.path(), 0o600, false);
    let host = Keys::generate();
    let base = [
        "new".to_string(),
        "--name".into(),
        "dupe".into(),
        "--out".into(),
        tmp.path().to_str().unwrap().into(),
        "--owner-key-file".into(),
        owner.key_file.to_str().unwrap().into(),
    ];

    // Uppercase host pubkey rejected before any secret is read.
    let mut args: Vec<String> = base.to_vec();
    args.extend(["--host".into(), host.public_key().to_hex().to_uppercase()]);
    let out = run(&args.iter().map(String::as_str).collect::<Vec<_>>());
    assert!(!out.status.success());
    assert!(stderr(&out).contains("64 lowercase hex"));

    // First mint succeeds; second refuses to overwrite.
    let mut args: Vec<String> = base.to_vec();
    args.extend(["--host".into(), host.public_key().to_hex()]);
    let argv: Vec<&str> = args.iter().map(String::as_str).collect();
    assert!(run(&argv).status.success());
    let out = run(&argv);
    assert!(!out.status.success());
    assert!(stderr(&out).contains("already exists"));
}

#[test]
fn inspect_accepts_fresh_bundle_and_catches_tampering() {
    let tmp = tempfile::tempdir().unwrap();
    let (_owner, _host, bundle, _so, _se) = mint_full(tmp.path());

    // Fresh bundle passes with no secret input.
    let out = run(&["inspect", bundle.to_str().unwrap()]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert!(stdout(&out).contains("inspect: OK"));

    // Tamper 1: swap manifest agent_pubkey ⇒ envelope disagreement +
    // attestation failure.
    let manifest_path = bundle.join("manifest.json");
    let original = fs::read_to_string(&manifest_path).unwrap();
    let mut manifest: Value = serde_json::from_str(&original).unwrap();
    manifest["agent_pubkey"] = Value::String(Keys::generate().public_key().to_hex());
    fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();
    let out = run(&["inspect", bundle.to_str().unwrap()]);
    assert!(!out.status.success(), "tampered manifest must fail");
    let se = stderr(&out);
    assert!(se.contains("agent_pubkey disagrees"), "got: {se}");
    assert!(se.contains("attestation"), "got: {se}");

    // Tamper 2: restore manifest, break one 9000 signature byte-for-byte.
    fs::write(&manifest_path, &original).unwrap();
    let ev_path = bundle.join("events").join(format!("9000-{CHAN_A}.json"));
    let mut ev: Value = serde_json::from_str(&fs::read_to_string(&ev_path).unwrap()).unwrap();
    ev["content"] = Value::String("tampered".into());
    fs::write(&ev_path, ev.to_string()).unwrap();
    let out = run(&["inspect", bundle.to_str().unwrap()]);
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("id/signature invalid"),
        "got: {}",
        stderr(&out)
    );

    // Tamper 3: drop an event file ⇒ missing-channel finding.
    fs::remove_file(&ev_path).unwrap();
    let out = run(&["inspect", bundle.to_str().unwrap()]);
    assert!(!out.status.success());
    assert!(stderr(&out).contains(&format!("missing events/9000-{CHAN_A}.json")));
}
