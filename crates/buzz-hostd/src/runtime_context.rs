//! Fixed runtime modes share the selected local account or legacy systemd context.
use crate::{Catalog, Instance, Prepared};
use anyhow::{anyhow, ensure, Result};
use buzz_core::host_deployment::observation::{LoginState, Models, UnavailableReason};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::{io::AsyncReadExt, process::Command};

/// A closed mode selector; never client argv.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    Conversation,
    Login,
    Models,
}
/// Controller-generated immutable systemd execution credential.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Execution {
    pub unit: String,
    pub agent: String,
    pub generation: String,
    pub catalog: String,
    pub operation: String,
    pub revision: u64,
    pub nonce: String,
    pub deadline: u64,
    pub mode: Mode,
}
impl Execution {
    /// Bind the fixed action to the recorded prepared instance.
    pub fn new(i: &Instance, mode: Mode, now: u64) -> Result<Self> {
        let d = i
            .deployment
            .as_ref()
            .ok_or_else(|| anyhow!("managed instance required"))?;
        Ok(Self {
            unit: i.unit_id.clone(),
            agent: i.agent.clone(),
            generation: i.scope.generation.clone(),
            catalog: i.revision.clone(),
            operation: d.operation_id.clone(),
            revision: d.revision,
            nonce: hex::encode(rand::random::<[u8; 32]>()),
            deadline: now + if mode == Mode::Login { 600 } else { 60 },
            mode,
        })
    }
    /// Reject stale credentials before spawning a child.
    pub fn validate(&self, i: &Instance, now: u64) -> Result<()> {
        let d = i
            .deployment
            .as_ref()
            .ok_or_else(|| anyhow!("managed instance required"))?;
        ensure!(
            self.unit == i.unit_id
                && self.agent == i.agent
                && self.generation == i.scope.generation
                && self.catalog == i.revision
                && self.operation == d.operation_id
                && self.revision == d.revision
                && crate::id(&self.nonce),
            "execution binding mismatch"
        );
        if self.mode != Mode::Conversation {
            ensure!(
                now < self.deadline
                    && self.deadline - now <= if self.mode == Mode::Login { 600 } else { 60 },
                "execution expired"
            );
        }
        Ok(())
    }
    /// Fixed basename derived from the protected nonce, never a caller path.
    pub fn filename(&self) -> String {
        format!(".buzz-action-{}", self.nonce)
    }
}
/// Legacy offline policy and the trusted-account route, still gated on S2 readiness.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NetworkPolicy {
    Disabled,
    /// Normal trusted-account networking; conversation still awaits S2 readiness.
    TrustedAccount,
}
impl NetworkPolicy {
    /// The installed conversation entrypoint requires reviewed online networking.
    pub fn require_conversation(self) -> Result<()> {
        ensure!(self != Self::Disabled, "network-disabled");
        ensure!(
            self != Self::TrustedAccount,
            "current readiness unavailable"
        );
        Ok(())
    }
}
/// Fixed local action result. Raw login bytes exist only in the unit runtime directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionResult {
    pub execution: Execution,
    pub finished: bool,
    pub observed_at: u64,
    pub login: LoginState,
    pub login_at: Option<u64>,
    pub models: Models,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub login_output: String,
}
/// Paths come from the fixed unit or operator-configured trusted account.
pub struct Context {
    /// Present only for the ordinary trusted-account route.
    pub account: Option<crate::user_service::LocalContext>,
    pub home: PathBuf,
    pub runtime: PathBuf,
    pub notify: std::ffi::OsString,
}
impl Context {
    /// Require the canonical per-unit paths supplied by the installed systemd unit.
    pub fn from_systemd(unit: &str) -> Result<Self> {
        let home = PathBuf::from(
            env::var_os("STATE_DIRECTORY").ok_or_else(|| anyhow!("state directory missing"))?,
        );
        let runtime = PathBuf::from(
            env::var_os("RUNTIME_DIRECTORY").ok_or_else(|| anyhow!("runtime directory missing"))?,
        );
        ensure!(
            home == Path::new("/var/lib").join(format!("buzz-agent-{unit}"))
                && runtime == Path::new("/run").join(format!("buzz-agent-{unit}")),
            "wrong systemd context"
        );
        Ok(Self {
            account: None,
            home,
            runtime,
            notify: env::var_os("NOTIFY_SOCKET")
                .ok_or_else(|| anyhow!("notification socket missing"))?,
        })
    }
    /// The common environment is intentionally independent of inherited provider variables.
    pub fn environment(&self) -> BTreeMap<String, String> {
        if let Some(account) = &self.account {
            return account.environment();
        }
        [
            ("HOME", self.home.clone()),
            ("GROK_HOME", self.home.join(".grok")),
            ("XDG_CONFIG_HOME", self.home.join(".config")),
            ("XDG_RUNTIME_DIR", self.runtime.clone()),
        ]
        .into_iter()
        .map(|(k, v)| (k.into(), v.to_string_lossy().into_owned()))
        .chain([(
            "PATH".into(),
            "/opt/buzz-runtime/bin:/usr/local/bin:/usr/bin:/bin".into(),
        )])
        .collect()
    }
    fn command(&self, program: &Path) -> Command {
        let mut c = Command::new(program);
        c.env_clear()
            .envs(self.environment())
            .current_dir(
                self.account
                    .as_ref()
                    .map_or(self.home.as_path(), |a| a.workspace.as_path()),
            )
            .stdin(Stdio::null())
            .kill_on_drop(true);
        c
    }
}
fn publish(dir: &Path, result: &ActionResult) -> Result<()> {
    let mut f = tempfile::NamedTempFile::new_in(dir)?;
    serde_json::to_writer(&mut f, result)?;
    f.flush()?;
    f.as_file().sync_all()?;
    f.persist(dir.join(result.execution.filename()))?;
    fs::File::open(dir)?.sync_all()?;
    Ok(())
}
fn notify(path: &std::ffi::OsStr) -> Result<()> {
    use std::os::{
        linux::net::SocketAddrExt,
        unix::{
            ffi::OsStrExt,
            net::{SocketAddr, UnixDatagram},
        },
    };
    let addr = match path.as_bytes().strip_prefix(b"@") {
        Some(name) => SocketAddr::from_abstract_name(name)?,
        None => SocketAddr::from_pathname(path)?,
    };
    UnixDatagram::unbound()?.send_to_addr(b"READY=1", &addr)?;
    Ok(())
}
/// Execute the actual installed finite action. The offline policy does not substitute a stub.
pub async fn finite(
    context: &Context,
    prepared: &Prepared,
    catalog: &Catalog,
    execution: &Execution,
    policy: NetworkPolicy,
) -> Result<ActionResult> {
    let i = &prepared.instance;
    execution.validate(i, nostr::Timestamp::now().as_secs())?;
    ensure!(
        prepared.provider.is_empty(),
        "provider environment forbidden"
    );
    let rt = &catalog.runtimes[&i.runtime_id];
    ensure!(
        context.account.is_none() || execution.mode == Mode::Models,
        "native login is external"
    );
    ensure!(execution.mode != Mode::Conversation, "finite mode required");
    let mut command = match execution.mode {
        Mode::Login => {
            ensure!(
                rt.harness_id.as_deref() == Some("grok"),
                "native login unsupported"
            );
            let mut c = context.command(&rt.adapter.path);
            c.args(["login", "--device-auth"]);
            c
        }
        Mode::Models => {
            let mut c = context.command(&rt.harness.path);
            c.args(["models", "--json"])
                .env("BUZZ_ACP_AGENT_COMMAND", &rt.adapter.path)
                .env("BUZZ_ACP_AGENT_ARGS", rt.args.join(","));
            c
        }
        Mode::Conversation => unreachable!(),
    };
    let mut result = ActionResult {
        execution: execution.clone(),
        finished: false,
        observed_at: nostr::Timestamp::now().as_secs(),
        login_at: None,
        login: if execution.mode == Mode::Login {
            LoginState::InProgress
        } else {
            LoginState::NotObserved
        },
        models: Models::Unavailable {
            reason: UnavailableReason::NetworkDisabled,
        },
        login_output: String::new(),
    };
    command.process_group(0);
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let _group = ProcessGroup(
        rustix::process::Pid::from_raw(
            child.id().ok_or_else(|| anyhow!("missing child ID"))? as i32
        )
        .ok_or_else(|| anyhow!("invalid child ID"))?,
    );
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("stdout missing"))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow!("stderr missing"))?;
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    // Readiness means supervised action started, not authentication or available models.
    notify(&context.notify)?;
    let timeout = tokio::time::sleep(Duration::from_secs(
        execution
            .deadline
            .saturating_sub(nostr::Timestamp::now().as_secs()),
    ));
    tokio::pin!(timeout);
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let (mut a, mut b) = ([0u8; 4096], [0u8; 4096]);
    let (mut out_end, mut err_end, mut status) = (false, false, None);
    let failure = loop {
        if out_end && err_end && status.is_some() {
            break None;
        }
        tokio::select! {
            _=&mut timeout=>break Some(UnavailableReason::Timeout),
            _=terminate.recv()=>break Some(UnavailableReason::Execution),
            s=child.wait(), if status.is_none()=>{status=Some(s?);},
            n=stdout.read(&mut a), if !out_end=>{let n=n?;out_end=n==0;out.extend_from_slice(&a[..n]);},
            n=stderr.read(&mut b), if !err_end=>{let n=n?;err_end=n==0;err.extend_from_slice(&b[..n]);},
        }
        if out.len() + err.len() > 65536 {
            break Some(UnavailableReason::Oversized);
        }
        if execution.mode == Mode::Login {
            result.login_output =
                String::from_utf8_lossy(&out).into_owned() + &String::from_utf8_lossy(&err);
            publish(&context.runtime, &result)?;
        }
    };
    if status.is_none() {
        child.kill().await?;
        child.wait().await?;
    }
    let success = failure.is_none() && status.is_some_and(|s| s.success());
    result.finished = true;
    result.observed_at = nostr::Timestamp::now().as_secs().min(execution.deadline);
    result.login_output.clear();
    if execution.mode == Mode::Login {
        result.login_at = Some(result.observed_at);
        result.login = if success {
            LoginState::Succeeded
        } else if failure == Some(UnavailableReason::Timeout) {
            LoginState::Cancelled
        } else {
            LoginState::Failed
        };
    } else {
        let parsed = if success {
            serde_json::from_slice::<serde_json::Value>(&out)
                .ok()
                .map(|raw| buzz_core::agent_models::normalize_agent_models(&raw, None))
        } else {
            None
        };
        let local = match parsed {
            Some(c) if buzz_core::host_deployment::observation::validate_models(&c).is_ok() => {
                if c.models.is_empty() {
                    Models::Empty
                } else {
                    Models::Available { catalog: c }
                }
            }
            Some(_) => Models::Unavailable {
                reason: UnavailableReason::Oversized,
            },
            None => Models::Unavailable {
                reason: failure.unwrap_or(if success {
                    UnavailableReason::InvalidOutput
                } else {
                    UnavailableReason::Execution
                }),
            },
        };
        // Enumerating cached/local models offline cannot establish online readiness.
        result.models = if rt.harness_id.is_none() {
            Models::Unsupported
        } else {
            match (policy, local) {
                (NetworkPolicy::Disabled, Models::Available { .. } | Models::Empty) => {
                    Models::Unavailable {
                        reason: UnavailableReason::NetworkDisabled,
                    }
                }
                (_, other) => other,
            }
        };
    }
    // Terminate descendants before publishing the terminal receipt; systemd also
    // confirms the service cgroup is empty before observe exposes this result.
    drop(_group);
    publish(&context.home, &result)?;
    let live = context.runtime.join(execution.filename());
    if context.runtime != context.home && live.try_exists()? {
        fs::remove_file(live)?;
    }
    // systemd KillMode=control-group reaps any descendants when this main process exits.
    Ok(result)
}
/// Validate installed credentials and run one fixed mode; called by the actual binary.
pub async fn run() -> Result<()> {
    ensure!(
        !rustix::process::geteuid().is_root(),
        "refusing root runtime"
    );
    let creds = PathBuf::from(
        env::var_os("CREDENTIALS_DIRECTORY").ok_or_else(|| anyhow!("credentials missing"))?,
    );
    let prepared: Prepared =
        serde_json::from_slice(&crate::bounded(&creds.join("instance"), 131072)?)?;
    let execution: Execution =
        serde_json::from_slice(&crate::bounded(&creds.join("execution"), 4096)?)?;
    let policy: NetworkPolicy =
        serde_json::from_slice(&crate::bounded(&creds.join("network-policy"), 128)?)?;
    let i = &prepared.instance;
    execution.validate(i, nostr::Timestamp::now().as_secs())?;
    let context = Context::from_systemd(&i.unit_id)?;
    let (catalog, rev) = Catalog::read(Path::new(crate::OPERATOR_CATALOG))?;
    i.validate(&catalog, &rev)?;
    let rt = &catalog.runtimes[&i.runtime_id];
    rt.harness.verify()?;
    rt.adapter.verify()?;
    ensure!(catalog.current(&rev)?, "catalog changed before execution");
    if execution.mode != Mode::Conversation {
        finite(&context, &prepared, &catalog, &execution, policy).await?;
        return Ok(());
    }
    // N1 must add a reviewed online policy and native evidence before this can launch.
    policy.require_conversation()?;
    let (login, at): (LoginState, u64) =
        serde_json::from_slice(&crate::bounded(&creds.join("native-login"), 256)?)?;
    ensure!(
        login == LoginState::Succeeded && at <= nostr::Timestamp::now().as_secs(),
        "native login unresolved"
    );
    // A future online policy must still pass this same-context fresh discovery.
    let probe = Execution::new(i, Mode::Models, nostr::Timestamp::now().as_secs())?;
    let result = finite(&context, &prepared, &catalog, &probe, policy).await?;
    let selected = match &i
        .deployment
        .as_ref()
        .ok_or_else(|| anyhow!("managed configuration missing"))?
        .config
        .model
    {
        buzz_core::host_deployment::ModelSelection::Selected { id } => id,
        _ => return Err(anyhow!("model selection pending")),
    };
    ensure!(
        matches!(result.models,Models::Available{ref catalog} if catalog.models.iter().any(|m| &m.id==selected)),
        "selected model unavailable"
    );
    ensure!(catalog.current(&rev)?, "catalog changed before launch");
    let mut command = context.command(&rt.harness.path);
    command
        .envs(crate::deployment_environment(i)?)
        .env(
            "NOTIFY_SOCKET",
            env::var_os("NOTIFY_SOCKET").ok_or_else(|| anyhow!("notification socket missing"))?,
        )
        .env(
            "BUZZ_PAIRED_STARTUP_CHANNELS",
            serde_json::to_string(&i.channels)?,
        )
        .env("BUZZ_PRIVATE_KEY", &i.secret)
        .env("BUZZ_AUTH_TAG", &i.oa)
        .env(
            "BUZZ_RELAY_URL",
            i.scope.community.replacen("https://", "wss://", 1),
        )
        .env("BUZZ_ACP_AGENT_OWNER", i.scope.owner.to_hex())
        .env("BUZZ_ACP_AGENT_COMMAND", &rt.adapter.path)
        .env("BUZZ_ACP_AGENT_ARGS", rt.args.join(","))
        .env("BUZZ_ACP_RESPOND_TO", &i.policy.respond_to)
        .env(
            "BUZZ_ACP_RESPOND_TO_ALLOWLIST",
            i.policy.respond_to_allowlist.join(","),
        );
    use std::os::unix::process::CommandExt;
    Err(command.as_std_mut().exec().into())
}

// The runtime service owns the whole process group even when the direct child exits early.
struct ProcessGroup(rustix::process::Pid);
impl Drop for ProcessGroup {
    fn drop(&mut self) {
        let _ = rustix::process::kill_process_group(self.0, rustix::process::Signal::KILL);
    }
}

/// Fixed user-service entry; argv and environment come only from local configuration.
pub async fn run_user(state: &Path, unit: &str) -> Result<()> {
    let service = crate::user_service::UserService::installed(state)?;
    ensure!(crate::id(unit), "invalid unit ID");
    let dir = state.join("hosted").join(unit);
    crate::user_service::directory(&dir)?;
    ensure!(
        !state
            .join("hosted")
            .join(format!("{unit}.closed"))
            .try_exists()?,
        "unit fenced"
    );
    let creds = PathBuf::from(
        env::var_os("CREDENTIALS_DIRECTORY").ok_or_else(|| anyhow!("credentials missing"))?,
    );
    // systemd credential copies can be read-only (0400), and are owned by this user.
    let prepared: Prepared = serde_json::from_slice(&crate::user_service::private_file(
        &creds.join("instance"),
        131072,
    )?)?;
    let execution: Execution = serde_json::from_slice(&crate::user_service::private_file(
        &creds.join("execution"),
        4096,
    )?)?;
    let (catalog, revision) = service.catalog()?;
    prepared.instance.validate(&catalog, &revision)?;
    ensure!(
        prepared.instance.unit_id == unit && execution.mode == Mode::Models,
        "conversation unavailable until current readiness is implemented"
    );
    execution.validate(&prepared.instance, nostr::Timestamp::now().as_secs())?;
    let rt = &catalog.runtimes[&prepared.instance.runtime_id];
    rt.harness.available()?;
    rt.adapter.available()?;
    let context = Context {
        account: catalog.local.clone(),
        home: dir.clone(),
        runtime: dir,
        notify: env::var_os("NOTIFY_SOCKET").ok_or_else(|| anyhow!("notify socket missing"))?,
    };
    ensure!(catalog.current(&revision)?, "local configuration changed");
    finite(
        &context,
        &prepared,
        &catalog,
        &execution,
        NetworkPolicy::TrustedAccount,
    )
    .await?;
    Ok(())
}
