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

fn main() -> Result<()> {
    ensure!(
        rustix::process::geteuid().is_root(),
        "root-installed helper required"
    );
    let uid = service_uid()?;
    ensure!(peer_allowed(std::io::stdin(), uid)?, "wrong control peer");
    let mut request = String::new();
    std::io::stdin().take(74).read_to_string(&mut request)?;
    let layout = buzz_hostd::activation::Layout::installed();
    buzz_hostd::trusted(&layout.template)?;
    let reply =
        buzz_hostd::activation::apply(&layout, uid, &request, &mut |args: &[&str],
                                                                    unit: &str|
         -> Result<Output> {
            Ok(Command::new("/usr/bin/systemctl")
                .env_clear()
                .args(args)
                .arg(unit)
                .stdin(Stdio::null())
                .output()?)
        })?;
    std::io::stdout().write_all(reply.as_bytes())?;
    Ok(())
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
