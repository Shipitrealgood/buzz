//! End-to-end tests for `buzz-provision host-init`, run against the real
//! binary with `--config-dir` pointed at a temp dir — the real
//! `~/.config/buzz-provision` is never touched.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use nostr::nips::nip19::ToBech32;
use nostr::{Keys, SecretKey};

const OWNER_A: &str = "f5c79eab2f44284ff278780c34aab621aafb149be4d7dba8f9625bfd99419733";

fn run(config_dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_buzz-provision"))
        .arg("host-init")
        .args(args)
        .arg("--config-dir")
        .arg(config_dir)
        .env_remove("BUZZ_PROVISION_CONFIG_DIR")
        .output()
        .expect("failed to spawn buzz-provision")
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

/// Fresh, not-yet-created config dir inside a tempdir.
fn fresh_config(tmp: &tempfile::TempDir) -> PathBuf {
    tmp.path().join("buzz-provision")
}

/// A second valid owner pubkey, generated at runtime.
fn owner_b() -> String {
    Keys::generate().public_key().to_hex()
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

#[test]
fn fresh_init_creates_canonical_files_and_prints_matching_pubkey() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = fresh_config(&tmp);

    let out = run(&cfg, &["--allow-owner", OWNER_A]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));

    // Dir + file modes exact.
    assert_eq!(mode_of(&cfg), 0o700, "config dir mode");
    let key_path = cfg.join("host.key");
    let owners_path = cfg.join("allowed-owners");
    assert_eq!(mode_of(&key_path), 0o600, "host.key mode");
    assert_eq!(mode_of(&owners_path), 0o600, "allowed-owners mode");

    // Key is exactly 64 lowercase hex, no trailing newline.
    let key = fs::read(&key_path).unwrap();
    assert_eq!(key.len(), 64, "host.key must be exactly 64 bytes");
    assert!(
        key.iter().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')),
        "host.key must be lowercase hex only"
    );

    // Derived pubkey matches what was printed.
    let sk = SecretKey::from_hex(std::str::from_utf8(&key).unwrap()).unwrap();
    let keys = Keys::new(sk);
    let so = stdout(&out);
    assert!(
        so.contains(&keys.public_key().to_hex()),
        "stdout must contain the derived pubkey hex"
    );
    assert!(
        so.contains(&keys.public_key().to_bech32().unwrap()),
        "stdout must contain the derived npub"
    );

    // Pin file has exactly the one owner.
    assert_eq!(
        fs::read_to_string(&owners_path).unwrap(),
        format!("{OWNER_A}\n")
    );
    assert!(so.contains(OWNER_A), "stdout must list pinned owners");
}

#[test]
fn second_init_without_force_refuses_and_changes_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = fresh_config(&tmp);
    assert!(run(&cfg, &["--allow-owner", OWNER_A]).status.success());

    let key_before = fs::read(cfg.join("host.key")).unwrap();
    let owners_before = fs::read(cfg.join("allowed-owners")).unwrap();

    let out = run(&cfg, &["--allow-owner", &owner_b()]);
    assert!(
        !out.status.success(),
        "second init must refuse without --force"
    );
    let se = stderr(&out);
    assert!(se.contains("refusing"), "stderr: {se}");
    assert!(se.contains("--force"), "stderr must mention --force: {se}");

    // Byte-identical: neither the key nor the pin file moved.
    assert_eq!(fs::read(cfg.join("host.key")).unwrap(), key_before);
    assert_eq!(fs::read(cfg.join("allowed-owners")).unwrap(), owners_before);
}

#[test]
fn force_rotates_key_and_prints_loud_warning() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = fresh_config(&tmp);
    assert!(run(&cfg, &["--allow-owner", OWNER_A]).status.success());
    let old_key = fs::read(cfg.join("host.key")).unwrap();

    let out = run(&cfg, &["--allow-owner", OWNER_A, "--force"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));

    let new_key = fs::read(cfg.join("host.key")).unwrap();
    assert_ne!(new_key, old_key, "--force must rotate the key");
    assert_eq!(mode_of(&cfg.join("host.key")), 0o600);

    let se = stderr(&out);
    assert!(se.contains("WARNING"), "rotation must warn loudly: {se}");
    assert!(
        se.contains("lineage"),
        "warning must name the lineage consequence: {se}"
    );

    // Printed pubkey matches the NEW key.
    let sk = SecretKey::from_hex(std::str::from_utf8(&new_key).unwrap()).unwrap();
    assert!(stdout(&out).contains(&Keys::new(sk).public_key().to_hex()));
}

#[test]
fn allowed_owners_appends_and_dedups() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = fresh_config(&tmp);
    let b = owner_b();

    // Same owner passed twice in one invocation → written once.
    assert!(
        run(&cfg, &["--allow-owner", OWNER_A, "--allow-owner", OWNER_A])
            .status
            .success()
    );
    assert_eq!(
        fs::read_to_string(cfg.join("allowed-owners")).unwrap(),
        format!("{OWNER_A}\n")
    );

    // Re-init (--force) with A and B → appends B, keeps A first, no dup A.
    assert!(run(
        &cfg,
        &["--allow-owner", OWNER_A, "--allow-owner", &b, "--force"]
    )
    .status
    .success());
    assert_eq!(
        fs::read_to_string(cfg.join("allowed-owners")).unwrap(),
        format!("{OWNER_A}\n{b}\n")
    );

    // Again with only B → B already pinned, file unchanged.
    assert!(run(&cfg, &["--allow-owner", &b, "--force"])
        .status
        .success());
    assert_eq!(
        fs::read_to_string(cfg.join("allowed-owners")).unwrap(),
        format!("{OWNER_A}\n{b}\n")
    );
}

#[test]
fn invalid_owner_hex_is_rejected_before_anything_is_written() {
    let tmp = tempfile::tempdir().unwrap();

    let uppercase = OWNER_A.to_uppercase();
    let short = &OWNER_A[..63];
    let nonhex = "zz".repeat(32);
    for bad in [uppercase.as_str(), short, nonhex.as_str(), "nsec1abcdef"] {
        let cfg = fresh_config(&tmp).join(format!("case-{}", bad.len()));
        let out = run(&cfg, &["--allow-owner", bad]);
        assert!(!out.status.success(), "must reject invalid owner: {bad:?}");
        assert!(
            !cfg.join("host.key").exists() && !cfg.join("allowed-owners").exists(),
            "nothing may be written on invalid input"
        );
        // The rejected value is never echoed back (it could be a secret).
        assert!(
            !stderr(&out).contains(bad),
            "error message must not echo the rejected value"
        );
    }
}

#[test]
fn missing_allow_owner_is_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = fresh_config(&tmp);
    let out = run(&cfg, &[]);
    assert!(!out.status.success());
    assert!(stderr(&out).contains("--allow-owner"));
    assert!(!cfg.exists(), "nothing may be created without an owner pin");
}

#[test]
fn secret_never_appears_in_stdout_or_stderr() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = fresh_config(&tmp);

    let init = run(&cfg, &["--allow-owner", OWNER_A]);
    assert!(init.status.success());
    let first_key = fs::read_to_string(cfg.join("host.key")).unwrap();

    let show = run(&cfg, &["--show"]);
    assert!(show.status.success());

    let refuse = run(&cfg, &["--allow-owner", OWNER_A]);
    assert!(!refuse.status.success());

    let force = run(&cfg, &["--allow-owner", OWNER_A, "--force"]);
    assert!(force.status.success());
    let second_key = fs::read_to_string(cfg.join("host.key")).unwrap();

    let surfaces: Vec<String> = [&init, &show, &refuse, &force]
        .iter()
        .flat_map(|o| [stdout(o), stderr(o)])
        .collect();
    let refs: Vec<&str> = surfaces.iter().map(String::as_str).collect();
    assert_no_secret_leak(&first_key, &refs);
    assert_no_secret_leak(&second_key, &refs);
}

#[test]
fn show_prints_existing_identity_and_touches_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = fresh_config(&tmp);
    assert!(run(&cfg, &["--allow-owner", OWNER_A]).status.success());

    let key_before = fs::read(cfg.join("host.key")).unwrap();
    let owners_before = fs::read(cfg.join("allowed-owners")).unwrap();

    let out = run(&cfg, &["--show"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));

    let sk = SecretKey::from_hex(std::str::from_utf8(&key_before).unwrap()).unwrap();
    let keys = Keys::new(sk);
    let so = stdout(&out);
    assert!(so.contains(&keys.public_key().to_hex()));
    assert!(so.contains(&keys.public_key().to_bech32().unwrap()));
    assert!(so.contains(OWNER_A));

    assert_eq!(fs::read(cfg.join("host.key")).unwrap(), key_before);
    assert_eq!(fs::read(cfg.join("allowed-owners")).unwrap(), owners_before);
}

#[test]
fn show_without_identity_gives_clear_error() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = fresh_config(&tmp);
    let out = run(&cfg, &["--show"]);
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("host-init"),
        "error must point at host-init"
    );
}

#[test]
fn existing_config_dir_contents_are_left_alone() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = fresh_config(&tmp);

    // Simulate T2's pre-existing state: config dir with wrong mode plus a
    // creds/ subdir holding live credentials.
    fs::create_dir_all(cfg.join("creds")).unwrap();
    fs::set_permissions(&cfg, fs::Permissions::from_mode(0o755)).unwrap();
    fs::set_permissions(cfg.join("creds"), fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(cfg.join("creds").join("agent.nsec"), b"sentinel").unwrap();

    let out = run(&cfg, &["--allow-owner", OWNER_A]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));

    // Parent corrected to 0700; existing children untouched.
    assert_eq!(mode_of(&cfg), 0o700, "parent must be tightened to 0700");
    assert_eq!(
        mode_of(&cfg.join("creds")),
        0o755,
        "creds/ mode must be untouched"
    );
    assert_eq!(
        fs::read(cfg.join("creds").join("agent.nsec")).unwrap(),
        b"sentinel",
        "creds/ contents must be untouched"
    );
}
