//! buzz-mint — the owner-side half of the Buzz virtual mint.
//!
//! Runs where the owner key lives (Windows workstation is the eventual
//! primary locus). One mint ceremony:
//!
//! **`attest`**: the agent key was already born on the host
//! (`buzz-provision keygen`); the owner attests the exported PUBLIC key
//! (NIP-OA auth tag, empty conditions — upstream `compute_auth_tag`'s own
//! external-pubkey shape) and PUBLISHES the owner-signed kind:9000 bot-add
//! batch directly to the relay with fresh timestamps (the relay refuses
//! third-party submission of owner-signed events, and ±15min created_at
//! freshness kills pre-signing). Output bundle is `manifest.json` only —
//! 100% public data, transportable over any channel.
//!
//! Key-handling rules (binding): the owner secret arrives via a 0600 file
//! or a silent prompt — never argv/env; it is held in [`Zeroizing`]/[`Keys`]
//! only as long as signing takes. The agent secret never exists owner-side
//! at all. No print/log surface ever formats secret material — pubkeys,
//! paths, and event ids only. clap argv errors are redacted before reaching
//! stderr (shared redactor).

use std::fs::{self, File};
use std::io::{IsTerminal, Read, Write};
#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use buzz_provision::bridge::{self, BridgeError};
use buzz_provision::bundle::{self, BundleMode, Manifest, ManifestConfig, MANIFEST_VERSION};
use buzz_provision::{is_canonical_64_hex, redact_key_material};
use clap::{Args, Parser, Subcommand, ValueEnum};
use nostr::nips::nip19::ToBech32;
use nostr::{Event, Keys, PublicKey, SecretKey};
use uuid::Uuid;
use zeroize::Zeroizing;

#[derive(Parser)]
#[command(
    name = "buzz-mint",
    version,
    about = "Owner-side mint for VM-deployed Buzz agents: attest a host-born agent key and \
             publish its channel adds"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Attest a host-born agent PUBLIC key.
    ///
    /// Run `buzz-provision keygen --name <agent>` on the host first; it
    /// prints the agent pubkey this command attests. Signs the NIP-OA auth
    /// tag, publishes the owner kind:9000 bot-add batch directly to the
    /// relay, and emits a manifest-only bundle (100% public data).
    Attest(AttestArgs),
    /// Validate a mint bundle's structure and internal consistency.
    ///
    /// Pure pre-flight: uses public data only — no secret input.
    Inspect(InspectArgs),
    /// Publish the NIP-IA kind:9035 archive request for a retired agent —
    /// the OWNER half of teardown, and the REAL revocation (run this LAST).
    ///
    /// ⚑ ORDERING CONTRACT: run `buzz-provision retire <name>` on the HOST
    /// first — it authenticates as the agent (leaves channels, tombstones
    /// the kind:10100 directory entry) and must happen while the credential
    /// still exists. Then run this. The relay accepts the archive only while
    /// the agent's LIVE kind:0 carries a currently-valid auth tag naming
    /// this owner, which is why the host half never touches the kind:0.
    ///
    /// Why this step is not optional: with NIP-OA auth enabled on the relay,
    /// the attestation is a standing relay credential (empty conditions,
    /// native parity). Host teardown alone leaves that key valid on the
    /// door; the archive is what revokes it. A FRESH auth tag is minted and
    /// the request freshly signed here because the relay enforces a ±120s
    /// created_at window on kind:9035.
    Retire(RetireArgs),
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum RespondTo {
    /// All events forwarded (no author filtering).
    Anyone,
    /// Only the agent's registered owner (ACP default).
    OwnerOnly,
    /// Owner + explicit pubkey list (configured host-side).
    Allowlist,
}

impl RespondTo {
    fn as_str(self) -> &'static str {
        match self {
            Self::Anyone => "anyone",
            Self::OwnerOnly => "owner-only",
            Self::Allowlist => "allowlist",
        }
    }
}

#[derive(Args)]
struct AttestArgs {
    /// Agent PUBLIC key to attest (exactly 64 lowercase hex) — printed by
    /// `buzz-provision keygen --name <agent>` on the host. The secret never
    /// leaves the host; this ceremony only ever sees public data.
    #[arg(long, value_name = "PUBKEY_64HEX")]
    agent: String,

    /// Host PUBLIC key (exactly 64 lowercase hex) — the lineage anchor,
    /// printed by `buzz-provision host-init --show` on the VM.
    #[arg(long, value_name = "PUBKEY_64HEX")]
    host: String,

    /// Agent name (bundle dir becomes <name>-mint/; must equal the name
    /// given to `buzz-provision keygen`).
    #[arg(long)]
    name: String,

    /// Channel id to add the agent to (owner-signed kind:9000, role=bot,
    /// published directly to --relay-url). Repeatable.
    #[arg(long = "channel", value_name = "CHANNEL_UUID")]
    channels: Vec<Uuid>,

    /// Who the agent responds to (written to the public manifest config).
    #[arg(long = "respond-to", value_enum, default_value = "owner-only")]
    respond_to: RespondTo,

    /// Harness command for the host-side unit (public manifest config).
    #[arg(long = "harness-cmd", value_name = "CMD")]
    harness_cmd: Option<String>,

    /// Relay URL: the kind:9000 batch is published here (owner NIP-98,
    /// fresh timestamps) and it becomes the agent's BUZZ_RELAY_URL.
    #[arg(long = "relay-url", value_name = "WS_URL")]
    relay_url: String,

    /// Output directory the bundle dir is created under (default: cwd).
    #[arg(long, value_name = "DIR")]
    out: Option<PathBuf>,

    /// Read the owner SECRET key from this file (must not be group/world
    /// accessible — 0600). Without this flag, a silent prompt is used.
    /// There is deliberately NO --owner-key <hex> flag: argv leaks into
    /// shell history and process listings.
    #[arg(long = "owner-key-file", value_name = "PATH")]
    owner_key_file: Option<PathBuf>,
}

#[derive(Args)]
struct InspectArgs {
    /// Path to the <name>-mint bundle directory.
    bundle_dir: PathBuf,
}

#[derive(Args)]
struct RetireArgs {
    /// Agent PUBLIC key to archive (exactly 64 lowercase hex) — printed by
    /// the host half (`buzz-provision retire <name>`), which must run FIRST.
    #[arg(long, value_name = "PUBKEY_64HEX")]
    agent: String,

    /// Relay URL the archive request is published to (owner NIP-98, fresh
    /// created_at — the relay enforces a ±120s window on kind:9035).
    #[arg(long = "relay-url", value_name = "WS_URL")]
    relay_url: String,

    /// Machine-readable reason code recorded on the archive (NIP-IA
    /// suggested codes: retired, rotated, bot-rebuilt, left-organization,
    /// spam, ...).
    #[arg(long, default_value = "retired")]
    reason: String,

    /// Read the owner SECRET key from this file (must not be group/world
    /// accessible — 0600). Without this flag, a silent prompt is used.
    /// There is deliberately NO --owner-key <hex> flag: argv leaks into
    /// shell history and process listings.
    #[arg(long = "owner-key-file", value_name = "PATH")]
    owner_key_file: Option<PathBuf>,
}

fn main() {
    // try_parse instead of parse: clap's own error rendering echoes argv
    // values verbatim, and a secret pasted without its flag prefix must not
    // land on stderr (same measured leak channel as buzz-provision, chuck
    // T1-4 — one redactor, all mint binaries).
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            let is_error = e.use_stderr();
            let rendered = redact_key_material(&e.render().to_string());
            if is_error {
                eprint!("{rendered}");
            } else {
                print!("{rendered}");
            }
            std::process::exit(e.exit_code());
        }
    };
    let result = match cli.command {
        Command::Attest(args) => attest(&args),
        Command::Inspect(args) => inspect(&args.bundle_dir),
        Command::Retire(args) => retire(&args),
    };
    if let Err(e) = result {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

// ─── Owner key input ─────────────────────────────────────────────────────────

/// Parse the owner secret key from a raw input buffer.
///
/// Accepts canonical 64-lowercase-hex or `nsec1…` bech32 (the format desktop
/// clients export). Trailing ASCII whitespace (`\n`, `\r\n`) from files and
/// pipes is stripped — this is a *consumed* input, unlike the produced
/// `agent_sk`, which stays strictly canonical. The value is NEVER echoed by
/// any error path.
fn parse_owner_secret(raw: &[u8]) -> Result<Keys> {
    let end = raw
        .iter()
        .rposition(|b| !b.is_ascii_whitespace())
        .map(|i| i + 1)
        .unwrap_or(0);
    let trimmed = &raw[..end];
    if trimmed.is_empty() {
        bail!("owner key input is empty");
    }
    let s = Zeroizing::new(
        std::str::from_utf8(trimmed)
            .map_err(|_| anyhow::anyhow!("owner key input is not valid UTF-8"))?
            .to_owned(),
    );
    let looks_hex = is_canonical_64_hex(s.as_bytes());
    let looks_nsec = s.starts_with("nsec1");
    if !looks_hex && !looks_nsec {
        bail!(
            "owner key input (got {} chars) is neither canonical 64-lowercase-hex nor nsec1… \
             bech32 (value withheld from this message on purpose)",
            s.len()
        );
    }
    let secret = SecretKey::parse(s.as_str()).map_err(|_| {
        anyhow::anyhow!("owner key input does not parse as a secret key (value withheld)")
    })?;
    Ok(Keys::new(secret))
}

/// Refuse owner key files readable by group/other. The check is the actual
/// security property behind "must be 0600".
#[cfg(unix)]
fn check_owner_key_file_mode(path: &Path) -> Result<()> {
    let meta = fs::metadata(path)
        .with_context(|| format!("cannot stat owner key file {}", path.display()))?;
    let mode = meta.permissions().mode() & 0o7777;
    if mode & 0o077 != 0 {
        bail!(
            "owner key file {} has mode {mode:04o} — it is group/world accessible. \
             Refusing to read it; chmod 600 the file first",
            path.display()
        );
    }
    Ok(())
}

#[cfg(not(unix))]
fn check_owner_key_file_mode(_path: &Path) -> Result<()> {
    // TODO(windows): unix mode bits do not exist on Windows — enforce an
    // equivalent DACL check (owner-only ACL) before the Windows release.
    Ok(())
}

fn read_owner_keys(owner_key_file: Option<&Path>) -> Result<Keys> {
    if let Some(path) = owner_key_file {
        check_owner_key_file_mode(path)?;
        let raw = Zeroizing::new(
            fs::read(path)
                .with_context(|| format!("cannot read owner key file {}", path.display()))?,
        );
        return parse_owner_secret(&raw).with_context(|| format!("in {}", path.display()));
    }
    // Silent prompt on a TTY; plain line read when stdin is piped (scripts,
    // tests) — either way the key never appears in argv or shell history.
    let raw: Zeroizing<String> = if std::io::stdin().is_terminal() {
        Zeroizing::new(
            rpassword::prompt_password("owner secret key (input hidden): ")
                .context("cannot read owner key from terminal")?,
        )
    } else {
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .context("cannot read owner key from stdin")?;
        Zeroizing::new(buf)
    };
    parse_owner_secret(raw.as_bytes()).context("from stdin")
}

// ─── Validation helpers ──────────────────────────────────────────────────────

/// Bundle dirs are created from the name: keep it path-safe and unit-safe.
/// Shared rule with the provision-side consumer (buzz_provision::bundle).
fn validate_name(name: &str) -> Result<()> {
    bundle::validate_agent_name(name).map_err(|e| anyhow::anyhow!("--name: {e}"))
}

/// Validate a `--relay-url`: ws/wss scheme, single line (the value is later
/// written verbatim into an `EnvironmentFile=` line host-side, so a newline
/// here would be an env-injection vector).
fn validate_relay_url(s: &str) -> Result<()> {
    if !(s.starts_with("ws://") || s.starts_with("wss://")) {
        bail!("invalid --relay-url {s:?}: expected a ws:// or wss:// URL");
    }
    if s.chars().any(|c| c.is_whitespace() || c.is_control()) {
        bail!("invalid --relay-url: must not contain whitespace or control characters");
    }
    Ok(())
}

/// Strictly validate a 64-lowercase-hex PUBLIC key argument.
///
/// The value is NEVER echoed back: a mistakenly pasted secret key is
/// indistinguishable from a public key (both are 64 hex chars), so no error
/// path may reproduce the input. `what` names the argument; `hint` says
/// where the right value comes from.
fn validate_pubkey_arg(s: &str, what: &str, hint: &str) -> Result<PublicKey> {
    if !is_canonical_64_hex(s.as_bytes()) {
        bail!(
            "invalid {what} value (got {} chars): expected a PUBLIC key as exactly \
             64 lowercase hex characters, {hint} (value withheld from this message on purpose)",
            s.len()
        );
    }
    PublicKey::from_hex(s).map_err(|_| {
        anyhow::anyhow!(
            "invalid {what} value: not a valid x-only secp256k1 public key \
             (value withheld from this message on purpose)"
        )
    })
}

fn validate_host_pubkey(s: &str) -> Result<PublicKey> {
    validate_pubkey_arg(
        s,
        "--host",
        "as printed by `buzz-provision host-init --show`",
    )
}

// ─── File writing ────────────────────────────────────────────────────────────

/// Create a fresh directory with mode 0700; refuses to reuse an existing path.
fn create_dir_0700(dir: &Path) -> Result<()> {
    #[cfg_attr(not(unix), allow(unused_mut))]
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    builder.mode(0o700);
    // TODO(windows): 0700 has no direct equivalent; restrict via DACL.
    builder
        .create(dir)
        .with_context(|| format!("cannot create directory {}", dir.display()))
}

/// Write a public file inside the (fresh, 0700) bundle dir, creation-only
/// (`create_new`), fsynced. Bundle contents are all public data (0644).
fn write_bundle_file(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut opts = fs::OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    opts.mode(0o644);
    let mut f = opts
        .open(path)
        .with_context(|| format!("cannot create {}", path.display()))?;
    f.write_all(bytes)
        .with_context(|| format!("cannot write {}", path.display()))?;
    f.sync_all()
        .with_context(|| format!("cannot fsync {}", path.display()))?;
    Ok(())
}

fn fsync_dir(dir: &Path) -> Result<()> {
    File::open(dir)
        .and_then(|d| d.sync_all())
        .with_context(|| format!("cannot fsync directory {}", dir.display()))
}

// ─── attest ──────────────────────────────────────────────────────────────────

/// Attest an EXTERNAL, host-born agent pubkey and publish its kind:9000
/// bot-add batch directly (owner NIP-98, fresh created_at) — the relay
/// refuses events whose pubkey differs from the authenticated submitter,
/// and its ±15min freshness window kills pre-signing, so the owner is the
/// only party that can land these adds.
fn attest(args: &AttestArgs) -> Result<()> {
    // 1) Validate everything before touching secrets or the network.
    validate_name(&args.name)?;
    let agent_pubkey = validate_pubkey_arg(
        &args.agent,
        "--agent",
        "as printed by `buzz-provision keygen --name <agent>` on the host",
    )?;
    let host_pubkey = validate_host_pubkey(&args.host)?;
    validate_relay_url(&args.relay_url)?;
    if agent_pubkey == host_pubkey {
        bail!(
            "--agent equals the HOST pubkey; expected the AGENT pubkey printed by \
             `buzz-provision keygen`, not the host identity"
        );
    }
    let channels = args.channels.clone();
    {
        let mut deduped = channels.clone();
        deduped.sort();
        deduped.dedup();
        if deduped.len() != channels.len() {
            bail!("duplicate --channel values given");
        }
    }
    if args.respond_to == RespondTo::Allowlist {
        eprintln!(
            "note: --respond-to allowlist requires the allowlist pubkeys to be configured \
             host-side (BUZZ_ACP_RESPOND_TO_ALLOWLIST) at provision time."
        );
    }
    let out_dir = args.out.clone().unwrap_or_else(|| PathBuf::from("."));
    let bundle_dir = out_dir.join(format!("{}-mint", args.name));
    if bundle_dir.exists() {
        bail!(
            "bundle directory {} already exists — refusing to overwrite. Re-attesting the \
             same agent key is safe (remove the old bundle or choose another --out); the \
             kind:9000 adds are idempotent",
            bundle_dir.display()
        );
    }

    // 2) Owner key in, attestation + 9000 batch signed and PUBLISHED, owner
    //    key dropped. The key must live through publishing: each POST is
    //    owner-NIP-98-authenticated (that is the whole point of the ruling).
    let owner_keys = read_owner_keys(args.owner_key_file.as_deref())?;
    let owner_pubkey = owner_keys.public_key();
    if owner_pubkey == host_pubkey {
        bail!(
            "--host equals the OWNER pubkey; expected the host identity printed by \
             `buzz-provision host-init --show`"
        );
    }
    if owner_pubkey == agent_pubkey {
        bail!(
            "--agent equals the OWNER pubkey — an agent identity must be its own key \
             (run `buzz-provision keygen` on the host and attest the pubkey it prints)"
        );
    }

    // NIP-OA auth tag over the EXTERNAL pubkey, empty conditions — this is
    // upstream compute_auth_tag's own shape: it never needs the agent secret.
    let attestation = buzz_sdk::nip_oa::compute_auth_tag(&owner_keys, &agent_pubkey, "")
        .map_err(|e| anyhow::anyhow!("cannot compute NIP-OA auth tag: {e}"))?;
    let created_at = nostr::Timestamp::now().as_secs();

    // Owner-signed kind:9000 batch, FRESH created_at (sign time), published
    // immediately below. Same h/p/role shape the desktop emits.
    let mut events: Vec<(Uuid, Event)> = Vec::with_capacity(channels.len());
    for channel in &channels {
        let builder = buzz_sdk::builders::build_add_member(
            *channel,
            &agent_pubkey.to_hex(),
            Some(buzz_sdk::MemberRole::Bot),
        )
        .map_err(|e| anyhow::anyhow!("cannot build kind:9000 for channel {channel}: {e}"))?;
        let event = builder
            .sign_with_keys(&owner_keys)
            .with_context(|| format!("cannot sign kind:9000 for channel {channel}"))?;
        events.push((*channel, event));
    }

    // Publish the batch directly, owner-authenticated. A rejected or
    // unreachable add is a WARNING, not a mint failure — the relay enforces
    // per-channel preconditions (not archived, owner an active member), and
    // `buzz-provision agent` re-verifies memberships host-side.
    let mut outcomes: Vec<(Uuid, std::result::Result<String, BridgeError>)> = Vec::new();
    if !events.is_empty() {
        let http_base = bridge::http_base_url(&args.relay_url);
        let client =
            bridge::client().map_err(|e| anyhow::anyhow!("cannot build HTTP client: {e}"))?;
        for (channel, event) in &events {
            outcomes.push((
                *channel,
                bridge::post_event(&client, &http_base, &owner_keys, None, event),
            ));
        }
    }

    // Owner key is done — drop it before any disk write.
    drop(owner_keys);

    // 3) Bundle fallout: dir 0700 + manifest.json ONLY (all public data).
    let manifest = Manifest {
        v: MANIFEST_VERSION,
        name: args.name.clone(),
        agent_pubkey,
        owner_pubkey,
        host_pubkey,
        attestation,
        created_at,
        mode: BundleMode::A,
        config: ManifestConfig {
            respond_to: args.respond_to.as_str().to_owned(),
            channels: channels.clone(),
            harness_cmd: args.harness_cmd.clone(),
            relay_url: Some(args.relay_url.clone()),
        },
    };
    create_dir_0700(&bundle_dir)?;
    write_bundle_file(
        &bundle_dir.join("manifest.json"),
        serde_json::to_string_pretty(&manifest)
            .context("manifest serialization failed")?
            .as_bytes(),
    )?;
    fsync_dir(&bundle_dir)?;

    // 4) Report — pubkeys, paths, event ids ONLY.
    println!(
        "attested agent {:?} (Design A: key born on the host)",
        args.name
    );
    println!("agent pubkey (hex):  {}", agent_pubkey.to_hex());
    match agent_pubkey.to_bech32() {
        Ok(npub) => println!("agent pubkey (npub): {npub}"),
        Err(_) => println!("agent pubkey (npub): <bech32 encoding failed>"),
    }
    println!("owner pubkey:        {}", owner_pubkey.to_hex());
    println!("host pubkey:         {}", host_pubkey.to_hex());
    println!("bundle: {}", bundle_dir.display());
    println!("  manifest.json  (public: name, pubkeys, attestation, config — nothing sealed)");
    let mut warnings = 0usize;
    for (channel, outcome) in &outcomes {
        match outcome {
            Ok(id) => println!("channel {channel}: kind:9000 bot add ACCEPTED  id {id}"),
            Err(e) => {
                warnings += 1;
                eprintln!(
                    "warning: channel {channel}: kind:9000 bot add NOT accepted: {e}\n\
                     warning: this is not a mint failure — fix the precondition (channel \
                     archived? owner not an active member? relay reachable?) and re-run \
                     attest, or add the agent from a client; adds are idempotent."
                );
            }
        }
    }
    println!();
    println!("next: copy the bundle to the host and run `buzz-provision agent` there.");
    if warnings > 0 {
        eprintln!(
            "note: {warnings} channel add(s) did not land; `buzz-provision agent` verifies \
             memberships and will warn about the same channels until the adds land."
        );
    }
    Ok(())
}

// ─── retire (owner half of teardown — the real revocation) ──────────────────

/// Build the owner-signed NIP-IA kind:9035 archive request. Mirrors the
/// native delete path's `build_agent_archive_request` shape (desktop
/// `commands/agents.rs`): content empty, exactly one NIP-70 protected tag
/// `["-"]`, exactly one `["p", target]`, machine-readable `reason`, and
/// exactly ONE freshly-minted owner auth tag — the relay's
/// `verify_owner_consent` requires the single auth tag to name the request
/// signer AND the target's live kind:0 to still attest the same owner.
fn build_retire_archive_request(
    owner_keys: &Keys,
    agent_pubkey: &PublicKey,
    reason: &str,
) -> Result<Event> {
    // FRESH auth tag at request-build time — native's delete path also mints
    // a new tag rather than reusing a stored one, and the flush loop re-signs
    // with a fresh created_at because of the relay's ±120s window on 9035s
    // (persona_events.rs re-sign-before-publish pattern). Here build and
    // publish are one step, so signing now IS the fresh signature.
    let attestation = buzz_sdk::nip_oa::compute_auth_tag(owner_keys, agent_pubkey, "")
        .map_err(|e| anyhow::anyhow!("cannot compute NIP-OA auth tag: {e}"))?;
    let auth_tag = buzz_sdk::nip_oa::parse_auth_tag(&attestation)
        .map_err(|e| anyhow::anyhow!("cannot parse freshly-minted auth tag: {e}"))?;
    let tags = vec![
        nostr::Tag::parse(["-"]).map_err(|e| anyhow::anyhow!("cannot build protected tag: {e}"))?,
        nostr::Tag::parse(["p", &agent_pubkey.to_hex()])
            .map_err(|e| anyhow::anyhow!("cannot build p tag: {e}"))?,
        nostr::Tag::parse(["reason", reason])
            .map_err(|e| anyhow::anyhow!("cannot build reason tag: {e}"))?,
        auth_tag,
    ];
    // `.allow_self_tagging()` mirrors the desktop wire builder: our p tag
    // names the agent (never the owner signer), but the builder must never
    // silently reshape the wire form either way.
    nostr::EventBuilder::new(nostr::Kind::Custom(9035), "")
        .tags(tags)
        .allow_self_tagging()
        .sign_with_keys(owner_keys)
        .map_err(|e| anyhow::anyhow!("cannot sign kind:9035 archive request: {e}"))
}

/// Validate a `--reason` code: short single token, no whitespace/control
/// (it lands verbatim in a tag and in relay logs).
fn validate_reason(reason: &str) -> Result<()> {
    if reason.is_empty() || reason.len() > 64 {
        bail!("--reason must be 1..=64 characters");
    }
    if !reason
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        bail!("--reason must be a single [A-Za-z0-9_-] token (e.g. retired, rotated)");
    }
    Ok(())
}

/// The owner half of teardown: mint a fresh auth tag, build the kind:9035
/// NIP-IA archive request (reason defaults to "retired"), and publish it
/// over owner NIP-98 with a fresh created_at. See the subcommand docs for
/// the ordering contract (host half FIRST; kind:0 must still be live).
fn retire(args: &RetireArgs) -> Result<()> {
    // 1) Validate everything before touching secrets or the network.
    let agent_pubkey = validate_pubkey_arg(
        &args.agent,
        "--agent",
        "as printed by `buzz-provision retire <name>` on the host",
    )?;
    validate_relay_url(&args.relay_url)?;
    validate_reason(&args.reason)?;

    // 2) Owner key in, request built + published, owner key dropped.
    let owner_keys = read_owner_keys(args.owner_key_file.as_deref())?;
    let owner_pubkey = owner_keys.public_key();
    if owner_pubkey == agent_pubkey {
        bail!(
            "--agent equals the OWNER pubkey — this command archives an AGENT identity \
             via the owner consent path; archiving your own key is a different ceremony"
        );
    }

    let event = build_retire_archive_request(&owner_keys, &agent_pubkey, &args.reason)?;
    let http_base = bridge::http_base_url(&args.relay_url);
    let client = bridge::client().map_err(|e| anyhow::anyhow!("cannot build HTTP client: {e}"))?;
    let outcome = bridge::post_event(&client, &http_base, &owner_keys, None, &event);

    // Owner key is done — drop it before reporting.
    drop(owner_keys);

    // 3) Report — pubkeys and event ids ONLY.
    match outcome {
        Ok(id) => {
            println!("archived agent identity {}", agent_pubkey.to_hex());
            println!("owner pubkey:  {}", owner_pubkey.to_hex());
            println!(
                "kind:9035 archive request ACCEPTED  id {id}  (reason: {})",
                args.reason
            );
            println!();
            println!(
                "revocation complete: the attestation no longer admits this key anywhere \
                 the archive is honored. The agent's kind:0 stops mattering now."
            );
            Ok(())
        }
        Err(e) => {
            bail!(
                "kind:9035 archive request NOT accepted: {e}\n\
                 The relay's owner consent path requires (a) this owner to be a relay \
                 member, (b) the request to carry exactly one fresh auth tag naming this \
                 owner, and (c) the agent's LIVE kind:0 to still attest this owner — if \
                 the kind:0 was deleted or replaced, owner revocation is no longer \
                 possible from here (a relay owner/admin can still archive it). \
                 Run the HOST half (`buzz-provision retire`) before, not after, and \
                 never remove the agent's kind:0."
            )
        }
    }
}

// ─── inspect ─────────────────────────────────────────────────────────────────

/// Owner-side pre-flight: validate bundle structure and manifest consistency
/// using PUBLIC data only. The check it cannot make — that the host's
/// credential derives to the attested pubkey — runs host-side at provision.
fn inspect(bundle_dir: &Path) -> Result<()> {
    let bundle = bundle::load(bundle_dir).map_err(|e| anyhow::anyhow!("{e:#}"))?;
    let problems = bundle::validate(&bundle);
    let manifest = &bundle.manifest;

    println!("bundle:       {}", bundle_dir.display());
    println!("name:         {}", manifest.name);
    println!("mode:         {}", manifest.mode.as_str());
    println!("agent pubkey: {}", manifest.agent_pubkey.to_hex());
    println!("owner pubkey: {}", manifest.owner_pubkey.to_hex());
    println!("host pubkey:  {}", manifest.host_pubkey.to_hex());
    println!("respond_to:   {}", manifest.config.respond_to);
    println!("channels:     {}", manifest.config.channels.len());

    if problems.is_empty() {
        println!(
            "inspect: OK — structure and public-field consistency verified (everything is \
             public; the host verifies its credential derives to the attested pubkey at \
             provision time)"
        );
        Ok(())
    } else {
        for p in &problems {
            eprintln!("inspect: FAIL: {p}");
        }
        bail!("{} problem(s) found", problems.len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_validation() {
        assert!(validate_name("teacup").is_ok());
        assert!(validate_name("agent_2-b").is_ok());
        assert!(validate_name("").is_err());
        assert!(validate_name("../evil").is_err());
        assert!(validate_name("a b").is_err());
        assert!(validate_name("-lead").is_err());
        assert!(validate_name(&"x".repeat(65)).is_err());
    }

    #[test]
    fn retire_archive_request_matches_relay_consent_shape() {
        // Mirror of the relay-side validators the request must satisfy
        // (identity_archive.rs): exactly one NIP-70 "-" tag, exactly one
        // 64-hex p tag, exactly one 4-element auth tag verifying for the
        // TARGET and naming the SIGNER, fresh created_at, empty content.
        let owner = Keys::generate();
        let agent = Keys::generate();
        let event = build_retire_archive_request(&owner, &agent.public_key(), "retired").unwrap();

        assert_eq!(event.kind, nostr::Kind::Custom(9035));
        assert!(event.content.is_empty());
        event.verify().unwrap();
        assert_eq!(event.pubkey, owner.public_key(), "owner-signed");

        let count_named = |name: &str| {
            event
                .tags
                .iter()
                .filter(|t| t.as_slice().first().map(|s| s.as_str()) == Some(name))
                .count()
        };
        assert_eq!(count_named("-"), 1, "exactly one protected tag");
        assert_eq!(count_named("p"), 1, "exactly one p tag");
        assert_eq!(count_named("auth"), 1, "exactly one auth tag");
        assert_eq!(count_named("reason"), 1);

        let p_val = event
            .tags
            .iter()
            .find_map(|t| {
                let parts = t.as_slice();
                (parts.first().map(|s| s.as_str()) == Some("p"))
                    .then(|| parts.get(1).unwrap().to_string())
            })
            .unwrap();
        assert_eq!(p_val, agent.public_key().to_hex());

        // The auth tag must verify for the target and resolve to the signer
        // (verify_owner_consent: request auth owner == request signer).
        let auth_parts: Vec<String> = event
            .tags
            .iter()
            .find(|t| t.as_slice().first().map(|s| s.as_str()) == Some("auth"))
            .unwrap()
            .as_slice()
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(
            auth_parts.len(),
            4,
            "auth tag must have exactly four elements"
        );
        let auth_json = serde_json::to_string(&auth_parts).unwrap();
        let resolved_owner =
            buzz_sdk::nip_oa::verify_auth_tag(&auth_json, &agent.public_key()).unwrap();
        assert_eq!(resolved_owner, owner.public_key());

        // Freshly signed: created_at within the relay's ±120s 9035 window.
        let now = nostr::Timestamp::now().as_secs() as i64;
        assert!((event.created_at.as_secs() as i64 - now).abs() <= 5);
    }

    #[test]
    fn retire_reason_validation() {
        assert!(validate_reason("retired").is_ok());
        assert!(validate_reason("bot-rebuilt").is_ok());
        assert!(validate_reason("left_organization").is_ok());
        assert!(validate_reason("").is_err());
        assert!(validate_reason("two words").is_err());
        assert!(validate_reason("line\nbreak").is_err());
        assert!(validate_reason(&"x".repeat(65)).is_err());
    }

    #[test]
    fn owner_secret_parse_rejects_junk_without_echo() {
        let junk = "definitely-not-a-key-material-string-0000000000000000000000000";
        let err = parse_owner_secret(junk.as_bytes()).unwrap_err();
        assert!(!format!("{err:#}").contains(junk));
        // Canonical hex with trailing newline parses (consumed input).
        let keys = Keys::generate();
        let hex = Zeroizing::new(keys.secret_key().to_secret_hex());
        let parsed = parse_owner_secret(format!("{}\r\n", hex.as_str()).as_bytes()).unwrap();
        assert_eq!(parsed.public_key(), keys.public_key());
        // nsec parses.
        let nsec = Zeroizing::new(keys.secret_key().to_bech32().unwrap());
        let parsed = parse_owner_secret(nsec.as_bytes()).unwrap();
        assert_eq!(parsed.public_key(), keys.public_key());
    }
}
