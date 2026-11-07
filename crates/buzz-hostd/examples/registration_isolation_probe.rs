//! Disposable service gate: run as ExecStartPre inside the actual registration
//! unit's sandbox. Its only extra bind is this probe executable itself.
use anyhow::{ensure, Result};
fn main() -> Result<()> {
    ensure!(
        !rustix::process::geteuid().is_root(),
        "probe unexpectedly privileged"
    );
    for path in [
        "/home/buzz-existing-agent/credential",
        "/var/lib/buzz-existing-agent/token",
        "/etc/buzz-hostd/providers/synthetic.json",
    ] {
        ensure!(
            std::fs::read(path).is_err(),
            "registration service read credential canary: {path}"
        );
    }
    ensure!(
        std::process::Command::new("/usr/local/libexec/buzz-host-activate")
            .status()
            .is_err(),
        "registration service executed root helper canary"
    );
    ensure!(
        std::os::unix::net::UnixStream::connect("/run/buzz-hostd-activate.sock").is_err(),
        "registration service connected to activation socket"
    );
    std::fs::write(
        "/state/isolation-probe",
        b"dedicated unprivileged service; credential/home/helper denied\n",
    )?;
    println!("PASS actual service namespace: unprivileged, state writable, credential/home/helper denied");
    Ok(())
}
