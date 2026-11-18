//! Main-owned portable harness facts and interpretation. Resolvers and processes stay with callers.
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
/// Static runtime metadata and effort contracts.
pub mod runtime_metadata;
#[macro_use]
mod windows_install;
mod catalog;
/// Static preset metadata.
pub mod presets;
pub use catalog::KNOWN_ACP_RUNTIMES;
pub use runtime_metadata::KnownAcpRuntime;
/// Main catalog avatar URL.
pub const GOOSE_AVATAR_URL: &str = "https://goose-docs.ai/img/logo_dark.png";
/// Main catalog avatar URL.
pub const CLAUDE_CODE_AVATAR_URL: &str = "https://anthropic.gallerycdn.vsassets.io/extensions/anthropic/claude-code/2.1.77/1773707456892/Microsoft.VisualStudio.Services.Icons.Default";
/// Main catalog avatar URL.
pub const CODEX_AVATAR_URL: &str = "https://openai.gallerycdn.vsassets.io/extensions/openai/chatgpt/26.5313.41514/1773706730621/Microsoft.VisualStudio.Services.Icons.Default";
/// Main catalog avatar URL.
pub const BUZZ_AGENT_AVATAR_URL: &str =
    "https://raw.githubusercontent.com/block/buzz/refs/heads/main/crates/buzz-agent/buzz-agent.png";
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
/// Main installation classification, shared unchanged with Desktop.
pub enum AcpAvailabilityStatus {
    /// Adapter and required CLI are available.
    Available,
    /// Underlying CLI exists, but the adapter does not.
    AdapterMissing,
    /// Adapter binary is present but unsupported — either the deprecated
    /// package or a version below the supported floor. Reinstall required.
    AdapterOutdated,
    /// Adapter exists, but its required underlying CLI does not.
    CliMissing,
    /// Neither a usable adapter nor underlying CLI is installed.
    NotInstalled,
}

/// Classify the caller-resolved main adapter and underlying CLI.
pub fn classify_runtime(
    adapter_result: Option<(&str, PathBuf)>,
    underlying_cli: Option<&str>,
    underlying_cli_found: bool,
) -> (AcpAvailabilityStatus, Option<String>, Option<String>) {
    if let Some((cmd, path)) = adapter_result {
        if underlying_cli.is_some() && !underlying_cli_found {
            (
                AcpAvailabilityStatus::CliMissing,
                Some(cmd.to_string()),
                Some(path.display().to_string()),
            )
        } else {
            (
                AcpAvailabilityStatus::Available,
                Some(cmd.to_string()),
                Some(path.display().to_string()),
            )
        }
    } else if underlying_cli.is_some() && underlying_cli_found {
        (AcpAvailabilityStatus::AdapterMissing, None, None)
    } else {
        (AcpAvailabilityStatus::NotInstalled, None, None)
    }
}

/// The oldest `codex-acp` version supported by Buzz managed agents.
///
/// Older 1.x adapters are detected successfully, but can still bundle a Codex runtime
/// that cannot use newer models. Adapter 1.6.2 bundles Codex 0.148.x, which rejects
/// GPT-6 Astra even when the separately installed Codex CLI has been updated.
/// Published adapter 1.10.0 depends on `@openai/codex ^0.153.3`.
///
/// Bump policy: raise this only when a newer adapter fixes a defect that breaks managed
/// agents, and only to a version already published on npm — every user below the floor is
/// offered a reinstall on their next discovery pass.
pub const MIN_CODEX_ACP_VERSION: (u64, u64, u64) = (1, 10, 0);

/// Outcome of a CLI login-status probe.
#[derive(Debug, PartialEq, Eq)]
pub enum ProbeOutcome {
    /// The CLI reported a successful login (exit 0).
    LoggedIn,
    /// The CLI exited non-zero without a config-parse signal — treat as
    /// "not authenticated."
    LoggedOut,
    /// The CLI exited non-zero and its stderr contains a config-parse error
    /// (e.g. from `~/.codex/config.toml`). The user needs to fix their
    /// config, not re-run login.
    ConfigInvalid {
        /// A trimmed excerpt of the stderr message to surface in the nudge.
        stderr_excerpt: String,
    },
}

/// Signals emitted to stderr by codex (and related CLI tools) when they
/// fail to parse their config file. We check these to distinguish a
/// config-parse failure from a genuine "not authenticated" exit.
///
/// The real codex error reads:
///   `Error loading configuration: .../.codex/config.toml:... unknown variant ...`
/// So we require BOTH "error loading configuration" AND "unknown variant" to be
/// present, avoiding false positives from unrelated errors that mention only
/// one term.
pub const CONFIG_PARSE_SIGNALS: &[&str] = &["error loading configuration", "unknown variant"];

/// Classify collected probe output into a `ProbeOutcome`.
///
/// Shared between `login_probe` (which has the full `Output`) and the
/// process-level timeout path in `probe_auth_status` (which drains stderr
/// on a background thread and collects it separately).
pub fn classify_probe_output(stderr_bytes: &[u8], exit_success: bool) -> ProbeOutcome {
    if exit_success {
        return ProbeOutcome::LoggedIn;
    }
    let stderr = String::from_utf8_lossy(stderr_bytes);
    let stderr_lower = stderr.to_lowercase();
    if CONFIG_PARSE_SIGNALS
        .iter()
        .all(|sig| stderr_lower.contains(sig))
    {
        let excerpt = stderr.trim().lines().next().unwrap_or("").to_string();
        ProbeOutcome::ConfigInvalid {
            stderr_excerpt: excerpt,
        }
    } else {
        ProbeOutcome::LoggedOut
    }
}

/// Parse the exact release version emitted by the main adapter version probe.
pub fn parse_version(bytes: &[u8]) -> Option<(u64, u64, u64)> {
    let text = String::from_utf8_lossy(bytes);
    let mut parts = text.split_whitespace().last()?.split('.');
    let result = (
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    );
    if parts.next().is_some() {
        return None;
    }
    Some(result)
}
/// Sanitized current readiness, independent of login history and model entitlement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Readiness {
    /// Current native probe succeeded.
    LoggedIn,
    /// Current native probe reported no login.
    LoggedOut,
    /// Main defines no authentication probe; never means authenticated.
    NotApplicable,
    /// No current usable evidence (including timeout and execution failure).
    Unknown,
    /// Native configuration cannot be parsed.
    ConfigInvalid,
    /// Main's required configuration is not supported by this host projection.
    Unsupported,
}
impl Readiness {
    /// Permit an attempt only; models, runtime, policy and authority are separate gates.
    pub fn permits_attempt(self) -> bool {
        matches!(self, Self::LoggedIn | Self::NotApplicable)
    }
}
impl From<ProbeOutcome> for Readiness {
    fn from(value: ProbeOutcome) -> Self {
        match value {
            ProbeOutcome::LoggedIn => Self::LoggedIn,
            ProbeOutcome::LoggedOut => Self::LoggedOut,
            ProbeOutcome::ConfigInvalid { .. } => Self::ConfigInvalid,
        }
    }
}
/// A candidate projected from the main tables, without host or desktop paths.
pub struct Candidate {
    /// Canonical main identity.
    pub id: &'static str,
    /// Adapter search names in preference order.
    pub commands: Vec<&'static str>,
    /// Main launch arguments.
    pub args: &'static [&'static str],
    /// Required underlying CLI, when distinct from the adapter.
    pub underlying_cli: Option<&'static str>,
    /// Main status probe argv, including the executable name.
    pub auth_probe: Option<&'static [&'static str]>,
    /// Optional adapter version floor.
    pub minimum_version: Option<(u64, u64, u64)>,
    /// Requires normalized configuration beyond this finite hosted projection.
    pub requires_config: bool,
}
/// All main compiled candidates; custom/user-defined harnesses stay unsupported.
pub fn candidates() -> Vec<Candidate> {
    KNOWN_ACP_RUNTIMES
        .iter()
        .map(|r| Candidate {
            id: r.id,
            commands: r.commands.to_vec(),
            args: default_args(r.id).unwrap_or(&[]),
            underlying_cli: r.underlying_cli,
            auth_probe: r.auth_probe_args,
            minimum_version: (r.id == "codex").then_some(MIN_CODEX_ACP_VERSION),
            requires_config: !r.required_normalized_fields.is_empty(),
        })
        .chain(presets::PRESET_HARNESSES.iter().map(|r| Candidate {
            id: r.id,
            commands: vec![r.command],
            args: r.args,
            underlying_cli: r.underlying_cli,
            auth_probe: None,
            minimum_version: None,
            requires_config: false,
        }))
        .collect()
}
/// Canonical builtin defaults used by desktop argument normalization and host launch.
pub fn default_args(identity: &str) -> Option<&'static [&'static str]> {
    KNOWN_ACP_RUNTIMES
        .iter()
        .find(|r| {
            r.id == identity || r.commands.contains(&identity) || r.aliases.contains(&identity)
        })
        .map(|r| {
            if r.id == "goose" {
                &["acp"][..]
            } else {
                &[][..]
            }
        })
}

/// Bounded inventory facts. No filesystem paths, account details or provider output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeFacts {
    /// Main adapter/underlying CLI classification.
    pub availability: AcpAvailabilityStatus,
    /// Parsed release version, only when main requires a version check.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Whether the hosted configuration projection supports this definition.
    pub config_supported: bool,
}
impl RuntimeFacts {
    /// Validate the complete bounded projection, without truncation.
    pub fn valid(&self) -> bool {
        self.version
            .as_ref()
            .is_none_or(|v| v.len() <= 64 && parse_version(v.as_bytes()).is_some())
    }
}

/// Apply a main-owned adapter version floor, treating unknown versions as unavailable.
pub fn version_availability(
    version: Option<(u64, u64, u64)>,
    minimum: (u64, u64, u64),
) -> AcpAvailabilityStatus {
    if version.is_some_and(|v| v >= minimum) {
        AcpAvailabilityStatus::Available
    } else {
        AcpAvailabilityStatus::AdapterOutdated
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn main_registry_projection_preserves_candidates_and_defaults() {
        let candidates = candidates();
        let mut ids = std::collections::BTreeSet::new();
        for c in &candidates {
            assert!(ids.insert(c.id));
        }
        let claude = candidates.iter().find(|c| c.id == "claude").unwrap();
        assert_eq!(claude.commands, ["claude-agent-acp", "claude-code-acp"]);
        assert_eq!(claude.auth_probe, Some(&["claude", "auth", "status"][..]));
        let grok = candidates.iter().find(|c| c.id == "grok").unwrap();
        assert_eq!(grok.args, ["agent", "--always-approve", "stdio"]);
        assert!(grok.auth_probe.is_none());
        assert!(Readiness::NotApplicable.permits_attempt());
        assert_ne!(Readiness::NotApplicable, Readiness::LoggedIn);
        assert_eq!(default_args("goose"), Some(&["acp"][..]));
        assert_eq!(default_args("claude-code"), Some(&[][..]));
        assert_eq!(default_args("custom"), None);
        assert!(
            candidates
                .iter()
                .find(|c| c.id == "goose")
                .unwrap()
                .requires_config
        );
    }
    #[test]
    fn shared_availability_version_and_auth_classifiers() {
        let adapter = Some(("adapter", PathBuf::from("/fake/adapter")));
        assert_eq!(
            classify_runtime(adapter.clone(), Some("cli"), false).0,
            AcpAvailabilityStatus::CliMissing
        );
        assert_eq!(
            classify_runtime(None, Some("cli"), true).0,
            AcpAvailabilityStatus::AdapterMissing
        );
        assert_eq!(
            classify_runtime(None, Some("cli"), false).0,
            AcpAvailabilityStatus::NotInstalled
        );
        assert_eq!(
            classify_runtime(adapter, Some("cli"), true).0,
            AcpAvailabilityStatus::Available
        );
        for input in ["codex-acp 1.2", "codex-acp 1.10.0-rc1", "invalid"] {
            assert!(parse_version(input.as_bytes()).is_none());
        }
        assert_eq!(
            version_availability(parse_version(b"codex-acp 1.10.0"), MIN_CODEX_ACP_VERSION),
            AcpAvailabilityStatus::Available
        );
        assert_eq!(
            version_availability(parse_version(b"codex-acp 1.6.2"), MIN_CODEX_ACP_VERSION),
            AcpAvailabilityStatus::AdapterOutdated
        );
        assert_eq!(
            Readiness::from(classify_probe_output(b"not logged in", true)),
            Readiness::LoggedIn
        );
        assert_eq!(
            Readiness::from(classify_probe_output(b"not logged in", false)),
            Readiness::LoggedOut
        );
        assert_eq!(
            Readiness::from(classify_probe_output(
                b"Error loading configuration: unknown variant SECRET",
                false
            )),
            Readiness::ConfigInvalid
        );
        assert!(!Readiness::Unknown.permits_attempt());
    }
}

/// Find a main builtin by its canonical identity, independent of the current machine.
pub fn known_acp_runtime_exact(id: &str) -> Option<&'static KnownAcpRuntime> {
    KNOWN_ACP_RUNTIMES.iter().find(|runtime| runtime.id == id)
}
