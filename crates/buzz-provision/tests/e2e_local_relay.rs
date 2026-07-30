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
//! A second test runs the full **Design-A ceremony** (the default mint,
//! T4b #2694): `keygen` → `buzz-mint attest` (publishes the 9000s LIVE,
//! owner-authenticated, fresh timestamps) → `buzz-provision agent` on the
//! manifest-only bundle (kind:0 + 10100 ViaOwner, membership VERIFIED via
//! bridge read, nothing owner-signed submitted host-side) — fully green
//! including the adds.
//!
//! # Running
//!
//! ```text
//! docker compose up -d postgres redis        # repo root
//! cargo build -p buzz-relay -p buzz-mint -p buzz-provision
//! cargo test -p buzz-provision --test e2e_local_relay -- --ignored --nocapture
//! ```
//!
//! A third test runs the full **T5 retire lifecycle** (#2688): keygen →
//! attest → provision → `buzz-provision retire` (channels left via 9022,
//! 10100 tombstoned not-mentionable, credential shredded, kind:0 left
//! ALIVE) → converged re-run → `buzz-mint retire` kind:9035 archive,
//! accepted through the REAL owner consent path (the owner is seeded as a
//! plain relay `member`, so the relay cannot take the admin shortcut).
//!
//! Ports: B test 3033/8098/9203, A test 3034/8099/9204, retire test
//! 3035/8100/9205 (disjoint so the parallel runner cannot collide them).
//! Systemd is never touched (`--no-systemd`); the throwaway agents exist
//! only in the local DB.

use std::fs;
use std::io::Write as _;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use base64::Engine;
use nostr::{Event, EventBuilder, JsonUtil, Keys, Kind, Tag};

/// Ports for one relay instance. The two e2e tests use disjoint sets so the
/// default parallel test runner cannot collide them (each seeds its own
/// community keyed by its own `localhost:<port>` host).
#[derive(Copy, Clone)]
struct Ports {
    relay: u16,
    health: u16,
    metrics: u16,
}

/// B-ceremony (sealed transit) test ports.
const PORTS_B: Ports = Ports {
    relay: 3033,
    health: 8098,
    metrics: 9203,
};

/// A-ceremony (keygen → attest → provision) test ports.
const PORTS_A: Ports = Ports {
    relay: 3034,
    health: 8099,
    metrics: 9204,
};

/// Retire-lifecycle (T5) test ports.
const PORTS_R: Ports = Ports {
    relay: 3035,
    health: 8100,
    metrics: 9205,
};

fn relay_host(ports: Ports) -> String {
    format!("localhost:{}", ports.relay)
}

fn http_base(ports: Ports) -> String {
    format!("http://{}", relay_host(ports))
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

fn spawn_relay(tmp: &std::path::Path, owner_pubkey_hex: &str, ports: Ports) -> RelayGuard {
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
        .env("BUZZ_BIND_ADDR", format!("127.0.0.1:{}", ports.relay))
        .env("BUZZ_HEALTH_PORT", ports.health.to_string())
        .env("BUZZ_METRICS_PORT", ports.metrics.to_string())
        .env("RELAY_URL", format!("ws://{}", relay_host(ports)))
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

async fn wait_for_relay(client: &reqwest::Client, ports: Ports) {
    let deadline = Instant::now() + Duration::from_secs(90);
    loop {
        if let Ok(resp) = client
            .get(format!("{}/health", http_base(ports)))
            .send()
            .await
        {
            if resp.status().is_success() {
                return;
            }
        }
        assert!(
            Instant::now() < deadline,
            "relay did not become healthy on {} within 90s",
            http_base(ports)
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
    ports: Ports,
    signer: &Keys,
    auth_tag: Option<&str>,
    event: &Event,
) -> (u16, String) {
    let url = format!("{}/events", http_base(ports));
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
    ports: Ports,
    signer: &Keys,
    auth_tag: Option<&str>,
    filter: serde_json::Value,
) -> Vec<serde_json::Value> {
    let url = format!("{}/query", http_base(ports));
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

/// Connect Postgres, seed this relay instance's community, and enroll the
/// throwaway owner as a relay member with the given role (direct DB seed,
/// same pattern as buzz-test-client's e2e suite). Returns (pool, community_id).
///
/// Role matters for T5: an owner seeded as relay `owner`/`admin` short-cuts
/// the 9035 consent check to the Admin path; seeding `member` forces the
/// REAL owner consent path (`verify_owner_consent`: fresh auth tag + live
/// kind:0 attestation).
async fn seed_community_and_member(
    ports: Ports,
    owner: &Keys,
    role: &str,
) -> (sqlx::PgPool, uuid::Uuid) {
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
    .bind(relay_host(ports))
    .execute(&pool)
    .await
    .expect("seed community");
    let community_id: uuid::Uuid =
        sqlx::query_scalar("SELECT id FROM communities WHERE lower(host) = lower($1)")
            .bind(relay_host(ports))
            .fetch_one(&pool)
            .await
            .expect("lookup community");
    sqlx::query(
        "INSERT INTO relay_members (community_id, pubkey, role, added_by) VALUES ($1,$2,$3,NULL) \
         ON CONFLICT (community_id, pubkey) DO UPDATE SET role=$3, updated_at=now()",
    )
    .bind(community_id)
    .bind(owner.public_key().to_hex())
    .bind(role)
    .execute(&pool)
    .await
    .expect("enroll throwaway owner");
    (pool, community_id)
}

/// Owner creates a fresh open stream channel over the bridge.
async fn create_channel(client: &reqwest::Client, ports: Ports, owner: &Keys) -> uuid::Uuid {
    let channel = uuid::Uuid::new_v4();
    let create = EventBuilder::new(Kind::Custom(9007), "")
        .tags(vec![
            Tag::parse(["h", &channel.to_string()]).unwrap(),
            Tag::parse(["name", &format!("mint-e2e-{channel}")]).unwrap(),
            Tag::parse(["channel_type", "stream"]).unwrap(),
            Tag::parse(["visibility", "open"]).unwrap(),
        ])
        .sign_with_keys(owner)
        .unwrap();
    let (status, body) = post_event(client, ports, owner, None, &create).await;
    assert_eq!(status, 200, "channel create failed: {body}");
    channel
}

#[tokio::test]
#[ignore] // needs docker postgres+redis and built buzz-relay/buzz-mint binaries
async fn full_virtual_mint_pipeline_against_local_relay() {
    let tmp = tempfile::tempdir().unwrap();
    let ports = PORTS_B;
    // Throwaway owner generated first: the relay bootstraps it as the
    // deployment owner (RELAY_OWNER_PUBKEY) of the RELAY_URL-host community.
    let owner = Keys::generate();
    let _relay = spawn_relay(tmp.path(), &owner.public_key().to_hex(), ports);
    let client = reqwest::Client::new();
    wait_for_relay(&client, ports).await;
    let (pool, community_id) = seed_community_and_member(ports, &owner, "owner").await;

    // ── Owner creates a channel over the bridge. ──
    let channel = create_channel(&client, ports, &owner).await;

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
            &format!("ws://{}", relay_host(ports)),
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
        ports,
        &agent_keys,
        Some(&attestation),
        serde_json::json!({"kinds": [0], "authors": [agent_pubkey], "limit": 1}),
    )
    .await;
    assert_eq!(kind0.len(), 1, "kind:0 must be readable back");

    let k10100 = query(
        &client,
        ports,
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
            "SELECT role::text FROM channel_members WHERE community_id=$1 AND channel_id=$2 \
             AND encode(pubkey, 'hex')=$3",
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
        let (status, body) = post_event(&client, ports, &owner, None, &presigned_9000).await;
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

/// Full Design-A ceremony (the DEFAULT mint, T4b #2694): keygen on the host
/// → owner attests the exported pubkey and publishes the 9000s DIRECTLY
/// (owner NIP-98, fresh timestamps) → provision consumes the manifest-only
/// bundle, publishes kind:0 + kind:10100 ViaOwner, and VERIFIES the channel
/// membership instead of submitting anything owner-signed.
///
/// This is the ceremony the B test's step 5 pinned empirically: the relay
/// refuses third-party submission of owner-signed events, so A goes fully
/// green — including the adds — because the owner is the submitter.
#[tokio::test]
#[ignore] // needs docker postgres+redis and built buzz-relay/buzz-mint binaries
async fn full_design_a_ceremony_against_local_relay() {
    let tmp = tempfile::tempdir().unwrap();
    let ports = PORTS_A;
    let owner = Keys::generate();
    let _relay = spawn_relay(tmp.path(), &owner.public_key().to_hex(), ports);
    let client = reqwest::Client::new();
    wait_for_relay(&client, ports).await;
    let (pool, community_id) = seed_community_and_member(ports, &owner, "owner").await;

    // ── Owner creates a channel over the bridge. ──
    let channel = create_channel(&client, ports, &owner).await;

    // ── host-init → keygen, on the "host" (real binaries, temp dirs). ──
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
        "e2e-akey-{}",
        &uuid::Uuid::new_v4().simple().to_string()[..8]
    );
    let so = run_checked(
        Command::new(provision_bin)
            .args(["keygen", "--name", &agent_name])
            .arg("--config-dir")
            .arg(&cfg_dir)
            .arg("--env-dir")
            .arg(&env_dir),
        "keygen",
    );
    let agent_pubkey = so
        .lines()
        .find_map(|l| l.strip_prefix("agent pubkey (hex):"))
        .expect("agent pubkey line")
        .trim()
        .to_owned();
    // keygen prints the exact attest command with agent + host filled in.
    assert!(
        so.contains(&format!("--agent {agent_pubkey}"))
            && so.contains(&format!("--host {host_pubkey}")),
        "keygen must print the filled-in attest command: {so}"
    );

    // ── Owner attests the exported pubkey; 9000s published LIVE. ──
    let mut attest = Command::new(&mint_bin)
        .args([
            "attest",
            "--name",
            &agent_name,
            "--agent",
            &agent_pubkey,
            "--host",
            &host_pubkey,
            "--channel",
            &channel.to_string(),
            "--relay-url",
            &format!("ws://{}", relay_host(ports)),
            "--harness-cmd",
            "/usr/bin/true",
        ])
        .arg("--out")
        .arg(&bundle_out)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn buzz-mint attest");
    attest
        .stdin
        .take()
        .unwrap()
        .write_all(owner.secret_key().to_secret_hex().as_bytes())
        .unwrap();
    let attest_out = attest.wait_with_output().unwrap();
    let attest_stdout = String::from_utf8_lossy(&attest_out.stdout).into_owned();
    let attest_stderr = String::from_utf8_lossy(&attest_out.stderr).into_owned();
    println!("--- attest stdout ---\n{attest_stdout}\n--- attest stderr ---\n{attest_stderr}");
    assert!(
        attest_out.status.success(),
        "buzz-mint attest failed: {attest_stderr}"
    );
    assert!(
        attest_stdout.contains("ACCEPTED"),
        "the owner-authenticated 9000 must be accepted live: {attest_stdout}"
    );

    // A-mode bundle: manifest.json ONLY.
    let bundle_dir = bundle_out.join(format!("{agent_name}-mint"));
    assert!(bundle_dir.join("manifest.json").exists());
    assert!(!bundle_dir.join("sealed.json").exists(), "A-mode: no seal");
    assert!(!bundle_dir.join("events").exists(), "A-mode: no events dir");
    let manifest: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(bundle_dir.join("manifest.json")).unwrap())
            .unwrap();
    assert_eq!(manifest["mode"], "a");
    assert_eq!(manifest["agent_pubkey"], agent_pubkey);
    let attestation = manifest["attestation"].as_str().unwrap().to_owned();

    // The add landed: channel_members row with role bot, added by the owner.
    // (channel_members.pubkey is bytea, unlike relay_members.pubkey.)
    let member_role: Option<String> = sqlx::query_scalar(
        "SELECT role::text FROM channel_members WHERE community_id=$1 AND channel_id=$2 \
         AND encode(pubkey, 'hex')=$3",
    )
    .bind(community_id)
    .bind(channel)
    .bind(&agent_pubkey)
    .fetch_optional(&pool)
    .await
    .expect("channel member check");
    assert_eq!(
        member_role.as_deref(),
        Some("bot"),
        "owner-published 9000 must have materialized the bot membership"
    );

    // ── Provision from the manifest-only bundle (no systemd). Must go
    // fully green: nothing owner-signed is submitted host-side. ──
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
    assert!(
        prov_out.status.success(),
        "A-mode provision must be fully green: {prov_stderr}"
    );
    assert!(prov_stdout.contains("published kind:0"));
    assert!(prov_stdout.contains("published kind:10100"));
    assert!(
        prov_stdout.contains(&format!("membership verified: channel {channel}")),
        "provision must verify the owner's add via bridge read: {prov_stdout}"
    );
    assert!(
        !prov_stdout.contains("kind:9000 bot add (owner-signed, published verbatim)"),
        "A-mode must not submit any owner-signed event host-side"
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

    // ── Read the identity back over the bridge AS THE AGENT (NIP-98 +
    // x-auth-tag), signing with the keygen-born credential. ──
    let cred_hex = fs::read_to_string(cfg_dir.join("creds").join(format!("{agent_name}.nsec")))
        .expect("credential written by keygen");
    let agent_keys = Keys::parse(&cred_hex).expect("credential parses");
    assert_eq!(agent_keys.public_key().to_hex(), agent_pubkey);

    let kind0 = query(
        &client,
        ports,
        &agent_keys,
        Some(&attestation),
        serde_json::json!({"kinds": [0], "authors": [agent_pubkey], "limit": 1}),
    )
    .await;
    assert_eq!(kind0.len(), 1, "kind:0 must be readable back");

    let k10100 = query(
        &client,
        ports,
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
    assert_eq!(content["channel_ids"][0], channel.to_string());

    // The agent's kind:9000 add (owner-signed) is queryable in-channel too.
    let k9000 = query(
        &client,
        ports,
        &agent_keys,
        Some(&attestation),
        serde_json::json!({
            "kinds": [9000],
            "#h": [channel.to_string()],
            "#p": [agent_pubkey],
            "limit": 1
        }),
    )
    .await;
    assert_eq!(
        k9000.len(),
        1,
        "the owner's live-published 9000 must be readable as the agent"
    );
    assert_eq!(
        k9000[0]["pubkey"],
        owner.public_key().to_hex(),
        "the add must be owner-signed"
    );

    println!("RESULT: full Design-A ceremony green — keygen → attest (live 9000s) → provision (verify-not-submit)");
}

/// Full T5 retire lifecycle (#2688): keygen → attest → provision → HOST
/// retire (channels left, 10100 tombstoned, credential gone, kind:0 ALIVE)
/// → retire re-run converges → OWNER kind:9035 archive accepted through the
/// REAL owner consent path.
///
/// The agent's owner is seeded as relay role `member` (NOT `owner`/`admin`)
/// and a separate throwaway key takes RELAY_OWNER_PUBKEY: this forces the
/// relay's `determine_consent_path` past the admin shortcut and through
/// `verify_owner_consent` — fresh auth tag naming the signer + the target's
/// live kind:0 still attesting the same owner. That is exactly the ordering
/// contract under test: the host half must leave the kind:0 alone or this
/// archive would be impossible.
#[tokio::test]
#[ignore] // needs docker postgres+redis and built buzz-relay/buzz-mint binaries
async fn full_retire_lifecycle_against_local_relay() {
    let tmp = tempfile::tempdir().unwrap();
    let ports = PORTS_R;
    // Separate relay operator: the agent's owner must NOT be relay
    // owner/admin, or the 9035 would take the Admin consent shortcut.
    let relay_op = Keys::generate();
    let owner = Keys::generate();
    let _relay = spawn_relay(tmp.path(), &relay_op.public_key().to_hex(), ports);
    let client = reqwest::Client::new();
    wait_for_relay(&client, ports).await;
    let (pool, community_id) = seed_community_and_member(ports, &owner, "member").await;

    // ── Owner (plain member) creates a channel over the bridge. ──
    let channel = create_channel(&client, ports, &owner).await;

    // ── keygen → attest → provision (the Design-A default ceremony). ──
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
        "e2e-ret-{}",
        &uuid::Uuid::new_v4().simple().to_string()[..8]
    );
    let so = run_checked(
        Command::new(provision_bin)
            .args(["keygen", "--name", &agent_name])
            .arg("--config-dir")
            .arg(&cfg_dir)
            .arg("--env-dir")
            .arg(&env_dir),
        "keygen",
    );
    let agent_pubkey = so
        .lines()
        .find_map(|l| l.strip_prefix("agent pubkey (hex):"))
        .expect("agent pubkey line")
        .trim()
        .to_owned();

    let mut attest = Command::new(&mint_bin)
        .args([
            "attest",
            "--name",
            &agent_name,
            "--agent",
            &agent_pubkey,
            "--host",
            &host_pubkey,
            "--channel",
            &channel.to_string(),
            "--relay-url",
            &format!("ws://{}", relay_host(ports)),
            "--harness-cmd",
            "/usr/bin/true",
        ])
        .arg("--out")
        .arg(&bundle_out)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn buzz-mint attest");
    attest
        .stdin
        .take()
        .unwrap()
        .write_all(owner.secret_key().to_secret_hex().as_bytes())
        .unwrap();
    let attest_out = attest.wait_with_output().unwrap();
    assert!(
        attest_out.status.success(),
        "buzz-mint attest failed: {}",
        String::from_utf8_lossy(&attest_out.stderr)
    );

    let bundle_dir = bundle_out.join(format!("{agent_name}-mint"));
    run_checked(
        Command::new(provision_bin)
            .arg("agent")
            .arg(&bundle_dir)
            .arg("--no-systemd")
            .arg("--config-dir")
            .arg(&cfg_dir)
            .arg("--env-dir")
            .arg(&env_dir),
        "provision agent",
    );

    // ── Pre-retire ground truth: bot membership + online directory entry. ──
    let member_role: Option<String> = sqlx::query_scalar(
        "SELECT role::text FROM channel_members WHERE community_id=$1 AND channel_id=$2 \
         AND encode(pubkey, 'hex')=$3",
    )
    .bind(community_id)
    .bind(channel)
    .bind(&agent_pubkey)
    .fetch_optional(&pool)
    .await
    .expect("channel member check");
    assert_eq!(member_role.as_deref(), Some("bot"), "provisioned as bot");

    // ── HOST HALF: buzz-provision retire (agent still authenticates). ──
    let retire_out = run_checked(
        Command::new(provision_bin)
            .args(["retire", &agent_name, "--no-systemd"])
            .arg("--config-dir")
            .arg(&cfg_dir)
            .arg("--env-dir")
            .arg(&env_dir),
        "retire (host half)",
    );
    println!("--- retire stdout ---\n{retire_out}");
    assert!(
        retire_out.contains("published kind:9022 leave"),
        "retire must leave the channel: {retire_out}"
    );
    assert!(
        retire_out.contains("published kind:10100 tombstone"),
        "retire must tombstone the directory entry: {retire_out}"
    );
    // The exact owner-half command is printed (keygen UX pattern).
    assert!(
        retire_out.contains(&format!(
            "buzz-mint retire --agent {agent_pubkey} --relay-url ws://{}",
            relay_host(ports)
        )),
        "retire must print the exact owner-half command: {retire_out}"
    );

    // Membership row GONE.
    let member_role: Option<String> = sqlx::query_scalar(
        "SELECT role::text FROM channel_members WHERE community_id=$1 AND channel_id=$2 \
         AND encode(pubkey, 'hex')=$3 AND left_at IS NULL",
    )
    .bind(community_id)
    .bind(channel)
    .bind(&agent_pubkey)
    .fetch_optional(&pool)
    .await
    .unwrap_or(None);
    assert_eq!(
        member_role, None,
        "kind:9022 leave must have removed the bot membership"
    );

    // Directory entry: tombstoned, NOT-mentionable under the client
    // contract (respond_to nobody + empty channel_ids), lineage preserved.
    let k10100 = query(
        &client,
        ports,
        &owner,
        None,
        serde_json::json!({"kinds": [10100], "authors": [agent_pubkey], "limit": 1}),
    )
    .await;
    assert_eq!(k10100.len(), 1, "tombstone must replace the 10100");
    let content: serde_json::Value =
        serde_json::from_str(k10100[0]["content"].as_str().unwrap()).unwrap();
    assert_eq!(content["status"], "offline");
    assert_eq!(content["respond_to"], "nobody");
    assert_eq!(content["channel_ids"].as_array().unwrap().len(), 0);
    assert_eq!(content["channel_add_policy"], "owner_only");
    assert_eq!(
        content["host"], host_pubkey,
        "lineage survives the tombstone"
    );

    // kind:0 STILL ALIVE — the ordering constraint made material: the
    // owner's archive below is only possible because retire left it alone.
    let kind0 = query(
        &client,
        ports,
        &owner,
        None,
        serde_json::json!({"kinds": [0], "authors": [agent_pubkey], "limit": 1}),
    )
    .await;
    assert_eq!(
        kind0.len(),
        1,
        "host retire must NEVER remove/replace the agent kind:0"
    );

    // Host files: credential shredded, .pub removed, env archived 0600.
    assert!(
        !cfg_dir
            .join("creds")
            .join(format!("{agent_name}.nsec"))
            .exists(),
        "credential must be gone"
    );
    assert!(
        !env_dir.join(format!("{agent_name}.pub")).exists(),
        ".pub marker must be gone"
    );
    assert!(
        !env_dir.join(format!("{agent_name}.env")).exists(),
        "live env file must be gone"
    );
    let retired_path = env_dir.join(format!("{agent_name}.env.retired"));
    assert!(retired_path.exists(), ".env.retired archive must remain");
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&retired_path).unwrap().permissions().mode() & 0o7777,
            0o600,
            ".env.retired must keep 0600"
        );
    }
    // Bundle dir untouched: public lineage record.
    assert!(
        bundle_dir.join("manifest.json").exists(),
        "manifest must never be deleted by retire"
    );

    // ── Idempotence: a second retire converges without error. ──
    let rerun_out = run_checked(
        Command::new(provision_bin)
            .args(["retire", &agent_name, "--no-systemd"])
            .arg("--config-dir")
            .arg(&cfg_dir)
            .arg("--env-dir")
            .arg(&env_dir),
        "retire (re-run, must converge)",
    );
    assert!(
        rerun_out.contains("already retired"),
        "re-run must report convergence: {rerun_out}"
    );

    // ── OWNER HALF: buzz-mint retire → kind:9035 NIP-IA archive. ──
    let mut mint_retire = Command::new(&mint_bin)
        .args([
            "retire",
            "--agent",
            &agent_pubkey,
            "--relay-url",
            &format!("ws://{}", relay_host(ports)),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn buzz-mint retire");
    mint_retire
        .stdin
        .take()
        .unwrap()
        .write_all(owner.secret_key().to_secret_hex().as_bytes())
        .unwrap();
    let mr_out = mint_retire.wait_with_output().unwrap();
    let mr_stdout = String::from_utf8_lossy(&mr_out.stdout).into_owned();
    let mr_stderr = String::from_utf8_lossy(&mr_out.stderr).into_owned();
    println!("--- mint retire stdout ---\n{mr_stdout}\n--- mint retire stderr ---\n{mr_stderr}");
    assert!(
        mr_out.status.success(),
        "buzz-mint retire failed: {mr_stderr}"
    );
    assert!(
        mr_stdout.contains("kind:9035 archive request ACCEPTED"),
        "{mr_stdout}"
    );

    // Archived state per relay, through the REAL owner consent path (the
    // owner is a plain relay member, so 'admin' here would mean the test
    // stopped exercising verify_owner_consent).
    let row: Option<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT consent_path, actor, reason FROM archived_identities \
         WHERE community_id=$1 AND pubkey=$2",
    )
    .bind(community_id)
    .bind(&agent_pubkey)
    .fetch_optional(&pool)
    .await
    .expect("archived_identities lookup");
    let (consent_path, actor, reason) = row.expect("identity must be archived");
    assert_eq!(
        consent_path, "owner",
        "archive must have gone through verify_owner_consent (fresh auth tag + live kind:0)"
    );
    assert_eq!(actor, owner.public_key().to_hex());
    assert_eq!(reason.as_deref(), Some("retired"));

    println!(
        "RESULT: full T5 lifecycle green — provision → host retire (leave + tombstone + \
         files, kind:0 untouched) → converged re-run → owner 9035 archive via owner consent"
    );
}
