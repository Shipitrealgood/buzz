//! Registration controller with explicit opt-in deployment reconciliation.
//! Default registration behavior remains independent of the execution helper.
use anyhow::{ensure, Context, Result};
use buzz_hostd::registration::{Host, RegistrationClock};
use clap::Subcommand;
use nostr::PublicKey;
use std::{
    fs::OpenOptions,
    io::{IsTerminal, Write},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Subcommand)]
pub enum Command {
    /// Initialize a separate private registration controller state directory.
    Init,
    /// Print a private v2 invitation only to an operator terminal.
    Offer {
        #[arg(long)]
        community: String,
    },
    /// Print public identity and local registration states only.
    Status,
    /// Persist revocation locally, including when the relay is unavailable.
    Unpair {
        #[arg(long)]
        generation: String,
    },
    /// Reconcile registration. The relay public key is operator-pinned.
    Run {
        #[arg(long)]
        community: String,
        #[arg(long)]
        relay_key: String,
        #[arg(long, default_value = "Linux host")]
        label: String,
        /// Explicit private community CA. TLS identity verification stays enabled.
        #[arg(long)]
        ca_cert: Option<PathBuf>,
        /// One bounded pass, useful for operator checks and disposable fixtures.
        #[arg(long)]
        once: bool,
        /// Enable private deployment only in a compatible fixed helper installation.
        #[arg(long)]
        deployment: bool,
    },
}

pub async fn execute(dir: &Path, command: Command) -> Result<()> {
    let mut host = Host::at(dir, matches!(command, Command::Init))?;
    match command {
        Command::Init => println!("{}", host.public_key()?.to_hex()),
        Command::Offer { community } => {
            ensure!(
                std::io::stdout().is_terminal(),
                "invitation requires a private operator terminal"
            );
            let invitation = host.offer(community, &RegistrationClock::read()?)?;
            writeln!(std::io::stdout().lock(), "{}", &*invitation)?;
        }
        Command::Status => {
            println!("host {}", host.public_key()?.to_hex());
            for (community, generation, state) in host.registrations()? {
                println!("{community} {generation} {state}");
            }
        }
        Command::Unpair { generation } => host.revoke(&generation, &RegistrationClock::read()?)?,
        Command::Run {
            community,
            relay_key,
            label,
            ca_cert,
            once,
            deployment,
        } => {
            ensure!(
                buzz_core::host_registry::origin(&community),
                "invalid community"
            );
            ensure!(
                buzz_core::paired_message::display_name(&label),
                "invalid host label"
            );
            let relay = PublicKey::from_hex(&relay_key)?;
            ensure!(relay.to_hex() == relay_key, "invalid relay public key");
            if deployment {
                buzz_hostd::deployment::Helper::installed(dir)?;
                buzz_hostd::Catalog::read(Path::new(buzz_hostd::OPERATOR_CATALOG))?;
            }
            let _lock = controller_lock(dir)?;
            let connector = connector(ca_cert.as_deref())?;
            crate::registration_refresh::run(
                dir,
                crate::registration_refresh::Settings {
                    url: community.replacen("https://", "wss://", 1),
                    community,
                    relay,
                    label,
                    connector,
                    deployment,
                },
                once,
            )
            .await?;
        }
    }
    Ok(())
}

fn controller_lock(dir: &Path) -> Result<std::fs::File> {
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .mode(0o600)
        .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
        .open(dir.join("reconciler.lock"))?;
    rustix::fs::flock(&lock, rustix::fs::FlockOperation::NonBlockingLockExclusive)
        .context("another host controller owns this state directory")?;
    Ok(lock)
}
fn connector(path: Option<&Path>) -> Result<Option<buzz_ws_client::Connector>> {
    use rustls::pki_types::pem::PemObject;
    let Some(path) = path else { return Ok(None) };
    let pem = buzz_hostd::bounded(path, 65536)?;
    let mut roots = rustls::RootCertStore::empty();
    for cert in rustls::pki_types::CertificateDer::pem_slice_iter(&pem) {
        roots.add(cert?)?;
    }
    ensure!(!roots.is_empty(), "empty community CA");
    let config = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    Ok(Some(buzz_ws_client::Connector::Rustls(Arc::new(config))))
}
