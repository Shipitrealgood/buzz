//! Main-owned portable harness facts and interpretation. Resolvers and processes stay with callers.
use serde::{Deserialize, Serialize};
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
