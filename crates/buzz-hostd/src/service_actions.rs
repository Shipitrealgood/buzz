//! Short, flock-serialized transactions for the same unit's fixed finite modes.
use crate::{
    activation::Layout,
    runtime_context::{ActionResult, Execution, Mode},
    Prepared,
};
use anyhow::{anyhow, ensure, Result};
use rustix::fs::{open, openat, Mode as FileMode, OFlags};
use std::{
    fs,
    io::{Read, Write},
    os::unix::fs::{DirBuilderExt, MetadataExt},
    path::{Path, PathBuf},
    process::Output,
};

const FINITE_DROPIN: &[u8] = b"[Service]\nRestart=no\n";
fn path(layout: &Layout, id: &str, ext: &str) -> PathBuf {
    layout.private.join(format!("{id}.{ext}"))
}
fn dropin(layout: &Layout, id: &str) -> Result<PathBuf> {
    Ok(layout
        .template
        .parent()
        .ok_or_else(|| anyhow!("unit directory missing"))?
        .join(format!("buzz-paired-agent@{id}.service.d"))
        .join("50-buzz-execution-mode.conf"))
}
fn owned_file(p: &Path, uid: u32, limit: u64) -> Result<Vec<u8>> {
    let fd = open(
        p,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK,
        FileMode::empty(),
    )?;
    let f = fs::File::from(fd);
    let m = f.metadata()?;
    ensure!(
        m.is_file() && m.uid() == uid && m.nlink() == 1 && m.mode() & 0o022 == 0,
        "unsafe root artifact"
    );
    let mut b = Vec::new();
    f.take(limit + 1).read_to_end(&mut b)?;
    ensure!(b.len() as u64 <= limit, "oversized artifact");
    Ok(b)
}
/// Recognize only our fixed finite-mode drop-in and matching protected descriptor.
pub fn shape(layout: &Layout, id: &str, output: &str) -> Result<String> {
    let expected = dropin(layout, id)?;
    let line = format!("DropInPaths={}", expected.display());
    if output.lines().any(|s| s == line) {
        let parent = expected
            .parent()
            .ok_or_else(|| anyhow!("drop-in parent missing"))?;
        let m = fs::symlink_metadata(parent)?;
        ensure!(
            m.is_dir() && m.uid() == layout.root_uid && m.mode() & 0o022 == 0,
            "unsafe drop-in directory"
        );
        ensure!(
            owned_file(&expected, layout.root_uid, 128)? == FINITE_DROPIN,
            "foreign mode override"
        );
        let e: Execution = serde_json::from_slice(&owned_file(
            &path(layout, id, "execution"),
            layout.root_uid,
            4096,
        )?)?;
        ensure!(
            e.unit == id && e.mode != Mode::Conversation,
            "wrong execution mode override"
        );
        Ok(output.replace(&line, "DropInPaths="))
    } else {
        Ok(output.into())
    }
}
/// Inactive also requires no remaining main/control tasks. No mode implicitly stops conversation.
pub fn inactive(output: &str) -> bool {
    output
        .lines()
        .any(|s| matches!(s, "ActiveState=inactive" | "ActiveState=failed"))
        && ["MainPID=0", "ControlPID=0"]
            .iter()
            .all(|s| output.lines().any(|l| l == *s))
        && output
            .lines()
            .any(|s| matches!(s, "TasksCurrent=0" | "TasksCurrent=[not set]"))
}
fn save(layout: &Layout, id: &str, ext: &str, value: &impl serde::Serialize) -> Result<()> {
    let mut f = tempfile::NamedTempFile::new_in(&layout.private)?;
    serde_json::to_writer(&mut f, value)?;
    f.flush()?;
    f.as_file().sync_all()?;
    f.persist(path(layout, id, ext))?;
    fs::File::open(&layout.private)?.sync_all()?;
    Ok(())
}
/// Create a conversation selector at preparation; ordinary boot is never a finite action.
pub fn conversation(layout: &Layout, i: &crate::Instance) -> Result<()> {
    if !path(layout, &i.unit_id, "login").try_exists()? {
        save(
            layout,
            &i.unit_id,
            "login",
            &(
                buzz_core::host_deployment::observation::LoginState::NotObserved,
                0u64,
            ),
        )?;
    }
    save(
        layout,
        &i.unit_id,
        "execution",
        &Execution::new(i, Mode::Conversation, nostr::Timestamp::now().as_secs())?,
    )
}
fn set_mode(
    layout: &Layout,
    id: &str,
    finite: bool,
    system: &mut impl FnMut(&[&str], &str) -> Result<Output>,
) -> Result<()> {
    let file = dropin(layout, id)?;
    if finite {
        let parent = file
            .parent()
            .ok_or_else(|| anyhow!("missing drop-in directory"))?;
        if !parent.try_exists()? {
            fs::DirBuilder::new().mode(0o700).create(parent)?;
        }
        let m = fs::symlink_metadata(parent)?;
        ensure!(
            m.is_dir() && m.uid() == layout.root_uid && m.mode() & 0o022 == 0,
            "unsafe drop-in parent"
        );
        let mut temp = tempfile::NamedTempFile::new_in(parent)?;
        temp.write_all(FINITE_DROPIN)?;
        temp.as_file().sync_all()?;
        temp.persist(&file)?;
    } else if file.try_exists()? {
        ensure!(
            owned_file(&file, layout.root_uid, 128)? == FINITE_DROPIN,
            "foreign drop-in"
        );
        fs::remove_file(file)?;
    }
    // Manager adapter omits the unit argument for this one fixed operation.
    ensure!(
        system(&["daemon-reload"], "")?.status.success(),
        "manager reload failed"
    );
    Ok(())
}
/// Restore the fixed ordinary restart policy before conversation activation.
pub fn restore(
    layout: &Layout,
    i: &crate::Instance,
    system: &mut impl FnMut(&[&str], &str) -> Result<Output>,
) -> Result<()> {
    conversation(layout, i)?;
    set_mode(layout, &i.unit_id, false, system)
}
/// Anchored, non-following output read. Neither the filename nor its parents come from an agent.
fn read_output(anchor: &Path, id: &str, e: &Execution) -> Result<Option<ActionResult>> {
    let root = open(
        anchor,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW,
        FileMode::empty(),
    )?;
    let dir = match openat(
        &root,
        format!("buzz-agent-{id}"),
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW,
        FileMode::empty(),
    ) {
        Ok(fd) => fs::File::from(fd),
        Err(rustix::io::Errno::NOENT) => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let dm = dir.metadata()?;
    ensure!(dm.mode() & 0o077 == 0, "unsafe action directory");
    let fd = match openat(
        &dir,
        e.filename(),
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK,
        FileMode::empty(),
    ) {
        Ok(fd) => fd,
        Err(rustix::io::Errno::NOENT) => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let f = fs::File::from(fd);
    let m = f.metadata()?;
    ensure!(
        m.is_file() && m.nlink() == 1 && m.uid() == dm.uid() && m.mode() & 0o077 == 0,
        "unsafe action result"
    );
    // Escaped JSON can expand the 64 KiB captured bytes up to sixfold.
    let mut raw = Vec::new();
    f.take(400001).read_to_end(&mut raw)?;
    ensure!(raw.len() <= 400000, "oversized action result");
    let result: ActionResult = serde_json::from_slice(&raw)?;
    ensure!(
        result.execution == *e && result.login_output.len() <= 196608,
        "action result mismatch"
    );
    ensure!(
        !result.finished || result.login_output.is_empty(),
        "terminal raw output forbidden"
    );
    ensure!(
        result.observed_at <= nostr::Timestamp::now().as_secs() && result.observed_at <= e.deadline,
        "action timestamp mismatch"
    );
    Ok(Some(result))
}
/// Remove only known action files using the same anchored directory custody.
fn remove_output(anchor: &Path, id: &str, e: &Execution) -> Result<()> {
    let root = open(
        anchor,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW,
        FileMode::empty(),
    )?;
    let dir = match openat(
        &root,
        format!("buzz-agent-{id}"),
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW,
        FileMode::empty(),
    ) {
        Ok(fd) => fd,
        Err(rustix::io::Errno::NOENT) => return Ok(()),
        Err(e) => return Err(e.into()),
    };
    match rustix::fs::unlinkat(&dir, e.filename(), rustix::fs::AtFlags::empty()) {
        Ok(()) | Err(rustix::io::Errno::NOENT) => Ok(()),
        Err(e) => Err(e.into()),
    }
}
/// Clear only owned mode/output artifacts after the caller confirmed cgroup stop/disable.
pub fn clear(
    layout: &Layout,
    id: &str,
    system: &mut impl FnMut(&[&str], &str) -> Result<Output>,
) -> Result<()> {
    if path(layout, id, "execution").try_exists()? {
        let e: Execution = serde_json::from_slice(&owned_file(
            &path(layout, id, "execution"),
            layout.root_uid,
            4096,
        )?)?;
        remove_output(&layout.state_root, id, &e)?;
        remove_output(&layout.runtime_root, id, &e)?;
        set_mode(layout, id, false, system)?;
        fs::remove_file(path(layout, id, "execution"))?;
    }
    if path(layout, id, "login").try_exists()? {
        fs::remove_file(path(layout, id, "login"))?;
    }
    Ok(())
}
/// Dispatch only named finite actions while the caller holds the existing helper lock.
pub fn apply(
    layout: &Layout,
    id: &str,
    verb: &str,
    nonce: Option<&str>,
    local: bool,
    shape: &str,
    system: &mut impl FnMut(&[&str], &str) -> Result<Output>,
) -> Result<String> {
    ensure!(!path(layout, id, "closed").try_exists()?, "unit fenced");
    let stored: Prepared = serde_json::from_slice(&owned_file(
        &path(layout, id, "cred"),
        layout.root_uid,
        131072,
    )?)?;
    let i = &stored.instance;
    ensure!(
        i.unit_id == id && i.deployment.is_some(),
        "wrong managed instance"
    );
    let (cat, rev) = crate::Catalog::read(&layout.catalog)?;
    i.validate(&cat, &rev)?;
    let unit = format!("buzz-paired-agent@{id}.service");
    if matches!(verb, "models" | "login") {
        ensure!(verb != "login" || local, "login is root-local only");
        ensure!(inactive(shape), "unit busy");
        ensure!(
            matches!(
                i.deployment.as_ref().map(|d| &d.config.model),
                Some(buzz_core::host_deployment::ModelSelection::PendingLogin {})
            ),
            "finite action requires inactive preparation"
        );
        let e = Execution::new(
            i,
            if verb == "login" {
                Mode::Login
            } else {
                Mode::Models
            },
            nostr::Timestamp::now().as_secs(),
        )?;
        ensure!(
            system(&["disable"], &unit)?.status.success(),
            "disable failed"
        );
        if path(layout, id, "execution").try_exists()? {
            let old: Execution = serde_json::from_slice(&owned_file(
                &path(layout, id, "execution"),
                layout.root_uid,
                4096,
            )?)?;
            remove_output(&layout.state_root, id, &old)?;
            remove_output(&layout.runtime_root, id, &old)?;
        }
        save(layout, id, "execution", &e)?;
        set_mode(layout, id, true, system)?;
        if !system(&["start", "--no-block"], &unit)?.status.success() {
            crate::activation::abort(system, &unit)?;
            return Err(anyhow!("finite start failed"));
        }
        return Ok(serde_json::to_string(&e)?);
    }
    let e: Execution = serde_json::from_slice(&owned_file(
        &path(layout, id, "execution"),
        layout.root_uid,
        4096,
    )?)?;
    ensure!(
        nonce == Some(e.nonce.as_str()) && e.unit == id && e.mode != Mode::Conversation,
        "wrong action correlation"
    );
    ensure!(
        local || e.mode == Mode::Models,
        "login output is root-local only"
    );
    if verb == "cancel" || nostr::Timestamp::now().as_secs() >= e.deadline {
        crate::activation::abort(system, &unit)?;
        remove_output(&layout.runtime_root, id, &e)?;
        remove_output(&layout.state_root, id, &e)?;
        return Ok("cancelled\n".into());
    }
    ensure!(verb == "observe", "unsupported action");
    let result =
        read_output(&layout.state_root, id, &e)?.or(read_output(&layout.runtime_root, id, &e)?);
    if let Some(mut result) = result {
        if result.finished {
            if !inactive(shape) {
                return Ok("pending\n".into());
            }
            if e.mode == Mode::Login {
                save(
                    layout,
                    id,
                    "login",
                    &(result.login.clone(), result.observed_at),
                )?;
            }
        }
        if e.mode == Mode::Models {
            result.login_output.clear();
            if path(layout, id, "login").try_exists()? {
                let (state, at): (buzz_core::host_deployment::observation::LoginState, u64) =
                    serde_json::from_slice(&owned_file(
                        &path(layout, id, "login"),
                        layout.root_uid,
                        256,
                    )?)?;
                result.login = state;
                result.login_at = (result.login
                    != buzz_core::host_deployment::observation::LoginState::NotObserved)
                    .then_some(at);
            }
        }
        return Ok(serde_json::to_string(&result)?);
    }
    ensure!(
        !shape.lines().any(|s| s == "ActiveState=failed"),
        "finite action failed"
    );
    Ok("pending\n".into())
}

#[cfg(test)]
#[path = "service_actions_tests.rs"]
mod tests;
