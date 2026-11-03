//! Root socket helper. The authenticated service UID may provision bounded
//! instances from the root catalog. Selectors never name programs or paths.
use anyhow::{anyhow, ensure, Result};
use buzz_hostd::{Catalog, Instance, Prepared, OPERATOR_CATALOG};
use rustix::fs::{flock, FlockOperation};
use std::{
    fs,
    io::{Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
    process::{Command, Output, Stdio},
};

const STAGE: &str = "/var/lib/buzz-hostd";
const PRIVATE: &str = "/var/lib/buzz-paired-units";
const TEMPLATE: &str = "/etc/systemd/system/buzz-paired-agent@.service";

fn systemctl(args: &[&str], unit: &str) -> Result<Output> {
    Ok(Command::new("/usr/bin/systemctl")
        .env_clear()
        .args(args)
        .arg(unit)
        .stdin(Stdio::null())
        .output()?)
}

fn owned_shape(output: &str, existing: bool) -> bool {
    output
        .lines()
        .any(|s| s == format!("FragmentPath={TEMPLATE}"))
        && output.lines().any(|s| s == "DropInPaths=")
        && (existing
            || (output.lines().any(|s| s == "ActiveState=inactive")
                && output.lines().any(|s| s == "UnitFileState=disabled")))
}

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

fn stage(uid: u32, name: &str) -> Result<Instance> {
    let source = Path::new(STAGE);
    let dir = fs::symlink_metadata(source)?;
    ensure!(
        dir.is_dir() && dir.uid() == uid && dir.mode() & 0o077 == 0,
        "invalid host staging directory"
    );
    let f = fs::OpenOptions::new()
        .read(true)
        .custom_flags((rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK).bits() as i32)
        .open(source.join(name))?;
    let m = f.metadata()?;
    ensure!(
        m.is_file() && m.nlink() == 1 && m.uid() == uid && m.mode() & 0o077 == 0,
        "invalid staged input"
    );
    let mut bytes = Vec::new();
    f.take(65537).read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= 65536, "oversized input");
    Ok(serde_json::from_slice(&bytes)?)
}

fn has_capacity(root: &Path, capacity: u32) -> Result<bool> {
    let mut used = 0;
    for entry in fs::read_dir(root)? {
        if entry?.path().extension().is_some_and(|s| s == "cred") {
            used += 1;
            if used >= capacity {
                return Ok(false);
            }
        }
    }
    Ok(capacity > 0)
}
fn abort(unit: &str) -> Result<()> {
    ensure!(
        systemctl(&["disable", "--now"], unit)?.status.success(),
        "unit cleanup incomplete"
    );
    let result = systemctl(&["show", "-p", "ActiveState", "-p", "UnitFileState"], unit)?;
    let output = std::str::from_utf8(&result.stdout)?;
    ensure!(
        result.status.success()
            && output
                .lines()
                .any(|s| matches!(s, "ActiveState=inactive" | "ActiveState=failed"))
            && output.lines().any(|s| s == "UnitFileState=disabled"),
        "unit cleanup unconfirmed"
    );
    Ok(())
}

fn main() -> Result<()> {
    ensure!(
        rustix::process::geteuid().is_root(),
        "root-installed helper required"
    );
    let uid = service_uid()?;
    ensure!(peer_allowed(std::io::stdin(), uid)?, "wrong control peer");
    let mut request = String::new();
    std::io::stdin().take(73).read_to_string(&mut request)?;
    let (verb, id) = request
        .split_once(' ')
        .ok_or_else(|| anyhow!("invalid action"))?;
    ensure!(
        matches!(verb, "prepare" | "start" | "abort" | "settle") && buzz_hostd::id(id),
        "invalid action"
    );
    let root = Path::new(PRIVATE);
    let dir = fs::symlink_metadata(root)?;
    ensure!(
        dir.is_dir() && dir.uid() == 0 && dir.mode() & 0o077 == 0,
        "unsafe root credential store"
    );
    let lock = fs::File::open(root)?;
    flock(&lock, FlockOperation::LockExclusive)?;
    let name = format!("{id}.cred");
    let dest = root.join(&name);
    let existing = dest.try_exists()?;
    if verb == "abort" && !existing {
        println!("inactive");
        return Ok(());
    }
    if matches!(verb, "start" | "settle") && !existing {
        println!("failed");
        return Ok(());
    }
    let unit = format!("buzz-paired-agent@{id}.service");
    if !existing
        && ["/var/lib", "/var/lib/private"].iter().any(|dir| {
            fs::symlink_metadata(Path::new(dir).join(format!("buzz-agent-{id}"))).is_ok()
        })
    {
        println!("failed");
        return Ok(());
    }
    let shape = systemctl(
        &[
            "show",
            "-p",
            "ActiveState",
            "-p",
            "UnitFileState",
            "-p",
            "FragmentPath",
            "-p",
            "DropInPaths",
        ],
        &unit,
    )?;
    ensure!(shape.status.success(), "cannot inspect system unit");
    if !owned_shape(std::str::from_utf8(&shape.stdout)?, existing) {
        if !existing && verb != "abort" {
            println!("failed");
            return Ok(());
        }
        return Err(anyhow!("foreign unit; operator reconciliation required"));
    }
    buzz_hostd::trusted(Path::new(TEMPLATE))?;
    let fence = root.join(format!("{id}.closed"));
    let sealed = root.join(format!("{id}.settled"));
    if verb == "settle" && !fence.try_exists()? {
        fs::File::create(&sealed)?.sync_all()?;
        lock.sync_all()?;
        println!("settled");
        return Ok(());
    }
    if sealed.try_exists()? {
        println!("settled");
        return Ok(());
    }
    if verb == "abort" {
        // Fence before side effects: crash/retry may repeat cleanup, never start.
        fs::File::create(&fence)?.sync_all()?;
        lock.sync_all()?;
        abort(&unit)?;
        println!("inactive");
        return Ok(());
    }
    if fence.try_exists()? {
        abort(&unit)?;
        println!("failed");
        return Ok(());
    }
    let parsed = if existing {
        let stored: Prepared = serde_json::from_slice(&buzz_hostd::bounded(&dest, 131072)?)?;
        if verb == "prepare" {
            let submitted = stage(uid, &name)?;
            ensure!(
                serde_json::to_vec(&stored.instance)? == serde_json::to_vec(&submitted)?,
                "existing instance conflict"
            );
        }
        stored.instance
    } else {
        stage(uid, &name)?
    };
    ensure!(parsed.unit_id == id, "instance id mismatch");
    let (catalog, revision) = Catalog::read(Path::new(OPERATOR_CATALOG))?;
    let allowed = parsed.validate(&catalog, &revision);
    if allowed.is_err() {
        if existing {
            fs::File::create(&fence)?.sync_all()?;
            lock.sync_all()?;
            abort(&unit)?;
        }
        println!("failed");
        return Ok(());
    }
    let rt = &catalog.runtimes[&parsed.runtime_id];
    if !existing {
        if !has_capacity(root, catalog.capacity)? {
            println!("failed");
            return Ok(());
        }
        let Ok(provider) = buzz_hostd::provider(&rt.credential_profile) else {
            println!("failed");
            return Ok(());
        };
        let mut f = tempfile::NamedTempFile::new_in(root)?;
        serde_json::to_writer(
            &mut f,
            &Prepared {
                instance: parsed,
                provider,
            },
        )?;
        f.flush()?;
        f.as_file().sync_all()?;
        f.persist_noclobber(&dest)?;
        lock.sync_all()?;
    }
    if verb == "prepare" {
        println!("prepared");
        return Ok(());
    }
    // Keep the root transaction serialized through systemctl and cleanup.
    // Last catalog check before start. Worker independently repeats it on every boot.
    if !catalog.current(&revision)?
        || !systemctl(&["enable", "--now"], &unit)?.status.success()
        || !systemctl(&["is-enabled", "--quiet"], &unit)?
            .status
            .success()
        || !systemctl(&["is-active", "--quiet"], &unit)?
            .status
            .success()
    {
        fs::File::create(&fence)?.sync_all()?;
        fs::File::open(root)?.sync_all()?;
        abort(&unit)?;
        println!("failed");
    } else {
        println!("active");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn counts_all_irreversible_reservations_at_the_root_boundary() {
        let root = tempfile::tempdir().unwrap();
        assert!(has_capacity(root.path(), 1).unwrap());
        fs::write(root.path().join("one.cred"), b"reserved").unwrap();
        assert!(!has_capacity(root.path(), 1).unwrap());
    }
    #[test]
    fn authenticates_connecting_uid_not_inode_group() {
        let (a, _b) = std::os::unix::net::UnixStream::pair().unwrap();
        let uid = rustix::process::geteuid().as_raw();
        assert!(peer_allowed(&a, uid).unwrap());
        assert!(!peer_allowed(&a, uid.wrapping_add(1)).unwrap());
    }
    #[test]
    fn rejects_overridden_or_preexisting_foreign_instance() {
        let base = format!(
            "FragmentPath={TEMPLATE}\nDropInPaths=\nActiveState=inactive\nUnitFileState=disabled"
        );
        assert!(owned_shape(&base, false));
        assert!(!owned_shape(&base.replace("disabled", "enabled"), false));
        assert!(!owned_shape(
            &base.replace("DropInPaths=", "DropInPaths=/etc/override.conf"),
            true
        ));
        assert!(!owned_shape(
            &base.replace("@.service", "@other.service"),
            true
        ));
    }
}
