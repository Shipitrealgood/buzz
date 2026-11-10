//! Bounded, immutable observations carried only inside encrypted operation results.
use super::*;
use crate::agent_models::AgentModelsResponse;

/// Outcome of the fixed native login command, not subscription entitlement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LoginState {
    /// No native login command was observed.
    NotObserved,
    /// The finite command is running.
    InProgress,
    /// Native command exited successfully; chat/refresh remain unproven.
    Succeeded,
    /// Operator cancellation or action expiry.
    Cancelled,
    /// Native command failed.
    Failed,
}
/// Sanitized failure categories; raw provider output never crosses this boundary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UnavailableReason {
    /// Installed policy intentionally disables provider networking.
    NetworkDisabled,
    /// The complete catalog cannot fit the bounded projection.
    Oversized,
    /// Native process or ACP discovery failed.
    Execution,
    /// Finite action exceeded its deadline.
    Timeout,
    /// Native output is not a valid model projection.
    InvalidOutput,
}
/// Model enumeration is independent of native authentication outcome.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Models {
    /// Complete bounded catalog, with unchanged native identifiers.
    Available {
        /// Shared main projection; no raw ACP objects.
        catalog: AgentModelsResponse,
    },
    /// Successful enumeration with no models.
    Empty,
    /// Enumeration could not establish a complete catalog.
    Unavailable {
        /// Fixed, non-sensitive reason.
        reason: UnavailableReason,
    },
    /// No supported metadata mapping for this installed runtime.
    Unsupported,
}
/// Exact context and freshness of one finite discovery action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    /// Existing enrollment and owner scope.
    pub scope: Scope,
    /// Agent whose private HOME supplied the observation.
    pub agent: PublicKey,
    /// Installed runtime identifier.
    pub runtime_id: String,
    /// Hash of the approved catalog bytes.
    pub catalog_revision: String,
    /// Exact preparation/refresh operation.
    pub operation_id: String,
    /// Exact applied placement revision.
    pub revision: u64,
    /// Fresh finite-action correlation nonce.
    pub action_id: String,
    /// Host observation time.
    pub observed_at: u64,
    /// Exclusive freshness deadline, at most 120 seconds later.
    pub expires_at: u64,
    /// Independently observed native command outcome.
    pub login: LoginState,
    /// Native login outcome timestamp, absent only when not observed.
    pub login_at: Option<u64>,
    /// Independent complete model observation.
    pub models: Models,
}
fn text(s: &str, max: usize) -> bool {
    !s.is_empty() && s.len() <= max && !s.chars().any(char::is_control)
}
/// Bound the shared projection without truncating or changing model identifiers.
pub fn validate_models(c: &AgentModelsResponse) -> Result<()> {
    let mut ids = std::collections::BTreeSet::new();
    if c.models.len() > 64
        || !text(&c.agent_name, 128)
        || !text(&c.agent_version, 128)
        || c.models.iter().any(|m| {
            !text(&m.id, 256)
                || !ids.insert(&m.id)
                || m.name.as_deref().is_some_and(|s| !text(s, 128))
                || m.description.as_deref().is_some_and(|s| !text(s, 256))
        })
        || c.agent_default_model
            .as_deref()
            .is_some_and(|s| !text(s, 256))
        || c.selected_model.as_deref().is_some_and(|s| !text(s, 256))
        || c.supports_switching != !c.models.is_empty()
    {
        return Err(Error);
    }
    Ok(())
}
impl Observation {
    /// Validate historical binding. Freshness is checked separately at consumption.
    pub fn validate(&self, r: &DeploymentResult) -> Result<()> {
        r.validate(&r.request)?;
        if self.scope != r.request.scope
            || self.agent != r.request.agent
            || self.operation_id != r.request.operation_id
            || Some(self.revision) != r.applied_revision
            || r.outcome != Outcome::NeedsLogin
            || !runtime_id(&self.runtime_id)
            || !identifier(&self.catalog_revision)
            || !identifier(&self.action_id)
            || self.expires_at <= self.observed_at
            || self.expires_at.saturating_sub(self.observed_at) > 120
            || (self.login == LoginState::NotObserved) != self.login_at.is_none()
            || self.login_at.is_some_and(|t| t > self.observed_at)
            || serde_json::to_vec(self).map_err(|_| Error)?.len() > 2048
        {
            return Err(Error);
        }
        if let Models::Available { catalog } = &self.models {
            validate_models(catalog)?;
            if catalog.models.is_empty() {
                return Err(Error);
            }
        }
        Ok(())
    }
    /// Require fresh evidence for this exact saved runtime/catalog selection.
    pub fn current(&self, r: &DeploymentResult, config: &Deploy, now: u64) -> Result<()> {
        self.validate(r)?;
        if self.runtime_id != config.runtime_id
            || self.catalog_revision != config.catalog_revision
            || now < self.observed_at
            || now >= self.expires_at
        {
            return Err(Error);
        }
        Ok(())
    }
}
/// Encrypted result body. The signed public header remains result-only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultBody {
    /// Must exactly match the signed result header.
    pub result: DeploymentResult,
    /// Optional immutable observation; never placed in tags.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observation: Option<Observation>,
}
impl ResultBody {
    /// Validate exact result and optional observation binding.
    pub fn validate(&self, request: &Authority) -> Result<()> {
        self.result.validate(request)?;
        if let Some(o) = &self.observation {
            o.validate(&self.result)?;
        }
        Ok(())
    }
}
