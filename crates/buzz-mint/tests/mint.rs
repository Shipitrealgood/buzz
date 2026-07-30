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

// ─── attest (Design A) ───────────────────────────────────────────────────────

mod attest {
    use super::*;
    use std::io::{BufRead, BufReader, Read as _};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    /// One captured bridge request: request line, Authorization header, body.
    pub struct Req {
        pub line: String,
        pub authorization: String,
        pub body: String,
    }

    /// Minimal HTTP/1.1 mock relay: answers every request with
    /// `response_json` (200) and records what it saw. The listener thread
    /// runs until process exit (test-scoped, one per test).
    fn spawn_mock_relay(response_json: &'static str) -> (String, Arc<Mutex<Vec<Req>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let log: Arc<Mutex<Vec<Req>>> = Arc::new(Mutex::new(Vec::new()));
        let log2 = log.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                let mut reader = BufReader::new(stream);
                let mut line = String::new();
                if reader.read_line(&mut line).is_err() || line.is_empty() {
                    continue;
                }
                let mut authorization = String::new();
                let mut content_length = 0usize;
                loop {
                    let mut header = String::new();
                    if reader.read_line(&mut header).is_err() {
                        break;
                    }
                    let h = header.trim_end();
                    if h.is_empty() {
                        break;
                    }
                    // hyper lowercases header names on the wire.
                    let (name, value) = h.split_once(':').unwrap_or((h, ""));
                    match name.to_ascii_lowercase().as_str() {
                        "authorization" => authorization = value.trim().to_owned(),
                        "content-length" => content_length = value.trim().parse().unwrap_or(0),
                        _ => {}
                    }
                }
                let mut body = vec![0u8; content_length];
                if reader.read_exact(&mut body).is_err() {
                    continue;
                }
                log2.lock().unwrap().push(Req {
                    line: line.trim_end().to_owned(),
                    authorization,
                    body: String::from_utf8_lossy(&body).into_owned(),
                });
                let mut stream = reader.into_inner();
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
                     Content-Length: {}\r\nConnection: close\r\n\r\n{}",
                    response_json.len(),
                    response_json
                );
                use std::io::Write as _;
                let _ = stream.write_all(resp.as_bytes());
                let _ = stream.flush();
            }
        });
        (format!("ws://127.0.0.1:{port}"), log)
    }

    /// Decode a `Nostr <base64>` NIP-98 header back into its 27235 event.
    fn decode_nip98(header: &str) -> Event {
        use base64::Engine as _;
        let b64 = header.strip_prefix("Nostr ").expect("Nostr scheme");
        let json = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .unwrap();
        Event::from_json(String::from_utf8(json).unwrap()).unwrap()
    }

    fn run_attest(
        owner: &Owner,
        agent_pubkey: &PublicKey,
        host_pubkey: &PublicKey,
        relay_url: &str,
        out_dir: &Path,
        channels: &[&str],
    ) -> Output {
        let mut args: Vec<String> = vec![
            "attest".into(),
            "--name".into(),
            "hostborn".into(),
            "--agent".into(),
            agent_pubkey.to_hex(),
            "--host".into(),
            host_pubkey.to_hex(),
            "--relay-url".into(),
            relay_url.into(),
            "--harness-cmd".into(),
            "/usr/bin/true".into(),
            "--out".into(),
            out_dir.to_str().unwrap().into(),
            "--owner-key-file".into(),
            owner.key_file.to_str().unwrap().into(),
        ];
        for c in channels {
            args.push("--channel".into());
            args.push((*c).into());
        }
        run(&args.iter().map(String::as_str).collect::<Vec<_>>())
    }

    #[test]
    fn attest_publishes_fresh_owner_authed_9000s_and_emits_manifest_only_bundle() {
        let tmp = tempfile::tempdir().unwrap();
        let owner = owner_key_file(tmp.path(), 0o600, true);
        let agent = Keys::generate(); // stands in for a host-born key
        let host = Keys::generate();
        let (relay_url, log) = spawn_mock_relay(r#"{"accepted":true,"event_id":"deadbeef"}"#);

        let before = nostr::Timestamp::now().as_secs();
        let out = run_attest(
            &owner,
            &agent.public_key(),
            &host.public_key(),
            &relay_url,
            tmp.path(),
            &[CHAN_A, CHAN_B],
        );
        let so = stdout(&out);
        let se = stderr(&out);
        assert!(out.status.success(), "attest failed: {se}");

        // ── Bundle: manifest.json ONLY (A-mode is 100% public data).
        let bundle = tmp.path().join("hostborn-mint");
        assert!(bundle.join("manifest.json").exists());
        assert!(!bundle.join("sealed.json").exists(), "A-mode: no seal");
        assert!(!bundle.join("events").exists(), "A-mode: no events dir");
        let entries: Vec<_> = fs::read_dir(&bundle).unwrap().collect();
        assert_eq!(entries.len(), 1, "manifest must be the only member");

        // ── Manifest shape.
        let manifest: Value =
            serde_json::from_str(&fs::read_to_string(bundle.join("manifest.json")).unwrap())
                .unwrap();
        assert_eq!(manifest["mode"], "a");
        assert_eq!(manifest["agent_pubkey"], agent.public_key().to_hex());
        assert_eq!(manifest["owner_pubkey"], owner.keys.public_key().to_hex());
        assert_eq!(manifest["host_pubkey"], host.public_key().to_hex());
        assert_eq!(manifest["config"]["relay_url"], relay_url);
        assert_eq!(manifest["config"]["channels"].as_array().unwrap().len(), 2);

        // ── Attestation verifies for the EXTERNAL pubkey; attester == owner.
        let attester = buzz_sdk::nip_oa::verify_auth_tag(
            manifest["attestation"].as_str().unwrap(),
            &agent.public_key(),
        )
        .expect("attestation must verify for the external agent pubkey");
        assert_eq!(attester, owner.keys.public_key());

        // ── Published 9000s: two POSTs, owner-NIP-98-authenticated, fresh.
        let reqs = log.lock().unwrap();
        assert_eq!(reqs.len(), 2, "one POST per channel");
        let mut seen: Vec<String> = Vec::new();
        for req in reqs.iter() {
            assert!(req.line.starts_with("POST /events"), "line: {}", req.line);
            let auth = decode_nip98(&req.authorization);
            auth.verify().expect("valid NIP-98 event");
            assert_eq!(auth.kind.as_u16(), 27235);
            assert_eq!(
                auth.pubkey,
                owner.keys.public_key(),
                "9000s must be OWNER-authenticated (the relay refuses anyone else)"
            );
            let event = Event::from_json(&req.body).unwrap();
            event.verify().expect("valid 9000");
            assert_eq!(event.kind.as_u16(), 9000);
            assert_eq!(event.pubkey, owner.keys.public_key());
            let now = nostr::Timestamp::now().as_secs();
            let ts = event.created_at.as_secs();
            assert!(
                ts >= before && ts <= now,
                "created_at must be FRESH (signed at attest time): {ts}"
            );
            let tag = |name: &str| -> Option<String> {
                event.tags.iter().find_map(|t| {
                    let s = t.as_slice();
                    (s.first().map(String::as_str) == Some(name))
                        .then(|| s.get(1).cloned())
                        .flatten()
                })
            };
            assert_eq!(
                tag("p").as_deref(),
                Some(agent.public_key().to_hex().as_str())
            );
            assert_eq!(tag("role").as_deref(), Some("bot"));
            seen.push(tag("h").unwrap());
        }
        seen.sort();
        let mut want = vec![CHAN_A.to_string(), CHAN_B.to_string()];
        want.sort();
        assert_eq!(seen, want, "one add per requested channel");
        drop(reqs);

        assert!(
            so.contains("ACCEPTED"),
            "per-channel outcome reported: {so}"
        );

        // ── No secret anywhere (owner sk is the only secret in this flow).
        let owner_sk = Zeroizing::new(owner.keys.secret_key().to_secret_hex());
        let manifest_raw = fs::read_to_string(bundle.join("manifest.json")).unwrap();
        assert_no_secret_leak(&owner_sk, &[&so, &se, &manifest_raw]);

        // ── inspect handles the A-mode bundle.
        let out = run(&["inspect", bundle.to_str().unwrap()]);
        assert!(out.status.success(), "inspect: {}", stderr(&out));
        let so = stdout(&out);
        assert!(so.contains("inspect: OK"), "got: {so}");
        assert!(so.contains("mode:         a"), "mode printed: {so}");
    }

    #[test]
    fn attest_rejected_add_is_warning_not_mint_failure() {
        let tmp = tempfile::tempdir().unwrap();
        let owner = owner_key_file(tmp.path(), 0o600, false);
        let agent = Keys::generate();
        let host = Keys::generate();
        let (relay_url, _log) =
            spawn_mock_relay(r#"{"accepted":false,"message":"channel is archived"}"#);

        let out = run_attest(
            &owner,
            &agent.public_key(),
            &host.public_key(),
            &relay_url,
            tmp.path(),
            &[CHAN_A],
        );
        assert!(
            out.status.success(),
            "rejected add must NOT fail the mint: {}",
            stderr(&out)
        );
        let se = stderr(&out);
        assert!(se.contains("NOT accepted"), "warning surfaced: {se}");
        assert!(
            se.contains("channel is archived"),
            "relay reason echoed: {se}"
        );
        assert!(
            tmp.path()
                .join("hostborn-mint")
                .join("manifest.json")
                .exists(),
            "bundle must still be written"
        );
    }

    #[test]
    fn attest_unreachable_relay_warns_but_still_mints() {
        let tmp = tempfile::tempdir().unwrap();
        let owner = owner_key_file(tmp.path(), 0o600, false);
        let agent = Keys::generate();
        let host = Keys::generate();

        let out = run_attest(
            &owner,
            &agent.public_key(),
            &host.public_key(),
            "ws://127.0.0.1:1",
            tmp.path(),
            &[CHAN_A],
        );
        assert!(
            out.status.success(),
            "unreachable relay must NOT fail the mint: {}",
            stderr(&out)
        );
        assert!(stderr(&out).contains("NOT accepted"));
        assert!(tmp
            .path()
            .join("hostborn-mint")
            .join("manifest.json")
            .exists());
    }

    #[test]
    fn attest_validates_agent_hex_without_echoing_the_value() {
        let tmp = tempfile::tempdir().unwrap();
        let owner = owner_key_file(tmp.path(), 0o600, false);
        let host = Keys::generate();

        // A pasted SECRET key is 64 hex chars too — the error must not echo.
        let sk_shaped = Keys::generate().secret_key().to_secret_hex().to_uppercase();
        let out = run(&[
            "attest",
            "--name",
            "oops",
            "--agent",
            &sk_shaped,
            "--host",
            &host.public_key().to_hex(),
            "--relay-url",
            "ws://127.0.0.1:1",
            "--owner-key-file",
            owner.key_file.to_str().unwrap(),
        ]);
        assert!(!out.status.success());
        let se = stderr(&out);
        assert!(se.contains("--agent"), "names the argument: {se}");
        assert!(se.contains("64 lowercase hex"), "explains the format: {se}");
        assert!(!se.contains(&sk_shaped), "must not echo the value: {se}");
        assert!(
            !tmp.path().join("oops-mint").exists(),
            "no bundle on refusal"
        );

        // clap-level redaction: key-shaped junk argv never reaches stderr.
        let nsec = "nsec1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq";
        let out = run(&["attest", nsec]);
        assert!(!out.status.success());
        let se = stderr(&out);
        assert!(!se.contains(nsec), "clap error must be redacted: {se}");
        assert!(se.contains("<redacted-key-material>"), "got: {se}");
    }

    #[test]
    fn attest_refuses_owner_host_agent_identity_confusion() {
        let tmp = tempfile::tempdir().unwrap();
        let owner = owner_key_file(tmp.path(), 0o600, false);
        let host = Keys::generate();

        // --agent == owner pubkey: an agent must be its own key.
        let out = run(&[
            "attest",
            "--name",
            "selfie",
            "--agent",
            &owner.keys.public_key().to_hex(),
            "--host",
            &host.public_key().to_hex(),
            "--relay-url",
            "ws://127.0.0.1:1",
            "--owner-key-file",
            owner.key_file.to_str().unwrap(),
        ]);
        assert!(!out.status.success());
        assert!(stderr(&out).contains("OWNER"), "got: {}", stderr(&out));

        // --agent == host pubkey: pasted the wrong printed value.
        let out = run(&[
            "attest",
            "--name",
            "hostkey",
            "--agent",
            &host.public_key().to_hex(),
            "--host",
            &host.public_key().to_hex(),
            "--relay-url",
            "ws://127.0.0.1:1",
            "--owner-key-file",
            owner.key_file.to_str().unwrap(),
        ]);
        assert!(!out.status.success());
        assert!(stderr(&out).contains("HOST"), "got: {}", stderr(&out));
    }
}
