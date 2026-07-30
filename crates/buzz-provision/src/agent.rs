//! `buzz-provision agent` — consume a buzz-mint bundle end-to-end (T4 + T4b).
//!
//! Two ceremonies, one shared pipeline (fail-closed at every step;
//! sealed-mint-bundle spec, task #2687 amendments, and the #2694 Design-A
//! ruling binding):
//!
//! **A-mode (default mint)** — the bundle is `manifest.json` only; the agent
//! key was born here via `buzz-provision keygen`:
//! 1. Validate bundle structure; require the credential to ALREADY exist and
//!    the manifest owner to be pinned (same trust rule as B, no decrypt).
//! 2. Credential re-read + pubkey re-derivation against the manifest — the
//!    A-mode splice analogue: a wrong-bundle-for-this-key refuses BEFORE any
//!    publish or systemd action.
//!
//! **B-mode (sealed transit)**:
//! 1. Unseal via [`buzz_seal::unseal`] with the host key + pinned owners.
//! 2. Credential BEFORE any systemd action: write
//!    `~/.config/buzz-provision/creds/<name>.nsec` byte-exact (tempfile +
//!    rename, 0600, dir 0700), then re-read and re-derive the pubkey against
//!    the manifest — the structural fix for the restart-loop hazard
//!    (Restart=on-failure + bad credential = unbounded loop, measured).
//!    Shred `sealed.json` only after that verification; manifest + events
//!    stay (they ARE the on-disk lineage record; re-runs converge).
//!
//! **Shared pipeline from there**:
//! 3. Secret-free env file `<env-dir>/<name>.env`, written directly (never
//!    via shell: bash mangles BUZZ_AUTH_TAG's literal JSON quoting).
//! 4. Publish identity events over NIP-98 HTTP, agent-signed, with the
//!    attestation as `x-auth-tag`: kind:0 (auth tag verified then embedded)
//!    THEN full kind:10100 with `channel_add_policy: owner_only` and the
//!    `host` lineage field. RULED ordering (P0 #2683).
//! 5. Memberships: B-mode submits the pre-signed owner kind:9000 batch
//!    verbatim (never re-signed). A-mode submits NOTHING — the owner already
//!    published the adds at attest time (relay refuses third-party
//!    submission, ingest pubkey-match) — and instead VERIFIES each manifest
//!    channel's add via a bridge read as the agent, warning (never failing)
//!    on channels whose adds are pending or rejected.
//! 6. Enable + restart `buzz-agent@<name>` (restart, not start: LoadCredential
//!    snapshots at unit start, chuck T2-4) and verify the journal for
//!    connected/presence/owner-resolved markers.
//!
//! `--dry-run` performs steps 1-3 plus event construction, then stops:
//! nothing is POSTed, systemd is untouched, sealed.json is not shredded.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use buzz_seal::bridge;
use buzz_seal::bundle::{self, BundleMode, Manifest, SealedPresence};
use buzz_seal::{unseal, UnsealWarning};
use clap::Args;
use nostr::{Event, EventBuilder, Keys, Kind, PublicKey, SecretKey, Tag, Timestamp};
use zeroize::Zeroizing;

use crate::util::{
    config_dir, credential_pubkey, default_env_dir, ensure_dir_0700, is_canonical_64_hex,
    load_host_keys, merge_owners, shred_file, sweep_stale_tempfiles, validate_pubkey_arg,
    write_atomic_0600, Clobber, ALLOWED_OWNERS_FILE, HOST_KEY_FILE,
};

/// Journal markers that prove the harness came up healthy (buzz-acp
/// `lib.rs`: connect, presence, owner resolution from the auth tag).
const JOURNAL_MARKER_CONNECTED: &str = "connected to relay";
const JOURNAL_MARKER_PRESENCE: &str = "presence set to online";

#[derive(Args)]
pub struct AgentArgs {
    /// Path to the <name>-mint bundle directory produced by `buzz-mint
    /// attest` (A-mode, the default) or `buzz-mint new` (B-mode).
    bundle_dir: PathBuf,

    /// Relay URL (ws:// or wss://). Overrides the manifest's relay_url;
    /// required when the manifest does not carry one.
    #[arg(long = "relay-url", value_name = "WS_URL")]
    relay_url: Option<String>,

    /// Re-provision an existing instance: overwrite its env file (and, when
    /// re-minting, its credential). Without this flag an existing env file
    /// refuses to be clobbered.
    #[arg(long)]
    force: bool,

    /// Steps 1-3 + event construction only: print what would be published,
    /// but do not POST, do not touch systemd, do not shred sealed.json.
    #[arg(long = "dry-run")]
    dry_run: bool,

    /// Skip every systemd action (unit enable/restart + journal verify).
    /// For hosts managed out-of-band and for tests — publishing still runs.
    #[arg(long = "no-systemd")]
    no_systemd: bool,

    /// Trust an additional owner PUBLIC key for THIS run only (not written
    /// to the pin list — use `pin-owner add` for durable trust). Repeatable.
    #[arg(long = "trust-owner", value_name = "PUBKEY_64HEX")]
    trust_owner: Vec<String>,

    /// Allowlist pubkeys for `respond_to = allowlist` manifests (comma
    /// separated or repeated). Owner is always implicitly included by the
    /// harness.
    #[arg(
        long = "respond-to-allowlist",
        value_name = "PUBKEY_64HEX",
        value_delimiter = ','
    )]
    respond_to_allowlist: Vec<String>,

    /// Seconds to wait for the journal health markers after unit restart.
    #[arg(long = "verify-timeout-secs", default_value_t = 90)]
    verify_timeout_secs: u64,

    /// Override the env-file directory (default: ~/services/buzz-agents).
    #[arg(long, hide = true, value_name = "DIR")]
    env_dir: Option<PathBuf>,

    /// Override the config directory (default: ~/.config/buzz-provision).
    #[arg(long, hide = true, value_name = "DIR")]
    config_dir: Option<PathBuf>,
}

/// A constructed identity event plus how we refer to it in output.
struct Prepared {
    label: &'static str,
    event: Event,
}

pub fn provision_agent(args: &AgentArgs) -> Result<()> {
    // ── Phase 0: pure validation — nothing on disk or the network moves. ──
    let bundle = bundle::load(&args.bundle_dir, SealedPresence::Optional)
        .map_err(|e| anyhow::anyhow!("{e:#}"))?;
    let problems = bundle::validate(&bundle);
    if !problems.is_empty() {
        for p in &problems {
            eprintln!("bundle: FAIL: {p}");
        }
        bail!(
            "{} problem(s) in bundle {} — refusing to provision",
            problems.len(),
            args.bundle_dir.display()
        );
    }
    let manifest = &bundle.manifest;
    bundle::validate_agent_name(&manifest.name)
        .map_err(|e| anyhow::anyhow!("manifest name is not a safe instance name: {e}"))?;
    let name = manifest.name.as_str();

    let relay_url = args
        .relay_url
        .as_deref()
        .or(manifest.config.relay_url.as_deref())
        .context("no relay URL: pass --relay-url (the manifest carries none)")?
        .trim_end_matches('/')
        .to_owned();
    if !(relay_url.starts_with("ws://") || relay_url.starts_with("wss://")) {
        bail!("relay URL must be ws:// or wss:// (got a different scheme)");
    }
    let harness_cmd = manifest
        .config
        .harness_cmd
        .as_deref()
        .context(
            "manifest config.harness_cmd is missing — edit manifest.json (it is public, \
             operator-editable) or re-mint with --harness-cmd",
        )?
        .to_owned();
    let respond_to = manifest.config.respond_to.as_str();
    if !["owner-only", "anyone", "allowlist", "nobody"].contains(&respond_to) {
        bail!("manifest config.respond_to {respond_to:?} is not a known mode");
    }
    for pk in &args.respond_to_allowlist {
        validate_pubkey_arg(pk, "--respond-to-allowlist")?;
    }
    if respond_to == "allowlist" && args.respond_to_allowlist.is_empty() {
        bail!(
            "manifest respond_to is \"allowlist\" but no --respond-to-allowlist pubkeys were \
             given — the harness would refuse to start (owner-only behavior needs no allowlist)"
        );
    }

    let env_dir = default_env_dir(args.env_dir.as_deref())?;
    let env_path = env_dir.join(format!("{name}.env"));
    if env_path.exists() && !args.force {
        bail!(
            "env file {} already exists — this instance looks provisioned. \
             Re-run with --force to re-provision it (safe: env is secret-free and \
             re-publishing identity events is idempotent)",
            env_path.display()
        );
    }

    // ── Phase 1: trust root + secret acquisition. ──
    let cfg_dir = config_dir(args.config_dir.as_deref())?;
    let key_path = cfg_dir.join(HOST_KEY_FILE);
    let owners_path = cfg_dir.join(ALLOWED_OWNERS_FILE);
    if !key_path.exists() {
        bail!(
            "no host identity at {} — run `buzz-provision host-init --allow-owner <owner>` first",
            key_path.display()
        );
    }
    let host_keys = load_host_keys(&key_path)?;
    let host_pubkey = host_keys.public_key();

    let mut pinned_hex: Vec<String> = if owners_path.exists() {
        merge_owners(&owners_path, &[])?
    } else {
        Vec::new()
    };
    for raw in &args.trust_owner {
        validate_pubkey_arg(raw, "--trust-owner")?;
        if !pinned_hex.iter().any(|p| p == raw) {
            eprintln!("note: --trust-owner extends the pin list for THIS run only");
            pinned_hex.push(raw.clone());
        }
    }
    if pinned_hex.is_empty() {
        bail!(
            "no pinned owners ({} is empty or missing) — refusing every bundle. \
             Pin your owner with `buzz-provision pin-owner add <owner pubkey>`",
            owners_path.display()
        );
    }
    let pinned: Vec<PublicKey> = pinned_hex
        .iter()
        .map(|h| validate_pubkey_arg(h, "pin-list"))
        .collect::<Result<_>>()?;

    let creds_dir = cfg_dir.join("creds");
    ensure_dir_0700(&creds_dir)?;
    sweep_stale_tempfiles(&cfg_dir);
    sweep_stale_tempfiles(&creds_dir);
    let cred_path = creds_dir.join(format!("{name}.nsec"));

    let sealed_path = bundle.dir.join("sealed.json");
    let mut api_key_paths: Vec<PathBuf> = Vec::new();
    match manifest.mode {
        // ── A-mode (default mint): nothing was ever sealed. The credential
        // must ALREADY exist — born here by `buzz-provision keygen` — and
        // the manifest's attester must be a pinned owner. Same trust rule
        // as B, no decrypt step. Both checks fire BEFORE any publish or
        // systemd action (fail-closed).
        BundleMode::A => {
            if !cred_path.exists() {
                bail!(
                    "A-mode bundle but no credential at {} — the agent key is born on the \
                     host FIRST: run `buzz-provision keygen --name {name}` here, have the \
                     owner attest the printed pubkey with `buzz-mint attest`, then re-run. \
                     (If keygen ran on a different host, this bundle belongs there.)",
                    cred_path.display()
                );
            }
            if !pinned.contains(&manifest.owner_pubkey) {
                bail!(
                    "manifest owner {} is not a pinned owner — refusing a foreign manifest \
                     (extend trust explicitly with pin-owner add or --trust-owner)",
                    manifest.owner_pubkey.to_hex()
                );
            }
            // The pubkey re-derivation against the manifest (the A-mode
            // splice analogue: wrong-bundle-for-this-key) happens in
            // verify_credential below, before anything irreversible.
        }
        BundleMode::B => match &bundle.envelope {
            Some(envelope) => {
                // Steps 1-6 of the spec's unseal order, fail-closed, in buzz-seal.
                let unsealed = unseal(&host_keys, envelope, &pinned)
                    .map_err(|e| anyhow::anyhow!("unseal refused: {e}"))?;
                for w in &unsealed.warnings {
                    match w {
                        UnsealWarning::EnvelopeAgentPubkeyMismatch { .. }
                        | UnsealWarning::Stale { .. } => {
                            eprintln!("warning: {w}");
                        }
                    }
                }
                let inner = &unsealed.inner;
                // The manifest drives the env file, unit name, and published
                // events; the inner payload is the authoritative identity. They
                // must agree or the provision would be self-inconsistent.
                if inner.agent_pubkey != manifest.agent_pubkey {
                    bail!(
                        "inner (authoritative) agent_pubkey {} disagrees with manifest {} — \
                     the manifest was tampered with or belongs to another mint",
                        inner.agent_pubkey.to_hex(),
                        manifest.agent_pubkey.to_hex()
                    );
                }
                if inner.host_pubkey != manifest.host_pubkey {
                    bail!("inner host_pubkey disagrees with manifest host_pubkey");
                }

                // Name-collision guard: never silently replace a DIFFERENT
                // agent's credential under the same instance name.
                if cred_path.exists() && !args.force {
                    let existing = credential_pubkey(&cred_path);
                    if existing.as_ref() != Some(&manifest.agent_pubkey) {
                        bail!(
                            "credential {} already exists and does not derive to this bundle's \
                         agent pubkey — another agent uses this instance name. Re-run with \
                         --force only if you intend to replace it",
                            cred_path.display()
                        );
                    }
                }

                // Step 7 (write half): byte-exact canonical hex, 0600, atomic.
                write_atomic_0600(
                    &cred_path,
                    inner.agent_sk.expose_canonical_hex().as_bytes(),
                    Clobber::Allow,
                )
                .context("cannot write agent credential")?;

                // Optional sealed api keys: same secret tier, credential files
                // next to the nsec. Unit wiring (LoadCredential=) is manual in v1.
                if let Some(api_keys) = &inner.api_keys {
                    for (env_name, value) in api_keys {
                        let p = creds_dir.join(format!("{name}.{env_name}"));
                        write_atomic_0600(&p, value.expose().as_bytes(), Clobber::Allow)
                            .with_context(|| format!("cannot write sealed api key {env_name}"))?;
                        api_key_paths.push(p);
                    }
                }
            }
            None => {
                // Converged re-run: sealed.json was already consumed +
                // shredded. The credential on disk is the secret source;
                // owner pinning still gates the (public, unauthenticated)
                // manifest.
                if !cred_path.exists() {
                    bail!(
                        "bundle has no sealed.json and no credential exists at {} — nothing \
                         to provision from. Re-mint the agent to get a fresh sealed bundle",
                        cred_path.display()
                    );
                }
                if !pinned.contains(&manifest.owner_pubkey) {
                    bail!(
                        "manifest owner {} is not a pinned owner — refusing a foreign \
                         manifest (extend trust explicitly with pin-owner add or \
                         --trust-owner)",
                        manifest.owner_pubkey.to_hex()
                    );
                }
            }
        },
    }

    // ── Phase 2: credential verification BEFORE anything else happens
    // (binding amendment: never rely on systemd 243/CREDENTIALS self-healing).
    let agent_keys = verify_credential(&cred_path, &manifest.agent_pubkey)?;

    // Shred sealed.json now that the secret is verified on disk (spec step
    // 7). A later failure leaves a state a plain re-run converges from.
    let mut shredded = false;
    if !args.dry_run && bundle.envelope.is_some() {
        shred_file(&sealed_path).context("cannot shred sealed.json")?;
        shredded = true;
    }

    // ── Phase 3: secret-free env file + pubkey marker file. ──
    fs::create_dir_all(&env_dir)
        .with_context(|| format!("cannot create env dir {}", env_dir.display()))?;
    sweep_stale_tempfiles(&env_dir);
    let env_content = render_env_file(
        manifest,
        &relay_url,
        &harness_cmd,
        respond_to,
        &args.respond_to_allowlist,
        &host_pubkey,
    )?;
    write_atomic_0600(&env_path, env_content.as_bytes(), Clobber::Allow)?;
    let pub_path = env_dir.join(format!("{name}.pub"));
    write_atomic_0600(
        &pub_path,
        format!("{}\n", manifest.agent_pubkey.to_hex()).as_bytes(),
        Clobber::Allow,
    )?;

    // ── Phase 4: construct identity events (RULED order: 0 → 10100 → 9000s).
    let auth_tag = verified_auth_tag(&manifest.attestation, &manifest.agent_pubkey)?;
    let kind0 = buzz_sdk::builders::build_profile(Some(name), Some(name), None, None, None)
        .map_err(|e| anyhow::anyhow!("cannot build kind:0: {e}"))?
        .tags([auth_tag.clone()])
        .sign_with_keys(&agent_keys)
        .context("cannot sign kind:0")?;

    let content_10100 = agent_profile_content(
        manifest,
        respond_to,
        &args.respond_to_allowlist,
        &host_pubkey,
    );
    let kind10100 = EventBuilder::new(Kind::Custom(10100), content_10100)
        .tags([auth_tag])
        .sign_with_keys(&agent_keys)
        .context("cannot sign kind:10100")?;

    let mut prepared = vec![
        Prepared {
            label: "kind:0 profile (agent-signed, auth tag embedded)",
            event: kind0,
        },
        Prepared {
            label: "kind:10100 agent profile (agent-signed, host lineage)",
            event: kind10100,
        },
    ];
    for (channel, event) in &bundle.events {
        let _ = channel;
        prepared.push(Prepared {
            label: "kind:9000 bot add (owner-signed, published verbatim)",
            event: event.clone(),
        });
    }

    if args.dry_run {
        println!("DRY RUN — events constructed and signed, nothing POSTed:");
        for p in &prepared {
            println!("  {}  id {}", p.label, p.event.id);
        }
        println!("DRY RUN — systemd untouched, sealed.json not shredded.");
        print_summary(
            manifest,
            &host_pubkey,
            &env_path,
            &cred_path,
            &api_key_paths,
            &[],
            args.no_systemd,
            true,
            shredded,
        );
        return Ok(());
    }

    // ── Phase 5: publish over the NIP-98 HTTP bridge (shared bridge client
    // — the same code path buzz-mint attest publishes with). ──
    let http_base = bridge::http_base_url(&relay_url);
    let client = bridge::client().map_err(|e| anyhow::anyhow!("cannot build HTTP client: {e}"))?;
    let mut published: Vec<(String, String)> = Vec::new();
    for p in &prepared {
        let receipt = bridge::post_event(
            &client,
            &http_base,
            &agent_keys,
            Some(&manifest.attestation),
            &p.event,
        )
        .map_err(anyhow::Error::from)
        .with_context(|| format!("publishing {} failed", p.label))?;
        println!("published {}  id {receipt}", p.label);
        published.push((p.label.to_owned(), receipt));
    }

    // ── Phase 5b (A-mode): VERIFY memberships instead of submitting 9000s —
    // the owner published the adds at attest time; the relay refuses them
    // from anyone else (ingest pubkey-match). Warn, never fail: an add may
    // be pending (owner offline mid-ceremony) or rejected by relay
    // preconditions, and neither invalidates the provision.
    if manifest.mode == BundleMode::A {
        for channel in &manifest.config.channels {
            let filter = serde_json::json!({
                "kinds": [9000],
                "#h": [channel.to_string()],
                "#p": [manifest.agent_pubkey.to_hex()],
                "limit": 1,
            });
            match bridge::query_events(
                &client,
                &http_base,
                &agent_keys,
                Some(&manifest.attestation),
                &filter,
            ) {
                Ok(events) if !events.is_empty() => {
                    println!("membership verified: channel {channel} (owner kind:9000 found)");
                }
                Ok(_) => eprintln!(
                    "warning: channel {channel}: no kind:9000 add found for this agent — the \
                     owner's add may be pending or was rejected by relay preconditions. \
                     Re-run `buzz-mint attest` (adds are idempotent) or add the agent from \
                     a client; this provision remains valid."
                ),
                Err(e) => eprintln!(
                    "warning: channel {channel}: membership check failed ({e}) — verify the \
                     agent's channel membership manually; this provision remains valid."
                ),
            }
        }
    }

    // ── Phase 6: systemd unit + journal verification. ──
    if !args.no_systemd {
        let unit = format!("buzz-agent@{name}.service");
        let since = Timestamp::now().as_secs().saturating_sub(1);
        systemctl_user(&["daemon-reload"])?;
        systemctl_user(&["enable", &unit])?;
        // restart, not start: LoadCredential snapshots at unit start, so a
        // re-provisioned credential is only picked up by a fresh start.
        systemctl_user(&["restart", &unit])?;
        verify_journal(
            &unit,
            since,
            &manifest.owner_pubkey,
            Duration::from_secs(args.verify_timeout_secs),
        )?;
        println!("unit {unit}: healthy (connected, presence online, owner resolved)");
    }

    print_summary(
        manifest,
        &host_pubkey,
        &env_path,
        &cred_path,
        &api_key_paths,
        &published,
        args.no_systemd,
        false,
        shredded,
    );
    Ok(())
}

/// Read back the credential file, require canonical 64-lowercase-hex, derive
/// its pubkey, and require it to equal the manifest's agent pubkey. This is
/// the gate every systemd action hides behind.
fn verify_credential(cred_path: &Path, expected: &PublicKey) -> Result<Keys> {
    let raw = Zeroizing::new(
        fs::read(cred_path)
            .with_context(|| format!("cannot re-read credential {}", cred_path.display()))?,
    );
    if !is_canonical_64_hex(&raw) {
        bail!(
            "credential {} is not canonical 64-lowercase-hex after write — refusing to \
             enable the unit (this is the restart-loop guard)",
            cred_path.display()
        );
    }
    let sk_str =
        std::str::from_utf8(&raw).map_err(|_| anyhow::anyhow!("credential is not valid UTF-8"))?;
    let secret = SecretKey::from_hex(sk_str).map_err(|_| {
        anyhow::anyhow!(
            "credential {} does not parse as a secp256k1 secret key (value withheld)",
            cred_path.display()
        )
    })?;
    let keys = Keys::new(secret);
    if keys.public_key() != *expected {
        bail!(
            "credential {} derives to {} but the manifest agent_pubkey is {} — refusing to \
             enable the unit",
            cred_path.display(),
            keys.public_key().to_hex(),
            expected.to_hex()
        );
    }
    Ok(keys)
}

/// Verify the NIP-OA attestation for this agent and return it as an event
/// tag — mirrors the desktop `build_profile_event`: verify before injecting.
fn verified_auth_tag(attestation: &str, agent_pubkey: &PublicKey) -> Result<Tag> {
    buzz_sdk::nip_oa::verify_auth_tag(attestation, agent_pubkey)
        .map_err(|e| anyhow::anyhow!("attestation does not verify for the agent pubkey: {e}"))?;
    buzz_sdk::nip_oa::parse_auth_tag(attestation)
        .map_err(|e| anyhow::anyhow!("cannot parse verified auth tag: {e}"))
}

/// The full kind:10100 content (RULED shape, P0 #2683): directory profile +
/// `channel_add_policy: owner_only` + the `host` lineage anchor.
fn agent_profile_content(
    manifest: &Manifest,
    respond_to: &str,
    respond_to_allowlist: &[String],
    host_pubkey: &PublicKey,
) -> String {
    serde_json::json!({
        "name": manifest.name,
        "display_name": manifest.name,
        "agent_type": "agent",
        "channels": [],
        "channel_ids": manifest.config.channels.iter().map(|c| c.to_string()).collect::<Vec<_>>(),
        "capabilities": [],
        "status": "online",
        "respond_to": respond_to,
        "respond_to_allowlist": respond_to_allowlist,
        "channel_add_policy": "owner_only",
        "host": host_pubkey.to_hex(),
    })
    .to_string()
}

/// Render the secret-free env file. Every value is checked single-line so a
/// hostile manifest cannot inject extra env assignments; `BUZZ_AUTH_TAG` is
/// the literal JSON line systemd's `EnvironmentFile=` reads verbatim.
fn render_env_file(
    manifest: &Manifest,
    relay_url: &str,
    harness_cmd: &str,
    respond_to: &str,
    respond_to_allowlist: &[String],
    host_pubkey: &PublicKey,
) -> Result<String> {
    for (what, value) in [
        ("relay URL", relay_url),
        ("harness command", harness_cmd),
        ("attestation", manifest.attestation.as_str()),
    ] {
        if value
            .chars()
            .any(|c| c == '\n' || c == '\r' || c.is_control())
        {
            bail!("{what} contains control characters — refusing to write the env file");
        }
    }
    let mut out = String::new();
    out.push_str(&format!(
        "# {} agent — generated by buzz-provision agent; DO NOT add secrets here.\n",
        manifest.name
    ));
    out.push_str(&format!(
        "# agent pubkey: {}\n",
        manifest.agent_pubkey.to_hex()
    ));
    out.push_str(&format!(
        "# owner pubkey: {}\n",
        manifest.owner_pubkey.to_hex()
    ));
    out.push_str(&format!("# host (lineage): {}\n", host_pubkey.to_hex()));
    out.push_str("# BUZZ_PRIVATE_KEY is NOT here — it arrives via systemd LoadCredential.\n");
    out.push_str(&format!("BUZZ_RELAY_URL={relay_url}\n"));
    out.push_str(&format!("BUZZ_ACP_AGENT_COMMAND={harness_cmd}\n"));
    out.push_str(&format!("BUZZ_ACP_RESPOND_TO={respond_to}\n"));
    if !respond_to_allowlist.is_empty() {
        out.push_str(&format!(
            "BUZZ_ACP_RESPOND_TO_ALLOWLIST={}\n",
            respond_to_allowlist.join(",")
        ));
    }
    out.push_str("BUZZ_ACP_SUBSCRIBE=mentions\n");
    // Literal JSON line — never routed through a shell (bash would mangle
    // the quoting; systemd EnvironmentFile= reads it verbatim).
    out.push_str(&format!("BUZZ_AUTH_TAG={}\n", manifest.attestation));
    Ok(out)
}

/// Run `systemctl --user <args>`; non-zero exit is a hard error carrying the
/// command's stderr.
fn systemctl_user(args: &[&str]) -> Result<()> {
    let out = Command::new("systemctl")
        .arg("--user")
        .args(args)
        .output()
        .context("cannot run systemctl --user (is a user manager available?)")?;
    if !out.status.success() {
        bail!(
            "systemctl --user {} failed ({}): {}",
            args.join(" "),
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}

/// Poll the user journal for the three health markers, failing loud with a
/// journal excerpt when they do not all appear within `timeout`.
fn verify_journal(
    unit: &str,
    since_unix: u64,
    owner_pubkey: &PublicKey,
    timeout: Duration,
) -> Result<()> {
    let owner_marker = format!(
        "owner resolved from BUZZ_AUTH_TAG: {}",
        owner_pubkey.to_hex()
    );
    let markers: [&str; 3] = [
        JOURNAL_MARKER_CONNECTED,
        JOURNAL_MARKER_PRESENCE,
        &owner_marker,
    ];
    let since = format!("@{since_unix}");
    let started = Instant::now();
    let last_output = loop {
        let out = Command::new("journalctl")
            .args([
                "--user",
                "-u",
                unit,
                "--since",
                &since,
                "-o",
                "cat",
                "--no-pager",
            ])
            .output()
            .context("cannot run journalctl --user")?;
        let text = String::from_utf8_lossy(&out.stdout).into_owned();
        if markers.iter().all(|m| text.contains(m)) {
            return Ok(());
        }
        if started.elapsed() >= timeout {
            break text;
        }
        std::thread::sleep(Duration::from_secs(2));
    };
    let missing: Vec<&str> = markers
        .iter()
        .copied()
        .filter(|m| !last_output.contains(m))
        .collect();
    let tail: Vec<&str> = last_output.lines().rev().take(40).collect();
    let tail: Vec<&str> = tail.into_iter().rev().collect();
    bail!(
        "unit {unit} did not become healthy within {}s — missing markers: {missing:?}\n\
         --- journal tail ---\n{}\n--- end journal tail ---",
        timeout.as_secs(),
        tail.join("\n")
    );
}

#[allow(clippy::too_many_arguments)]
fn print_summary(
    manifest: &Manifest,
    host_pubkey: &PublicKey,
    env_path: &Path,
    cred_path: &Path,
    api_key_paths: &[PathBuf],
    published: &[(String, String)],
    no_systemd: bool,
    dry_run: bool,
    shredded: bool,
) {
    println!();
    println!("provisioned agent {:?}", manifest.name);
    println!("agent pubkey:   {}", manifest.agent_pubkey.to_hex());
    println!("owner pubkey:   {}", manifest.owner_pubkey.to_hex());
    println!("lineage host:   {}", host_pubkey.to_hex());
    println!("unit:           buzz-agent@{}.service", manifest.name);
    println!("env file:       {}", env_path.display());
    println!("credential:     {} (0600)", cred_path.display());
    for p in api_key_paths {
        println!(
            "sealed api key: {} (0600 — wire into the unit via LoadCredential= manually)",
            p.display()
        );
    }
    for (label, id) in published {
        println!("published:      {label}  id {id}");
    }
    if dry_run {
        println!("mode:           DRY RUN (no publish, no systemd, no shred)");
    } else if no_systemd {
        println!("mode:           --no-systemd (unit not enabled here)");
    }
    if shredded {
        println!("sealed.json:    shredded (manifest + events stay: lineage record)");
    }
}
