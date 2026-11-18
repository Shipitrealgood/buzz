//! Root socket boundary: fixed installed paths and authenticated service UID.
use anyhow::{ensure, Result};
use std::{
    io::{Read, Write},
    process::{Command, Output, Stdio},
};
fn peer_allowed(fd: impl std::os::fd::AsFd, uid: u32) -> Result<bool> {
    Ok(rustix::net::sockopt::socket_peercred(fd)?.uid.as_raw() == uid)
}
fn service_uid() -> Result<u32> {
    let output = Command::new("/usr/bin/id")
        .env_clear()
        .args(["-u", "buzz-hostd"])
        .output()?;
    ensure!(output.status.success(), "dedicated service account missing");
    let uid = std::str::from_utf8(&output.stdout)?.trim().parse()?;
    ensure!(uid != 0, "dedicated service uid required");
    Ok(uid)
}

fn system(args: &[&str], unit: &str) -> Result<Output> {
    let mut command = Command::new("/usr/bin/systemctl");
    command.env_clear().args(args).stdin(Stdio::null());
    if !unit.is_empty() {
        command.arg(unit);
    }
    Ok(command.output()?)
}
async fn run() -> Result<()> {
    ensure!(
        rustix::process::geteuid().is_root(),
        "root-installed helper required"
    );
    let uid = service_uid()?;
    let layout = buzz_hostd::activation::Layout::installed();
    buzz_hostd::trusted(&layout.template)?;
    let args: Vec<_> = std::env::args().skip(1).collect();
    if !args.is_empty() {
        ensure!(
            args.len() == 2 && args[0] == "--login" && buzz_hostd::id(&args[1]),
            "only --login UNIT is supported locally"
        );
        let id = &args[1];
        let prepared: buzz_hostd::Prepared = serde_json::from_slice(&buzz_hostd::bounded(
            &layout.private.join(format!("{id}.cred")),
            131072,
        )?)?;
        eprintln!("Native login for agent {}", prepared.instance.agent);
        let action = buzz_hostd::activation::apply_local(
            &layout,
            uid,
            &format!("login {id}"),
            true,
            &mut system,
        )?;
        let action: buzz_hostd::runtime_context::Execution = serde_json::from_str(&action)?;
        let mut printed = 0;
        loop {
            tokio::select! {
                _=tokio::signal::ctrl_c()=> {
                    buzz_hostd::activation::apply_local(&layout,uid,&format!("cancel {id} {}",action.nonce),true,&mut system)?;
                    anyhow::bail!("native login cancelled");
                },
                _=tokio::time::sleep(std::time::Duration::from_millis(500))=>{}
            }
            let reply = buzz_hostd::activation::apply_local(
                &layout,
                uid,
                &format!("observe {id} {}", action.nonce),
                true,
                &mut system,
            )?;
            if reply == "pending\n" {
                continue;
            }
            ensure!(reply != "cancelled\n", "native login expired");
            let result: buzz_hostd::runtime_context::ActionResult = serde_json::from_str(&reply)?;
            if result.login_output.len() > printed {
                // Native bytes remain on this root-local terminal only.
                std::io::stdout().write_all(&result.login_output.as_bytes()[printed..])?;
                std::io::stdout().flush()?;
                printed = result.login_output.len();
            }
            if result.finished {
                eprintln!(
                    "Native login outcome: {:?}; networking remains disabled",
                    result.login
                );
                return Ok(());
            }
        }
    }
    ensure!(peer_allowed(std::io::stdin(), uid)?, "wrong control peer");
    let mut request = String::new();
    std::io::stdin().take(145).read_to_string(&mut request)?;
    ensure!(request.len() < 145, "oversized action");
    let reply = buzz_hostd::activation::apply(&layout, uid, &request, &mut system)?;
    std::io::stdout().write_all(reply.as_bytes())?;
    Ok(())
}
#[tokio::main]
async fn main() -> Result<()> {
    run()
        .await
        .map_err(|_| anyhow::anyhow!("fixed host action unavailable"))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authenticates_connecting_uid_not_inode_group() {
        let (a, _b) = std::os::unix::net::UnixStream::pair().unwrap();
        let uid = rustix::process::geteuid().as_raw();
        assert!(peer_allowed(&a, uid).unwrap());
        assert!(!peer_allowed(&a, uid.wrapping_add(1)).unwrap());
    }
}
