//! Full virtual-mint pipeline e2e against a LOCAL relay (no prod relay is
//! touched — #2690's flag flip has not happened, so nothing may be published
//! to wss://buzz.interleaven.com from tests).
//!
//! Proves, against a relay running `BUZZ_REQUIRE_RELAY_MEMBERSHIP=true` +
//! `BUZZ_ALLOW_NIP_OA_AUTH=true`:
//!
//! 1. throwaway OWNER enrolled as relay member (direct DB seed, same pattern
//!    as buzz-test-client's e2e suite);
//! 2. owner creates a channel over the NIP-98 bridge;
//! 3. `buzz-provision host-init` → `buzz-mint new` (owner key via stdin) →
//!    `buzz-provision agent --no-systemd` — the real binaries end-to-end;
//! 4. the agent's kind:0 and kind:10100 are accepted (admission necessarily
//!    ViaOwner: the agent is asserted NOT to be a relay member) and read
//!    back over the bridge with agent NIP-98 auth + x-auth-tag;
//! 5. the owner-pre-signed kind:9000 outcome is captured EXPLICITLY: if the
//!    relay refuses host-side third-party submission (ingest requires
//!    event.pubkey == authenticated identity), the test proves the refusal
//!    is submitter-identity-shaped by re-submitting the SAME event with
//!    owner NIP-98 auth and asserting acceptance.
//!
//! # Running
//!
//! ```text
//! docker compose up -d postgres redis        # repo root
//! cargo build -p buzz-relay -p buzz-mint -p buzz-provision
//! cargo test -p buzz-provision --test e2e_local_relay -- --ignored --nocapture
//! ```
//!
//! Ports used: 3033 (relay), 8098 (health), 9203 (metrics). Systemd is never
//! touched (`--no-systemd`); the throwaway agent exists only in the local DB.

use std::fs;
use std::io::Write as _;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use base64::Engine;
use nostr::{Event, EventBuilder, JsonUtil, Keys, Kind, Tag};

const RELAY_PORT: u16 = 3033;
const HEALTH_PORT: u16 = 8098;
const METRICS_PORT: u16 = 9203;

fn relay_host() -> String {
    format!("localhost:{RELAY_PORT}")
}

fn http_base() -> String {
    format!("http://{}", relay_host())
}

fn database_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://buzz:buzz_dev@localhost:5432/buzz".to_string())
}

fn target_debug_dir() -> PathBuf {
    // current_exe = target/debug/deps/e2e_local_relay-<hash>
    let mut p = std::env::current_exe().expect("current_exe");
    p.pop(); // deps/
    p.pop(); // debug/
    p
}

struct RelayGuard(Child);

impl Drop for RelayGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn spawn_relay(tmp: &std::path::Path, owner_pubkey_hex: &str) -> RelayGuard {
    let relay_bin = target_debug_dir().join("buzz-relay");
    assert!(
        relay_bin.exists(),
        "buzz-relay binary missing at {} — run `cargo build -p buzz-relay` first",
        relay_bin.display()
    );
    // NIP-43 closed relay preconditions: a bootstrap owner pubkey and a
    // stable relay signing key (both throwaway here).
    let relay_sk = Keys::generate().secret_key().to_secret_hex();
    let child = Command::new(&relay_bin)
        .current_dir(tmp)
        .env("BUZZ_BIND_ADDR", format!("127.0.0.1:{RELAY_PORT}"))
        .env("BUZZ_HEALTH_PORT", HEALTH_PORT.to_string())
        .env("BUZZ_METRICS_PORT", METRICS_PORT.to_string())
        .env("RELAY_URL", format!("ws://{}", relay_host()))
        .env("DATABASE_URL", database_url())
        .env("BUZZ_AUTO_MIGRATE", "1")
        .env("BUZZ_REQUIRE_RELAY_MEMBERSHIP", "true")
        .env("BUZZ_ALLOW_NIP_OA_AUTH", "true")
        .env("RELAY_OWNER_PUBKEY", owner_pubkey_hex)
        .env("BUZZ_RELAY_PRIVATE_KEY", relay_sk)
        // No MinIO in this e2e: skip the S3 git conformance probe (git is
        // out of scope here; only /events + /query are exercised).
        .env("BUZZ_GIT_CONFORMANCE_PROBE", "false")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn buzz-relay");
    RelayGuard(child)
}

async fn wait_for_relay(client: &reqwest::Client) {
    let deadline = Instant::now() + Duration::from_secs(90);
    loop {
        if let Ok(resp) = client.get(format!("{}/health", http_base())).send().await {
            if resp.status().is_success() {
                return;
            }
        }
        assert!(
            Instant::now() < deadline,
            "relay did not become healthy on {} within 90s",
            http_base()
        );
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

/// NIP-98 Authorization header (same tag shape the CLI and provision use).
fn nip98_header(keys: &Keys, method: &str, url: &str, body: &[u8]) -> String {
    use nostr::hashes::{sha256, Hash};
    let tags = vec![
        Tag::parse(["u", url]).unwrap(),
        Tag::parse(["method", method]).unwrap(),
        Tag::parse(["payload", &sha256::Hash::hash(body).to_string()]).unwrap(),
        Tag::parse(["nonce", &uuid::Uuid::new_v4().to_string()]).unwrap(),
    ];
    let event = EventBuilder::new(Kind::Custom(27235), "")
        .tags(tags)
        .sign_with_keys(keys)
        .unwrap();
    format!(
        "Nostr {}",
        base64::engine::general_purpose::STANDARD.encode(event.as_json())
    )
}

/// POST a signed event to /events. Returns (status, body).
async fn post_event(
    client: &reqwest::Client,
    signer: &Keys,
    auth_tag: Option<&str>,
    event: &Event,
) -> (u16, String) {
    let url = format!("{}/events", http_base());
    let body = event.as_json().into_bytes();
    let mut req = client
        .post(&url)
        .header("Authorization", nip98_header(signer, "POST", &url, &body))
        .header("Content-Type", "application/json");
    if let Some(tag) = auth_tag {
        req = req.header("x-auth-tag", tag);
    }
    let resp = req.body(body).send().await.expect("POST /events");
    let status = resp.status().as_u16();
    (status, resp.text().await.unwrap_or_default())
}

/// POST /query with NIP-98 auth; returns the JSON array of events.
async fn query(
    client: &reqwest::Client,
    signer: &Keys,
    auth_tag: Option<&str>,
    filter: serde_json::Value,
) -> Vec<serde_json::Value> {
    let url = format!("{}/query", http_base());
    let body = serde_json::to_vec(&[filter]).unwrap();
    let mut req = client
        .post(&url)
        .header("Authorization", nip98_header(signer, "POST", &url, &body))
        .header("Content-Type", "application/json");
    if let Some(tag) = auth_tag {
        req = req.header("x-auth-tag", tag);
    }
    let resp = req.body(body).send().await.expect("POST /query");
    assert!(
        resp.status().is_success(),
        "query failed: {} {}",
        resp.status(),
        resp.text().await.unwrap_or_default()
    );
    resp.json().await.expect("query response JSON")
}

fn run_checked(cmd: &mut Command, what: &str) -> String {
    let out = cmd.output().unwrap_or_else(|e| panic!("spawn {what}: {e}"));
    assert!(
        out.status.success(),
        "{what} failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[tokio::test]
#[ignore] // needs docker postgres+redis and built buzz-relay/buzz-mint binaries
async fn full_virtual_mint_pipeline_against_local_relay() {
    let tmp = tempfile::tempdir().unwrap();
    // Throwaway owner generated first: the relay bootstraps it as the
    // deployment owner (RELAY_OWNER_PUBKEY) of the RELAY_URL-host community.
    let owner = Keys::generate();
    let _relay = spawn_relay(tmp.path(), &owner.public_key().to_hex());
    let client = reqwest::Client::new();
    wait_for_relay(&client).await;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url())
        .await
        .expect("connect Postgres (docker compose up -d postgres redis?)");
    let community_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO communities (id, host) VALUES ($1, $2) ON CONFLICT (lower(host)) DO NOTHING",
    )
    .bind(community_id)
    .bind(relay_host())
    .execute(&pool)
    .await
    .expect("seed community");
    let community_id: uuid::Uuid =
        sqlx::query_scalar("SELECT id FROM communities WHERE lower(host) = lower($1)")
            .bind(relay_host())
            .fetch_one(&pool)
            .await
            .expect("lookup community");
    sqlx::query(
        "INSERT INTO relay_members (community_id, pubkey, role, added_by) VALUES ($1,$2,'owner',NULL) \
         ON CONFLICT (community_id, pubkey) DO UPDATE SET role='owner', updated_at=now()",
    )
    .bind(community_id)
    .bind(owner.public_key().to_hex())
    .execute(&pool)
    .await
    .expect("enroll throwaway owner");

    // ── Owner creates a channel over the bridge. ──
    let channel = uuid::Uuid::new_v4();
    let create = EventBuilder::new(Kind::Custom(9007), "")
        .tags(vec![
            Tag::parse(["h", &channel.to_string()]).unwrap(),
            Tag::parse(["name", &format!("mint-e2e-{channel}")]).unwrap(),
            Tag::parse(["channel_type", "stream"]).unwrap(),
            Tag::parse(["visibility", "open"]).unwrap(),
        ])
        .sign_with_keys(&owner)
        .unwrap();
    let (status, body) = post_event(&client, &owner, None, &create).await;
    assert_eq!(status, 200, "channel create failed: {body}");

    // ── host-init → mint → provision, with the real binaries. ──
    let provision_bin = env!("CARGO_BIN_EXE_buzz-provision");
    let mint_bin = target_debug_dir().join("buzz-mint");
    assert!(
        mint_bin.exists(),
        "buzz-mint binary missing at {} — run `cargo build -p buzz-mint` first",
        mint_bin.display()
    );
    let cfg_dir = tmp.path().join("cfg");
    let env_dir = tmp.path().join("env");
    let bundle_out = tmp.path().join("out");
    fs::create_dir_all(&bundle_out).unwrap();

    let so = run_checked(
        Command::new(provision_bin)
            .args(["host-init", "--allow-owner", &owner.public_key().to_hex()])
            .arg("--config-dir")
            .arg(&cfg_dir),
        "host-init",
    );
    let host_pubkey = so
        .lines()
        .find_map(|l| l.strip_prefix("host pubkey (hex):"))
        .expect("host pubkey line")
        .trim()
        .to_owned();

    let agent_name = format!(
        "e2e-mint-{}",
        &uuid::Uuid::new_v4().simple().to_string()[..8]
    );
    let mut mint = Command::new(&mint_bin)
        .args([
            "new",
            "--name",
            &agent_name,
            "--host",
            &host_pubkey,
            "--channel",
            &channel.to_string(),
            "--relay-url",
            &format!("ws://{}", relay_host()),
            "--harness-cmd",
            "/usr/bin/true",
        ])
        .arg("--out")
        .arg(&bundle_out)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn buzz-mint");
    mint.stdin
        .take()
        .unwrap()
        .write_all(owner.secret_key().to_secret_hex().as_bytes())
        .unwrap();
    let mint_out = mint.wait_with_output().unwrap();
    assert!(
        mint_out.status.success(),
        "buzz-mint failed: {}",
        String::from_utf8_lossy(&mint_out.stderr)
    );
    let bundle_dir = bundle_out.join(format!("{agent_name}-mint"));
    let manifest: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(bundle_dir.join("manifest.json")).unwrap())
            .unwrap();
    let agent_pubkey = manifest["agent_pubkey"].as_str().unwrap().to_owned();
    let attestation = manifest["attestation"].as_str().unwrap().to_owned();
    let presigned_9000 = Event::from_json(
        fs::read_to_string(
            bundle_dir
                .join("events")
                .join(format!("9000-{channel}.json")),
        )
        .unwrap(),
    )
    .unwrap();

    // ── Provision (no systemd). Capture the outcome either way. ──
    let prov_out = Command::new(provision_bin)
        .arg("agent")
        .arg(&bundle_dir)
        .arg("--no-systemd")
        .arg("--config-dir")
        .arg(&cfg_dir)
        .arg("--env-dir")
        .arg(&env_dir)
        .output()
        .expect("spawn buzz-provision agent");
    let prov_stdout = String::from_utf8_lossy(&prov_out.stdout).into_owned();
    let prov_stderr = String::from_utf8_lossy(&prov_out.stderr).into_owned();
    println!("--- provision stdout ---\n{prov_stdout}\n--- provision stderr ---\n{prov_stderr}");

    // kind:0 and kind:10100 must have been published before any 9000 —
    // ruled ordering — so they are asserted unconditionally.
    assert!(
        prov_stdout.contains("published kind:0"),
        "kind:0 was not published"
    );
    assert!(
        prov_stdout.contains("published kind:10100"),
        "kind:10100 was not published"
    );
    assert!(
        !bundle_dir.join("sealed.json").exists(),
        "sealed.json must be shredded after credential verification"
    );

    // ── The agent must NOT be a relay member: acceptance proves ViaOwner. ──
    let direct_member: Option<String> =
        sqlx::query_scalar("SELECT role FROM relay_members WHERE community_id=$1 AND pubkey=$2")
            .bind(community_id)
            .bind(&agent_pubkey)
            .fetch_optional(&pool)
            .await
            .expect("member check");
    assert!(
        direct_member.is_none(),
        "agent must not be a direct relay member — admission must be ViaOwner"
    );

    // ── Read the identity back over the bridge, AS THE AGENT (NIP-98 +
    // x-auth-tag: the virtual-membership read path). ──
    let cred_hex = fs::read_to_string(cfg_dir.join("creds").join(format!("{agent_name}.nsec")))
        .expect("credential written");
    let agent_keys = Keys::parse(&cred_hex).expect("credential parses");
    assert_eq!(agent_keys.public_key().to_hex(), agent_pubkey);

    let kind0 = query(
        &client,
        &agent_keys,
        Some(&attestation),
        serde_json::json!({"kinds": [0], "authors": [agent_pubkey], "limit": 1}),
    )
    .await;
    assert_eq!(kind0.len(), 1, "kind:0 must be readable back");

    let k10100 = query(
        &client,
        &agent_keys,
        Some(&attestation),
        serde_json::json!({"kinds": [10100], "authors": [agent_pubkey], "limit": 1}),
    )
    .await;
    assert_eq!(k10100.len(), 1, "kind:10100 must be readable back");
    let content: serde_json::Value =
        serde_json::from_str(k10100[0]["content"].as_str().unwrap()).unwrap();
    assert_eq!(content["channel_add_policy"], "owner_only");
    assert_eq!(
        content["host"], host_pubkey,
        "lineage anchor must be the host pubkey"
    );
    assert_eq!(content["agent_type"], "agent");
    assert_eq!(content["status"], "online");
    assert_eq!(content["channel_ids"][0], channel.to_string());

    // ── kind:9000 outcome, captured explicitly. ──
    if prov_out.status.success() {
        println!("RESULT: relay ACCEPTED host-side submission of the owner-pre-signed 9000");
        let member_row: Option<String> = sqlx::query_scalar(
            "SELECT role FROM channel_members WHERE community_id=$1 AND channel_id=$2 AND pubkey=$3",
        )
        .bind(community_id)
        .bind(channel)
        .bind(&agent_pubkey)
        .fetch_optional(&pool)
        .await
        .unwrap_or(None);
        println!("agent channel role after provision: {member_row:?}");
    } else {
        assert!(
            prov_stderr.contains("publishing kind:9000"),
            "provision failed somewhere OTHER than the 9000 step: {prov_stderr}"
        );
        println!(
            "RESULT: relay REFUSED host-side (agent-authenticated) submission of the \
             owner-pre-signed 9000 — expected from ingest's event.pubkey == auth-identity rule"
        );
        // Control: the SAME pre-signed event, submitted with OWNER NIP-98
        // auth, must be accepted — proving the event is valid and fresh and
        // the refusal is purely submitter-identity-shaped.
        let (status, body) = post_event(&client, &owner, None, &presigned_9000).await;
        assert_eq!(
            status, 200,
            "owner-authenticated submission of the same 9000 must succeed: {body}"
        );
        let parsed: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(
            parsed["accepted"], true,
            "owner-authenticated 9000 must be accepted: {body}"
        );
        println!("CONTROL: same 9000 accepted when submitted with owner NIP-98 auth");
    }
}
