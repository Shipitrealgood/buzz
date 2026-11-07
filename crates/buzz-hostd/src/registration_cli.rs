//! The existing executable's registration-only entry point. It never calls the
//! provisioning poller, runtime catalog, activation helper or agent unit code.
use anyhow::{ensure, Context, Result};
use buzz_hostd::registration::{Host, RegistrationClock};
use buzz_ws_client::NostrWsConnection;
use clap::Subcommand;
use nostr::PublicKey;
use std::{
    fs::OpenOptions,
    io::{IsTerminal, Write},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
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
    /// Reconcile registration only. The relay public key is operator-pinned.
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
            let _lock = controller_lock(dir)?;
            let connector = connector(ca_cert.as_deref())?;
            let mut failures = 0u32;
            loop {
                let mut failed = false;
                for generation in host.work(&community, &RegistrationClock::read()?)? {
                    let result = tokio::time::timeout(Duration::from_secs(45), async {
                        let url = community.replacen("https://", "wss://", 1);
                        let mut ws = match &connector {
                            Some(tls) => {
                                NostrWsConnection::connect_with_tls_connector(&url, tls.clone())
                                    .await?
                            }
                            None => NostrWsConnection::connect(&url).await?,
                        };
                        host.authenticate(&mut ws, &generation).await?;
                        host.reconcile(
                            &mut ws,
                            relay,
                            &generation,
                            &label,
                            &RegistrationClock::read()?,
                        )
                        .await?;
                        ws.disconnect().await?;
                        Ok::<_, anyhow::Error>(())
                    })
                    .await;
                    if !matches!(result, Ok(Ok(()))) {
                        failed = true;
                    }
                }
                if once {
                    ensure!(
                        !failed,
                        "registration reconciliation incomplete; exact retry remains in journal"
                    );
                    return Ok(());
                }
                failures = if failed { failures + 1 } else { 0 };
                ensure!(failures < 6, "registration unavailable after six bounded attempts; restart controller to retry");
                if failed {
                    eprintln!("registration unavailable; exact retry retained");
                }
                // One controller, one socket at a time, bounded backoff and a
                // terminal failure. No tight reconnect loop on persistent denial.
                tokio::time::sleep(Duration::from_secs(if failed {
                    (2u64 << failures).min(60)
                } else {
                    5
                }))
                .await;
            }
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
