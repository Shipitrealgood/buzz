//! buzz-provision — host-side half of the Buzz virtual mint.
//!
//! `host-init` mints this VM's host identity: the keypair every sealed mint
//! bundle is encrypted to (the seal target), the lineage anchor agents carry
//! ("lives-on this host pubkey"), and the pre-staged noded identity. It also
//! writes the allowed-owners pin file: only sealed bundles attested by a
//! pinned owner will ever be accepted by `buzz-provision agent`.
//!
//! Key-handling rules (binding, from the sealed-mint-bundle spec):
//! the host secret is canonical 64-lowercase-hex, no trailing newline; it is
//! held only in [`Zeroizing`] buffers, never formatted, never printed, never
//! logged. Every output surface carries pubkeys and paths only.

use std::fs::{self, File};
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::{Args, Parser, Subcommand};
use nostr::nips::nip19::ToBech32;
use nostr::{Keys, PublicKey, SecretKey};
use zeroize::Zeroizing;

/// Filename of the host secret key (canonical 64-lowercase-hex, 0600).
const HOST_KEY_FILE: &str = "host.key";
/// Filename of the owner pin list (one 64-hex pubkey per line).
const ALLOWED_OWNERS_FILE: &str = "allowed-owners";

#[derive(Parser)]
#[command(
    name = "buzz-provision",
    version,
    about = "Host-side provisioning for VM-deployed Buzz agents (virtual mint)"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Mint this host's identity keypair and pin the allowed owner(s).
    ///
    /// The printed host pubkey is the seal target for `buzz-mint` and the
    /// lineage anchor for every agent provisioned on this host. The secret
    /// key never leaves `host.key` and is never printed.
    HostInit(HostInitArgs),
}

#[derive(Args)]
struct HostInitArgs {
    /// Pin an owner PUBLIC key (exactly 64 lowercase hex). Repeatable.
    ///
    /// Only sealed mint bundles attested by a pinned owner are accepted at
    /// provision time. There is no default owner. If the pin file already
    /// exists, new owners are appended (deduplicated), never clobbered.
    #[arg(long = "allow-owner", value_name = "PUBKEY_64HEX")]
    allow_owner: Vec<String>,

    /// Rotate to a NEW host identity even if one already exists.
    ///
    /// DANGER: every blob previously sealed to the old host pubkey becomes
    /// permanently unopenable.
    #[arg(long)]
    force: bool,

    /// Print the existing host pubkey and pinned owners; touch nothing.
    ///
    /// This is what `buzz-mint` users run to obtain the seal target.
    #[arg(long, conflicts_with_all = ["allow_owner", "force"])]
    show: bool,

    /// Override the config directory (default: ~/.config/buzz-provision).
    #[arg(
        long,
        env = "BUZZ_PROVISION_CONFIG_DIR",
        hide = true,
        value_name = "DIR"
    )]
    config_dir: Option<PathBuf>,
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::HostInit(args) => {
            if args.show {
                host_show(&args)
            } else {
                host_init(&args)
            }
        }
    };
    if let Err(e) = result {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

/// Resolve the config directory: explicit flag/env override, else
/// `$HOME/.config/buzz-provision`.
fn config_dir(args: &HostInitArgs) -> Result<PathBuf> {
    if let Some(dir) = &args.config_dir {
        return Ok(dir.clone());
    }
    let home = std::env::var_os("HOME")
        .filter(|h| !h.is_empty())
        .context("HOME is not set; pass --config-dir explicitly")?;
    Ok(PathBuf::from(home).join(".config").join("buzz-provision"))
}

/// True iff `s` is exactly 64 ASCII lowercase-hex characters.
fn is_canonical_64_hex(s: &[u8]) -> bool {
    s.len() == 64 && s.iter().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// Strictly validate an owner pubkey argument.
///
/// The value is NEVER echoed back in errors: a mistakenly pasted secret key
/// is indistinguishable from a public key (both are 64 hex chars), so no
/// error path may reproduce the input.
fn validate_owner_pubkey(s: &str) -> Result<String> {
    if !is_canonical_64_hex(s.as_bytes()) {
        bail!(
            "invalid --allow-owner value (got {} chars): expected the owner PUBLIC key \
             as exactly 64 lowercase hex characters (no npub/nsec bech32, no uppercase)",
            s.len()
        );
    }
    if PublicKey::from_hex(s).is_err() {
        bail!(
            "invalid --allow-owner value: not a valid x-only secp256k1 public key \
             (value withheld from this message on purpose)"
        );
    }
    Ok(s.to_string())
}

/// Ensure the config directory exists with mode 0700.
///
/// Existing contents (e.g. `creds/`) are never touched; if the directory
/// already exists only its own mode is corrected to 0700.
fn ensure_config_dir(dir: &Path) -> Result<()> {
    if dir.symlink_metadata().is_ok() {
        let meta = fs::metadata(dir)
            .with_context(|| format!("cannot stat config dir {}", dir.display()))?;
        if !meta.is_dir() {
            bail!(
                "config path {} exists but is not a directory",
                dir.display()
            );
        }
        if meta.permissions().mode() & 0o7777 != 0o700 {
            fs::set_permissions(dir, fs::Permissions::from_mode(0o700))
                .with_context(|| format!("cannot set 0700 on {}", dir.display()))?;
        }
    } else {
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)
            .with_context(|| format!("cannot create config dir {}", dir.display()))?;
    }
    Ok(())
}

/// Atomically write `bytes` to `path` with mode 0600: tempfile in the same
/// directory, permissions set before content lands, fsync, rename, dir fsync.
fn write_atomic_0600(path: &Path, bytes: &[u8]) -> Result<()> {
    let dir = path
        .parent()
        .with_context(|| format!("{} has no parent directory", path.display()))?;
    let mut tmp = tempfile::Builder::new()
        .prefix(".buzz-provision.")
        .tempfile_in(dir)
        .with_context(|| format!("cannot create tempfile in {}", dir.display()))?;
    // NamedTempFile is created 0600 on Unix already; set it explicitly so the
    // invariant does not rest on a platform default.
    tmp.as_file()
        .set_permissions(fs::Permissions::from_mode(0o600))
        .context("cannot set 0600 on tempfile")?;
    tmp.as_file_mut()
        .write_all(bytes)
        .context("cannot write tempfile")?;
    tmp.as_file().sync_all().context("cannot fsync tempfile")?;
    tmp.persist(path)
        .map_err(|e| e.error)
        .with_context(|| format!("cannot rename tempfile into {}", path.display()))?;
    // Make the rename durable.
    File::open(dir)
        .and_then(|d| d.sync_all())
        .with_context(|| format!("cannot fsync directory {}", dir.display()))?;
    Ok(())
}

/// Merge owner pins: existing file entries first (validated), then new
/// owners, order-preserving, deduplicated.
fn merge_owners(owners_path: &Path, new: &[String]) -> Result<Vec<String>> {
    let mut merged: Vec<String> = Vec::new();
    if owners_path.exists() {
        let existing = fs::read_to_string(owners_path)
            .with_context(|| format!("cannot read {}", owners_path.display()))?;
        for line in existing.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let v = validate_owner_pubkey(line).with_context(|| {
                format!(
                    "existing pin file {} contains an invalid entry",
                    owners_path.display()
                )
            })?;
            if !merged.contains(&v) {
                merged.push(v);
            }
        }
    }
    for owner in new {
        if !merged.contains(owner) {
            merged.push(owner.clone());
        }
    }
    Ok(merged)
}

/// Read and strictly validate the host key file; returns the parsed keys.
///
/// The raw buffer is zeroized on drop. No error path formats file content.
fn load_host_keys(key_path: &Path) -> Result<Keys> {
    let raw = Zeroizing::new(
        fs::read(key_path).with_context(|| format!("cannot read {}", key_path.display()))?,
    );
    if !is_canonical_64_hex(&raw) {
        bail!(
            "{} is not canonical: expected exactly 64 lowercase hex bytes \
             (no newline, no bech32); refusing to use it",
            key_path.display()
        );
    }
    // Charset already validated; from_utf8 cannot fail but stay fail-closed.
    let sk_str = std::str::from_utf8(&raw)
        .map_err(|_| anyhow::anyhow!("{} is not valid UTF-8", key_path.display()))?;
    let secret = SecretKey::from_hex(sk_str).map_err(|_| {
        // Deliberately drop the underlying error: it could echo input bytes.
        anyhow::anyhow!(
            "{} does not contain a valid secp256k1 secret key",
            key_path.display()
        )
    })?;
    Ok(Keys::new(secret))
}

fn print_identity(keys: &Keys, key_path: &Path, owners_path: &Path, owners: &[String]) {
    let pubkey = keys.public_key();
    println!("host pubkey (hex):  {}", pubkey.to_hex());
    match pubkey.to_bech32() {
        Ok(npub) => println!("host pubkey (npub): {npub}"),
        Err(_) => println!("host pubkey (npub): <bech32 encoding failed>"),
    }
    println!("host key:           {} (0600)", key_path.display());
    println!("allowed owners:     {}", owners_path.display());
    println!("pinned owners ({}):", owners.len());
    for owner in owners {
        println!("  {owner}");
    }
}

fn host_init(args: &HostInitArgs) -> Result<()> {
    // Validate arguments before touching anything on disk.
    let mut new_owners: Vec<String> = Vec::new();
    for raw in &args.allow_owner {
        let v = validate_owner_pubkey(raw)?;
        if !new_owners.contains(&v) {
            new_owners.push(v);
        }
    }
    if new_owners.is_empty() {
        bail!(
            "at least one --allow-owner <owner pubkey, 64 lowercase hex> is required; \
             there is no default owner (the pin list is what stops rogue sealed blobs)"
        );
    }

    let dir = config_dir(args)?;
    let key_path = dir.join(HOST_KEY_FILE);
    let owners_path = dir.join(ALLOWED_OWNERS_FILE);

    let rotating = key_path.exists();
    if rotating && !args.force {
        bail!(
            "host key already exists at {} — refusing to overwrite.\n\
             Use `buzz-provision host-init --show` to print the existing identity.\n\
             Re-run with --force ONLY to rotate to a NEW host identity \
             (this permanently orphans every blob sealed to the current host pubkey).",
            key_path.display()
        );
    }

    ensure_config_dir(&dir)?;

    // Generate the host keypair — same Keys::generate() family the desktop
    // native mint uses.
    let keys = Keys::generate();
    let sk_hex: Zeroizing<String> = Zeroizing::new(keys.secret_key().to_secret_hex());
    // Defense in depth: never write a non-canonical secret.
    if !is_canonical_64_hex(sk_hex.as_bytes()) {
        bail!(
            "internal error: generated secret is not canonical 64-lowercase-hex; nothing written"
        );
    }

    // Owners first, key last: host.key's existence is the "initialized"
    // marker, so a crash between the two writes leaves a state that a plain
    // re-run repairs (owner merge is idempotent) instead of one that demands
    // --force.
    let merged = merge_owners(&owners_path, &new_owners)?;
    let mut owners_content = String::with_capacity(merged.len() * 65);
    for owner in &merged {
        owners_content.push_str(owner);
        owners_content.push('\n');
    }
    write_atomic_0600(&owners_path, owners_content.as_bytes())?;

    // Canonical 64-lowercase-hex, NO trailing newline.
    write_atomic_0600(&key_path, sk_hex.as_bytes())?;

    if rotating {
        eprintln!(
            "WARNING: --force minted a NEW host identity.\n\
             WARNING: every blob previously sealed to the OLD host pubkey is now \
             permanently unopenable (orphaned).\n\
             WARNING: re-mint every agent against the new host pubkey printed below."
        );
    }

    print_identity(&keys, &key_path, &owners_path, &merged);
    Ok(())
}

fn host_show(args: &HostInitArgs) -> Result<()> {
    let dir = config_dir(args)?;
    let key_path = dir.join(HOST_KEY_FILE);
    let owners_path = dir.join(ALLOWED_OWNERS_FILE);

    if !key_path.exists() {
        bail!(
            "no host identity found at {} — run \
             `buzz-provision host-init --allow-owner <owner pubkey>` first",
            key_path.display()
        );
    }
    let keys = load_host_keys(&key_path)?;

    let mut owners: Vec<String> = Vec::new();
    if owners_path.exists() {
        owners = merge_owners(&owners_path, &[])?;
    }
    print_identity(&keys, &key_path, &owners_path, &owners);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_hex_check() {
        assert!(is_canonical_64_hex(
            b"f5c79eab2f44284ff278780c34aab621aafb149be4d7dba8f9625bfd99419733"
        ));
        // 63 chars
        assert!(!is_canonical_64_hex(
            b"f5c79eab2f44284ff278780c34aab621aafb149be4d7dba8f9625bfd9941973"
        ));
        // uppercase
        assert!(!is_canonical_64_hex(
            b"F5C79EAB2F44284FF278780C34AAB621AAFB149BE4D7DBA8F9625BFD99419733"
        ));
        // trailing newline
        assert!(!is_canonical_64_hex(
            b"f5c79eab2f44284ff278780c34aab621aafb149be4d7dba8f9625bfd99419733\n"
        ));
    }

    #[test]
    fn owner_validation_never_echoes_value() {
        let bogus = "NSEC1THISLOOKSLIKEASECRETVALUETHATMUSTNOTAPPEARINERRORS000000000";
        let err = validate_owner_pubkey(bogus).unwrap_err();
        assert!(!format!("{err:#}").contains(bogus));
    }
}
