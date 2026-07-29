//! Remote helper: runs on the Linux host that owns the agents.
//!
//! `deploy` persists the agent payload as a private config file, installs a
//! shared systemd user template unit, and (re)starts the agent's instance.
//! `run-agent` is that unit's entry point: it rebuilds the environment the
//! desktop would give a locally spawned agent (see `spawn_agent_child` in
//! `desktop/src-tauri/src/managed_agents/runtime.rs`) and execs `buzz-acp`.
//!
//! Agent identity → unit mapping is deterministic: the instance id is a
//! prefix of the agent's public key, so redeploys and renames always target
//! the same unit and never strand an old process under a stale name.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::protocol::{respond_ok, AgentPayload, ProviderConfig, ProviderRequest};

/// Identity and security env keys that agent-supplied `env_vars` must never
/// override. Mirrors `RESERVED_ENV_KEYS` in the desktop's
/// `managed_agents/env_vars.rs`; comparison is case-insensitive like there.
const RESERVED_ENV_KEYS: &[&str] = &[
    "BUZZ_PRIVATE_KEY",
    "NOSTR_PRIVATE_KEY",
    "BUZZ_AUTH_TAG",
    "BUZZ_API_TOKEN",
    "BUZZ_ACP_PRIVATE_KEY",
    "BUZZ_ACP_API_TOKEN",
    "BUZZ_RELAY_URL",
    "BUZZ_ACP_AGENT_COMMAND",
    "BUZZ_ACP_AGENT_ARGS",
    "BUZZ_ACP_MCP_COMMAND",
    "BUZZ_ACP_RESPOND_TO",
    "BUZZ_ACP_RESPOND_TO_ALLOWLIST",
    "BUZZ_ACP_AGENT_OWNER",
    "BUZZ_ACP_SETUP_PAYLOAD",
];

const UNIT_TEMPLATE_NAME: &str = "buzz-remote-agent@.service";

/// On-disk agent record: the deploy payload plus the runtime options the
/// unit entry point needs. Written 0600 — it contains the agent's key.
#[derive(Debug, Serialize, Deserialize)]
struct StoredAgent {
    agent: AgentPayload,
    acp_path: String,
    remote_path_prepend: String,
}

pub fn deploy(raw_request: &str) -> anyhow::Result<()> {
    let request: ProviderRequest =
        serde_json::from_str(raw_request).context("invalid deploy request JSON")?;
    if request.op != "deploy" {
        anyhow::bail!("remote-deploy expects op 'deploy', got '{}'", request.op);
    }
    if let Some(id) = &request.request_id {
        eprintln!("buzz-backend-ssh remote-deploy: request {id}");
    }
    let agent = request
        .agent
        .as_ref()
        .context("deploy request is missing 'agent'")?;
    let config = ProviderConfig::from_value(request.provider_config.as_ref());

    let agent_id = agent_id_for(&agent.private_key_nsec)?;
    let record = StoredAgent {
        agent: agent.clone(),
        acp_path: config.acp_path.clone(),
        remote_path_prepend: config.remote_path_prepend.clone(),
    };

    let config_path = agent_config_path(&agent_id)?;
    write_private_json(&config_path, &record)
        .with_context(|| format!("failed to write {}", config_path.display()))?;

    let unit_changed = install_unit_template()?;
    let mut warnings = Vec::new();
    start_unit(&agent_id, unit_changed)?;
    if let Some(warning) = linger_warning() {
        eprintln!("buzz-backend-ssh: {warning}");
        warnings.push(warning);
    }

    respond_ok(serde_json::json!({
        "agent_id": agent_id,
        "unit": format!("buzz-remote-agent@{agent_id}.service"),
        "warnings": warnings,
    }));
    Ok(())
}

pub fn run_agent(agent_id: &str) -> anyhow::Result<()> {
    validate_agent_id(agent_id)?;
    let config_path = agent_config_path(agent_id)?;
    let raw = std::fs::read_to_string(&config_path)
        .with_context(|| format!("failed to read {}", config_path.display()))?;
    let record: StoredAgent =
        serde_json::from_str(&raw).context("agent config is not valid JSON")?;

    let base_path = std::env::var("PATH").unwrap_or_default();
    let env = build_agent_env(&record, &base_path);

    exec_acp(&record, env)
}

fn exec_acp(record: &StoredAgent, env: BTreeMap<String, String>) -> anyhow::Result<()> {
    use std::os::unix::process::CommandExt;

    let mut cmd = std::process::Command::new(&record.acp_path);
    if let Ok(home) = std::env::var("HOME") {
        cmd.current_dir(home);
    }
    for (key, value) in &env {
        cmd.env(key, value);
    }
    // exec only returns on failure.
    let err = cmd.exec();
    Err(anyhow::anyhow!(
        "failed to exec '{}': {err}",
        record.acp_path
    ))
}

/// The stable agent id: a 16-hex-char prefix of the agent's public key,
/// derived from the deploy payload's private key (hex or nsec accepted).
fn agent_id_for(private_key: &str) -> anyhow::Result<String> {
    let keys = nostr::Keys::parse(private_key)
        .map_err(|_| anyhow::anyhow!("agent private key is not valid hex or nsec"))?;
    let hex = keys.public_key().to_hex();
    Ok(hex.chars().take(16).collect())
}

fn validate_agent_id(agent_id: &str) -> anyhow::Result<()> {
    // Lowercase-only: generated ids are lowercase hex, and accepting an
    // uppercase variant would resolve to a config path we never write.
    let valid =
        agent_id.len() == 16 && agent_id.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f'));
    if !valid {
        anyhow::bail!("invalid agent id '{agent_id}' (expected 16 lowercase hex chars)");
    }
    Ok(())
}

/// Build the full environment for the agent process, mirroring the desktop's
/// local spawn (`spawn_agent_child`): user `env_vars` first with reserved
/// keys stripped, then Buzz-owned identity/config values so they always win.
fn build_agent_env(record: &StoredAgent, base_path: &str) -> BTreeMap<String, String> {
    let agent = &record.agent;
    let mut env: BTreeMap<String, String> = BTreeMap::new();

    for (key, value) in &agent.env_vars {
        let reserved = RESERVED_ENV_KEYS
            .iter()
            .any(|r| r.eq_ignore_ascii_case(key));
        if !reserved {
            env.insert(key.clone(), value.clone());
        }
    }

    let path = if record.remote_path_prepend.is_empty() {
        base_path.to_string()
    } else {
        format!("{}:{base_path}", record.remote_path_prepend)
    };
    env.insert("PATH".to_string(), path.clone());
    if !env.contains_key("RUST_LOG") && std::env::var("RUST_LOG").is_err() {
        env.insert("RUST_LOG".to_string(), "info".to_string());
    }

    env.insert(
        "BUZZ_PRIVATE_KEY".to_string(),
        agent.private_key_nsec.clone(),
    );
    env.insert("BUZZ_RELAY_URL".to_string(), agent.relay_url.clone());
    if let Some(auth_tag) = &agent.auth_tag {
        env.insert("BUZZ_AUTH_TAG".to_string(), auth_tag.clone());
    }
    env.insert(
        "BUZZ_ACP_AGENT_COMMAND".to_string(),
        agent.agent_command.clone(),
    );
    env.insert(
        "BUZZ_ACP_AGENT_ARGS".to_string(),
        agent.agent_args.join(","),
    );
    // Only buzz-agent needs the bundled MCP sidecar; other harnesses bring
    // their own tools. Mirrors `known_acp_runtime(...).mcp_command`.
    let mcp = if Path::new(&agent.agent_command)
        .file_name()
        .is_some_and(|n| n == "buzz-agent")
    {
        "buzz-dev-mcp"
    } else {
        ""
    };
    env.insert("BUZZ_ACP_MCP_COMMAND".to_string(), mcp.to_string());
    env.insert("BUZZ_ACP_LAZY_POOL".to_string(), "false".to_string());

    if let Some(prompt) = &agent.system_prompt {
        env.insert("BUZZ_ACP_SYSTEM_PROMPT".to_string(), prompt.clone());
    }
    if let Some(model) = &agent.model {
        env.insert("BUZZ_ACP_MODEL".to_string(), model.clone());
    }
    if let Some(parallelism) = agent.parallelism {
        env.insert("BUZZ_ACP_AGENTS".to_string(), parallelism.to_string());
    }
    if let Some(idle) = agent.idle_timeout_seconds {
        env.insert("BUZZ_ACP_IDLE_TIMEOUT".to_string(), idle.to_string());
    }
    if let Some(max_duration) = agent.max_turn_duration_seconds {
        env.insert(
            "BUZZ_ACP_MAX_TURN_DURATION".to_string(),
            max_duration.to_string(),
        );
    }
    if let Some(respond_to) = &agent.respond_to {
        env.insert("BUZZ_ACP_RESPOND_TO".to_string(), respond_to.clone());
    }
    if !agent.respond_to_allowlist.is_empty() {
        env.insert(
            "BUZZ_ACP_RESPOND_TO_ALLOWLIST".to_string(),
            agent.respond_to_allowlist.join(","),
        );
    }
    env.insert("BUZZ_ACP_SESSION_TITLE".to_string(), agent.name.clone());
    // Parity with local spawn: queueing/steering behavior and the relay
    // observer stream that powers the desktop's live agent activity view.
    env.insert(
        "BUZZ_ACP_MULTIPLE_EVENT_HANDLING".to_string(),
        "steer".to_string(),
    );
    env.insert("BUZZ_ACP_DEDUP".to_string(), "queue".to_string());
    env.insert("BUZZ_ACP_RELAY_OBSERVER".to_string(), "true".to_string());

    // Automatic relay-git auth, scoped to this relay's git URL exactly like
    // the desktop does — required for agents to clone/push Buzz projects.
    if let Some(helper) = which_on_path("git-credential-nostr", &path) {
        let http_base = relay_http_base_url(&agent.relay_url);
        env.insert(
            "NOSTR_PRIVATE_KEY".to_string(),
            agent.private_key_nsec.clone(),
        );
        env.insert("GIT_TERMINAL_PROMPT".to_string(), "0".to_string());
        env.insert("GIT_CONFIG_COUNT".to_string(), "2".to_string());
        env.insert(
            "GIT_CONFIG_KEY_0".to_string(),
            format!("credential.{http_base}/git.helper"),
        );
        env.insert(
            "GIT_CONFIG_VALUE_0".to_string(),
            helper.to_string_lossy().to_string(),
        );
        env.insert(
            "GIT_CONFIG_KEY_1".to_string(),
            format!("credential.{http_base}/git.useHttpPath"),
        );
        env.insert("GIT_CONFIG_VALUE_1".to_string(), "true".to_string());
    } else {
        eprintln!(
            "buzz-backend-ssh: git-credential-nostr not found on PATH — \
             agent '{}' will not have automatic Buzz git auth",
            agent.name
        );
    }

    env
}

/// `ws(s)://` → `http(s)://`, no trailing slash — the credential scope the
/// desktop uses (`relay_http_base_url` in `desktop/src-tauri/src/relay.rs`).
fn relay_http_base_url(relay_url: &str) -> String {
    let base = relay_url.trim_end_matches('/');
    if let Some(rest) = base.strip_prefix("wss://") {
        format!("https://{rest}")
    } else if let Some(rest) = base.strip_prefix("ws://") {
        format!("http://{rest}")
    } else {
        base.to_string()
    }
}

fn which_on_path(binary: &str, path: &str) -> Option<PathBuf> {
    std::env::split_paths(path)
        .map(|dir| dir.join(binary))
        .find(|candidate| is_executable(candidate))
}

fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

// ── Filesystem layout ──────────────────────────────────────────────────────

fn config_home() -> anyhow::Result<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return Ok(PathBuf::from(xdg));
        }
    }
    let home = std::env::var("HOME").context("HOME is not set")?;
    Ok(PathBuf::from(home).join(".config"))
}

fn agent_config_path(agent_id: &str) -> anyhow::Result<PathBuf> {
    Ok(config_home()?
        .join("buzz-backend-ssh")
        .join("agents")
        .join(format!("{agent_id}.json")))
}

/// Write `value` as JSON to `path` with owner-only permissions, atomically
/// (temp file + rename) so a concurrent `run-agent` never reads a torn file.
fn write_private_json<T: Serialize>(path: &Path, value: &T) -> anyhow::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};

    let dir = path
        .parent()
        .context("agent config path has no parent directory")?;
    if !dir.exists() {
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)
            .with_context(|| format!("failed to create {}", dir.display()))?;
    }

    let tmp = path.with_extension("json.tmp");
    // create_new + removal of any stale tmp: never reuse an existing inode
    // whose permissions we do not control.
    let _ = std::fs::remove_file(&tmp);
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&tmp)
        .with_context(|| format!("failed to create {}", tmp.display()))?;
    let body = serde_json::to_vec_pretty(value).context("failed to serialize agent config")?;
    file.write_all(&body)
        .context("failed to write agent config")?;
    file.sync_all().ok();
    drop(file);
    std::fs::rename(&tmp, path).context("failed to move agent config into place")?;
    Ok(())
}

// ── systemd integration ────────────────────────────────────────────────────

/// The shared template unit. `%i` is the agent id; the entry point resolves
/// everything else from the agent's config file.
fn unit_template() -> anyhow::Result<String> {
    let exe = std::env::current_exe()
        .context("cannot resolve own executable path for the systemd unit")?;
    let exe = exe.canonicalize().unwrap_or(exe).display().to_string();
    Ok(format!(
        "# Installed by buzz-backend-ssh — do not edit; redeploys overwrite.\n\
         [Unit]\n\
         Description=Buzz managed agent (%i)\n\
         StartLimitIntervalSec=0\n\
         \n\
         [Service]\n\
         Type=exec\n\
         ExecStart=\"{exe}\" run-agent %i\n\
         Restart=on-failure\n\
         RestartSec=5\n\
         TimeoutStopSec=30\n\
         SyslogIdentifier=buzz-remote-agent-%i\n\
         \n\
         [Install]\n\
         WantedBy=default.target\n"
    ))
}

/// Install or refresh the template unit. Returns true when the file changed
/// (the caller must daemon-reload before starting instances).
fn install_unit_template() -> anyhow::Result<bool> {
    let unit_dir = config_home()?.join("systemd").join("user");
    std::fs::create_dir_all(&unit_dir)
        .with_context(|| format!("failed to create {}", unit_dir.display()))?;
    let unit_path = unit_dir.join(UNIT_TEMPLATE_NAME);
    let desired = unit_template()?;
    let current = std::fs::read_to_string(&unit_path).unwrap_or_default();
    if current == desired {
        return Ok(false);
    }
    std::fs::write(&unit_path, desired)
        .with_context(|| format!("failed to write {}", unit_path.display()))?;
    Ok(true)
}

/// Run `systemctl --user` with the session bus env defaulted for
/// non-interactive ssh sessions (where XDG_RUNTIME_DIR may be absent).
fn systemctl_user(args: &[&str]) -> anyhow::Result<std::process::Output> {
    let mut cmd = std::process::Command::new("systemctl");
    cmd.arg("--user").args(args);
    if std::env::var_os("XDG_RUNTIME_DIR").is_none() {
        let uid = effective_uid();
        cmd.env("XDG_RUNTIME_DIR", format!("/run/user/{uid}"));
        cmd.env(
            "DBUS_SESSION_BUS_ADDRESS",
            format!("unix:path=/run/user/{uid}/bus"),
        );
    }
    cmd.output()
        .with_context(|| format!("failed to run systemctl --user {}", args.join(" ")))
}

/// Effective uid without unsafe code or new dependencies: systemd user
/// runtime dirs are keyed by uid, which the kernel exposes via /proc.
fn effective_uid() -> String {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            status.lines().find_map(|line| {
                line.strip_prefix("Uid:")
                    .and_then(|rest| rest.split_whitespace().next())
                    .map(String::from)
            })
        })
        .unwrap_or_else(|| "0".to_string())
}

fn start_unit(agent_id: &str, unit_changed: bool) -> anyhow::Result<()> {
    let instance = format!("buzz-remote-agent@{agent_id}.service");

    if unit_changed {
        let reload = systemctl_user(&["daemon-reload"])?;
        if !reload.status.success() {
            anyhow::bail!(
                "systemctl --user daemon-reload failed: {}",
                stderr_snippet(&reload)
            );
        }
    }

    let enable = systemctl_user(&["enable", &instance])?;
    if !enable.status.success() {
        anyhow::bail!("failed to enable {instance}: {}", stderr_snippet(&enable));
    }

    // restart also starts a stopped unit, making redeploys idempotent.
    let restart = systemctl_user(&["restart", &instance])?;
    if !restart.status.success() {
        anyhow::bail!(
            "failed to start {instance}: {}. Recent log: {}",
            stderr_snippet(&restart),
            journal_tail(&instance)
        );
    }

    // Catch immediate crashes (bad acp path, bad relay URL): give the
    // process a moment, then require the unit to still be active.
    std::thread::sleep(std::time::Duration::from_secs(2));
    let state = systemctl_user(&["is-active", &instance])?;
    let state_str = String::from_utf8_lossy(&state.stdout).trim().to_string();
    if state_str != "active" {
        anyhow::bail!(
            "{instance} did not stay running (state: {state_str}). Recent log: {}",
            journal_tail(&instance)
        );
    }
    Ok(())
}

fn journal_tail(instance: &str) -> String {
    let output = std::process::Command::new("journalctl")
        .args([
            "--user",
            "-u",
            instance,
            "-n",
            "12",
            "--no-pager",
            "-o",
            "cat",
        ])
        .output();
    match output {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout);
            let trimmed = text.trim();
            if trimmed.is_empty() {
                "(no journal entries)".to_string()
            } else {
                truncate_chars(trimmed, 1500)
            }
        }
        _ => "(journal unavailable)".to_string(),
    }
}

fn stderr_snippet(output: &std::process::Output) -> String {
    let text = String::from_utf8_lossy(&output.stderr);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        format!("exit status {}", output.status)
    } else {
        truncate_chars(trimmed, 1000)
    }
}

fn truncate_chars(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

/// Without lingering, systemd stops user units when the last session ends —
/// deployed agents would die on ssh disconnect from an otherwise idle host.
fn linger_warning() -> Option<String> {
    let user = std::env::var("USER").ok()?;
    let output = std::process::Command::new("loginctl")
        .args(["show-user", &user, "--property=Linger", "--value"])
        .output()
        .ok()?;
    let linger = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if output.status.success() && linger == "no" {
        Some(format!(
            "lingering is disabled for '{user}' — agents stop when your last \
             session ends. Fix: sudo loginctl enable-linger {user}"
        ))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_record() -> StoredAgent {
        let request: ProviderRequest =
            serde_json::from_value(crate::protocol::tests::deploy_request_fixture())
                .expect("fixture parses");
        let agent = request.agent.expect("agent present");
        let config = ProviderConfig::from_value(request.provider_config.as_ref());
        StoredAgent {
            agent,
            acp_path: config.acp_path,
            remote_path_prepend: config.remote_path_prepend,
        }
    }

    #[test]
    fn agent_id_is_stable_and_hex() {
        let record = fixture_record();
        let id = agent_id_for(&record.agent.private_key_nsec).expect("valid key");
        assert_eq!(id.len(), 16);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
        // Deterministic: same key, same id.
        let again = agent_id_for(&record.agent.private_key_nsec).expect("valid key");
        assert_eq!(id, again);
        validate_agent_id(&id).expect("own ids validate");
    }

    #[test]
    fn agent_id_rejects_garbage_key() {
        assert!(agent_id_for("not-a-key").is_err());
    }

    #[test]
    fn validate_agent_id_rejects_traversal_and_case() {
        assert!(validate_agent_id("../../etc/passwd").is_err());
        assert!(validate_agent_id("").is_err());
        assert!(validate_agent_id("ABCDEF0123456789").is_err());
    }

    #[test]
    fn env_identity_wins_over_user_env() {
        let mut record = fixture_record();
        record
            .agent
            .env_vars
            .insert("BUZZ_PRIVATE_KEY".to_string(), "attacker-key".to_string());
        record.agent.env_vars.insert(
            "buzz_relay_url".to_string(),
            "wss://evil.example".to_string(),
        );
        let env = build_agent_env(&record, "/usr/bin");
        assert_eq!(
            env.get("BUZZ_PRIVATE_KEY"),
            Some(&record.agent.private_key_nsec)
        );
        assert_eq!(env.get("BUZZ_RELAY_URL"), Some(&record.agent.relay_url));
        assert!(!env.contains_key("buzz_relay_url"));
    }

    #[test]
    fn env_carries_user_vars_and_acp_config() {
        let record = fixture_record();
        let env = build_agent_env(&record, "/usr/bin");
        assert_eq!(env.get("ANTHROPIC_API_KEY"), Some(&"sk-test".to_string()));
        assert_eq!(
            env.get("BUZZ_ACP_MODEL"),
            Some(&"claude-opus-4-7".to_string())
        );
        assert_eq!(
            env.get("BUZZ_ACP_SYSTEM_PROMPT"),
            Some(&"Be concise.\nUse the repo conventions.".to_string())
        );
        assert_eq!(
            env.get("BUZZ_ACP_RESPOND_TO"),
            Some(&"owner-only".to_string())
        );
        assert_eq!(env.get("BUZZ_ACP_AGENTS"), Some(&"1".to_string()));
        assert_eq!(
            env.get("BUZZ_ACP_MAX_TURN_DURATION"),
            Some(&"1800".to_string())
        );
        // Deprecated upstream — must NOT be emitted.
        assert!(!env.contains_key("BUZZ_ACP_TURN_TIMEOUT"));
        assert_eq!(
            env.get("BUZZ_ACP_RELAY_OBSERVER"),
            Some(&"true".to_string())
        );
        assert_eq!(
            env.get("BUZZ_ACP_SESSION_TITLE"),
            Some(&"Scout".to_string())
        );
        assert_eq!(
            env.get("PATH"),
            Some(&"/home/user/.local/bin:/usr/bin".to_string())
        );
    }

    #[test]
    fn mcp_sidecar_only_for_buzz_agent() {
        let mut record = fixture_record();
        let env = build_agent_env(&record, "/usr/bin");
        assert_eq!(env.get("BUZZ_ACP_MCP_COMMAND"), Some(&String::new()));

        record.agent.agent_command = "/opt/buzz/bin/buzz-agent".to_string();
        let env = build_agent_env(&record, "/usr/bin");
        assert_eq!(
            env.get("BUZZ_ACP_MCP_COMMAND"),
            Some(&"buzz-dev-mcp".to_string())
        );
    }

    #[test]
    fn relay_http_base_url_maps_schemes() {
        assert_eq!(
            relay_http_base_url("wss://buzz.example.com/"),
            "https://buzz.example.com"
        );
        assert_eq!(
            relay_http_base_url("ws://localhost:3000"),
            "http://localhost:3000"
        );
    }

    #[test]
    fn unit_template_execs_run_agent_instance() {
        let unit = unit_template().expect("template renders");
        assert!(unit.contains("run-agent %i"));
        assert!(unit.contains("Restart=on-failure"));
        assert!(unit.contains("WantedBy=default.target"));
    }
}
