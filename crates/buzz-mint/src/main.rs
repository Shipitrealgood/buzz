//! buzz-mint — the owner-side half of the Buzz virtual mint (D1b).
//!
//! Runs where the owner key lives (Windows workstation is the eventual
//! primary locus). `new` mirrors the native desktop mint pattern
//! (`create_managed_agent`): `Keys::generate()` → NIP-OA auth tag with empty
//! conditions → then, instead of a keyring, seals the agent secret to the
//! target host's pubkey (NIP-44 v2, owner as static sender) and pre-signs
//! the owner kind:9000 bot-membership batch. Output is a mint bundle
//! directory consumed by `buzz-provision agent` (T4).
//!
//! Key-handling rules (binding, sealed-mint-bundle spec): the owner secret
//! arrives via a 0600 file or a silent prompt — never argv/env; it is held
//! in [`Zeroizing`]/[`Keys`] only as long as signing takes. The agent secret
//! exists owner-side only between generation and seal. No print/log surface
//! ever formats secret material — pubkeys, paths, and event ids only.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{IsTerminal, Read, Write};
#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use buzz_seal::bundle::{self, Manifest, ManifestConfig, SealedPresence, MANIFEST_VERSION};
use buzz_seal::{is_canonical_64_hex, seal, AgentSecretKey, InnerPayload, SecretString};
use clap::{Args, Parser, Subcommand, ValueEnum};
use nostr::nips::nip19::ToBech32;
use nostr::{Event, JsonUtil, Keys, PublicKey, SecretKey};
use uuid::Uuid;
use zeroize::Zeroizing;

#[derive(Parser)]
#[command(
    name = "buzz-mint",
    version,
    about = "Owner-side mint for VM-deployed Buzz agents: keygen, attestation, seal-to-host bundle"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Mint a new agent: keypair, NIP-OA attestation, sealed bundle, 9000 batch.
    New(NewArgs),
    /// Validate a mint bundle's structure and internal consistency.
    ///
    /// Pure pre-flight: uses public data only — no secret input, no decrypt.
    Inspect(InspectArgs),
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
struct NewArgs {
    /// Agent name (bundle dir becomes <name>-mint/).
    #[arg(long)]
    name: String,

    /// Target host PUBLIC key (exactly 64 lowercase hex) — the seal target,
    /// printed by `buzz-provision host-init --show` on the VM.
    #[arg(long, value_name = "PUBKEY_64HEX")]
    host: String,

    /// Channel id to pre-sign an owner kind:9000 (role=bot) add for. Repeatable.
    #[arg(long = "channel", value_name = "CHANNEL_UUID")]
    channels: Vec<Uuid>,

    /// Who the agent responds to (written to the public manifest config).
    #[arg(long = "respond-to", value_enum, default_value = "owner-only")]
    respond_to: RespondTo,

    /// Harness command for the host-side unit (public manifest config).
    #[arg(long = "harness-cmd", value_name = "CMD")]
    harness_cmd: Option<String>,

    /// Relay URL the agent will connect to (public manifest config,
    /// `BUZZ_RELAY_URL`). Optional: provision accepts `--relay-url` instead.
    #[arg(long = "relay-url", value_name = "WS_URL")]
    relay_url: Option<String>,

    /// Extra secret sealed into the bundle as KEY=VALUE (e.g. a provider API
    /// key). Repeatable. The VALUE is secret-tier: sealed, never printed.
    #[arg(long = "seal-env", value_name = "KEY=VALUE")]
    seal_env: Vec<String>,

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

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::New(args) => mint_new(&args),
        Command::Inspect(args) => inspect(&args.bundle_dir),
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

fn read_owner_keys(args: &NewArgs) -> Result<Keys> {
    if let Some(path) = &args.owner_key_file {
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
/// Shared rule with the provision-side consumer (buzz_seal::bundle).
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

fn validate_host_pubkey(s: &str) -> Result<PublicKey> {
    if !is_canonical_64_hex(s.as_bytes()) {
        bail!(
            "invalid --host value (got {} chars): expected the host PUBLIC key as exactly \
             64 lowercase hex characters, as printed by `buzz-provision host-init --show`",
            s.len()
        );
    }
    PublicKey::from_hex(s).map_err(|_| {
        anyhow::anyhow!("invalid --host value: not a valid x-only secp256k1 public key")
    })
}

/// Parse `--seal-env KEY=VALUE`. The VALUE is secret-tier and never echoed.
fn parse_seal_env(entries: &[String]) -> Result<Option<BTreeMap<String, SecretString>>> {
    if entries.is_empty() {
        return Ok(None);
    }
    let mut map = BTreeMap::new();
    for entry in entries {
        let (key, value) = entry.split_once('=').ok_or_else(|| {
            anyhow::anyhow!("--seal-env entries must be KEY=VALUE (value withheld)")
        })?;
        let key_ok = !key.is_empty()
            && key
                .bytes()
                .next()
                .is_some_and(|b| b.is_ascii_uppercase() || b == b'_')
            && key
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_');
        if !key_ok {
            bail!("--seal-env key {key:?} is not a valid env var name ([A-Z_][A-Z0-9_]*)");
        }
        if value.is_empty() {
            bail!("--seal-env {key} has an empty value");
        }
        if map
            .insert(key.to_owned(), SecretString::new(value.to_owned()))
            .is_some()
        {
            bail!("--seal-env key {key} given more than once");
        }
    }
    Ok(Some(map))
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

/// Write a file inside the (fresh, 0700) bundle dir with the given mode,
/// creation-only (`create_new`), fsynced.
fn write_bundle_file(path: &Path, bytes: &[u8], mode: u32) -> Result<()> {
    let mut opts = fs::OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    opts.mode(mode);
    #[cfg(not(unix))]
    let _ = mode; // TODO(windows): DACL-restrict sealed.json equivalently.
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

// ─── new ─────────────────────────────────────────────────────────────────────

fn mint_new(args: &NewArgs) -> Result<()> {
    // 1) Validate everything before touching secrets or disk.
    validate_name(&args.name)?;
    let host_pubkey = validate_host_pubkey(&args.host)?;
    if let Some(relay_url) = &args.relay_url {
        validate_relay_url(relay_url)?;
    }
    let api_keys = parse_seal_env(&args.seal_env)?;
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
            "bundle directory {} already exists — refusing to overwrite. A re-mint generates \
             a NEW agent identity; remove the old bundle or choose another --out",
            bundle_dir.display()
        );
    }

    // 2) Owner key in, everything signed/sealed, owner key dropped — before
    //    any disk write, so a write failure never leaves half-signed state.
    let owner_keys = read_owner_keys(args)?;
    let owner_pubkey = owner_keys.public_key();
    if owner_pubkey == host_pubkey {
        bail!(
            "--host equals the OWNER pubkey; expected the host identity printed by \
             `buzz-provision host-init --show`"
        );
    }

    // Same Keys::generate() family as the native desktop mint (agents.rs:632).
    let agent_keys = Keys::generate();
    let agent_pubkey = agent_keys.public_key();

    // NIP-OA auth tag, empty conditions — native parity (agents.rs:667-677).
    let attestation = buzz_sdk::nip_oa::compute_auth_tag(&owner_keys, &agent_pubkey, "")
        .map_err(|e| anyhow::anyhow!("cannot compute NIP-OA auth tag: {e}"))?;

    let created_at = nostr::Timestamp::now().as_secs();
    let inner = InnerPayload {
        v: 1,
        agent_sk: AgentSecretKey::from_keys(&agent_keys)
            .map_err(|e| anyhow::anyhow!("generated key failed canonical validation: {e}"))?,
        agent_pubkey,
        attestation: attestation.clone(),
        host_pubkey,
        created_at,
        api_keys,
    };
    let envelope =
        seal(&owner_keys, &host_pubkey, &inner).map_err(|e| anyhow::anyhow!("seal failed: {e}"))?;

    // Pre-sign the owner kind:9000 batch — one add-member(role=bot) per
    // channel, exactly the h/p/role tag shape the desktop emits and the
    // relay's PUT_USER validator parses. Idempotent to replay: same-role
    // re-adds converge (side_effects.rs).
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

    // Owner key is done — drop it before any I/O.
    drop(owner_keys);

    // 3) Bundle fallout: dir 0700, manifest (public), sealed.json 0600, events/.
    let manifest = Manifest {
        v: MANIFEST_VERSION,
        name: args.name.clone(),
        agent_pubkey,
        owner_pubkey,
        host_pubkey,
        attestation,
        created_at,
        config: ManifestConfig {
            respond_to: args.respond_to.as_str().to_owned(),
            channels: channels.clone(),
            harness_cmd: args.harness_cmd.clone(),
            relay_url: args.relay_url.clone(),
        },
    };

    create_dir_0700(&bundle_dir)?;
    let manifest_path = bundle_dir.join("manifest.json");
    let sealed_path = bundle_dir.join("sealed.json");
    let events_dir = bundle_dir.join("events");
    write_bundle_file(
        &manifest_path,
        serde_json::to_string_pretty(&manifest)
            .context("manifest serialization failed")?
            .as_bytes(),
        0o644,
    )?;
    // The envelope holds only ciphertext + routing hints, but it is the
    // secret-carrying artifact: 0600 per spec.
    write_bundle_file(
        &sealed_path,
        serde_json::to_string(&envelope)
            .context("envelope serialization failed")?
            .as_bytes(),
        0o600,
    )?;
    create_dir_0700(&events_dir)?;
    for (channel, event) in &events {
        write_bundle_file(
            &events_dir.join(format!("9000-{channel}.json")),
            event.as_json().as_bytes(),
            0o644,
        )?;
    }
    fsync_dir(&events_dir)?;
    fsync_dir(&bundle_dir)?;

    // 4) Report — pubkeys, paths, event ids ONLY.
    println!("minted agent {:?}", args.name);
    println!("agent pubkey (hex):  {}", agent_pubkey.to_hex());
    match agent_pubkey.to_bech32() {
        Ok(npub) => println!("agent pubkey (npub): {npub}"),
        Err(_) => println!("agent pubkey (npub): <bech32 encoding failed>"),
    }
    println!("owner pubkey:        {}", owner_pubkey.to_hex());
    println!("host pubkey:         {}", host_pubkey.to_hex());
    println!("bundle: {}", bundle_dir.display());
    println!("  manifest.json  (public: name, pubkeys, attestation, config)");
    println!("  sealed.json    (0600: NIP-44 v2 envelope, opens only on the host)");
    for (channel, event) in &events {
        println!(
            "  events/9000-{channel}.json  (owner-signed bot add, id {})",
            event.id
        );
    }
    println!();
    println!("next: copy the bundle to the host and run `buzz-provision agent`.");
    if !events.is_empty() {
        println!(
            "9000 preconditions (enforced by the relay at publish time): each channel must \
             not be archived and the owner must be an active member; re-publishing the batch \
             is idempotent."
        );
    }
    Ok(())
}

// ─── inspect ─────────────────────────────────────────────────────────────────

/// Owner-side pre-flight: validate bundle structure and manifest/envelope
/// consistency using PUBLIC data only. No secret input, no decrypt — the
/// checks it cannot make (inner payload authority) are exactly the sealed
/// spec's unseal steps 2-6, run host-side.
fn inspect(bundle_dir: &Path) -> Result<()> {
    let bundle =
        bundle::load(bundle_dir, SealedPresence::Required).map_err(|e| anyhow::anyhow!("{e:#}"))?;
    #[cfg_attr(not(unix), allow(unused_mut))]
    let mut problems = bundle::validate(&bundle);
    let manifest = &bundle.manifest;

    // Mode hygiene (unix; Windows locus gets DACL checks later).
    #[cfg(unix)]
    {
        let sealed_path = bundle_dir.join("sealed.json");
        let mode = |p: &Path| -> Option<u32> {
            fs::metadata(p)
                .ok()
                .map(|m| m.permissions().mode() & 0o7777)
        };
        if let Some(m) = mode(bundle_dir) {
            if m & 0o077 != 0 {
                problems.push(format!(
                    "bundle dir mode {m:04o} is group/world accessible (want 0700)"
                ));
            }
        }
        if let Some(m) = mode(&sealed_path) {
            if m & 0o077 != 0 {
                problems.push(format!(
                    "sealed.json mode {m:04o} is group/world accessible (want 0600)"
                ));
            }
        }
    }
    // TODO(windows): replace unix mode checks with a DACL owner-only check.

    println!("bundle:       {}", bundle_dir.display());
    println!("name:         {}", manifest.name);
    println!("agent pubkey: {}", manifest.agent_pubkey.to_hex());
    println!("owner pubkey: {}", manifest.owner_pubkey.to_hex());
    println!("host pubkey:  {}", manifest.host_pubkey.to_hex());
    println!("respond_to:   {}", manifest.config.respond_to);
    println!("channels:     {}", manifest.config.channels.len());

    if problems.is_empty() {
        println!(
            "inspect: OK — structure and public-field consistency verified (inner payload \
             authority is verified host-side at unseal)"
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
    fn seal_env_parsing_never_echoes_value() {
        let secret = "super-secret-value-000";
        let err = parse_seal_env(&[format!("bad-key={secret}")]).unwrap_err();
        assert!(!format!("{err:#}").contains(secret));
        let err = parse_seal_env(&[secret.to_string()]).unwrap_err();
        assert!(!format!("{err:#}").contains(secret));
        let ok = parse_seal_env(&[format!("GOOD_KEY={secret}")])
            .unwrap()
            .unwrap();
        assert_eq!(ok.get("GOOD_KEY").unwrap().expose(), secret);
        assert!(
            parse_seal_env(&[format!("GOOD_KEY={secret}"), format!("GOOD_KEY={secret}")]).is_err()
        );
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
