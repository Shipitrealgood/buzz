//! Types shared by the isolated paired host, its narrowly scoped installer and agent launcher.
#![forbid(unsafe_code)]
use anyhow::{ensure, Result};
use buzz_core::paired_message::{display_name, runtime_id, Policy, Scope};
use nostr::PublicKey;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Executable {
    pub path: PathBuf,
    pub sha256: String,
}

impl Executable {
    pub fn verify(&self) -> Result<()> {
        let path = &self.path;
        // All path components must be root-owned and non-writable to agents.
        ensure!(
            path.is_absolute() && path.canonicalize()? == *path,
            "noncanonical catalog program"
        );
        for p in path.ancestors() {
            let m = fs::symlink_metadata(p)?;
            ensure!(
                m.uid() == 0 && m.mode() & 0o022 == 0,
                "unsafe catalog program ownership"
            );
        }
        let mut f = fs::File::open(path)?;
        let m = f.metadata()?;
        ensure!(
            m.is_file() && m.mode() & 0o111 != 0 && m.mode() & 0o6000 == 0,
            "unsafe catalog executable"
        );
        let mut digest = Sha256::new();
        let mut chunk = [0u8; 65536];
        loop {
            let count = f.read(&mut chunk)?;
            if count == 0 {
                break;
            }
            digest.update(&chunk[..count]);
        }
        ensure!(
            hex::encode(digest.finalize()) == self.sha256,
            "catalog executable changed"
        );
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Runtime {
    pub harness: Executable,
    pub adapter: Executable,
    pub args: Vec<String>,
    pub credential_profile: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    #[serde(default)]
    pub community: Option<String>,
    #[serde(skip)]
    pub source: Option<PathBuf>,
    pub label: String,
    pub capacity: u32,
    pub relay_key: String,
    pub runtimes: BTreeMap<String, Runtime>,
}
impl Catalog {
    pub fn read(path: &Path) -> Result<(Self, String)> {
        if path == Path::new(OPERATOR_CATALOG) {
            trusted(path)?;
        }
        let bytes = bounded(path, 16384)?;
        let mut cat: Self = serde_json::from_slice(&bytes)?;
        cat.validate()?;
        cat.source = Some(path.into());
        Ok((cat, hex::encode(Sha256::digest(bytes))))
    }
    pub fn current(&self, revision: &str) -> Result<bool> {
        let path = self
            .source
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("missing catalog source"))?;
        Ok(Self::read(path)?.1 == revision)
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            display_name(&self.label)
                && self.capacity > 0
                && !self.runtimes.is_empty()
                && self.runtimes.len() <= 64,
            "invalid catalog"
        );
        let key = PublicKey::from_hex(&self.relay_key)?;
        ensure!(
            key.to_hex() == self.relay_key && key.xonly().is_ok(),
            "invalid relay signing key"
        );
        for (id, rt) in &self.runtimes {
            ensure!(
                runtime_id(id)
                    && token(&rt.credential_profile)
                    && rt.args.len() <= 32
                    && rt.args.iter().all(|a| !a.contains(',')
                        && !a.chars().any(char::is_control)
                        && a.len() <= 256),
                "invalid local runtime descriptor"
            );
        }
        Ok(())
    }
}
fn token(t: &str) -> bool {
    !t.is_empty()
        && t.len() <= 64
        && t.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Instance {
    /// Opt-in management fields; absent preserves legacy provisioning semantics.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deployment: Option<ManagedDeployment>,
    pub unit_id: String,
    pub scope: Scope,
    pub request_id: String,
    pub runtime_id: String,
    pub revision: String,
    pub channels: Vec<String>,
    pub agent: String,
    pub secret: String,
    pub oa: String,
    pub policy: Policy,
}

/// Structured main configuration tied to one monotonic placement revision.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManagedDeployment {
    /// Current exact owner operation, separate from the stable initial unit ID.
    pub operation_id: String,
    /// Accepted monotonic placement revision.
    pub revision: u64,
    /// Main-resolved configuration; no program, path, environment or credentials.
    pub config: buzz_core::host_deployment::Deploy,
}

pub fn id(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

pub fn bounded(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    fs::File::open(path)?
        .take(limit + 1)
        .read_to_end(&mut buf)?;
    ensure!((buf.len() as u64) <= limit, "oversized private local file");
    Ok(buf)
}

pub fn provider_path(profile: &str) -> Result<Option<PathBuf>> {
    ensure!(token(profile), "invalid profile selector");
    if profile == "none" {
        return Ok(None);
    }
    let path = Path::new("/etc/buzz-hostd/providers").join(format!("{profile}.json"));
    trusted(&path)?;
    let m = path.metadata()?;
    ensure!(
        m.is_file() && m.mode() & 0o077 == 0,
        "provider credential requires private root custody"
    );
    Ok(Some(path))
}
pub fn provider(profile: &str) -> Result<BTreeMap<String, String>> {
    let Some(path) = provider_path(profile)? else {
        return Ok(BTreeMap::new());
    };
    let values: BTreeMap<String, String> = serde_json::from_slice(&bounded(&path, 16384)?)?;
    ensure!(
        values.keys().all(|k| matches!(
            k.as_str(),
            "ANTHROPIC_API_KEY" | "OPENAI_API_KEY" | "GEMINI_API_KEY" | "GOOGLE_API_KEY"
        )),
        "unsupported provider credential"
    );
    Ok(values)
}

pub const OPERATOR_CATALOG: &str = "/etc/buzz-hostd/catalog.json";
pub fn trusted(path: &Path) -> Result<()> {
    ensure!(
        path.is_absolute() && path.canonicalize()? == path,
        "noncanonical operator file"
    );
    for p in path.ancestors() {
        let m = fs::symlink_metadata(p)?;
        ensure!(
            m.uid() == 0 && m.mode() & 0o022 == 0,
            "untrusted operator file"
        );
    }
    Ok(())
}
pub fn unit_id(scope: &Scope, request: &str) -> String {
    hex::encode(Sha256::digest(format!(
        "buzz:paired-host:unit:v1:{}:{}:{}",
        scope.host, scope.generation, request
    )))
}
impl Instance {
    pub fn validate(&self, scope_catalog: &Catalog, revision: &str) -> Result<()> {
        use nostr::{Keys, SecretKey};
        ensure!(
            self.revision == revision
                && id(&self.request_id)
                && id(&self.scope.generation)
                && self.unit_id == unit_id(&self.scope, &self.request_id)
                && scope_catalog.community.as_ref() == Some(&self.scope.community)
                && scope_catalog.runtimes.contains_key(&self.runtime_id),
            "instance is outside approved catalog"
        );
        if let Some(d) = &self.deployment {
            d.config.validate()?;
            ensure!(
                id(&d.operation_id)
                    && d.revision > 0
                    && d.config.runtime_id == self.runtime_id
                    && d.config.catalog_revision == self.revision
                    && d.config.policy == self.policy,
                "invalid deployment binding"
            );
            ensure!(
                scope_catalog.runtimes[&self.runtime_id].credential_profile == "none",
                "host deployment never borrows provider credentials"
            );
            ensure!(
                d.config.provider.is_none(),
                "this native host adapter does not support provider overrides"
            );
        }
        let agent = Keys::new(SecretKey::from_hex(&self.secret)?);
        ensure!(
            agent.public_key().to_hex() == self.agent
                && self.scope.host != self.scope.owner
                && agent.public_key() != self.scope.owner
                && agent.public_key() != self.scope.host,
            "invalid candidate identity"
        );
        let oa: [String; 4] = serde_json::from_str(&self.oa)?;
        ensure!(
            oa[2].is_empty()
                && buzz_sdk::nip_oa::verify_auth_tag(&self.oa, &agent.public_key())?
                    == self.scope.owner,
            "wrong credential"
        );
        ensure!(
            self.channels.len() <= 8
                && self.channels.windows(2).all(|w| w[0] < w[1])
                && self
                    .channels
                    .iter()
                    .all(|s| uuid::Uuid::parse_str(s).is_ok_and(|u| u.to_string() == *s))
                && self.policy.valid(),
            "invalid worker policy"
        );
        Ok(())
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Prepared {
    pub instance: Instance,
    pub provider: BTreeMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn staged_instance_cannot_select_root_program_provider_or_foreign_catalog() {
        use nostr::Keys;
        let keys = Keys::generate();
        let host = Keys::generate();
        let owner = Keys::generate();
        let scope = Scope {
            community: "https://demo.test".into(),
            generation: "aa".repeat(32),
            host: host.public_key(),
            owner: owner.public_key(),
        };
        let req = "ab".repeat(32);
        let rev = "cc".repeat(32);
        let mut i = Instance {
            deployment: None,
            unit_id: unit_id(&scope, &req),
            request_id: req,
            scope,
            runtime_id: "local/approved".into(),
            revision: rev.clone(),
            channels: vec![],
            agent: keys.public_key().to_hex(),
            secret: keys.secret_key().to_secret_hex(),
            oa: buzz_sdk::nip_oa::compute_auth_tag(&owner, &keys.public_key(), "").unwrap(),
            policy: Policy {
                respond_to: "owner-only".into(),
                respond_to_allowlist: vec![],
                channel_add_policy: "owner_only".into(),
            },
        };
        let exe = Executable {
            path: "/usr/bin/true".into(),
            sha256: "dd".repeat(32),
        };
        let cat = Catalog {
            community: Some("https://demo.test".into()),
            source: None,
            label: "local".into(),
            capacity: 1,
            relay_key: host.public_key().to_hex(),
            runtimes: [(
                "local/approved".into(),
                Runtime {
                    adapter: exe.clone(),
                    harness: exe,
                    args: vec![],
                    credential_profile: "none".into(),
                },
            )]
            .into(),
        };
        assert!(i.validate(&cat, &rev).is_ok());
        for field in ["runtime", "provider", "args", "path", "source"] {
            let mut draft = serde_json::to_value(&i).unwrap();
            draft[field] = serde_json::json!("/bin/sh");
            assert!(serde_json::from_value::<Instance>(draft).is_err());
        }
        assert!(i.validate(&cat, &"ef".repeat(32)).is_err());
        i.runtime_id = "foreign/runner".into();
        assert!(i.validate(&cat, &rev).is_err());
        i.runtime_id = "local/approved".into();
        i.scope.community = "https://other.test".into();
        assert!(i.validate(&cat, &rev).is_err());
    }
}

// The library and daemon share the actual durable store and existing journals.
// Registration and opt-in deployment facades reuse this same journal.
extern crate self as buzz_hostd;
mod clock;
// Also built in the daemon; registration links only its durable-store subset.
#[allow(dead_code)]
mod store;
// Also built in the daemon; registration links only its durable-store subset.
#[allow(dead_code)]
mod request;
// Also built in the daemon; registration links only its durable-store subset.
#[allow(dead_code)]
mod onboard;
// Also built in the daemon; registration links only its durable-store subset.
#[allow(dead_code)]
mod transport;
// Also built in the daemon; registration links only its durable-store subset.
/// Durable registration-only host facade; no process/provider operations.
pub mod registration;
#[allow(dead_code)]
mod unit;

use clock::Clock;
use store::Store;

/// Fixed privileged operation implementation; executable owns peer admission.
pub mod activation;

/// Opt-in native deployment reconciliation on the existing registration journal.
pub mod deployment;

/// Map the bounded resolved configuration to existing ACP configuration names.
/// Provider overrides are refused by Instance::validate, never borrowed locally.
pub fn deployment_environment(i: &Instance) -> Result<BTreeMap<String, String>> {
    let mut env = BTreeMap::new();
    if let Some(d) = &i.deployment {
        ensure!(
            !matches!(
                d.config.model,
                buzz_core::host_deployment::ModelSelection::PendingLogin {}
            ),
            "native login/model selection pending"
        );
        d.config.validate()?;
        env.insert("BUZZ_ACP_AGENTS".into(), d.config.parallelism.to_string());
        env.insert("BUZZ_ACP_DISPLAY_NAME".into(), d.config.name.clone());
        env.insert("BUZZ_ACP_EXIT_AFTER_INACTIVITY".into(), "0".into());
        env.insert("BUZZ_ACP_LAZY_POOL".into(), "true".into());
        if let Some(prompt) = &d.config.system_prompt {
            env.insert("BUZZ_ACP_SYSTEM_PROMPT".into(), prompt.clone());
        }
        if let Some(effort) = &d.config.effort {
            env.insert("BUZZ_ACP_EFFORT_LEVEL".into(), effort.clone());
        }
        if let buzz_core::host_deployment::ModelSelection::Selected { id } = &d.config.model {
            env.insert("BUZZ_ACP_MODEL".into(), id.clone());
        }
    }
    Ok(env)
}
