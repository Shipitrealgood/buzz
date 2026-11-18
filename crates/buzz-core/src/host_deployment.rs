//! Inert registered-host deployment contract. No transport admission or execution
//! is enabled by these types. Registration is never deployment authority.
use crate::private_managed_agent::{validate_auth_tag, PrivateIdentity};
use crate::{
    host_registry::{identifier, origin},
    paired_message::{display_name, runtime_id, Policy, Scope},
};
use nostr::{Keys, PublicKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Maximum encoded encrypted command, before transport framing.
pub const MAX_CIPHERTEXT: usize = 48 * 1024;
/// Contract validation failure; never includes private input.
#[derive(Debug, thiserror::Error)]
#[error("invalid or unsupported host deployment contract")]
pub struct Error;
type Result<T> = std::result::Result<T, Error>;

/// Create-only destination; deliberately separate from persisted agent backends.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Destination {
    /// Exact private registration binding selected by the owner.
    RegisteredHost {
        /// Existing owner/community/host/enrollment scope.
        scope: Scope,
    },
}

/// Validate the existing registration identity without treating it as authority.
pub fn validate_scope(scope: &Scope) -> Result<()> {
    if !origin(&scope.community)
        || !identifier(&scope.generation)
        || scope.owner == scope.host
        || scope.owner.xonly().is_err()
        || scope.host.xonly().is_err()
    {
        return Err(Error);
    }
    Ok(())
}

/// Installed runtime identifiers only; capability facts remain in the shared
/// Rust runtime metadata. Models require this new agent's native authentication.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Capability {
    /// Registration protocol provides no deployment evidence.
    RegistrationOnly {},
    /// An explicit deployment response reports no protocol support.
    Unsupported {},
    /// Approved hostd catalog hash and installed runtime IDs, possibly empty.
    Supported {
        /// SHA-256 of the actual approved hostd catalog bytes.
        catalog_revision: String,
        /// Unique hostd catalog runtime IDs; no commands, paths or credentials.
        runtimes: Vec<String>,
        /// Approved installed runtime to canonical harness mapping; absent means unsupported.
        #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
        harnesses: std::collections::BTreeMap<String, String>,
    },
}
impl Capability {
    /// Check the bounded catalog projection, preserving supported-but-empty.
    pub fn validate(&self) -> Result<()> {
        if let Self::Supported {
            catalog_revision,
            runtimes,
            harnesses,
        } = self
        {
            if !identifier(catalog_revision)
                || runtimes.len() > 64
                || runtimes.iter().any(|id| !runtime_id(id))
                || runtimes.windows(2).any(|w| w[0] >= w[1])
                || harnesses
                    .iter()
                    .any(|(id, harness)| !runtimes.contains(id) || !harness_id(harness))
            {
                return Err(Error);
            }
        }
        Ok(())
    }
}

/// Canonical main harness key, distinct from the installed runtime's namespaced ID.
pub fn harness_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"-_".contains(&b))
}

/// Freshness/authority of a private status observation, never relay presence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Availability {
    /// No observation; does not mean offline.
    Unknown,
    /// Current authenticated observation.
    Current,
    /// Observation expired.
    Stale,
    /// Query/authority service unavailable; does not mean offline.
    Unavailable,
    /// Enrollment was revoked.
    Revoked,
}

/// Private status snapshot. Slice 2 must authenticate and scope it before use.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Status {
    /// Existing registration identity.
    pub scope: Scope,
    /// Observation validity, independent of capability.
    pub availability: Availability,
    /// Explicit supported protocol/catalog evidence.
    pub capability: Capability,
    /// Main candidate facts from this host, absent on older hosts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inventory: Option<std::collections::BTreeMap<String, crate::harness::RuntimeFacts>>,
    /// Exclusive freshness deadline of this observation.
    pub expires_at: u64,
}
impl Status {
    /// Validate bounded facts without accepting paths, output or unknown runtime IDs.
    pub fn validate_inventory(&self) -> Result<()> {
        if let Some(inventory) = &self.inventory {
            let Capability::Supported { runtimes, .. } = &self.capability else {
                return Err(Error);
            };
            if inventory.len() > 64
                || inventory
                    .iter()
                    .any(|(id, fact)| !runtimes.contains(id) || !fact.valid())
            {
                return Err(Error);
            }
        }
        Ok(())
    }
    /// Validate current scope and approved catalog for a prospective deployment.
    /// This is data validation, not a signature/membership/revocation check.
    pub fn permits(&self, scope: &Scope, catalog: &str, runtime: &str, now: u64) -> Result<()> {
        validate_scope(scope)?;
        self.capability.validate()?;
        self.validate_inventory()?;
        if self.scope != *scope
            || self.availability != Availability::Current
            || now >= self.expires_at
        {
            return Err(Error);
        }
        match &self.capability {
            Capability::Supported {
                catalog_revision,
                runtimes,
                ..
            } if catalog_revision == catalog && runtimes.iter().any(|id| id == runtime) => Ok(()),
            _ => Err(Error),
        }
    }
}

/// The only execution actions. Resume is another deploy of the same identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Action {
    /// Create/update an assigned identity at its existing placement.
    Deploy,
    /// Fence activation and durably reconcile stop/disable/removal.
    Cleanup,
}

/// Fields that slice 2's signed envelope must bind byte-for-byte. This struct
/// alone carries no signature and is never sufficient execution authority.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Authority {
    /// Explicit deployment contract version, separate from registration v2.
    pub v: u8,
    /// Existing registration identity.
    pub scope: Scope,
    /// New agent public identity; never the owner's account key.
    pub agent: PublicKey,
    /// Allowed action.
    pub action: Action,
    /// Existing 32-byte operation/request identity convention.
    pub operation_id: String,
    /// Compare-and-set placement revision; zero means absent placement.
    pub expected_revision: u64,
    /// Exclusive command expiry.
    pub expires_at: u64,
    /// SHA-256 of the exact persisted encrypted content bytes.
    pub content_sha256: String,
}
impl Authority {
    /// Validate against trusted scope/revision and exact encrypted bytes.
    /// Membership, signature, revocation and placement ownership remain caller gates.
    pub fn validate(
        &self,
        scope: &Scope,
        revision: u64,
        now: u64,
        ciphertext: &[u8],
    ) -> Result<()> {
        validate_scope(scope)?;
        if self.v != 1
            || self.scope != *scope
            || !identifier(&self.operation_id)
            || self.agent.xonly().is_err()
            || self.agent == scope.owner
            || self.agent == scope.host
            || self.expected_revision != revision
            || revision == u64::MAX
            || now >= self.expires_at
            || self.expires_at.saturating_sub(now) > 600
            || ciphertext.is_empty()
            || ciphertext.len() > MAX_CIPHERTEXT
            || self.content_sha256 != hex::encode(Sha256::digest(ciphertext))
        {
            return Err(Error);
        }
        Ok(())
    }
}

/// Model resolution must not borrow host/operator/sibling provider sessions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ModelSelection {
    /// No authenticated new-agent model catalog yet; agent is nonlaunchable.
    PendingLogin {},
    /// Explicit selection from the eventual authenticated native catalog.
    Selected {
        /// Native model identifier.
        id: String,
    },
}

/// Structured projection of main's resolved agent configuration, inside the
/// encrypted payload. Hostd maps the approved runtime ID to its own executable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Deploy {
    /// Approved hostd catalog byte hash, not a second catalog version scheme.
    pub catalog_revision: String,
    /// Hostd runtime key resolved from main's runtime metadata.
    pub runtime_id: String,
    /// Shared visible display name.
    pub name: String,
    /// Shared resolved response and membership policy.
    pub policy: Policy,
    /// Resolved model or explicit nonlaunchable state.
    pub model: ModelSelection,
    /// Main's resolved structured provider selection, never credentials.
    pub provider: Option<String>,
    /// Main's resolved system prompt.
    pub system_prompt: Option<String>,
    /// Main's resolved startup effort selection.
    pub effort: Option<String>,
    /// Main's resolved bounded concurrency.
    pub parallelism: u32,
}
impl Deploy {
    /// Check bounded structured input. No caller command/env/path overrides exist.
    pub fn validate(&self) -> Result<()> {
        let bounded = |s: &str| !s.is_empty() && s.len() <= 256 && !s.chars().any(char::is_control);
        if !identifier(&self.catalog_revision)
            || !runtime_id(&self.runtime_id)
            || !display_name(&self.name)
            || !self.policy.valid()
            || !(1..=32).contains(&self.parallelism)
            || self.provider.as_deref().is_some_and(|s| !bounded(s))
            || self.effort.as_deref().is_some_and(|s| !bounded(s))
            || self
                .system_prompt
                .as_ref()
                .is_some_and(|s| s.len() > 16 * 1024)
            || matches!(&self.model, ModelSelection::Selected { id } if !bounded(id))
        {
            return Err(Error);
        }
        Ok(())
    }
}

/// Decrypted native-only command. Cleanup retains fenced working data;
/// authentication/session operations and caller executable overrides are absent.
#[derive(PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Command {
    /// Create/deploy the same imported identity and resolved main configuration.
    Deploy {
        /// Structured resolved configuration.
        config: Box<Deploy>,
        /// New agent key; encrypt before journal/transport serialization.
        identity: PrivateIdentity,
    },
    /// Fence before stopping; retain data until an explicitly approved teardown.
    Cleanup {},
}
impl Command {
    /// Check inner/outer action and identity agreement. This does not authenticate
    /// a request or replace the fresh authority and capability checks.
    pub fn validate(&self, authority: &Authority) -> Result<()> {
        match (self, authority.action) {
            (Self::Deploy { config, identity }, Action::Deploy) => {
                config.validate()?;
                if identity.private_key_nsec.len() > 256 {
                    return Err(Error);
                }
                let key = Keys::parse(&identity.private_key_nsec).map_err(|_| Error)?;
                if key.public_key() != authority.agent
                    || authority.agent == authority.scope.owner
                    || authority.agent == authority.scope.host
                {
                    return Err(Error);
                }
                validate_auth_tag(
                    identity.auth_tag.as_deref().ok_or(Error)?,
                    &authority.scope.owner.to_hex(),
                    &authority.agent,
                )
                .map_err(|_| Error)
            }
            (Self::Cleanup {}, Action::Cleanup) => Ok(()),
            _ => Err(Error),
        }
    }
}

/// Durable outcome; presence is not a deployment result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Outcome {
    /// Durable accepted work; not observed success.
    Pending,
    /// Provisioned but model/auth pending; must not launch conversation.
    NeedsLogin,
    /// Deployment revision observed applied.
    Applied,
    /// Cleanup fence persisted; observed completion still pending.
    CleanupPending,
    /// Stop/disable/removal observed and resurrection tombstone retained.
    Cleaned,
    /// Failed work remains durable and retryable at the same operation ID.
    Failed,
}

/// Correlated private result, signed by the host in the future transport.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeploymentResult {
    /// Exact request commitment, including action/scope/identity/revision.
    pub request: Authority,
    /// Durable result state.
    pub outcome: Outcome,
    /// Applied placement revision, absent until a revision is observed applied.
    pub applied_revision: Option<u64>,
}
impl DeploymentResult {
    /// Reject a cross-operation result or impossible action/revision outcome.
    pub fn validate(&self, request: &Authority) -> Result<()> {
        let applied = matches!(
            self.outcome,
            Outcome::NeedsLogin | Outcome::Applied | Outcome::Cleaned
        );
        if request.expected_revision == u64::MAX
            || self.request != *request
            || (applied && self.applied_revision != request.expected_revision.checked_add(1))
            || (!applied && self.applied_revision.is_some())
            || matches!(
                (&request.action, &self.outcome),
                (Action::Deploy, Outcome::CleanupPending | Outcome::Cleaned)
                    | (Action::Cleanup, Outcome::NeedsLogin | Outcome::Applied)
            )
        {
            return Err(Error);
        }
        Ok(())
    }
}

/// Native-only durable intent representation. No plaintext identity key is
/// serializable here. Slice 2 must atomically persist this inside the protected
/// device-local journal BEFORE profile/channel publication or dispatch.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProtectedCreateIntent {
    /// Destination, agent and immutable command commitment.
    pub authority: Authority,
    /// Exact NIP-44 encrypted Command bytes, reused without re-encryption on retry.
    /// Recovery requires the original native owner signer and selected host key;
    /// the new agent key is inside this ciphertext, never a second plaintext copy.
    pub ciphertext: Vec<u8>,
}
impl ProtectedCreateIntent {
    /// Validate shape and commitment; does not claim envelope cryptographic validity.
    pub fn validate(&self, scope: &Scope, now: u64) -> Result<()> {
        self.authority.validate(scope, 0, now, &self.ciphertext)?;
        if self.authority.action != Action::Deploy {
            return Err(Error);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;

/// Authenticated private relay transport, separate from registration v2.
pub mod wire;

#[cfg(test)]
mod wire_tests;

/// Private per-operation discovery; never installation or entitlement evidence.
pub mod observation;
