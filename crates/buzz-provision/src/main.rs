//! buzz-provision — host-side half of the Buzz virtual mint.
//!
//! `host-init` mints this VM's host identity: the keypair every sealed mint
//! bundle is encrypted to (the seal target), the lineage anchor agents carry
//! ("lives-on this host pubkey"), and the pre-staged noded identity. It also
//! writes the allowed-owners pin file: only sealed bundles attested by a
//! pinned owner will ever be accepted by `buzz-provision agent`.
//!
//! `pin-owner add` appends an owner to the pin list without touching the
//! host key (owner trust and host identity are independent lifecycles).
//!
//! `agent` consumes a `buzz-mint` bundle end-to-end: unseal → credential →
//! secret-free env file → identity events → systemd unit → journal verify →
//! shred. See [`agent`].
//!
//! Key-handling rules (binding, from the sealed-mint-bundle spec):
//! secrets are canonical 64-lowercase-hex, no trailing newline; they are
//! held only in [`zeroize::Zeroizing`] buffers, never formatted, never
//! printed, never logged. Every output surface carries pubkeys and paths
//! only — including clap's own argv errors, which are redacted before they
//! reach stderr.

mod agent;
mod util;

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use clap::{Args, Parser, Subcommand};
use nostr::nips::nip19::ToBech32;
use nostr::Keys;
use zeroize::Zeroizing;

use util::{
    config_dir, ensure_dir_0700, is_canonical_64_hex, load_host_keys, merge_owners,
    redact_key_material, sweep_stale_tempfiles, validate_pubkey_arg, write_atomic_0600,
    write_owners_file, Clobber, ALLOWED_OWNERS_FILE, HOST_KEY_FILE,
};

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
    /// Manage the allowed-owners pin list without touching the host key.
    #[command(subcommand)]
    PinOwner(PinOwnerCommand),
    /// Provision an agent from a buzz-mint bundle directory.
    ///
    /// Unseals the bundle (fail-closed, owner-pinned), writes the systemd
    /// credential and secret-free env file, publishes the agent's identity
    /// events (kind:0 → kind:10100 → owner-signed kind:9000 batch), enables
    /// and verifies the buzz-agent@<name> unit, then shreds sealed.json.
    Agent(agent::AgentArgs),
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
    #[arg(long, hide = true, value_name = "DIR")]
    config_dir: Option<PathBuf>,
}

#[derive(Subcommand)]
enum PinOwnerCommand {
    /// Append an owner PUBLIC key to the allowed-owners pin list.
    ///
    /// Deduplicated; the host key is never read or written. Use this to
    /// trust an additional owner on an already-initialized host — no
    /// `--force`, no key rotation.
    Add(PinOwnerAddArgs),
}

#[derive(Args)]
struct PinOwnerAddArgs {
    /// Owner PUBLIC key, exactly 64 lowercase hex.
    #[arg(value_name = "PUBKEY_64HEX")]
    pubkey: String,

    /// Override the config directory (default: ~/.config/buzz-provision).
    #[arg(long, hide = true, value_name = "DIR")]
    config_dir: Option<PathBuf>,
}

fn main() {
    // try_parse instead of parse: clap's own error rendering echoes argv
    // values verbatim, and a secret pasted without its flag prefix must not
    // land on stderr (transcripts are a real leak channel — chuck T1-4).
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            let is_error = e.use_stderr();
            let rendered = redact_key_material(&e.render().to_string());
            if is_error {
                eprint!("{rendered}");
            } else {
                // --help / --version go to stdout with exit code 0.
                print!("{rendered}");
            }
            std::process::exit(e.exit_code());
        }
    };
    let result = match cli.command {
        Command::HostInit(args) => {
            if args.show {
                host_show(&args)
            } else {
                host_init(&args)
            }
        }
        Command::PinOwner(PinOwnerCommand::Add(args)) => pin_owner_add(&args),
        Command::Agent(args) => agent::provision_agent(&args),
    };
    if let Err(e) = result {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
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
        validate_pubkey_arg(raw, "--allow-owner")?;
        if !new_owners.iter().any(|o| o == raw) {
            new_owners.push(raw.clone());
        }
    }
    if new_owners.is_empty() {
        bail!(
            "at least one --allow-owner <owner pubkey, 64 lowercase hex> is required; \
             there is no default owner (the pin list is what stops rogue sealed blobs)"
        );
    }

    let dir = config_dir(args.config_dir.as_deref())?;
    let key_path = dir.join(HOST_KEY_FILE);
    let owners_path = dir.join(ALLOWED_OWNERS_FILE);

    let rotating = key_path.exists();
    if rotating && !args.force {
        bail!(
            "host key already exists at {} — refusing to overwrite.\n\
             To print the existing identity:  buzz-provision host-init --show\n\
             To pin an additional owner:      buzz-provision pin-owner add <owner pubkey>\n\
             Rotating to a NEW host identity (--force) is almost never what you want: it \
             permanently orphans every blob sealed to the current host pubkey.",
            key_path.display()
        );
    }

    ensure_dir_0700(&dir)?;
    sweep_stale_tempfiles(&dir);

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
    write_owners_file(&owners_path, &merged)?;

    // Canonical 64-lowercase-hex, NO trailing newline. On the fresh-init path
    // the rename itself refuses to clobber: two racing host-inits cannot both
    // win, so the printed pubkey always matches what is on disk (chuck T1-2).
    let clobber = if args.force {
        Clobber::Allow
    } else {
        Clobber::Refuse
    };
    write_atomic_0600(&key_path, sk_hex.as_bytes(), clobber)?;

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
    let dir = config_dir(args.config_dir.as_deref())?;
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

fn pin_owner_add(args: &PinOwnerAddArgs) -> Result<()> {
    validate_pubkey_arg(&args.pubkey, "pin-owner add")?;

    let dir = config_dir(args.config_dir.as_deref())?;
    let key_path = dir.join(HOST_KEY_FILE);
    let owners_path = dir.join(ALLOWED_OWNERS_FILE);

    if !key_path.exists() {
        bail!(
            "no host identity found at {} — run \
             `buzz-provision host-init --allow-owner <owner pubkey>` first \
             (host-init pins the first owner itself)",
            key_path.display()
        );
    }

    let key_bytes_before = fs::read(&key_path).ok();
    let merged = merge_owners(&owners_path, std::slice::from_ref(&args.pubkey))?;
    write_owners_file(&owners_path, &merged)?;
    // Invariant check, cheap and loud: pin-owner must never move the key.
    if fs::read(&key_path).ok() != key_bytes_before {
        bail!("internal error: host.key changed during pin-owner add — investigate immediately");
    }

    println!("allowed owners: {}", owners_path.display());
    println!("pinned owners ({}):", merged.len());
    for owner in &merged {
        println!("  {owner}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::util::*;

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
        let err = validate_pubkey_arg(bogus, "--allow-owner").unwrap_err();
        assert!(!format!("{err:#}").contains(bogus));
    }
}
