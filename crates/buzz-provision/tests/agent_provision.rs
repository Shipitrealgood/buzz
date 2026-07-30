//! Tests for `buzz-provision agent` (and `pin-owner add`), run against the
//! real binary with `--config-dir`/`--env-dir` pointed at temp dirs — the
//! real `~/.config/buzz-provision` and `~/services/buzz-agents` are never
//! touched, and `--no-systemd` keeps systemd out of every test.
//!
//! Bundles are constructed in-test with the same crates buzz-mint uses
//! (buzz-seal seal + buzz-sdk attestation/9000 builders), so these tests
//! exercise the exact artifact contract without a network or an owner
//! prompt.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use buzz_seal::bundle::{BundleMode, Manifest, ManifestConfig};
use buzz_seal::{seal, AgentSecretKey, InnerPayload};
use nostr::{JsonUtil, Keys, PublicKey};
use uuid::Uuid;

// ─── Harness ─────────────────────────────────────────────────────────────────

struct TestHost {
    #[allow(dead_code)]
    tmp: tempfile::TempDir,
    config_dir: PathBuf,
    env_dir: PathBuf,
    host_pubkey: PublicKey,
    owner: Keys,
}

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_buzz-provision")
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn mode_of(path: &Path) -> u32 {
    fs::metadata(path).unwrap().permissions().mode() & 0o7777
}

/// Assert no 16-char window of the secret appears in the given output text.
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

/// host-init a fresh temp host pinned to a fresh owner.
fn init_host() -> TestHost {
    let tmp = tempfile::tempdir().unwrap();
    let config_dir = tmp.path().join("cfg");
    let env_dir = tmp.path().join("env");
    let owner = Keys::generate();
    let out = Command::new(bin())
        .args([
            "host-init",
            "--allow-owner",
            &owner.public_key().to_hex(),
            "--config-dir",
        ])
        .arg(&config_dir)
        .output()
        .expect("spawn host-init");
    assert!(out.status.success(), "host-init failed: {}", stderr(&out));
    let so = stdout(&out);
    let host_hex = so
        .lines()
        .find_map(|l| l.strip_prefix("host pubkey (hex):"))
        .expect("host pubkey line")
        .trim()
        .to_owned();
    TestHost {
        tmp,
        config_dir,
        env_dir,
        host_pubkey: PublicKey::from_hex(&host_hex).unwrap(),
        owner,
    }
}

/// Build a mint bundle exactly the way buzz-mint does, returning
/// (bundle_dir, agent_keys, agent_sk_hex).
fn make_bundle(
    host: &TestHost,
    name: &str,
    channels: &[Uuid],
    sealed_to: Option<&PublicKey>, // override seal target (splice tests)
    owner: Option<&Keys>,          // override sealer/attester (foreign tests)
) -> (PathBuf, Keys, String) {
    let owner = owner.unwrap_or(&host.owner);
    let host_pubkey = *sealed_to.unwrap_or(&host.host_pubkey);
    let agent_keys = Keys::generate();
    let agent_pubkey = agent_keys.public_key();
    let agent_sk_hex = agent_keys.secret_key().to_secret_hex();

    let attestation = buzz_sdk::nip_oa::compute_auth_tag(owner, &agent_pubkey, "").unwrap();
    let created_at = nostr::Timestamp::now().as_secs();
    let inner = InnerPayload {
        v: 1,
        agent_sk: AgentSecretKey::from_keys(&agent_keys).unwrap(),
        agent_pubkey,
        attestation: attestation.clone(),
        host_pubkey,
        created_at,
        api_keys: None,
    };
    let envelope = seal(owner, &host_pubkey, &inner).unwrap();

    let manifest = Manifest {
        v: 1,
        name: name.to_owned(),
        agent_pubkey,
        owner_pubkey: owner.public_key(),
        host_pubkey,
        attestation,
        created_at,
        mode: BundleMode::B,
        config: ManifestConfig {
            respond_to: "owner-only".to_owned(),
            channels: channels.to_vec(),
            harness_cmd: Some("/usr/bin/true".to_owned()),
            relay_url: None,
        },
    };

    let dir = host.tmp.path().join(format!("{name}-mint"));
    fs::create_dir_all(dir.join("events")).unwrap();
    fs::write(
        dir.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();
    fs::write(
        dir.join("sealed.json"),
        serde_json::to_string(&envelope).unwrap(),
    )
    .unwrap();
    for channel in channels {
        let event = buzz_sdk::builders::build_add_member(
            *channel,
            &agent_pubkey.to_hex(),
            Some(buzz_sdk::MemberRole::Bot),
        )
        .unwrap()
        .sign_with_keys(owner)
        .unwrap();
        fs::write(
            dir.join("events").join(format!("9000-{channel}.json")),
            event.as_json(),
        )
        .unwrap();
    }
    (dir, agent_keys, agent_sk_hex)
}

/// Run `buzz-provision agent` with the standard test flags.
fn run_agent(host: &TestHost, bundle: &Path, extra: &[&str]) -> Output {
    Command::new(bin())
        .arg("agent")
        .arg(bundle)
        .args(["--relay-url", "ws://127.0.0.1:1", "--no-systemd"])
        .args(extra)
        .arg("--config-dir")
        .arg(&host.config_dir)
        .arg("--env-dir")
        .arg(&host.env_dir)
        .output()
        .expect("spawn buzz-provision agent")
}

// ─── Dry run: steps 1-3 + construction, nothing else ────────────────────────

#[test]
fn dry_run_writes_credential_and_env_but_never_posts_or_shreds() {
    let host = init_host();
    let chan = Uuid::new_v4();
    let (bundle, agent_keys, agent_sk) = make_bundle(&host, "teacup", &[chan], None, None);

    let out = run_agent(&host, &bundle, &["--dry-run"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let so = stdout(&out);

    // Credential: byte-exact canonical hex, 0600, dir 0700.
    let cred = host.config_dir.join("creds").join("teacup.nsec");
    assert_eq!(mode_of(&cred), 0o600);
    assert_eq!(mode_of(&host.config_dir.join("creds")), 0o700);
    let bytes = fs::read(&cred).unwrap();
    assert_eq!(bytes.len(), 64, "no trailing byte");
    assert_eq!(String::from_utf8(bytes).unwrap(), agent_sk);

    // Env file: secret-free, literal auth tag line, expected keys.
    let env = fs::read_to_string(host.env_dir.join("teacup.env")).unwrap();
    assert!(env.contains("BUZZ_RELAY_URL=ws://127.0.0.1:1\n"));
    assert!(env.contains("BUZZ_ACP_AGENT_COMMAND=/usr/bin/true\n"));
    assert!(env.contains("BUZZ_ACP_RESPOND_TO=owner-only\n"));
    assert!(env.contains("BUZZ_ACP_SUBSCRIBE=mentions\n"));
    assert!(env.contains("BUZZ_AUTH_TAG=[\"auth\""));
    assert!(
        !env.contains("BUZZ_PRIVATE_KEY="),
        "env must be secret-free"
    );
    assert_no_secret_leak(&agent_sk, &[&env]);

    // Pub marker file.
    assert_eq!(
        fs::read_to_string(host.env_dir.join("teacup.pub")).unwrap(),
        format!("{}\n", agent_keys.public_key().to_hex())
    );

    // Dry run: sealed.json survives; events constructed and listed.
    assert!(
        bundle.join("sealed.json").exists(),
        "dry-run must not shred"
    );
    assert!(so.contains("DRY RUN"));
    assert!(so.contains("kind:0"));
    assert!(so.contains("kind:10100"));
    assert!(so.contains("kind:9000"));
    assert!(so.contains(&agent_keys.public_key().to_hex()));
    assert!(
        so.contains(&host.host_pubkey.to_hex()),
        "lineage host in summary"
    );

    // No secret on any output surface.
    assert_no_secret_leak(&agent_sk, &[&so, &stderr(&out)]);
    let host_sk = fs::read_to_string(host.config_dir.join("host.key")).unwrap();
    assert_no_secret_leak(&host_sk, &[&so, &stderr(&out)]);
}

// ─── Real run against an unreachable relay: shred + convergence ─────────────

#[test]
fn credential_lands_and_sealed_is_shredded_before_publish_then_rerun_converges() {
    let host = init_host();
    let (bundle, agent_keys, agent_sk) = make_bundle(&host, "shredme", &[], None, None);

    // Relay is unreachable (port 1) — provisioning must fail at PUBLISH,
    // after the credential is verified and sealed.json is shredded.
    let out = run_agent(&host, &bundle, &[]);
    assert!(!out.status.success(), "unreachable relay must fail the run");
    let se = stderr(&out);
    assert!(
        se.contains("publishing"),
        "failure must be at publish: {se}"
    );

    let cred = host.config_dir.join("creds").join("shredme.nsec");
    assert_eq!(fs::read_to_string(&cred).unwrap(), agent_sk);
    assert!(
        !bundle.join("sealed.json").exists(),
        "sealed.json must be shredded once the credential is verified"
    );
    // Manifest + events stay: they are the lineage record.
    assert!(bundle.join("manifest.json").exists());

    // Converged re-run without sealed.json: same failure point (publish),
    // credential unchanged — the bundle stays re-runnable.
    let out2 = run_agent(&host, &bundle, &["--force"]);
    assert!(!out2.status.success());
    assert!(
        stderr(&out2).contains("publishing"),
        "rerun reaches publish"
    );
    assert_eq!(fs::read_to_string(&cred).unwrap(), agent_sk);

    assert_no_secret_leak(
        &agent_sk,
        &[&stdout(&out), &se, &stdout(&out2), &stderr(&out2)],
    );
    let _ = agent_keys;
}

// ─── Fail-closed paths ───────────────────────────────────────────────────────

#[test]
fn foreign_owner_is_refused_and_no_credential_is_written() {
    let host = init_host();
    let foreign = Keys::generate();
    // Self-consistent rogue bundle: valid attestation, sealed to OUR host,
    // but attested by an unpinned owner.
    let (bundle, _agent, agent_sk) = make_bundle(&host, "rogue", &[], None, Some(&foreign));

    let out = run_agent(&host, &bundle, &["--dry-run"]);
    assert!(!out.status.success(), "foreign owner must be refused");
    let se = stderr(&out);
    assert!(se.contains("step 5") || se.contains("pinned"), "got: {se}");
    assert!(
        !host.config_dir.join("creds").join("rogue.nsec").exists(),
        "no credential may be written for a refused bundle"
    );
    assert!(
        bundle.join("sealed.json").exists(),
        "refusal must not shred"
    );
    assert_no_secret_leak(&agent_sk, &[&stdout(&out), &se]);

    // Same bundle passes with an explicit run-scoped trust extension.
    let out = run_agent(
        &host,
        &bundle,
        &["--dry-run", "--trust-owner", &foreign.public_key().to_hex()],
    );
    assert!(
        out.status.success(),
        "explicit --trust-owner must admit it: {}",
        stderr(&out)
    );
}

#[test]
fn bundle_sealed_for_another_host_is_refused() {
    let host = init_host();
    let elsewhere = Keys::generate().public_key();
    let (bundle, _agent, _sk) = make_bundle(&host, "misdelivered", &[], Some(&elsewhere), None);

    let out = run_agent(&host, &bundle, &["--dry-run"]);
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("sealed for a different host") || stderr(&out).contains("step 1"),
        "got: {}",
        stderr(&out)
    );
}

#[test]
fn tampered_manifest_pubkey_fails_bundle_validation() {
    let host = init_host();
    let (bundle, _agent, _sk) = make_bundle(&host, "tampered", &[], None, None);

    let manifest_path = bundle.join("manifest.json");
    let mut manifest: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
    manifest["agent_pubkey"] = serde_json::Value::String(Keys::generate().public_key().to_hex());
    fs::write(&manifest_path, manifest.to_string()).unwrap();

    let out = run_agent(&host, &bundle, &["--dry-run"]);
    assert!(!out.status.success());
    let se = stderr(&out);
    assert!(
        se.contains("agent_pubkey disagrees") || se.contains("attestation"),
        "got: {se}"
    );
    assert!(!host.config_dir.join("creds").join("tampered.nsec").exists());
}

#[test]
fn existing_env_file_refuses_without_force() {
    let host = init_host();
    let (bundle, _agent, _sk) = make_bundle(&host, "clobber", &[], None, None);

    assert!(run_agent(&host, &bundle, &["--dry-run"]).status.success());
    let env_path = host.env_dir.join("clobber.env");
    let before = fs::read_to_string(&env_path).unwrap();

    let out = run_agent(&host, &bundle, &["--dry-run"]);
    assert!(
        !out.status.success(),
        "second run must refuse to clobber env"
    );
    assert!(stderr(&out).contains("--force"));
    assert_eq!(fs::read_to_string(&env_path).unwrap(), before);

    let out = run_agent(&host, &bundle, &["--dry-run", "--force"]);
    assert!(out.status.success(), "--force must allow: {}", stderr(&out));
}

#[test]
fn credential_name_collision_with_different_agent_is_refused() {
    let host = init_host();
    // First agent claims the instance name.
    let (bundle_a, _a, sk_a) = make_bundle(&host, "samename", &[], None, None);
    assert!(run_agent(&host, &bundle_a, &["--dry-run"]).status.success());

    // A second, different agent minted under the SAME name must not silently
    // replace the credential.
    fs::remove_dir_all(&bundle_a).unwrap();
    let (bundle_b, _b, _sk_b) = make_bundle(&host, "samename", &[], None, None);
    let out = run_agent(&host, &bundle_b, &["--dry-run"]);
    assert!(!out.status.success(), "name collision must refuse");
    assert!(
        stderr(&out).contains("--force"),
        "must explain the override"
    );
    assert_eq!(
        fs::read_to_string(host.config_dir.join("creds").join("samename.nsec")).unwrap(),
        sk_a,
        "original credential must be untouched"
    );
}

#[test]
fn allowlist_mode_requires_allowlist_pubkeys() {
    let host = init_host();
    let (bundle, _agent, _sk) = make_bundle(&host, "listy", &[], None, None);
    let manifest_path = bundle.join("manifest.json");
    let mut manifest: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
    manifest["config"]["respond_to"] = serde_json::Value::String("allowlist".into());
    fs::write(&manifest_path, manifest.to_string()).unwrap();

    let out = run_agent(&host, &bundle, &["--dry-run"]);
    assert!(!out.status.success());
    assert!(stderr(&out).contains("respond-to-allowlist"));

    let out = run_agent(
        &host,
        &bundle,
        &[
            "--dry-run",
            "--respond-to-allowlist",
            &Keys::generate().public_key().to_hex(),
        ],
    );
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let env = fs::read_to_string(host.env_dir.join("listy.env")).unwrap();
    assert!(env.contains("BUZZ_ACP_RESPOND_TO=allowlist\n"));
    assert!(env.contains("BUZZ_ACP_RESPOND_TO_ALLOWLIST="));
}

#[test]
fn empty_pin_list_refuses_everything() {
    let host = init_host();
    let (bundle, _agent, _sk) = make_bundle(&host, "unpinned", &[], None, None);
    fs::remove_file(host.config_dir.join("allowed-owners")).unwrap();

    let out = run_agent(&host, &bundle, &["--dry-run"]);
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("pin-owner add"),
        "must point at pin-owner"
    );
}

// ─── A-mode (Design A: key born host-side, manifest-only bundle) ────────────

/// Run `buzz-provision keygen` for `name`; return the credential's pubkey.
fn run_keygen(host: &TestHost, name: &str) -> PublicKey {
    let out = Command::new(bin())
        .args(["keygen", "--name", name])
        .arg("--config-dir")
        .arg(&host.config_dir)
        .arg("--env-dir")
        .arg(&host.env_dir)
        .output()
        .expect("spawn keygen");
    assert!(out.status.success(), "keygen failed: {}", stderr(&out));
    let sk = fs::read_to_string(host.config_dir.join("creds").join(format!("{name}.nsec")))
        .expect("credential written");
    Keys::parse(&sk).unwrap().public_key()
}

/// Build an A-mode bundle (manifest.json ONLY) attesting `agent_pubkey`.
fn make_a_bundle(
    host: &TestHost,
    name: &str,
    agent_pubkey: PublicKey,
    channels: &[Uuid],
    owner: Option<&Keys>, // override attester (foreign-owner tests)
) -> PathBuf {
    let owner = owner.unwrap_or(&host.owner);
    let attestation = buzz_sdk::nip_oa::compute_auth_tag(owner, &agent_pubkey, "").unwrap();
    let manifest = Manifest {
        v: 1,
        name: name.to_owned(),
        agent_pubkey,
        owner_pubkey: owner.public_key(),
        host_pubkey: host.host_pubkey,
        attestation,
        created_at: nostr::Timestamp::now().as_secs(),
        mode: BundleMode::A,
        config: ManifestConfig {
            respond_to: "owner-only".to_owned(),
            channels: channels.to_vec(),
            harness_cmd: Some("/usr/bin/true".to_owned()),
            relay_url: None,
        },
    };
    let dir = host.tmp.path().join(format!("{name}-mint"));
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();
    dir
}

#[test]
fn a_mode_happy_path_provisions_from_keygen_credential() {
    let host = init_host();
    let chan = Uuid::new_v4();
    let agent_pubkey = run_keygen(&host, "warbler");
    let sk = fs::read_to_string(host.config_dir.join("creds").join("warbler.nsec")).unwrap();
    let bundle = make_a_bundle(&host, "warbler", agent_pubkey, &[chan], None);

    // Dry run: credential validated, env written, events constructed —
    // and NO 9000 in the prepared list (the owner published those).
    let out = run_agent(&host, &bundle, &["--dry-run"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let so = stdout(&out);
    assert!(so.contains("kind:0"));
    assert!(so.contains("kind:10100"));
    assert!(
        !so.contains("kind:9000"),
        "A-mode must not submit (or list) any 9000: {so}"
    );

    // Env file: secret-free, same contract as B.
    let env = fs::read_to_string(host.env_dir.join("warbler.env")).unwrap();
    assert!(env.contains("BUZZ_ACP_RESPOND_TO=owner-only\n"));
    assert!(env.contains("BUZZ_AUTH_TAG=[\"auth\""));
    assert!(!env.contains("BUZZ_PRIVATE_KEY="));
    assert_no_secret_leak(sk.trim(), &[&env, &so, &stderr(&out)]);

    // Credential untouched by provisioning (it was born by keygen).
    assert_eq!(
        fs::read_to_string(host.config_dir.join("creds").join("warbler.nsec")).unwrap(),
        sk
    );

    // Real run against an unreachable relay: fails AT PUBLISH — everything
    // before (credential + pinning + env) already held.
    let out = run_agent(&host, &bundle, &["--force"]);
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("publishing"),
        "A-mode failure point must be publish: {}",
        stderr(&out)
    );
}

#[test]
fn a_mode_missing_credential_fails_closed_before_any_publish() {
    let host = init_host();
    let stray = Keys::generate().public_key();
    let bundle = make_a_bundle(&host, "ghost", stray, &[], None);

    // NOT a dry run: if the credential gate did not fire first, this would
    // reach the (unreachable) relay and fail with "publishing" instead.
    let out = run_agent(&host, &bundle, &[]);
    assert!(!out.status.success());
    let se = stderr(&out);
    assert!(
        se.contains("keygen"),
        "must route to buzz-provision keygen: {se}"
    );
    assert!(
        !se.contains("publishing"),
        "must fail BEFORE any publish: {se}"
    );
    assert!(
        !host.config_dir.join("creds").join("ghost.nsec").exists(),
        "A-mode must never create a credential"
    );
}

#[test]
fn a_mode_wrong_credential_pubkey_mismatch_fails_closed() {
    let host = init_host();
    run_keygen(&host, "mismatch");
    // Bundle attests a DIFFERENT key under the same instance name — the
    // A-mode splice analogue (wrong bundle for this host's key).
    let other = Keys::generate().public_key();
    let bundle = make_a_bundle(&host, "mismatch", other, &[], None);

    let out = run_agent(&host, &bundle, &[]);
    assert!(!out.status.success());
    let se = stderr(&out);
    assert!(
        se.contains("derives to") && se.contains("agent_pubkey"),
        "must name the derivation mismatch: {se}"
    );
    assert!(!se.contains("publishing"), "must fail BEFORE publish: {se}");

    // --force must NOT bypass the identity gate either.
    let out = run_agent(&host, &bundle, &["--force"]);
    assert!(
        !out.status.success(),
        "--force must not bypass pubkey verification"
    );
}

#[test]
fn a_mode_unpinned_attester_is_refused() {
    let host = init_host();
    let agent_pubkey = run_keygen(&host, "foreigner");
    let foreign = Keys::generate();
    let bundle = make_a_bundle(&host, "foreigner", agent_pubkey, &[], Some(&foreign));

    let out = run_agent(&host, &bundle, &["--dry-run"]);
    assert!(!out.status.success(), "unpinned attester must be refused");
    assert!(stderr(&out).contains("pinned"), "got: {}", stderr(&out));

    // Same trust-extension escape hatch as B-mode.
    let out = run_agent(
        &host,
        &bundle,
        &["--dry-run", "--trust-owner", &foreign.public_key().to_hex()],
    );
    assert!(
        out.status.success(),
        "explicit --trust-owner must admit it: {}",
        stderr(&out)
    );
}

#[test]
fn a_mode_bundle_carrying_sealed_or_events_is_refused() {
    let host = init_host();
    let agent_pubkey = run_keygen(&host, "impure");

    // Stray sealed.json in an A-mode bundle.
    let bundle = make_a_bundle(&host, "impure", agent_pubkey, &[], None);
    fs::write(bundle.join("sealed.json"), "{}").unwrap();
    let out = run_agent(&host, &bundle, &["--dry-run"]);
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("A-mode"),
        "must name the mode violation: {}",
        stderr(&out)
    );
    fs::remove_file(bundle.join("sealed.json")).unwrap();

    // Stray pre-signed event in an A-mode bundle.
    let chan = Uuid::new_v4();
    let ev = buzz_sdk::builders::build_add_member(
        chan,
        &agent_pubkey.to_hex(),
        Some(buzz_sdk::MemberRole::Bot),
    )
    .unwrap()
    .sign_with_keys(&host.owner)
    .unwrap();
    fs::create_dir_all(bundle.join("events")).unwrap();
    fs::write(
        bundle.join("events").join(format!("9000-{chan}.json")),
        ev.as_json(),
    )
    .unwrap();
    let out = run_agent(&host, &bundle, &["--dry-run", "--force"]);
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("A-mode"),
        "must name the mode violation: {}",
        stderr(&out)
    );
}

// ─── pin-owner add ───────────────────────────────────────────────────────────

#[test]
fn pin_owner_add_appends_dedups_and_never_touches_host_key() {
    let host = init_host();
    let key_before = fs::read(host.config_dir.join("host.key")).unwrap();
    let owner_a = host.owner.public_key().to_hex();
    let owner_b = Keys::generate().public_key().to_hex();

    let run_pin = |pk: &str| {
        Command::new(bin())
            .args(["pin-owner", "add", pk, "--config-dir"])
            .arg(&host.config_dir)
            .output()
            .expect("spawn pin-owner add")
    };

    let out = run_pin(&owner_b);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert_eq!(
        fs::read_to_string(host.config_dir.join("allowed-owners")).unwrap(),
        format!("{owner_a}\n{owner_b}\n")
    );
    // Dedup: adding again changes nothing.
    assert!(run_pin(&owner_b).status.success());
    assert_eq!(
        fs::read_to_string(host.config_dir.join("allowed-owners")).unwrap(),
        format!("{owner_a}\n{owner_b}\n")
    );
    assert_eq!(
        fs::read(host.config_dir.join("host.key")).unwrap(),
        key_before,
        "pin-owner add must never move the host key"
    );

    // Invalid value: rejected without echo.
    let bogus = "NSEC1NOTAREALKEYBUTSHAPEDLIKESECRETMATERIAL000000000000000000000";
    let out = run_pin(bogus);
    assert!(!out.status.success());
    assert!(!stderr(&out).contains(bogus), "must not echo the value");
}

#[test]
fn pin_owner_add_requires_initialized_host() {
    let tmp = tempfile::tempdir().unwrap();
    let out = Command::new(bin())
        .args([
            "pin-owner",
            "add",
            &Keys::generate().public_key().to_hex(),
            "--config-dir",
        ])
        .arg(tmp.path().join("nope"))
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(stderr(&out).contains("host-init"));
}

// ─── clap argv redaction (chuck T1-4) ────────────────────────────────────────

#[test]
fn clap_errors_never_echo_key_shaped_argv() {
    let hex_secret = Keys::generate().secret_key().to_secret_hex();
    let nsec = "nsec1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq";

    for pasted in [hex_secret.as_str(), nsec] {
        // Pasted as an unexpected positional after host-init.
        let out = Command::new(bin())
            .args(["host-init", pasted])
            .output()
            .unwrap();
        assert!(!out.status.success());
        let se = stderr(&out);
        assert!(
            !se.contains(pasted),
            "clap error must not echo key-shaped argv: {se}"
        );
        assert!(se.contains("<redacted-key-material>"), "got: {se}");
    }
}

// ─── Refusal message shape (chuck T1-1) ──────────────────────────────────────

#[test]
fn host_init_refusal_offers_pin_owner_before_force() {
    let host = init_host();
    let out = Command::new(bin())
        .args([
            "host-init",
            "--allow-owner",
            &Keys::generate().public_key().to_hex(),
            "--config-dir",
        ])
        .arg(&host.config_dir)
        .output()
        .unwrap();
    assert!(!out.status.success());
    let se = stderr(&out);
    assert!(
        se.contains("pin-owner add"),
        "must offer pin-owner add: {se}"
    );
    let pin_pos = se.find("pin-owner add").unwrap();
    let force_pos = se.find("--force").unwrap();
    assert!(
        pin_pos < force_pos,
        "pin-owner path must come before the --force warning"
    );
}
