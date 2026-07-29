//! Wire types for the desktop ↔ provider ↔ remote-helper protocol.
//!
//! The desktop side of this contract lives in
//! `desktop/src-tauri/src/managed_agents/backend.rs` (invocation, response
//! handling, `provider_config` validation) and
//! `desktop/src-tauri/src/commands/agents_deploy.rs` (`deploy_payload_json`,
//! the authoritative payload shape). Field additions there must stay
//! deserializable here; unknown fields are ignored by design.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// A provider request as written by the desktop to our stdin.
///
/// `op` is `"info"` or `"deploy"`. The `agent` payload and `provider_config`
/// are present for `deploy` only.
// Some fields are only read by the unix-only remote module.
#[cfg_attr(not(unix), allow(dead_code))]
#[derive(Debug, Deserialize)]
pub struct ProviderRequest {
    pub op: String,
    /// Correlation id for logging only — the stdin→stdout exchange is 1:1.
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(default)]
    pub agent: Option<AgentPayload>,
    #[serde(default)]
    pub provider_config: Option<serde_json::Value>,
}

/// The agent deploy payload, mirroring `deploy_payload_json` in the desktop.
///
/// Numeric fields are tolerant (`u64`) and optional so payload evolution on
/// the desktop side does not break older helpers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPayload {
    pub name: String,
    pub relay_url: String,
    pub private_key_nsec: String,
    #[serde(default)]
    pub auth_tag: Option<String>,
    pub agent_command: String,
    #[serde(default)]
    pub agent_args: Vec<String>,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    /// LLM inference provider id (e.g. "anthropic"). Harness-specific env
    /// mapping for this is a desktop concern; we surface it to the agent via
    /// user `env_vars` only.
    #[serde(default)]
    pub provider: Option<String>,
    /// Deprecated upstream (`BUZZ_ACP_TURN_TIMEOUT`); accepted but not applied.
    #[serde(default)]
    pub turn_timeout_seconds: Option<u64>,
    #[serde(default)]
    pub idle_timeout_seconds: Option<u64>,
    #[serde(default)]
    pub max_turn_duration_seconds: Option<u64>,
    #[serde(default)]
    pub parallelism: Option<u64>,
    /// Kebab-case wire string: `owner-only` | `allowlist` | `anyone`.
    #[serde(default)]
    pub respond_to: Option<String>,
    #[serde(default)]
    pub respond_to_allowlist: Vec<String>,
    #[serde(default)]
    pub env_vars: BTreeMap<String, String>,
}

/// Provider configuration collected by the desktop from the schema returned
/// by our `info` op. All values are non-secret scalars by desktop-enforced
/// policy (`validate_provider_config`), so ssh authentication must come from
/// the ambient ssh agent / config, never from these fields.
// `acp_path`/`remote_path_prepend` are only read by the unix-only remote module.
#[cfg_attr(not(unix), allow(dead_code))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// SSH destination: a `~/.ssh/config` alias or `user@host`.
    #[serde(default)]
    pub host: String,
    /// Extra `ssh` options, whitespace-separated (e.g. `-p 2222`).
    #[serde(default)]
    pub ssh_args: String,
    /// Path to this binary on the remote host.
    #[serde(default = "default_remote_command")]
    pub remote_command: String,
    /// Path to `buzz-acp` on the remote host ("buzz-acp" resolves via PATH).
    #[serde(default = "default_acp_path")]
    pub acp_path: String,
    /// Colon-separated directories prepended to PATH for the agent process
    /// (harness binaries, node, etc.).
    #[serde(default)]
    pub remote_path_prepend: String,
}

fn default_remote_command() -> String {
    "buzz-backend-ssh".to_string()
}

fn default_acp_path() -> String {
    "buzz-acp".to_string()
}

/// Must agree with the `serde(default)` attributes above — `from_value`
/// falls back to this when `provider_config` is absent entirely.
impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            host: String::new(),
            ssh_args: String::new(),
            remote_command: default_remote_command(),
            acp_path: default_acp_path(),
            remote_path_prepend: String::new(),
        }
    }
}

impl ProviderConfig {
    /// Parse from the request's `provider_config` value, tolerating absent
    /// or non-object values (all fields then take their defaults).
    pub fn from_value(value: Option<&serde_json::Value>) -> Self {
        value
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default()
    }

    /// Fill empty fields from operator defaults (see [`load_defaults_file`]).
    ///
    /// Deploy-side counterpart of the schema defaults: the current desktop
    /// re-probes on every keystroke and resets typed config to schema
    /// defaults, so a deploy can arrive with fields blanked. Empty means
    /// "not set" for every field here, so falling back is always safe.
    pub fn merge_defaults(mut self, defaults: &BTreeMap<String, String>) -> Self {
        let fill = |current: &mut String, name: &str| {
            if current.trim().is_empty() {
                if let Some(value) = defaults.get(name) {
                    *current = value.clone();
                }
            }
        };
        fill(&mut self.host, "host");
        fill(&mut self.ssh_args, "ssh_args");
        fill(&mut self.remote_command, "remote_command");
        fill(&mut self.acp_path, "acp_path");
        fill(&mut self.remote_path_prepend, "remote_path_prepend");
        self
    }
}

/// Load operator defaults for this machine: a flat JSON object of
/// `field -> value` stored next to the provider binary as
/// `<binary-name>.defaults.json` (e.g. `buzz-backend-agentbox.defaults.json`).
///
/// Renamed/symlinked binaries each get their own defaults file, so one
/// installed provider binary can serve several named backends with different
/// hosts. Missing or malformed files simply mean no defaults.
pub fn load_defaults_file() -> BTreeMap<String, String> {
    let Ok(exe) = std::env::current_exe() else {
        return BTreeMap::new();
    };
    let mut path = exe.into_os_string();
    path.push(".defaults.json");
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return BTreeMap::new();
    };
    match serde_json::from_str::<BTreeMap<String, String>>(&raw) {
        Ok(defaults) => defaults,
        Err(e) => {
            eprintln!(
                "buzz-backend-ssh: ignoring malformed defaults file {}: {e}",
                std::path::Path::new(&path).display()
            );
            BTreeMap::new()
        }
    }
}

/// The `info` response: identity plus the JSON schema the desktop renders as
/// provider configuration fields. Field names deliberately avoid the
/// desktop's secret-like-key denylist (`secret|password|token|key|credential`).
///
/// `defaults` (from [`load_defaults_file`]) become the schema `default`
/// values, which the desktop pre-fills into the form.
pub fn info_response(defaults: &BTreeMap<String, String>) -> serde_json::Value {
    let d = |name: &str, fallback: &str| -> String {
        defaults
            .get(name)
            .cloned()
            .unwrap_or_else(|| fallback.to_string())
    };
    serde_json::json!({
        "ok": true,
        "name": "buzz-backend-ssh",
        "version": env!("CARGO_PKG_VERSION"),
        "config_schema": {
            "type": "object",
            "required": ["host"],
            "properties": {
                "host": {
                    "type": "string",
                    "title": "SSH host",
                    "description": "SSH destination: an alias from your ssh config, or user@host. Authentication uses your ssh agent/config (BatchMode).",
                    "default": d("host", ""),
                },
                "ssh_args": {
                    "type": "string",
                    "title": "Extra ssh options",
                    "description": "Optional whitespace-separated ssh options, e.g. -p 2222",
                    "default": d("ssh_args", ""),
                },
                "remote_command": {
                    "type": "string",
                    "title": "Remote helper",
                    "description": "Path to buzz-backend-ssh on the remote host",
                    "default": d("remote_command", "buzz-backend-ssh"),
                },
                "acp_path": {
                    "type": "string",
                    "title": "buzz-acp path",
                    "description": "Path to buzz-acp on the remote host",
                    "default": d("acp_path", "buzz-acp"),
                },
                "remote_path_prepend": {
                    "type": "string",
                    "title": "Remote PATH prepend",
                    "description": "Colon-separated dirs prepended to PATH for the agent (harness binaries, node, ...)",
                    "default": d("remote_path_prepend", ""),
                },
            },
        },
    })
}

/// Serialize an error as the protocol's failure shape and print it on stdout.
///
/// The desktop treats `ok: false` as failure and surfaces `error` to the
/// user, so the message must be actionable and must never echo secrets.
pub fn respond_err(message: &str) {
    let body = serde_json::json!({ "ok": false, "error": message });
    println!("{body}");
}

/// Print a success response on stdout.
pub fn respond_ok(mut body: serde_json::Value) {
    if let Some(obj) = body.as_object_mut() {
        obj.insert("ok".to_string(), serde_json::Value::Bool(true));
    }
    println!("{body}");
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Fixture mirroring `deploy_payload_json` in
    /// `desktop/src-tauri/src/commands/agents_deploy.rs`.
    pub(crate) fn deploy_request_fixture() -> serde_json::Value {
        serde_json::json!({
            "op": "deploy",
            "request_id": "11111111-2222-3333-4444-555555555555",
            "agent": {
                "name": "Scout",
                "relay_url": "wss://relay.example.com",
                // Throwaway test vector key, not a real identity.
                "private_key_nsec": "5ee1c8000ab28edd64d74a7d951ac2dd559814887b1b9e1ac7c5f89e96125c12",
                "auth_tag": null,
                "agent_command": "claude-agent-acp",
                "agent_args": [],
                "system_prompt": "Be concise.\nUse the repo conventions.",
                "model": "claude-opus-4-7",
                "provider": "anthropic",
                "turn_timeout_seconds": 320,
                "idle_timeout_seconds": null,
                "max_turn_duration_seconds": 1800,
                "parallelism": 1,
                "respond_to": "owner-only",
                "respond_to_allowlist": [],
                "env_vars": { "ANTHROPIC_API_KEY": "sk-test", "FOO": "bar" },
            },
            "provider_config": {
                "host": "agentbox",
                "acp_path": "/home/user/buzz/target/release/buzz-acp",
                "remote_path_prepend": "/home/user/.local/bin",
            },
        })
    }

    #[test]
    fn deploy_request_parses() {
        let req: ProviderRequest =
            serde_json::from_value(deploy_request_fixture()).expect("fixture parses");
        assert_eq!(req.op, "deploy");
        let agent = req.agent.expect("agent present");
        assert_eq!(agent.name, "Scout");
        assert_eq!(agent.respond_to.as_deref(), Some("owner-only"));
        assert_eq!(agent.parallelism, Some(1));
        assert_eq!(agent.env_vars.len(), 2);
        let cfg = ProviderConfig::from_value(req.provider_config.as_ref());
        assert_eq!(cfg.host, "agentbox");
        assert_eq!(cfg.remote_command, "buzz-backend-ssh");
        assert_eq!(cfg.acp_path, "/home/user/buzz/target/release/buzz-acp");
    }

    #[test]
    fn unknown_payload_fields_are_tolerated() {
        let mut fixture = deploy_request_fixture();
        fixture["agent"]["future_field"] = serde_json::json!({ "nested": true });
        let req: ProviderRequest = serde_json::from_value(fixture).expect("still parses");
        assert!(req.agent.is_some());
    }

    #[test]
    fn info_schema_has_no_secret_like_keys() {
        // Mirrors the word-segment check in the desktop's
        // `validate_provider_config` — a schema key that trips it would make
        // every deploy fail after the user fills the form.
        let info = info_response(&BTreeMap::new());
        let props = info["config_schema"]["properties"]
            .as_object()
            .expect("properties object");
        let forbidden = ["secret", "password", "token", "key", "credential"];
        for key in props.keys() {
            for word in key.split(['_', '-', '.']) {
                assert!(
                    !forbidden.contains(&word.to_ascii_lowercase().as_str()),
                    "schema key '{key}' contains forbidden word '{word}'"
                );
            }
        }
    }

    #[test]
    fn provider_config_defaults_apply_when_absent() {
        let cfg = ProviderConfig::from_value(None);
        assert_eq!(cfg.host, "");
        assert_eq!(cfg.remote_command, "buzz-backend-ssh");
        assert_eq!(cfg.acp_path, "buzz-acp");
    }

    #[test]
    fn merge_defaults_fills_only_empty_fields() {
        let mut defaults = BTreeMap::new();
        defaults.insert("host".to_string(), "user@backend".to_string());
        defaults.insert("acp_path".to_string(), "/opt/buzz/buzz-acp".to_string());

        let typed: ProviderConfig =
            serde_json::from_value(serde_json::json!({ "host": "user@other", "acp_path": "" }))
                .expect("valid config");
        let merged = typed.merge_defaults(&defaults);
        // Typed value wins; empty falls back to the operator default.
        assert_eq!(merged.host, "user@other");
        assert_eq!(merged.acp_path, "/opt/buzz/buzz-acp");

        // A fully blanked config (the desktop's re-probe reset) recovers.
        let blanked: ProviderConfig =
            serde_json::from_value(serde_json::json!({ "host": "" })).expect("valid config");
        let merged = blanked.merge_defaults(&defaults);
        assert_eq!(merged.host, "user@backend");
    }

    #[test]
    fn info_response_surfaces_operator_defaults() {
        let mut defaults = BTreeMap::new();
        defaults.insert("host".to_string(), "user@backend".to_string());
        let info = info_response(&defaults);
        let props = &info["config_schema"]["properties"];
        assert_eq!(props["host"]["default"], "user@backend");
        // Untouched fields keep their built-in defaults.
        assert_eq!(props["remote_command"]["default"], "buzz-backend-ssh");
    }
}
