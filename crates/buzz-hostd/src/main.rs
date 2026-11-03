mod clock;
mod onboard;
mod request;
mod store;
mod transport;
mod unit;
use anyhow::{ensure, Result};
use clap::{Parser, Subcommand};
use clock::Clock;

use std::{
    fs::OpenOptions,
    io::{IsTerminal, Write},
    os::unix::fs::OpenOptionsExt,
    path::PathBuf,
    time::Duration,
};
use store::{Catalog, Store};

#[derive(Parser)]
struct Cli {
    /// Existing 0700 directory owned exclusively by the paired-host service UID.
    #[arg(long)]
    state_dir: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create a new local host identity; refuse an existing database.
    Init,
    /// Generate one new private console-only invitation.
    Offer {
        #[arg(long)]
        community: String,
    },
    /// Irreversibly revoke this exact generation, including while offline.
    Unpair {
        #[arg(long)]
        generation: String,
    },
    /// Print nonsecret identity and local grant states.
    Status,
    /// Poll enrolled community endpoints using the local control key.
    Serve {
        #[arg(long)]
        catalog: PathBuf,
        #[arg(long)]
        once: bool,
    },
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    ensure!(
        !rustix::process::geteuid().is_root(),
        "use a dedicated unprivileged host account"
    );
    let mut store = Store::load(&cli.state_dir, matches!(cli.command, Command::Init))?;
    match cli.command {
        Command::Init => println!("{}", store.keys()?.public_key().to_hex()),
        Command::Offer { community } => {
            ensure!(
                std::io::stdout().is_terminal(),
                "invitation requires a private operator terminal"
            );
            let value = store.offer(community, &Clock::read()?)?;
            writeln!(std::io::stdout().lock(), "{}", &*value)?;
        }
        Command::Unpair { generation } => store.revoke(&generation, &Clock::read()?)?,
        Command::Status => {
            println!("host {}", store.keys()?.public_key().to_hex());
            let mut stmt = store
                .0
                .prepare("SELECT c,g,state FROM grants ORDER BY rowid")?;
            let rows = stmt.query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })?;
            for row in rows {
                let (c, g, state) = row?;
                println!("{c} {g} {state}");
            }
        }
        Command::Serve { catalog, once } => {
            let lock = OpenOptions::new()
                .create(true)
                .truncate(false)
                .read(true)
                .write(true)
                .mode(0o600)
                .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
                .open(cli.state_dir.join("reconciler.lock"))?;
            rustix::fs::flock(&lock, rustix::fs::FlockOperation::NonBlockingLockExclusive)?;
            if cli.state_dir == std::path::Path::new("/var/lib/buzz-hostd") {
                ensure!(
                    catalog == std::path::Path::new(buzz_hostd::OPERATOR_CATALOG),
                    "installed host requires operator catalog"
                );
            }
            let net = transport::Http::new()?;
            loop {
                let run = match Catalog::read(&catalog) {
                    Ok((cat, revision)) => transport::poll(&mut store, &net, &cat, &revision).await,
                    Err(e) => {
                        onboard::unavailable_catalog(&mut store).await?;
                        Err(e)
                    }
                };
                if once {
                    return run;
                }
                if run.is_err() {
                    eprintln!("paired-host polling incomplete; retrying");
                }
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    }
    Ok(())
}
