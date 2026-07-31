//! buzz-provision — host-side half of the Buzz virtual mint.
//!
//! `host-init` mints this VM's host identity: the lineage anchor agents
//! carry ("lives-on this host pubkey") and the pre-staged noded identity.
//! It also writes the allowed-owners pin file: only bundles attested by a
//! pinned owner will ever be accepted by `buzz-provision agent`.
//!
//! `pin-owner add` appends an owner to the pin list without touching the
//! host key (owner trust and host identity are independent lifecycles).
//!
//! `keygen` births an agent keypair straight into its credential file;
//! `agent` consumes a `buzz-mint attest` bundle end-to-end: credential
//! verification → secret-free env file → identity events → systemd unit →
//! journal verify. See [`agent`].
//!
//! Key-handling rules (binding): secrets are canonical 64-lowercase-hex, no
//! trailing newline; they are held only in [`zeroize::Zeroizing`] buffers,
//! never formatted, never printed, never logged. Every output surface
//! carries pubkeys and paths only — including clap's own argv errors, which
//! are redacted before they reach stderr.

mod agent;
mod retire;
mod util;

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use clap::{Args, Parser, Subcommand};
use nostr::nips::nip19::ToBech32;
use nostr::Keys;
use zeroize::Zeroizing;

use util::{
    config_dir, credential_pubkey, default_env_dir, ensure_dir_0700, is_canonical_64_hex,
    load_host_keys, merge_owners, redact_key_material, sweep_stale_tempfiles, validate_pubkey_arg,
    write_atomic_0600, write_owners_file, Clobber, ALLOWED_OWNERS_FILE, HOST_KEY_FILE,
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
    /// The printed host pubkey is the lineage anchor for every agent
    /// provisioned on this host (`buzz-mint attest --host` names it). The
    /// secret key never leaves `host.key` and is never printed.
    HostInit(HostInitArgs),
    /// Manage the allowed-owners pin list without touching the host key.
    #[command(subcommand)]
    PinOwner(PinOwnerCommand),
    /// Birth an agent keypair on this host.
    ///
    /// The secret is written straight into its credential file and never
    /// exists anywhere else; the printed PUBLIC key is what the owner
    /// attests with `buzz-mint attest`. Nothing is published here.
    Keygen(KeygenArgs),
    /// Provision an agent from a buzz-mint bundle directory.
    ///
    /// Validates the manifest against the keygen-born credential
    /// (fail-closed, owner-pinned), writes the secret-free env file,
    /// publishes kind:0 → kind:10100, verifies channel memberships (the
    /// owner already published the 9000s at attest time), and enables +
    /// verifies the buzz-agent@<name> unit.
    Agent(agent::AgentArgs),
    /// Retire an agent — the HOST half of teardown (run this FIRST).
    ///
    /// As the agent (its credential must still exist): leaves every channel
    /// (kind:9022), republishes kind:10100 as a not-mentionable tombstone,
    /// stops + disables the unit, shreds the credential, and archives the
    /// env file as <name>.env.retired. The agent's kind:0 is deliberately
    /// NOT touched — the relay verifies the owner's NIP-IA archive request
    /// against it. Finish with `buzz-mint retire` on the OWNER machine: the
    /// kind:9035 archive is the real revocation of the attestation.
    Retire(retire::RetireArgs),
}

#[derive(Args)]
struct HostInitArgs {
    /// Pin an owner PUBLIC key (exactly 64 lowercase hex). Repeatable.
    ///
    /// Only mint bundles attested by a pinned owner are accepted at
    /// provision time. There is no default owner. If the pin file already
    /// exists, new owners are appended (deduplicated), never clobbered.
    #[arg(long = "allow-owner", value_name = "PUBKEY_64HEX")]
    allow_owner: Vec<String>,

    /// Rotate to a NEW host identity even if one already exists.
    ///
    /// DANGER: the published `host` lineage of every agent provisioned
    /// under the old identity stops matching this host.
    #[arg(long)]
    force: bool,

    /// Print the existing host pubkey and pinned owners; touch nothing.
    ///
    /// This is what `buzz-mint attest --host` names as the lineage anchor.
    #[arg(long, conflicts_with_all = ["allow_owner", "force"])]
    show: bool,

    /// Override the config directory (default: ~/.config/buzz-provision).
    #[arg(long, hide = true, value_name = "DIR")]
    config_dir: Option<PathBuf>,
}

#[derive(Args)]
struct KeygenArgs {
    /// Agent name — becomes the credential filename (<name>.nsec), the
    /// systemd instance name, and the bundle name the owner attests.
    #[arg(long)]
    name: String,

    /// Replace an existing credential with a NEW identity.
    ///
    /// DANGER: the old key is overwritten; anything attested or published
    /// under the old pubkey no longer belongs to this instance.
    #[arg(long)]
    force: bool,

    /// Override the env-file directory the <name>.pub marker is written
    /// beside (default: ~/services/buzz-agents). If the directory does not
    /// exist, the pubkey is print-only.
    #[arg(long, hide = true, value_name = "DIR")]
    env_dir: Option<PathBuf>,

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
        Command::Keygen(args) => keygen(&args),
        Command::Agent(args) => agent::provision_agent(&args),
        Command::Retire(args) => retire::retire(&args),
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
             there is no default owner (the pin list is what stops foreign manifests)"
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
             Rotating to a NEW host identity (--force) is almost never what you want: the \
             published `host` lineage of every agent provisioned here stops matching.",
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
             WARNING: the published `host` lineage of every agent provisioned under \
             the OLD identity no longer matches this host.\n\
             WARNING: re-attest every agent against the new host pubkey printed below."
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

/// Print the exact `buzz-mint attest` command the owner should run for this
/// agent. `host_hex` is the pinned host identity when one exists, else a
/// placeholder the owner must fill after `host-init`.
fn print_attest_command(name: &str, agent_pubkey_hex: &str, host_hex: &str) {
    println!("next, on the OWNER machine, attest this key and publish its channel adds:");
    println!();
    println!(
        "  buzz-mint attest --agent {agent_pubkey_hex} --host {host_hex} --name {name} \
         --relay-url <wss://your-relay> [--channel <uuid>]... [--harness-cmd <cmd>]"
    );
    println!();
    println!("then copy the resulting {name}-mint/ bundle back here and run:");
    println!("  buzz-provision agent {name}-mint");
}

fn keygen(args: &KeygenArgs) -> Result<()> {
    // Validate before touching disk. The name becomes a filename and a
    // systemd instance name — same rule the bundle schema enforces.
    buzz_seal::bundle::validate_agent_name(&args.name)
        .map_err(|e| anyhow::anyhow!("--name: {e}"))?;
    let name = args.name.as_str();

    let dir = config_dir(args.config_dir.as_deref())?;
    let creds_dir = dir.join("creds");
    let cred_path = creds_dir.join(format!("{name}.nsec"));
    let host_key_path = dir.join(HOST_KEY_FILE);
    // Host identity is optional for keygen itself (key birth must not depend
    // on it) but fills in the printed attest command when present.
    let host_hex = if host_key_path.exists() {
        load_host_keys(&host_key_path)?.public_key().to_hex()
    } else {
        eprintln!(
            "note: no host identity at {} yet — run `buzz-provision host-init` before \
             provisioning; the printed attest command carries a <host-pubkey> placeholder",
            host_key_path.display()
        );
        "<host-pubkey>".to_owned()
    };

    if cred_path.exists() && !args.force {
        // Non-steering refusal: the right next step for an existing key is
        // attesting it, not --force. Print the EXISTING pubkey so the owner
        // can do exactly that; mention --force last, with its cost.
        let existing = credential_pubkey(&cred_path);
        eprintln!(
            "credential {} already exists — refusing to overwrite.",
            cred_path.display()
        );
        match existing {
            Some(pubkey) => {
                eprintln!(
                    "The existing key is intact; its pubkey is {}.\n\
                     To (re-)attest it, run on the owner machine:",
                    pubkey.to_hex()
                );
                eprintln!(
                    "  buzz-mint attest --agent {} --host {host_hex} --name {name} \
                     --relay-url <wss://your-relay> [--channel <uuid>]...",
                    pubkey.to_hex()
                );
            }
            None => eprintln!(
                "It does not parse as a canonical credential — inspect it before anything else."
            ),
        }
        bail!(
            "generating a NEW identity (--force) overwrites the credential: anything attested \
             or published under the old pubkey no longer belongs to this instance"
        );
    }

    ensure_dir_0700(&dir)?;
    ensure_dir_0700(&creds_dir)?;
    sweep_stale_tempfiles(&dir);
    sweep_stale_tempfiles(&creds_dir);

    // Same Keys::generate() family as the native desktop mint; canonical
    // 64-lowercase-hex, byte-exact, tempfile+rename (host-init discipline).
    let keys = Keys::generate();
    let sk_hex: Zeroizing<String> = Zeroizing::new(keys.secret_key().to_secret_hex());
    if !is_canonical_64_hex(sk_hex.as_bytes()) {
        bail!(
            "internal error: generated secret is not canonical 64-lowercase-hex; nothing written"
        );
    }
    let clobber = if args.force {
        Clobber::Allow
    } else {
        // The rename itself refuses to clobber: two racing keygens cannot
        // both win, so the printed pubkey always matches what is on disk.
        Clobber::Refuse
    };
    write_atomic_0600(&cred_path, sk_hex.as_bytes(), clobber)?;
    let pubkey = keys.public_key();

    if args.force {
        eprintln!(
            "WARNING: --force replaced the previous credential; anything attested under the \
             old pubkey no longer belongs to this instance. Re-attest with the new pubkey below."
        );
    }

    // Public marker beside the env-file convention — only if the deployment
    // dir already exists (keygen creates credentials, not deployment trees).
    let env_dir = default_env_dir(args.env_dir.as_deref())?;
    let pub_path = env_dir.join(format!("{name}.pub"));
    let pub_written = if env_dir.is_dir() {
        write_atomic_0600(
            &pub_path,
            format!("{}\n", pubkey.to_hex()).as_bytes(),
            Clobber::Allow,
        )?;
        true
    } else {
        false
    };

    println!("agent {name:?}: key born on this host");
    println!("agent pubkey (hex):  {}", pubkey.to_hex());
    match pubkey.to_bech32() {
        Ok(npub) => println!("agent pubkey (npub): {npub}"),
        Err(_) => println!("agent pubkey (npub): <bech32 encoding failed>"),
    }
    println!(
        "credential:          {} (0600; never printed, never leaves this host)",
        cred_path.display()
    );
    if pub_written {
        println!("pubkey marker:       {}", pub_path.display());
    } else {
        println!(
            "pubkey marker:       not written ({} does not exist yet; provision creates it)",
            env_dir.display()
        );
    }
    println!();
    print_attest_command(name, &pubkey.to_hex(), &host_hex);
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
