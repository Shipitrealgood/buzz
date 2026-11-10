//! Fixed helper operations shared with the executable and isolated integration.
use crate::{Catalog, Instance, Prepared, OPERATOR_CATALOG};
use anyhow::{anyhow, ensure, Result};
use rustix::fs::{flock, FlockOperation};
use std::{
    fs,
    io::{Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    process::Output,
};
/// Installation boundary. Production constructs only `installed()`.
pub struct Layout {
    /// Dedicated host staging directory.
    pub staging: PathBuf,
    /// Root-owned private launch credentials.
    pub private: PathBuf,
    /// Fixed reviewed unit template.
    pub template: PathBuf,
    /// Root-approved runtime catalog.
    pub catalog: PathBuf,
    /// OS ownership boundary; zero in production.
    pub root_uid: u32,
    /// Root-protected physical DynamicUser state parent.
    pub state_root: PathBuf,
    /// Root-protected per-unit runtime directory parent.
    pub runtime_root: PathBuf,
}
impl Layout {
    /// Fixed paths; there is no CLI or protocol override.
    pub fn installed() -> Self {
        Self {
            staging: "/var/lib/buzz-hostd".into(),
            private: "/var/lib/buzz-paired-units".into(),
            template: "/etc/systemd/system/buzz-paired-agent@.service".into(),
            catalog: OPERATOR_CATALOG.into(),
            root_uid: 0,
            state_root: "/var/lib/private".into(),
            runtime_root: "/run".into(),
        }
    }
}
fn owned_shape(output: &str, existing: bool, template: &Path) -> bool {
    output
        .lines()
        .any(|s| s == format!("FragmentPath={}", template.display()))
        && output.lines().any(|s| s == "DropInPaths=")
        && (existing
            || (output.lines().any(|s| s == "ActiveState=inactive")
                && output.lines().any(|s| s == "UnitFileState=disabled")))
}

fn stage(layout: &Layout, uid: u32, name: &str) -> Result<Instance> {
    let source = layout.staging.as_path();
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
pub(crate) fn abort(
    system: &mut impl FnMut(&[&str], &str) -> Result<Output>,
    unit: &str,
) -> Result<()> {
    ensure!(
        system(&["disable", "--now"], unit)?.status.success(),
        "unit cleanup incomplete"
    );
    let result = system(
        &[
            "show",
            "-p",
            "ActiveState",
            "-p",
            "UnitFileState",
            "-p",
            "MainPID",
            "-p",
            "ControlPID",
            "-p",
            "TasksCurrent",
        ],
        unit,
    )?;
    let output = std::str::from_utf8(&result.stdout)?;
    ensure!(
        result.status.success()
            && output
                .lines()
                .any(|s| matches!(s, "ActiveState=inactive" | "ActiveState=failed"))
            && output.lines().any(|s| s == "UnitFileState=disabled")
            && crate::service_actions::inactive(output),
        "unit cleanup unconfirmed"
    );
    Ok(())
}

/// Apply one fixed selector under the root flock. Only the OS systemctl boundary
/// is injected; production authenticates the peer before calling this function.
pub fn apply(
    layout: &Layout,
    uid: u32,
    request: &str,
    system: &mut impl FnMut(&[&str], &str) -> Result<Output>,
) -> Result<String> {
    apply_local(layout, uid, request, false, system)
}
/// Root-local frontend shares the same transaction; socket callers never set local.
pub fn apply_local(
    layout: &Layout,
    uid: u32,
    request: &str,
    local: bool,
    system: &mut impl FnMut(&[&str], &str) -> Result<Output>,
) -> Result<String> {
    let parts: Vec<_> = request.split(' ').collect();
    ensure!((2..=3).contains(&parts.len()), "invalid action");
    let (verb, id) = (parts[0], parts[1]);
    let nonce = parts.get(2).copied();
    ensure!(
        matches!(
            verb,
            "prepare"
                | "start"
                | "abort"
                | "settle"
                | "cleanup"
                | "models"
                | "login"
                | "observe"
                | "cancel"
        ) && crate::id(id)
            && nonce.is_none_or(crate::id)
            && (matches!(verb, "observe" | "cancel") == nonce.is_some())
            && (verb != "login" || local),
        "invalid action"
    );
    let root = layout.private.as_path();
    let dir = fs::symlink_metadata(root)?;
    ensure!(
        dir.is_dir() && dir.uid() == layout.root_uid && dir.mode() & 0o077 == 0,
        "unsafe root credential store"
    );
    let lock = fs::File::open(root)?;
    flock(&lock, FlockOperation::LockExclusive)?;
    let name = format!("{id}.cred");
    let dest = root.join(&name);
    let existing = dest.try_exists()?;
    if verb == "abort" && !existing {
        return Ok("inactive\n".into());
    }
    if matches!(verb, "start" | "settle") && !existing {
        return Ok("failed\n".into());
    }
    let unit = format!("buzz-paired-agent@{id}.service");
    if !existing
        && verb != "cleanup"
        && ["/var/lib", "/var/lib/private"].iter().any(|dir| {
            fs::symlink_metadata(Path::new(dir).join(format!("buzz-agent-{id}"))).is_ok()
        })
    {
        return Ok("failed\n".into());
    }
    let shape = system(
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
            "-p",
            "MainPID",
            "-p",
            "ControlPID",
            "-p",
            "TasksCurrent",
        ],
        &unit,
    )?;
    ensure!(shape.status.success(), "cannot inspect system unit");
    let shape = crate::service_actions::shape(layout, id, std::str::from_utf8(&shape.stdout)?)?;
    if !owned_shape(&shape, existing, &layout.template) {
        if !existing && verb != "abort" {
            return Ok("failed\n".into());
        }
        return Err(anyhow!("foreign unit; operator reconciliation required"));
    }
    if matches!(verb, "models" | "login" | "observe" | "cancel") {
        return crate::service_actions::apply(layout, id, verb, nonce, local, &shape, system);
    }
    let fence = root.join(format!("{id}.closed"));
    let sealed = root.join(format!("{id}.settled"));
    if verb == "cleanup" {
        if existing {
            let stored: Prepared = serde_json::from_slice(&crate::bounded(&dest, 131072)?)?;
            ensure!(
                stored.instance.deployment.is_some(),
                "not a managed deployment"
            );
        } else if !fence.try_exists()? {
            let staged = stage(layout, uid, &name)?;
            ensure!(
                staged.unit_id == id && staged.deployment.is_some(),
                "unknown managed unit"
            );
        }
        fs::File::create(&fence)?.sync_all()?;
        lock.sync_all()?;
        abort(system, &unit)?;
        crate::service_actions::clear(layout, id, system)?;
        if dest.try_exists()? {
            fs::remove_file(&dest)?;
        }
        lock.sync_all()?;
        return Ok("cleaned\n".into());
    }

    if verb == "settle" && !fence.try_exists()? {
        fs::File::create(&sealed)?.sync_all()?;
        lock.sync_all()?;
        return Ok("settled\n".into());
    }
    let managed = if existing {
        serde_json::from_slice::<Prepared>(&crate::bounded(&dest, 131072)?)?
            .instance
            .deployment
            .is_some()
    } else {
        false
    };
    if sealed.try_exists()? && !managed {
        return Ok("settled\n".into());
    }
    if verb == "abort" {
        // Fence before side effects: crash/retry may repeat cleanup, never start.
        fs::File::create(&fence)?.sync_all()?;
        lock.sync_all()?;
        abort(system, &unit)?;
        return Ok("inactive\n".into());
    }
    if fence.try_exists()? {
        abort(system, &unit)?;
        return Ok("failed\n".into());
    }
    let parsed = if existing {
        let stored: Prepared = serde_json::from_slice(&crate::bounded(&dest, 131072)?)?;
        if verb == "prepare" {
            let submitted = stage(layout, uid, &name)?;
            if serde_json::to_vec(&stored.instance)? != serde_json::to_vec(&submitted)? {
                let previous = stored
                    .instance
                    .deployment
                    .as_ref()
                    .ok_or_else(|| anyhow!("legacy instance conflict"))?;
                let next = submitted
                    .deployment
                    .as_ref()
                    .ok_or_else(|| anyhow!("deployment format conflict"))?;
                ensure!(
                    next.revision > previous.revision
                        && stored.instance.scope == submitted.scope
                        && stored.instance.agent == submitted.agent
                        && stored.instance.secret == submitted.secret
                        && stored.instance.request_id == submitted.request_id,
                    "existing instance conflict"
                );
                let (catalog, revision) = Catalog::read(&layout.catalog)?;
                submitted.validate(&catalog, &revision)?;
                if matches!(
                    next.config.model,
                    buzz_core::host_deployment::ModelSelection::PendingLogin {}
                ) {
                    ensure!(crate::service_actions::inactive(&shape), "unit busy");
                }
                abort(system, &unit)?;
                crate::service_actions::restore(layout, &submitted, system)?;
                let mut temp = tempfile::NamedTempFile::new_in(root)?;
                serde_json::to_writer(
                    &mut temp,
                    &Prepared {
                        instance: submitted,
                        provider: Default::default(),
                    },
                )?;
                temp.flush()?;
                temp.as_file().sync_all()?;
                temp.persist(&dest)?;
                lock.sync_all()?;
                return Ok("prepared\n".into());
            }
        }
        stored.instance
    } else {
        stage(layout, uid, &name)?
    };
    ensure!(parsed.unit_id == id, "instance id mismatch");
    let (catalog, revision) = Catalog::read(&layout.catalog)?;
    let allowed = parsed.validate(&catalog, &revision);
    if allowed.is_err() {
        if existing {
            fs::File::create(&fence)?.sync_all()?;
            lock.sync_all()?;
            abort(system, &unit)?;
        }
        return Ok("failed\n".into());
    }
    if verb == "start"
        && parsed.deployment.as_ref().is_some_and(|d| {
            matches!(
                d.config.model,
                buzz_core::host_deployment::ModelSelection::PendingLogin {}
            )
        })
    {
        return Ok("needs-login\n".into());
    }
    let rt = &catalog.runtimes[&parsed.runtime_id];
    if !existing {
        if parsed.deployment.is_some() {
            crate::service_actions::conversation(layout, &parsed)?;
        }
        if !has_capacity(root, catalog.capacity)? {
            return Ok("failed\n".into());
        }
        let Ok(provider) = crate::provider(&rt.credential_profile) else {
            return Ok("failed\n".into());
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
        return Ok("prepared\n".into());
    }
    let stored: Prepared = serde_json::from_slice(&crate::bounded(&dest, 131072)?)?;
    if stored.instance.deployment.is_some() {
        crate::service_actions::restore(layout, &stored.instance, system)?;
    }
    // Keep the root transaction serialized through systemctl and cleanup.
    // Last catalog check before start. Worker independently repeats it on every boot.
    if !catalog.current(&revision)?
        || !system(&["enable", "--now"], &unit)?.status.success()
        || !system(&["is-enabled", "--quiet"], &unit)?.status.success()
        || !system(&["is-active", "--quiet"], &unit)?.status.success()
    {
        fs::File::create(&fence)?.sync_all()?;
        fs::File::open(root)?.sync_all()?;
        abort(system, &unit)?;
        Ok("failed\n".into())
    } else {
        Ok("active\n".into())
    }
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
    fn rejects_overridden_or_preexisting_foreign_instance() {
        let template = Layout::installed().template;
        let base = format!(
            "FragmentPath={}\nDropInPaths=\nActiveState=inactive\nUnitFileState=disabled",
            template.display()
        );
        assert!(owned_shape(&base, false, &template));
        assert!(!owned_shape(
            &base.replace("disabled", "enabled"),
            false,
            &template
        ));
        assert!(!owned_shape(
            &base.replace("DropInPaths=", "DropInPaths=/etc/override.conf"),
            true,
            &template
        ));
        assert!(!owned_shape(
            &base.replace("@.service", "@other.service"),
            true,
            &template
        ));
    }
}
