//! `buzz-provision retire` — the HOST half of agent teardown.
//!
//! Symmetric inverse of `provision agent`: a stale kind:10100 outlives a
//! dead agent and keeps it void-mentionable, so the directory retraction is
//! not optional.
//!
//! **Ordering contract:**
//!
//! 1. HOST half FIRST (this command). Everything here authenticates as the
//!    agent — ViaOwner virtual membership needs the credential + attestation
//!    — so it must run while both still exist:
//!    - leave every channel (kind:9022 per membership; the relay treats it
//!      as self-remove — `side_effects.rs` validates active membership and
//!      handles it identically to a self-targeted 9001);
//!    - republish kind:10100 as a minimal TOMBSTONE (status `offline`,
//!      `respond_to: "nobody"`, empty `channel_ids`) — replaceable events
//!      converge on re-run, unlike a kind:5 of a specific event id, and the
//!      client mentionability contract (`agentAutocompleteEligibility.ts`)
//!      renders `respond_to != anyone/allowlist` + no shared channels as
//!      never-mentionable;
//!    - stop + disable the systemd unit; delete the credential and archive
//!      the env file as `<name>.env.retired`. The `.pub` marker stays: it
//!      is the public record of which identity was retired here.
//!
//!    **The agent's kind:0 is NEVER touched here.** The owner's NIP-IA
//!    archive request is only accepted while the target's LIVE kind:0
//!    carries a currently-valid auth tag naming that owner
//!    (`identity_archive.rs::verify_owner_consent`). Deleting or replacing
//!    the kind:0 host-side would lock the owner out of revocation.
//! 2. OWNER half LAST: `buzz-mint retire` publishes the owner-signed
//!    kind:9035 NIP-IA archive request. **That is the real revocation** —
//!    with `BUZZ_ALLOW_NIP_OA_AUTH` enabled the attestation is a standing
//!    relay credential (unbounded conditions, native parity), and host-side
//!    teardown alone leaves that key valid on the door. After the archive
//!    the agent key no longer matters.
//!
//! Idempotent: every step converges. Re-running against an already-retired
//! name detects the `.env.retired` archive and exits 0, re-printing the
//! owner-half command.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use buzz_provision::bridge;
use buzz_provision::bundle;
use clap::Args;
use nostr::{Event, EventBuilder, Keys, Kind, PublicKey, Tag, Timestamp};

use crate::util::{
    config_dir, default_env_dir, load_host_keys, shred_file, validate_pubkey_arg, HOST_KEY_FILE,
};

#[derive(Args)]
pub struct RetireArgs {
    /// Agent instance name (the name given to keygen/provision).
    ///
    /// ORDERING CONTRACT: run this HOST half FIRST — it authenticates as
    /// the agent, which requires the credential and attestation to still
    /// exist. The agent's kind:0 is deliberately left alive: the relay only
    /// accepts the owner's kind:9035 archive request while the live kind:0
    /// attests that owner. Finish with `buzz-mint retire` on the OWNER
    /// machine — the NIP-IA archive is the real revocation of the
    /// attestation (a standing relay credential under NIP-OA auth).
    name: String,

    /// Relay URL override (ws:// or wss://). Defaults to the env file's
    /// BUZZ_RELAY_URL.
    #[arg(long = "relay-url", value_name = "WS_URL")]
    relay_url: Option<String>,

    /// Plan only: discover memberships and print every action that would be
    /// taken, but publish nothing, touch no file, and leave systemd alone.
    #[arg(long = "dry-run")]
    dry_run: bool,

    /// Skip unit stop/disable. The read-only live-unit guard STILL runs —
    /// use --force-live to retire a running agent. For hosts managed
    /// out-of-band and for tests.
    #[arg(long = "no-systemd")]
    no_systemd: bool,

    /// Proceed even though buzz-agent@<name> is currently RUNNING.
    ///
    /// Without this flag, retire refuses a runtime-active unit: a live
    /// production agent must be torn down as a conscious two-step decision,
    /// never by a mistyped name.
    #[arg(long = "force-live")]
    force_live: bool,

    /// Override the env-file directory (default: ~/services/buzz-agents).
    #[arg(long, hide = true, value_name = "DIR")]
    env_dir: Option<PathBuf>,

    /// Override the config directory (default: ~/.config/buzz-provision).
    #[arg(long, hide = true, value_name = "DIR")]
    config_dir: Option<PathBuf>,
}

/// Config recovered from the (secret-free, provision-owned) env file or its
/// `.retired` archive — everything the relay half needs besides the
/// credential. Only real KEY=VALUE lines are read (single-writer file:
/// provision generates it, retire parses it). Identity comes from the
/// `<name>.pub` marker and the host key, never from comments.
#[derive(Default)]
struct EnvConfig {
    relay_url: Option<String>,
    auth_tag: Option<String>,
}

/// A constructed teardown event plus how we refer to it in output.
struct Prepared {
    label: String,
    event: Event,
}

pub fn retire(args: &RetireArgs) -> Result<()> {
    bundle::validate_agent_name(&args.name)
        .map_err(|e| anyhow::anyhow!("agent name is not a safe instance name: {e}"))?;
    let name = args.name.as_str();

    let env_dir = default_env_dir(args.env_dir.as_deref())?;
    let cfg_dir = config_dir(args.config_dir.as_deref())?;
    let env_path = env_dir.join(format!("{name}.env"));
    let retired_path = env_dir.join(format!("{name}.env.retired"));
    let pub_path = env_dir.join(format!("{name}.pub"));
    let cred_path = cfg_dir.join("creds").join(format!("{name}.nsec"));

    let env_exists = env_path.exists();
    let retired_exists = retired_path.exists();
    let cred_exists = cred_path.exists();

    // A name with NO trace on this host is far more likely a typo than a
    // converged retire (a converged one leaves the .env.retired archive).
    if !env_exists && !retired_exists && !cred_exists && !pub_path.exists() {
        bail!(
            "no trace of agent {name:?} on this host — no {}, no {}, no credential. \
             Check the name; retiring is only meaningful where provision ran",
            env_path.display(),
            retired_path.display()
        );
    }

    // Config source: live env file first, else the .retired archive (which
    // keeps a re-run after a partial failure fully functional).
    let cfg = if env_exists {
        parse_env_config(&env_path)?
    } else if retired_exists {
        parse_env_config(&retired_path)?
    } else {
        EnvConfig::default()
    };

    // ── Runtime-active guard, BEFORE anything else moves. Always runs —
    // the probe is read-only, so --no-systemd (which only skips unit
    // MUTATIONS) does not bypass it.
    crate::systemd::guard_live_unit(name, args.force_live, Some("--force-live"))?;

    // Identity record: the <name>.pub marker written at keygen/provision.
    let marker_pubkey: Option<String> = fs::read_to_string(&pub_path)
        .ok()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty());

    // ── Relay half (as the agent — needs credential + attestation). ──
    let mut agent_pubkey_hex: Option<String> = marker_pubkey.clone();
    let mut left_channels: Vec<uuid::Uuid> = Vec::new();
    let mut already_left: Vec<uuid::Uuid> = Vec::new();
    let mut published: Vec<(String, String)> = Vec::new();

    if cred_exists {
        let agent_keys = load_agent_credential(&cred_path, marker_pubkey.as_deref())?;
        agent_pubkey_hex = Some(agent_keys.public_key().to_hex());

        let attestation = cfg.auth_tag.clone().with_context(|| {
            format!(
                "no BUZZ_AUTH_TAG found in {} — the relay teardown authenticates as the \
                 agent (ViaOwner) and needs the attestation. Restore the env file or \
                 tear down relay state manually before deleting the credential",
                if env_exists { &env_path } else { &retired_path }.display()
            )
        })?;
        let relay_url = args
            .relay_url
            .as_deref()
            .or(cfg.relay_url.as_deref())
            .context("no relay URL: pass --relay-url (no BUZZ_RELAY_URL in the env file)")?
            .trim_end_matches('/')
            .to_owned();
        buzz_provision::bundle::validate_relay_url(&relay_url).map_err(|e| anyhow::anyhow!(e))?;

        // Host lineage for the tombstone: this host's own identity if
        // initialized, else omitted.
        let host_hex = load_host_keys(&cfg_dir.join(HOST_KEY_FILE))
            .ok()
            .map(|k| k.public_key().to_hex());

        let http_base = bridge::http_base_url(&relay_url);
        let client =
            bridge::client().map_err(|e| anyhow::anyhow!("cannot build HTTP client: {e}"))?;

        // Discover memberships via bridge read: the owner-signed kind:9000
        // add log for this pubkey (the same filter provision's verify step
        // uses). Channels the agent has since left simply reject the leave
        // with "not an active member", which converges below.
        let channels = match bridge::query_events(
            &client,
            &http_base,
            &agent_keys,
            Some(&attestation),
            &serde_json::json!({
                "kinds": [9000],
                "#p": [agent_keys.public_key().to_hex()],
                "limit": 500,
            }),
        ) {
            Ok(events) => discover_channels(&events),
            Err(e) if args.dry_run => {
                eprintln!("warning: dry-run membership discovery failed ({e}) — plan shown without channel list");
                Vec::new()
            }
            Err(e) => return Err(anyhow::anyhow!("membership discovery failed: {e}")),
        };

        // Replaceable-tie guard: sign the tombstone strictly after the live
        // 10100 (one authority for replaceable timestamps — bridge).
        let tombstone_created_at = bridge::replaceable_created_at(
            &client,
            &http_base,
            &agent_keys,
            Some(&attestation),
            10100,
        );

        let prepared = build_retire_events(
            &agent_keys,
            &attestation,
            &channels,
            name,
            host_hex.as_deref(),
            tombstone_created_at,
        )?;
        // Structural invariant, loud in every mode: the host half must never
        // publish (and thereby replace) a kind:0 — the owner's archive
        // consent path dies with the live kind:0's auth tag.
        for p in &prepared {
            assert_ne!(
                p.event.kind,
                Kind::Metadata,
                "BUG: host retire constructed a kind:0 — this breaks owner revocation"
            );
        }

        if args.dry_run {
            println!("DRY RUN — teardown events constructed and signed, nothing POSTed:");
            for p in &prepared {
                println!("  {}  id {}", p.label, p.event.id);
            }
            println!("DRY RUN — systemd untouched, no file deleted or renamed.");
            print_summary(
                name,
                agent_pubkey_hex.as_deref(),
                args.relay_url.as_deref().or(cfg.relay_url.as_deref()),
                &[],
                &[],
                &[],
                true,
            );
            return Ok(());
        }

        for p in &prepared {
            let is_leave = p.event.kind == Kind::Custom(9022);
            match bridge::post_event(
                &client,
                &http_base,
                &agent_keys,
                Some(&attestation),
                &p.event,
            ) {
                Ok(receipt) => {
                    println!("published {}  id {receipt}", p.label);
                    if is_leave {
                        if let Some(c) = leave_channel_of(&p.event) {
                            left_channels.push(c);
                        }
                    }
                    published.push((p.label.clone(), receipt));
                }
                // Convergence: a leave for a channel the agent is no longer
                // in is rejected by the relay's 9022 validator with "actor
                // is not an active member" — that IS the desired end state.
                Err(bridge::BridgeError::Rejected { message })
                    if is_leave && message.contains("not an active member") =>
                {
                    println!("already left: {} ({message})", p.label);
                    if let Some(c) = leave_channel_of(&p.event) {
                        already_left.push(c);
                    }
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("publishing {} failed: {e}", p.label));
                }
            }
        }
    } else {
        // A previous retire is the LIKELY explanation, but a hand-deleted
        // credential leaves live relay state (10100 + memberships) that this
        // command can no longer re-verify — say so, never claim convergence.
        println!(
            "credential {} already gone — relay teardown cannot be re-verified from here \
             (a previous retire likely did it; if the credential was deleted by hand, the \
             directory entry and memberships may still be live — check from a client); \
             skipping",
            cred_path.display()
        );
        if args.dry_run {
            println!("DRY RUN — nothing left to plan beyond file/systemd state (all converged).");
            print_summary(
                name,
                agent_pubkey_hex.as_deref(),
                args.relay_url.as_deref().or(cfg.relay_url.as_deref()),
                &[],
                &[],
                &[],
                true,
            );
            return Ok(());
        }
    }

    // ── systemd: stop + disable, only if the unit exists. ──
    if !args.no_systemd {
        let unit = format!("buzz-agent@{name}.service");
        if crate::systemd::unit_exists(&unit)? {
            crate::systemd::systemctl_user_tolerant(&["stop", &unit])?;
            crate::systemd::systemctl_user_tolerant(&["disable", &unit])?;
            println!("unit {unit}: stopped + disabled");
        } else {
            println!("unit {unit}: not present — nothing to stop");
        }
    }

    // ── Files: credential shredded, env archived. The `.pub` marker STAYS —
    // it is the public identity record of what was retired here, and the
    // owner-half command is reprinted from it on converged re-runs. The
    // manifest/bundle dirs are likewise untouched (public lineage record),
    // and so is anything under creds/ belonging to other agents. The
    // operator's `<name>.local.env` is never touched by tooling. ──
    if cred_exists {
        shred_file(&cred_path).context("cannot shred agent credential")?;
        println!("credential:     {} shredded", cred_path.display());
    }
    if env_exists {
        // Same-directory rename: atomic, preserves the 0600 mode. The file
        // is secret-free post-T2 but keeps the config record (relay URL,
        // attestation, pubkeys) — which is also what makes THIS command
        // idempotent and the owner-half command reprintable later.
        fs::rename(&env_path, &retired_path).with_context(|| {
            format!(
                "cannot archive {} as {}",
                env_path.display(),
                retired_path.display()
            )
        })?;
        println!("env file:       archived as {}", retired_path.display());
    } else if retired_exists && !cred_exists && published.is_empty() {
        println!("agent {name:?} was already retired on this host — nothing to redo (converged).");
    }

    print_summary(
        name,
        agent_pubkey_hex.as_deref(),
        args.relay_url.as_deref().or(cfg.relay_url.as_deref()),
        &left_channels,
        &already_left,
        &published,
        false,
    );
    Ok(())
}

/// Parse the secret-free env file (or its `.retired` archive): the two env
/// lines the relay half needs, plus the pubkey comments provision wrote.
fn parse_env_config(path: &Path) -> Result<EnvConfig> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("cannot read env file {}", path.display()))?;
    let mut cfg = EnvConfig::default();
    for line in text.lines() {
        let line = line.trim_end();
        if let Some(v) = line.strip_prefix("BUZZ_RELAY_URL=") {
            cfg.relay_url = Some(v.to_owned());
        } else if let Some(v) = line.strip_prefix("BUZZ_AUTH_TAG=") {
            cfg.auth_tag = Some(v.to_owned());
        }
    }
    Ok(cfg)
}

/// Load + verify the agent credential; when the `<name>.pub` marker recorded
/// the agent pubkey, the derived key must match (never tear down the wrong
/// identity because a credential was swapped under an instance name).
fn load_agent_credential(cred_path: &Path, expected_hex: Option<&str>) -> Result<Keys> {
    // Same canonical-hex discipline as every other credential read.
    let keys = load_host_keys(cred_path)
        .with_context(|| format!("credential {} is not usable", cred_path.display()))?;
    if let Some(expected) = expected_hex {
        let expected: PublicKey = validate_pubkey_arg(expected, "pub-marker agent pubkey")?;
        if keys.public_key() != expected {
            bail!(
                "credential {} derives to {} but the pub marker records agent pubkey {} — \
                 refusing to tear down a mismatched identity",
                cred_path.display(),
                keys.public_key().to_hex(),
                expected.to_hex()
            );
        }
    }
    Ok(keys)
}

/// Extract the deduplicated channel ids from a kind:9000 query result.
fn discover_channels(events: &[serde_json::Value]) -> Vec<uuid::Uuid> {
    let mut channels: Vec<uuid::Uuid> = Vec::new();
    for ev in events {
        let Some(tags) = ev.get("tags").and_then(|t| t.as_array()) else {
            continue;
        };
        for tag in tags {
            let Some(parts) = tag.as_array() else {
                continue;
            };
            if parts.first().and_then(|s| s.as_str()) == Some("h") {
                if let Some(c) = parts
                    .get(1)
                    .and_then(|s| s.as_str())
                    .and_then(|s| uuid::Uuid::parse_str(s).ok())
                {
                    if !channels.contains(&c) {
                        channels.push(c);
                    }
                }
            }
        }
    }
    channels
}

/// The minimal 10100 tombstone content: renders NOT-mentionable under the
/// client contract (`agentAutocompleteEligibility.ts::relayAgentIsSharedWithUser`
/// — mentionable requires `respond_to == "anyone"` with a shared channel, or
/// `"allowlist"` naming the caller; `"nobody"` + empty `channel_ids` fails
/// both forever), while keeping the fields the relay's own 10100 handler
/// requires (`channel_add_policy`) and the lineage record.
fn tombstone_profile_content(name: &str, host_pubkey_hex: Option<&str>) -> String {
    let mut content = serde_json::json!({
        "name": name,
        "display_name": name,
        "agent_type": "agent",
        "channels": [],
        "channel_ids": [],
        "capabilities": [],
        "status": "offline",
        "respond_to": "nobody",
        "respond_to_allowlist": [],
        "channel_add_policy": "owner_only",
    });
    if let Some(host) = host_pubkey_hex {
        content["host"] = serde_json::Value::String(host.to_owned());
    }
    content.to_string()
}

/// Construct the full agent-signed teardown batch: one kind:9022 leave per
/// discovered channel, then the kind:10100 tombstone. NEVER a kind:0 — the
/// live kind:0 is the owner's revocation handle (see module docs).
///
/// kind:9022 over a self-targeted kind:9001: the relay treats them
/// identically ("functionally identical to self-remove via kind 9001",
/// side_effects.rs), but 9022 is the purpose-built leave kind with a
/// dedicated SDK builder and no p tag to strip/preserve.
fn build_retire_events(
    agent_keys: &Keys,
    attestation: &str,
    channels: &[uuid::Uuid],
    name: &str,
    host_pubkey_hex: Option<&str>,
    tombstone_created_at: Timestamp,
) -> Result<Vec<Prepared>> {
    let auth_tag: Tag = buzz_sdk::nip_oa::parse_auth_tag(attestation)
        .map_err(|e| anyhow::anyhow!("cannot parse attestation from env file: {e}"))?;
    let mut prepared = Vec::with_capacity(channels.len() + 1);
    for channel in channels {
        let event = buzz_sdk::builders::build_leave(*channel)
            .map_err(|e| anyhow::anyhow!("cannot build kind:9022 for channel {channel}: {e}"))?
            .sign_with_keys(agent_keys)
            .with_context(|| format!("cannot sign kind:9022 for channel {channel}"))?;
        prepared.push(Prepared {
            label: format!("kind:9022 leave (channel {channel})"),
            event,
        });
    }
    let tombstone = EventBuilder::new(
        Kind::Custom(10100),
        tombstone_profile_content(name, host_pubkey_hex),
    )
    .tags([auth_tag])
    // Strictly after the live 10100: replaceables tie-break equal
    // created_at by lowest id, so same-second republish can silently lose.
    .custom_created_at(tombstone_created_at)
    .sign_with_keys(agent_keys)
    .context("cannot sign kind:10100 tombstone")?;
    prepared.push(Prepared {
        label: "kind:10100 tombstone (status offline, respond_to nobody, no channels)".to_owned(),
        event: tombstone,
    });
    Ok(prepared)
}

/// The channel a prepared kind:9022 targets (its `h` tag).
fn leave_channel_of(event: &Event) -> Option<uuid::Uuid> {
    event.tags.iter().find_map(|t| {
        let parts = t.as_slice();
        if parts.first().map(|s| s.as_str()) == Some("h") {
            parts.get(1).and_then(|s| uuid::Uuid::parse_str(s).ok())
        } else {
            None
        }
    })
}

/// `systemctl --user is-active` state for the instance unit. `inactive` /
/// `failed` / unknown all report as their literal state string; only a
/// spawn failure is an error.
fn print_summary(
    name: &str,
    agent_pubkey_hex: Option<&str>,
    relay_url: Option<&str>,
    left: &[uuid::Uuid],
    already_left: &[uuid::Uuid],
    published: &[(String, String)],
    dry_run: bool,
) {
    println!();
    if dry_run {
        println!("retire plan for agent {name:?} (DRY RUN — nothing happened)");
    } else {
        println!("host-half retire of agent {name:?} complete");
    }
    if let Some(pk) = agent_pubkey_hex {
        println!("agent pubkey:   {pk}");
    }
    if !left.is_empty() {
        println!("channels left:  {}", left.len());
    }
    if !already_left.is_empty() {
        println!("already left:   {} (converged)", already_left.len());
    }
    for (label, id) in published {
        println!("published:      {label}  id {id}");
    }
    println!();
    println!("⚑ NOT DONE YET — the attestation is still a live relay credential.");
    println!("Finish on the OWNER machine with the NIP-IA archive (the real revocation):");
    println!();
    match (agent_pubkey_hex, relay_url) {
        (Some(pk), Some(url)) => {
            println!("  buzz-mint retire --agent {pk} --relay-url {url}");
        }
        (Some(pk), None) => {
            println!("  buzz-mint retire --agent {pk} --relay-url <wss://your-relay>");
        }
        _ => {
            println!("  buzz-mint retire --agent <agent-pubkey> --relay-url <wss://your-relay>");
        }
    }
    println!();
    println!("(The agent's kind:0 was deliberately left alive: the relay verifies the");
    println!(" owner's archive request against it. It stops mattering after the archive.)");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tombstone_is_never_mentionable_under_client_contract() {
        let content = tombstone_profile_content("teacup", Some(&"a".repeat(64)));
        let v: serde_json::Value = serde_json::from_str(&content).unwrap();
        // The client contract (agentAutocompleteEligibility.ts): mentionable
        // requires respond_to "anyone" (+ shared channel) or "allowlist"
        // (+ caller listed). The tombstone forecloses every path.
        assert_eq!(v["respond_to"], "nobody");
        assert_eq!(v["respond_to_allowlist"].as_array().unwrap().len(), 0);
        assert_eq!(v["channel_ids"].as_array().unwrap().len(), 0);
        assert_eq!(v["status"], "offline");
        // Relay-side handle_agent_profile requires this field in content.
        assert_eq!(v["channel_add_policy"], "owner_only");
        // Lineage record survives the tombstone.
        assert_eq!(v["host"], "a".repeat(64));

        // Host lineage optional (env archive missing the comment).
        let bare: serde_json::Value =
            serde_json::from_str(&tombstone_profile_content("teacup", None)).unwrap();
        assert!(bare.get("host").is_none());
    }

    #[test]
    fn retire_events_never_include_kind0() {
        let owner = Keys::generate();
        let agent = Keys::generate();
        let attestation =
            buzz_sdk::nip_oa::compute_auth_tag(&owner, &agent.public_key(), "").unwrap();
        let channels = vec![uuid::Uuid::new_v4(), uuid::Uuid::new_v4()];
        // A live 10100 signed "now" must lose to the tombstone even when
        // both fall in the same second (replaceable tie-break guard).
        let after_live = Timestamp::from(Timestamp::now().as_secs() + 1);
        let prepared =
            build_retire_events(&agent, &attestation, &channels, "teacup", None, after_live)
                .unwrap();
        assert_eq!(prepared.len(), 3, "two leaves + one tombstone");
        assert_eq!(
            prepared[2].event.created_at, after_live,
            "tombstone must be signed strictly after the live 10100"
        );
        for p in &prepared {
            assert_ne!(
                p.event.kind,
                Kind::Metadata,
                "host retire must NEVER touch kind:0 — it is the owner's revocation handle"
            );
            assert!(
                matches!(p.event.kind.as_u16(), 9022 | 10100),
                "unexpected teardown kind {}",
                p.event.kind.as_u16()
            );
        }
        // Leaves come first (agent must still be renderable as leaving), the
        // tombstone last.
        assert_eq!(prepared[0].event.kind, Kind::Custom(9022));
        assert_eq!(prepared[2].event.kind, Kind::Custom(10100));
        // Every event is agent-signed.
        for p in &prepared {
            assert_eq!(p.event.pubkey, agent.public_key());
            p.event.verify().unwrap();
        }
    }

    // The live-unit guard's decision table is tested where it lives now:
    // crate::systemd::tests.

    #[test]
    fn env_config_parses_provisioned_file() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("t.env");
        std::fs::write(
            &p,
            "# teacup agent — GENERATED by buzz-provision; regenerated on every run.\n\
             BUZZ_RELAY_URL=ws://localhost:3033\n\
             BUZZ_ACP_AGENT_COMMAND=/usr/bin/true\n\
             BUZZ_AUTH_TAG=[\"auth\",\"x\",\"\",\"sig==\"]\n",
        )
        .unwrap();
        let cfg = parse_env_config(&p).unwrap();
        assert_eq!(cfg.relay_url.as_deref(), Some("ws://localhost:3033"));
        assert_eq!(
            cfg.auth_tag.as_deref(),
            Some("[\"auth\",\"x\",\"\",\"sig==\"]")
        );
    }

    #[test]
    fn channel_discovery_dedupes_and_ignores_junk() {
        let c1 = uuid::Uuid::new_v4();
        let c2 = uuid::Uuid::new_v4();
        let events = vec![
            serde_json::json!({"tags": [["h", c1.to_string()], ["p", "x"]]}),
            serde_json::json!({"tags": [["h", c1.to_string()]]}),
            serde_json::json!({"tags": [["h", c2.to_string()]]}),
            serde_json::json!({"tags": [["h", "not-a-uuid"]]}),
            serde_json::json!({"no_tags": true}),
        ];
        let channels = discover_channels(&events);
        assert_eq!(channels, vec![c1, c2]);
    }
}
