//! Fixed local user-service transactions. Configuration is operator-owned, never remote input.
use crate::{
    runtime_context::{ActionResult, Execution, Mode},
    Catalog, Instance, Prepared,
};
use anyhow::{anyhow, ensure, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::{io::AsyncReadExt, process::Command};

/// Locally resolved account context included in the catalog revision.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalContext {
    /// Main ACP executable for automatic discovery of compiled main candidates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub buzz_acp: Option<PathBuf>,
    /// Normal account HOME, shared with native provider logins.
    pub home: PathBuf,
    /// Locally assigned working directory; cleanup never deletes it.
    pub workspace: PathBuf,
    /// Account tool search path, independent of the remote client.
    pub tool_path: String,
    /// Optional normal account XDG/Grok locations, never per-agent auth stores.
    #[serde(default)]
    pub environment: BTreeMap<String, String>,
    /// Existing directory on this account's user-manager unit search path.
    pub unit_dir: PathBuf,
    /// Locally installed buzz-host-runtime executable.
    pub launcher: PathBuf,
    /// Locally resolved systemctl executable (a disposable fake in tests).
    pub systemctl: PathBuf,
}
impl LocalContext {
    fn validate_cleanup(&self) -> Result<()> {
        ensure!(
            self.unit_dir.is_absolute()
                && self.unit_dir.canonicalize()? == self.unit_dir
                && self.unit_dir.is_dir(),
            "invalid recorded unit directory"
        );
        crate::Executable {
            path: self.systemctl.clone(),
            sha256: String::new(),
        }
        .available()
    }
    fn validate(&self) -> Result<()> {
        self.validate_cleanup()?;
        for p in [&self.home, &self.workspace] {
            ensure!(
                p.is_absolute() && p.canonicalize()? == *p && p.is_dir(),
                "invalid local account directory"
            );
        }
        crate::Executable {
            path: self.launcher.clone(),
            sha256: String::new(),
        }
        .available()?;
        ensure!(
            !self.tool_path.is_empty() && !self.tool_path.chars().any(char::is_control),
            "invalid tool PATH"
        );
        ensure!(
            self.environment.iter().all(|(k, v)| matches!(
                k.as_str(),
                "XDG_CONFIG_HOME"
                    | "XDG_CACHE_HOME"
                    | "XDG_DATA_HOME"
                    | "XDG_STATE_HOME"
                    | "XDG_RUNTIME_DIR"
                    | "GROK_HOME"
            ) && Path::new(v).is_absolute()
                && !v.chars().any(char::is_control)),
            "invalid account environment"
        );
        Ok(())
    }
    /// Explicit normal account environment; no inherited Buzz key or provider secret is copied.
    pub fn environment(&self) -> BTreeMap<String, String> {
        let mut env = self.environment.clone();
        env.insert("HOME".into(), self.home.to_string_lossy().into());
        env.insert("PATH".into(), self.tool_path.clone());
        env
    }
}

/// Executor for this controller's recorded units only.
#[derive(Clone)]
pub struct UserService {
    pub(crate) staging: PathBuf,
}
impl UserService {
    /// Bind the ordinary account's existing private controller directory.
    /// Reopening restrictive retries must not depend on usable launch configuration.
    pub fn installed(state: &Path) -> Result<Self> {
        ensure!(
            !rustix::process::geteuid().is_root(),
            "refusing root controller"
        );
        directory(state)?;
        let service = Self {
            staging: state.into(),
        };
        let root = state.join("hosted");
        if !root.try_exists()? {
            fs::DirBuilder::new().mode(0o700).create(&root)?;
        }
        directory(&root)?;
        Ok(service)
    }
    /// Current local configuration and revision, also used by status projection.
    pub fn catalog(&self) -> Result<(Catalog, String)> {
        let path = self.staging.join("deployment.json");
        private_file(&path, 16384)?;
        let (cat, rev) = Catalog::read(&path)?;
        cat.local
            .as_ref()
            .ok_or_else(|| anyhow!("trusted account configuration required"))?
            .validate()?;
        Ok((cat, rev))
    }
    fn root(&self) -> PathBuf {
        self.staging.join("hosted")
    }
    fn dir(&self, id: &str) -> PathBuf {
        self.root().join(id)
    }
    fn lock(&self, id: &str) -> Result<fs::File> {
        ensure!(crate::id(id), "invalid unit ID");
        directory(&self.root())?;
        let lock = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .mode(0o600)
            .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
            .open(self.root().join(format!("{id}.lock")))?;
        rustix::fs::flock(&lock, rustix::fs::FlockOperation::NonBlockingLockExclusive)?;
        Ok(lock)
    }
    pub(crate) fn stage(&self, i: &Instance) -> Result<()> {
        let _lock = self.lock(&i.unit_id)?;
        let (cat, rev) = self.catalog()?;
        i.validate(&cat, &rev)?;
        ensure!(i.deployment.is_some(), "managed instance required");
        ensure!(
            !self
                .root()
                .join(format!("{}.closed", i.unit_id))
                .try_exists()?,
            "unit fenced"
        );
        write_json(&self.staging, &format!("{}.cred", i.unit_id), i)
    }
    pub(crate) fn remove_staged(&self, id: &str) -> Result<()> {
        ensure!(crate::id(id), "invalid unit ID");
        remove_file(&self.staging.join(format!("{id}.cred")))
    }
    pub(crate) async fn action(&self, verb: &str, id: &str) -> Result<String> {
        self.request(verb, id, None).await
    }
    pub(crate) async fn request(
        &self,
        verb: &str,
        id: &str,
        nonce: Option<&str>,
    ) -> Result<String> {
        ensure!(
            matches!(
                verb,
                "prepare" | "start" | "cleanup" | "models" | "observe" | "cancel"
            ) && nonce.is_none_or(crate::id)
                && (matches!(verb, "observe" | "cancel") == nonce.is_some()),
            "invalid fixed action"
        );
        let _lock = self.lock(id)?;
        let dir = self.dir(id);
        let closed = self.root().join(format!("{id}.closed"));
        if verb == "cleanup" {
            return self.cleanup(id).await;
        }
        ensure!(!closed.try_exists()?, "unit fenced");
        let (cat, rev) = self.catalog()?;
        let local = cat
            .local
            .as_ref()
            .ok_or_else(|| anyhow!("local context missing"))?;
        if verb == "prepare" {
            let i: Instance = serde_json::from_slice(&private_file(
                &self.staging.join(format!("{id}.cred")),
                131072,
            )?)?;
            i.validate(&cat, &rev)?;
            ensure!(
                i.unit_id == id && i.deployment.is_some(),
                "wrong staged unit"
            );
            for exe in [
                &cat.runtimes[&i.runtime_id].harness,
                &cat.runtimes[&i.runtime_id].adapter,
            ] {
                exe.available()?;
            }
            if dir.try_exists()? {
                directory(&dir)?;
                if dir.join("instance").try_exists()? {
                    let old: Prepared =
                        serde_json::from_slice(&private_file(&dir.join("instance"), 131072)?)?;
                    let a = old
                        .instance
                        .deployment
                        .as_ref()
                        .ok_or_else(|| anyhow!("unmanaged record"))?;
                    let b = i
                        .deployment
                        .as_ref()
                        .ok_or_else(|| anyhow!("unmanaged stage"))?;
                    ensure!(
                        old.instance.scope == i.scope
                            && old.instance.agent == i.agent
                            && old.instance.secret == i.secret
                            && old.instance.request_id == i.request_id
                            && old.instance.unit_id == id
                            && (b.revision > a.revision
                                || serde_json::to_vec(&old.instance)? == serde_json::to_vec(&i)?),
                        "prepared identity/revision conflict"
                    );
                    let shape = show(local, id).await?;
                    if !crate::service_actions::inactive(&shape) {
                        let e: Execution =
                            serde_json::from_slice(&private_file(&dir.join("execution"), 4096)?)?;
                        e.validate(&i, nostr::Timestamp::now().as_secs())?;
                        ensure!(
                            e.mode == Mode::Conversation
                                && serde_json::to_vec(&old.instance)? == serde_json::to_vec(&i)?
                                && shape.lines().any(|l| l == "ActiveState=active"),
                            "unit busy"
                        );
                        return Ok("prepared\n".into());
                    }
                    let recorded: LocalContext =
                        serde_json::from_slice(&private_file(&dir.join("context"), 16384)?)?;
                    ensure!(
                        recorded.unit_dir == local.unit_dir
                            && recorded.systemctl == local.systemctl
                            && recorded.launcher == local.launcher,
                        "managed service location changed"
                    );
                }
            } else {
                fs::DirBuilder::new().mode(0o700).create(&dir)?;
            }
            write_json(&dir, "context", local)?;
            write_json(
                &dir,
                "instance",
                &Prepared {
                    instance: i,
                    provider: BTreeMap::new(),
                },
            )?;
            self.install(local, id).await?;
            return Ok("prepared\n".into());
        }
        directory(&dir)?;
        let prepared: Prepared =
            serde_json::from_slice(&private_file(&dir.join("instance"), 131072)?)?;
        prepared.instance.validate(&cat, &rev)?;
        ensure!(prepared.instance.unit_id == id, "wrong prepared unit");
        let shape = show(local, id).await?;
        if verb == "start" {
            ensure!(
                matches!(
                    prepared
                        .instance
                        .deployment
                        .as_ref()
                        .map(|d| &d.config.model),
                    Some(buzz_core::host_deployment::ModelSelection::Selected { .. })
                ),
                "model selection pending"
            );
            if !crate::service_actions::inactive(&shape) {
                let old: Execution =
                    serde_json::from_slice(&private_file(&dir.join("execution"), 4096)?)?;
                old.validate(&prepared.instance, nostr::Timestamp::now().as_secs())?;
                ensure!(
                    old.mode == Mode::Conversation
                        && shape.lines().any(|l| l == "ActiveState=active"),
                    "unit busy"
                );
                return Ok("active\n".into());
            }
            if dir.join("execution").try_exists()? {
                let old: Execution =
                    serde_json::from_slice(&private_file(&dir.join("execution"), 4096)?)?;
                ensure!(
                    old.unit == id && crate::id(&old.nonce),
                    "wrong retained execution"
                );
                remove_file(&dir.join(old.filename()))?;
            }
            let e = Execution::new(
                &prepared.instance,
                Mode::Conversation,
                nostr::Timestamp::now().as_secs(),
            )?;
            write_json(&dir, "execution", &e)?;
            // Blocking start waits for main ACP READY after the runtime's fresh source guards.
            // Restart=no preserves explicit agent shutdown; no idle stop or reboot policy added.
            if let Err(error) = system(local, &["start"], Some(id)).await {
                stop(local, id).await?;
                return Err(error);
            }
            ensure!(
                show(local, id)
                    .await?
                    .lines()
                    .any(|l| l == "ActiveState=active"),
                "conversation start unconfirmed"
            );
            return Ok("active\n".into());
        }
        if verb == "models" {
            ensure!(crate::service_actions::inactive(&shape), "unit busy");
            if dir.join("execution").try_exists()? {
                let old: Execution =
                    serde_json::from_slice(&private_file(&dir.join("execution"), 4096)?)?;
                ensure!(
                    old.unit == id && crate::id(&old.nonce),
                    "wrong retained execution"
                );
                remove_file(&dir.join(old.filename()))?;
            }
            let e = Execution::new(
                &prepared.instance,
                Mode::Models,
                nostr::Timestamp::now().as_secs(),
            )?;
            write_json(&dir, "execution", &e)?;
            // No enable: finite actions must never replay on login or reboot.
            if let Err(error) = system(local, &["start", "--no-block"], Some(id)).await {
                stop(local, id).await?;
                return Err(error);
            }
            return Ok(serde_json::to_string(&e)?);
        }
        let e: Execution = serde_json::from_slice(&private_file(&dir.join("execution"), 4096)?)?;
        ensure!(
            e.unit == id && e.mode == Mode::Models && nonce == Some(e.nonce.as_str()),
            "wrong action correlation"
        );
        // Identity/revision checks still apply after the deadline, so cancellation is retryable.
        e.validate(&prepared.instance, e.deadline.saturating_sub(1))?;
        if verb == "cancel" || nostr::Timestamp::now().as_secs() >= e.deadline {
            stop(local, id).await?;
            remove_file(&dir.join(e.filename()))?;
            return Ok("cancelled\n".into());
        }
        let output = dir.join(e.filename());
        if !output.try_exists()? || !crate::service_actions::inactive(&shape) {
            return Ok("pending\n".into());
        }
        let result: ActionResult = serde_json::from_slice(&private_file(&output, 65536)?)?;
        ensure!(
            result.execution == e
                && result.finished
                && result.login_output.is_empty()
                && result.login == buzz_core::host_deployment::observation::LoginState::NotObserved
                && result.login_at.is_none()
                && result.observed_at <= e.deadline
                && result.observed_at <= nostr::Timestamp::now().as_secs(),
            "invalid model result"
        );
        Ok(serde_json::to_string(&result)?)
    }
    async fn install(&self, local: &LocalContext, id: &str) -> Result<()> {
        let template = local.unit_dir.join("buzz-hosted-agent@.service");
        let bytes = include_bytes!("../deploy/buzz-hosted-agent@.service");
        if template.try_exists()? {
            ensure!(
                private_file(&template, 4096)? == bytes,
                "foreign user template"
            );
        } else {
            write_bytes(&local.unit_dir, "buzz-hosted-agent@.service", bytes)?;
        }
        let dropin = local
            .unit_dir
            .join(format!("buzz-hosted-agent@{id}.service.d"));
        if !dropin.try_exists()? {
            fs::DirBuilder::new().mode(0o700).create(&dropin)?;
        }
        directory(&dropin)?;
        let text = self.override_text(local, id, false)?;
        if dropin.join("50-buzz.conf").try_exists()? {
            let old = private_file(&dropin.join("50-buzz.conf"), 8192)?;
            // Prepare already checked the retained identity, service location and inactivity.
            // Repair only our exact former serialization, never an edited/foreign override.
            ensure!(
                old == text.as_bytes() || old == self.override_text(local, id, true)?.as_bytes(),
                "foreign user override"
            );
        }
        write_bytes(&dropin, "50-buzz.conf", text.as_bytes())?;
        system(local, &["daemon-reload"], None).await?;
        show(local, id).await?;
        Ok(())
    }
    fn override_text(&self, local: &LocalContext, id: &str, legacy: bool) -> Result<String> {
        let credential = |name: &str| -> Result<String> {
            let path = self.dir(id).join(name);
            if legacy {
                // Exact pre-repair bytes are recognized solely for same-instance retry.
                quote(Path::new(&format!("{name}:{}", path.display())))
            } else {
                // LoadCredential splits at ':', then expands specifiers. Unlike ExecStart,
                // its source path is not shell-quoted or backslash-unescaped.
                Ok(format!(
                    "{name}:{}",
                    service_path(&path)?.replace('%', "%%")
                ))
            }
        };
        Ok(format!(
            "[Service]\nExecStart=\nExecStart={} --user-service {} {}\nLoadCredential={}\nLoadCredential={}\n",
            quote(&local.launcher)?.replace('$', "$$"),
            quote(&self.staging)?.replace('$', "$$"),
            id,
            credential("instance")?,
            credential("execution")?,
        ))
    }
    async fn cleanup(&self, id: &str) -> Result<String> {
        let dir = self.dir(id);
        // Persist fencing before any stop: no prepare can resurrect a pending cleanup.
        write_json(&self.root(), &format!("{id}.closed"), &true)?;
        if !dir.try_exists()? {
            return Ok("cleaned\n".into());
        }
        directory(&dir)?;
        let context = dir.join("context");
        if !context.try_exists()? {
            ensure!(
                fs::read_dir(&dir)?.next().is_none(),
                "missing cleanup context"
            );
            fs::remove_dir(&dir)?;
            return Ok("cleaned\n".into());
        }
        let local: LocalContext = serde_json::from_slice(&private_file(&context, 16384)?)?;
        local.validate_cleanup()?;
        stop(&local, id).await?;
        let dropin = local
            .unit_dir
            .join(format!("buzz-hosted-agent@{id}.service.d"));
        if dropin.try_exists()? {
            directory(&dropin)?;
            remove_file(&dropin.join("50-buzz.conf"))?;
            fs::remove_dir(&dropin)?;
        }
        system(&local, &["daemon-reload"], None).await?;
        if dir.join("execution").try_exists()? {
            let e: Execution =
                serde_json::from_slice(&private_file(&dir.join("execution"), 4096)?)?;
            ensure!(
                e.unit == id && crate::id(&e.nonce),
                "wrong cleanup execution"
            );
            remove_file(&dir.join(e.filename()))?;
            remove_file(&dir.join("execution"))?;
        }
        remove_file(&dir.join("instance"))?;
        // Keep context until every other artifact has been removed successfully.
        ensure!(
            fs::read_dir(&dir)?.all(|e| e.is_ok_and(|e| e.file_name() == "context")),
            "unrecognized managed artifact"
        );
        remove_file(&context)?;
        fs::remove_dir(&dir)?;
        fs::File::open(self.root())?.sync_all()?;
        Ok("cleaned\n".into())
    }
}

pub(crate) fn directory(p: &Path) -> Result<()> {
    let m = fs::symlink_metadata(p)?;
    ensure!(
        p.is_absolute()
            && p.canonicalize()? == p
            && m.is_dir()
            && m.uid() == rustix::process::geteuid().as_raw()
            && m.mode() & 0o077 == 0,
        "unsafe managed directory"
    );
    Ok(())
}
pub(crate) fn private_file(p: &Path, limit: u64) -> Result<Vec<u8>> {
    let f = fs::OpenOptions::new()
        .read(true)
        .custom_flags((rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK).bits() as i32)
        .open(p)?;
    let m = f.metadata()?;
    ensure!(
        m.is_file()
            && m.uid() == rustix::process::geteuid().as_raw()
            && m.nlink() == 1
            && m.mode() & 0o077 == 0,
        "unsafe managed file"
    );
    let mut bytes = Vec::new();
    f.take(limit + 1).read_to_end(&mut bytes)?;
    ensure!(bytes.len() as u64 <= limit, "oversized managed file");
    Ok(bytes)
}
pub(crate) fn write_json(dir: &Path, name: &str, value: &impl Serialize) -> Result<()> {
    write_bytes(dir, name, &serde_json::to_vec(value)?)
}
fn write_bytes(dir: &Path, name: &str, value: &[u8]) -> Result<()> {
    if fs::symlink_metadata(dir.join(name)).is_ok() {
        private_file(&dir.join(name), 131072)?;
    }
    let mut f = tempfile::NamedTempFile::new_in(dir)?;
    f.write_all(value)?;
    f.as_file().sync_all()?;
    f.persist(dir.join(name))?;
    fs::File::open(dir)?.sync_all()?;
    Ok(())
}
fn remove_file(p: &Path) -> Result<()> {
    match fs::symlink_metadata(p) {
        Ok(_) => {
            private_file(p, 131072)?;
            fs::remove_file(p)?;
            fs::File::open(p.parent().ok_or_else(|| anyhow!("missing parent"))?)?.sync_all()?;
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    Ok(())
}
fn service_path(p: &Path) -> Result<&str> {
    let s = p.to_str().ok_or_else(|| anyhow!("non-UTF8 service path"))?;
    ensure!(!s.chars().any(char::is_control), "invalid service path");
    Ok(s)
}
fn quote(p: &Path) -> Result<String> {
    Ok(format!(
        "\"{}\"",
        service_path(p)?
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('%', "%%")
    ))
}
async fn system(local: &LocalContext, args: &[&str], id: Option<&str>) -> Result<String> {
    let mut c = Command::new(&local.systemctl);
    c.args(["--user", "--no-pager"])
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    if let Some(id) = id {
        ensure!(crate::id(id), "invalid unit ID");
        c.arg(format!("buzz-hosted-agent@{id}.service"));
    }
    let mut child = c.spawn()?;
    let mut out = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("manager output missing"))?
        .take(16385);
    let result = tokio::time::timeout(
        Duration::from_secs(if args == ["start"] { 80 } else { 15 }),
        async {
            let mut bytes = Vec::new();
            out.read_to_end(&mut bytes).await?;
            ensure!(bytes.len() <= 16384, "manager output exceeded limit");
            ensure!(
                child.wait().await?.success(),
                "user manager operation failed"
            );
            Ok::<_, anyhow::Error>(String::from_utf8(bytes)?)
        },
    )
    .await;
    match result {
        Ok(Ok(s)) => Ok(s),
        other => {
            child.kill().await?;
            child.wait().await?;
            match other {
                Ok(Err(e)) => Err(e),
                _ => Err(anyhow!("user manager deadline exceeded")),
            }
        }
    }
}
async fn show(local: &LocalContext, id: &str) -> Result<String> {
    let text = system(local, &["show", "--property=LoadState,FragmentPath,DropInPaths,ActiveState,MainPID,ControlPID,TasksCurrent"], Some(id)).await?;
    let fragment = format!(
        "FragmentPath={}",
        local.unit_dir.join("buzz-hosted-agent@.service").display()
    );
    let dropin = format!(
        "DropInPaths={}",
        local
            .unit_dir
            .join(format!("buzz-hosted-agent@{id}.service.d/50-buzz.conf"))
            .display()
    );
    ensure!(
        text.lines().any(|l| l == "LoadState=not-found")
            || (text.lines().any(|l| l == fragment)
                && (text.lines().any(|l| l == dropin)
                    || text.lines().any(|l| l == "DropInPaths="))),
        "foreign managed unit"
    );
    Ok(text)
}
async fn stop(local: &LocalContext, id: &str) -> Result<()> {
    let state = show(local, id).await?;
    if !state.lines().any(|l| l == "LoadState=not-found") {
        system(local, &["stop"], Some(id)).await?;
    }
    ensure!(
        crate::service_actions::inactive(&show(local, id).await?),
        "user service stop unconfirmed"
    );
    Ok(())
}

#[cfg(test)]
#[path = "user_service_tests.rs"]
mod tests;
