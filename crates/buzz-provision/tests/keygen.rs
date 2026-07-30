//! Tests for `buzz-provision keygen` (Design A host-side key birth), run
//! against the real binary with `--config-dir`/`--env-dir` pointed at temp
//! dirs. Same no-secret-leak discipline as the other suites: every output
//! surface is scanned for secret substrings.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use nostr::{Keys, PublicKey};

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

struct Ctx {
    #[allow(dead_code)]
    tmp: tempfile::TempDir,
    config_dir: PathBuf,
    env_dir: PathBuf,
}

fn ctx() -> Ctx {
    let tmp = tempfile::tempdir().unwrap();
    let config_dir = tmp.path().join("cfg");
    let env_dir = tmp.path().join("env");
    Ctx {
        tmp,
        config_dir,
        env_dir,
    }
}

fn run_keygen(ctx: &Ctx, name: &str, extra: &[&str]) -> Output {
    Command::new(bin())
        .args(["keygen", "--name", name])
        .args(extra)
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("--env-dir")
        .arg(&ctx.env_dir)
        .output()
        .expect("spawn buzz-provision keygen")
}

fn host_init(ctx: &Ctx, owner: &Keys) -> String {
    let out = Command::new(bin())
        .args([
            "host-init",
            "--allow-owner",
            &owner.public_key().to_hex(),
            "--config-dir",
        ])
        .arg(&ctx.config_dir)
        .output()
        .expect("spawn host-init");
    assert!(out.status.success(), "host-init failed: {}", stderr(&out));
    stdout(&out)
        .lines()
        .find_map(|l| l.strip_prefix("host pubkey (hex):"))
        .expect("host pubkey line")
        .trim()
        .to_owned()
}

#[test]
fn keygen_writes_credential_and_prints_matching_pubkey_and_attest_command() {
    let ctx = ctx();
    let host_hex = host_init(&ctx, &Keys::generate());
    fs::create_dir_all(&ctx.env_dir).unwrap();

    let out = run_keygen(&ctx, "sparrow", &[]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let so = stdout(&out);

    // Credential: canonical 64-lowercase-hex, byte-exact, 0600, dir 0700.
    let cred = ctx.config_dir.join("creds").join("sparrow.nsec");
    assert_eq!(mode_of(&cred), 0o600);
    assert_eq!(mode_of(&ctx.config_dir.join("creds")), 0o700);
    let bytes = fs::read(&cred).unwrap();
    assert_eq!(bytes.len(), 64, "no trailing byte");
    let sk_hex = String::from_utf8(bytes).unwrap();
    assert!(sk_hex
        .bytes()
        .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')));

    // Printed pubkey derives from the credential on disk.
    let derived = Keys::parse(&sk_hex).unwrap().public_key();
    assert!(
        so.contains(&derived.to_hex()),
        "printed pubkey must match the credential"
    );
    assert!(so.contains("npub"), "npub form printed");

    // Pub marker beside the env dir convention.
    assert_eq!(
        fs::read_to_string(ctx.env_dir.join("sparrow.pub")).unwrap(),
        format!("{}\n", derived.to_hex())
    );

    // Printed attest command line: agent + host + name filled in.
    let cmd_line = so
        .lines()
        .find(|l| l.trim_start().starts_with("buzz-mint attest"))
        .expect("attest command printed");
    assert!(cmd_line.contains(&format!("--agent {}", derived.to_hex())));
    assert!(cmd_line.contains(&format!("--host {host_hex}")));
    assert!(cmd_line.contains("--name sparrow"));
    assert!(cmd_line.contains("--relay-url"));

    // No secret on any output surface.
    assert_no_secret_leak(&sk_hex, &[&so, &stderr(&out)]);
    let host_sk = fs::read_to_string(ctx.config_dir.join("host.key")).unwrap();
    assert_no_secret_leak(&host_sk, &[&so, &stderr(&out)]);
}

#[test]
fn keygen_without_host_identity_or_env_dir_is_print_only_with_placeholder() {
    let ctx = ctx();
    // No host-init, env dir absent.
    let out = run_keygen(&ctx, "orphan", &[]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let so = stdout(&out);
    let se = stderr(&out);

    assert!(
        ctx.config_dir.join("creds").join("orphan.nsec").exists(),
        "credential must still be written"
    );
    assert!(
        !ctx.env_dir.exists(),
        "keygen must not create the deployment env dir"
    );
    assert!(so.contains("not written"), "pub marker must be print-only");
    assert!(
        so.contains("<host-pubkey>"),
        "attest command carries a placeholder host: {so}"
    );
    assert!(se.contains("host-init"), "note points at host-init: {se}");
}

#[test]
fn keygen_refuses_clobber_prints_existing_pubkey_and_force_rotates() {
    let ctx = ctx();
    let out = run_keygen(&ctx, "kestrel", &[]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let cred = ctx.config_dir.join("creds").join("kestrel.nsec");
    let sk_before = fs::read_to_string(&cred).unwrap();
    let pub_before = Keys::parse(&sk_before).unwrap().public_key();

    // Second run refuses; the message routes to attest-the-existing-key
    // (non-steering) and only then explains --force's cost.
    let out = run_keygen(&ctx, "kestrel", &[]);
    assert!(!out.status.success(), "second keygen must refuse");
    let se = stderr(&out);
    assert!(
        se.contains(&pub_before.to_hex()),
        "refusal must print the EXISTING pubkey: {se}"
    );
    assert!(
        se.contains("buzz-mint attest"),
        "refusal must route to attest: {se}"
    );
    let attest_pos = se.find("buzz-mint attest").unwrap();
    let force_pos = se.find("--force").unwrap();
    assert!(
        attest_pos < force_pos,
        "attest routing must come before the --force warning"
    );
    assert_eq!(
        fs::read_to_string(&cred).unwrap(),
        sk_before,
        "refusal must not touch the credential"
    );
    assert_no_secret_leak(&sk_before, &[&stdout(&out), &se]);

    // --force mints a NEW identity.
    let out = run_keygen(&ctx, "kestrel", &["--force"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let sk_after = fs::read_to_string(&cred).unwrap();
    assert_ne!(sk_after, sk_before, "--force must rotate the key");
    assert!(
        stderr(&out).contains("WARNING"),
        "rotation must warn loudly"
    );
    let pub_after = Keys::parse(&sk_after).unwrap().public_key();
    assert!(stdout(&out).contains(&pub_after.to_hex()));
}

#[test]
fn keygen_rejects_unsafe_names() {
    let ctx = ctx();
    for bad in ["", "../evil", "a b", "-lead", &"x".repeat(65)] {
        let out = run_keygen(&ctx, bad, &[]);
        assert!(!out.status.success(), "name {bad:?} must be rejected");
    }
    assert!(
        !ctx.config_dir.exists(),
        "rejected names must not create state"
    );
}

#[test]
fn keygen_pub_marker_matches_credential_after_force() {
    let ctx = ctx();
    fs::create_dir_all(&ctx.env_dir).unwrap();
    assert!(run_keygen(&ctx, "finch", &[]).status.success());
    assert!(run_keygen(&ctx, "finch", &["--force"]).status.success());

    let sk = fs::read_to_string(ctx.config_dir.join("creds").join("finch.nsec")).unwrap();
    let derived = Keys::parse(&sk).unwrap().public_key();
    let marker = fs::read_to_string(ctx.env_dir.join("finch.pub")).unwrap();
    assert_eq!(
        marker.trim(),
        derived.to_hex(),
        "pub marker must track the live credential"
    );
    let _: PublicKey = PublicKey::from_hex(marker.trim()).unwrap();
}
