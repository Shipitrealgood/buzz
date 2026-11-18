//! Operator-run synthetic controls for a registration deployment sandbox.
//! No identities, invitations, authentication or signed events are created.
use anyhow::{ensure, Context, Result};
use std::{net::TcpStream, os::unix::net::UnixStream, path::Path, time::Duration};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    ensure!(
        args.len() == 7,
        "expected outside|inside canary helper socket local-address relay-url"
    );
    ensure!(
        !rustix::process::geteuid().is_root(),
        "probe must be unprivileged"
    );
    let inside = match args[1].as_str() {
        "inside" => true,
        "outside" => false,
        _ => anyhow::bail!("invalid probe mode"),
    };
    for canary in args[2].split(',') {
        let readable = std::fs::read(canary).is_ok();
        ensure!(
            readable != inside,
            "file control did not match sandbox boundary: {canary}"
        );
    }
    let executable = std::process::Command::new(&args[3]).status();
    if inside {
        ensure!(executable.is_err(), "helper execution was not denied");
    } else {
        ensure!(executable?.success(), "helper positive control failed");
    }
    ensure!(
        UnixStream::connect(&args[4]).is_ok() != inside,
        "Unix socket control did not match boundary"
    );
    ensure!(
        TcpStream::connect_timeout(&args[5].parse()?, Duration::from_secs(2)).is_ok() != inside,
        "loopback control did not match address policy"
    );
    if inside {
        std::fs::write(
            Path::new("/state").join("deployment-probe"),
            b"synthetic state write\n",
        )?;
        ensure!(
            std::fs::OpenOptions::new()
                .write(true)
                .open("/etc/hosts")
                .is_err(),
            "service can modify relay address configuration"
        );
        let state_exec = Path::new("/state").join("synthetic-executable");
        std::fs::copy(std::env::current_exe()?, &state_exec)?;
        ensure!(
            std::process::Command::new(&state_exec).status().is_err(),
            "service can execute writable state"
        );
        std::fs::remove_file(state_exec)?;
        let _ = rustls::crypto::ring::default_provider().install_default();
        // Same default-root connector as registration_cli::Run without --ca-cert.
        // Connect and close only; never authenticate or send a Nostr event.
        let ws = tokio::time::timeout(
            Duration::from_secs(15),
            buzz_ws_client::NostrWsConnection::connect(&args[6]),
        )
        .await
        .context("relay TLS/WebSocket timeout")??;
        tokio::time::timeout(Duration::from_secs(5), ws.disconnect()).await??;
        println!("PASS sandbox: state writable; canary/helper/Unix/loopback denied; native-root relay TLS/WebSocket connected");
    } else {
        println!("PASS outside: canary/helper/Unix/loopback positive controls");
    }
    Ok(())
}
