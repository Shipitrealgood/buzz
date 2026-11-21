//! Prepare a protected, non-roaming directory before the Windows client opens SQLite.
use anyhow::{anyhow, ensure, Context, Result};
use std::os::windows::process::CommandExt;
use std::{
    collections::HashSet,
    ffi::OsStr,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{LazyLock, Mutex},
    time::{Duration, Instant},
};

/// Create or validate the journal directory under a native, trusted per-user
/// base. The production caller supplies OS-resolved paths, never IPC input.
pub fn protect_windows_journal(base: &Path, dir: &Path) -> Result<()> {
    static PREPARED: LazyLock<Mutex<HashSet<PathBuf>>> =
        LazyLock::new(|| Mutex::new(HashSet::new()));
    let mut ready = PREPARED
        .lock()
        .map_err(|_| anyhow!("journal initialization unavailable"))?;
    if ready.contains(dir) {
        return Ok(());
    }
    let root = PathBuf::from(
        std::env::var_os("SystemRoot").context("Windows system directory unavailable")?,
    );
    ensure!(root.is_absolute(), "invalid Windows system directory");
    let executable = root.join(r"System32\WindowsPowerShell\v1.0\powershell.exe");
    let mut child = Command::new(executable)
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            include_str!("../windows/protect-journal.ps1"),
        ])
        .env("BUZZ_JOURNAL_BASE", base.as_os_str())
        .env("BUZZ_JOURNAL_PATH", dir.as_os_str())
        .env("PSModulePath", OsStr::new(""))
        .creation_flags(0x08000000)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("could not verify Windows journal permissions")?;
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            ensure!(
                status.success(),
                "Windows journal permissions could not be secured"
            );
            break;
        }
        if started.elapsed() > Duration::from_secs(15) {
            let _ = child.kill();
            let _ = child.wait();
            anyhow::bail!("Windows journal permissions check timed out");
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    ready.insert(dir.into());
    Ok(())
}
