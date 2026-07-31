//! Mint bundle schema + shared structural validation.
//!
//! The bundle directory (`<name>-mint/`) is produced by `buzz-mint attest`
//! and consumed by `buzz-provision agent`. Both binaries must agree on its
//! schema and on what "structurally valid" means, so both live here — one
//! authority, two consumers (never grow a second parser for the same
//! artifact).
//!
//! A bundle is `manifest.json` only — 100% public data. The agent key is
//! born host-side (`buzz-provision keygen`); the owner attests the exported
//! pubkey and publishes the kind:9000 batch directly at attest time, so
//! nothing secret and no pre-signed events ever ride the bundle.

use std::fs;
use std::path::{Path, PathBuf};

use nostr::PublicKey;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Manifest schema version.
pub const MANIFEST_VERSION: u32 = 1;

/// Which mint ceremony produced the bundle.
///
/// Only **A** (public attestation bundle; key born host-side) is accepted.
/// **B** (sealed key transit) was removed: its provision path could never
/// converge against the relay (the relay refuses third-party submission of
/// owner-signed events), and the A ceremony covers every supported locus.
/// The variant is retained so a sealed-era bundle fails with a precise
/// message instead of a parse error.
///
/// Serde default is **B**: every bundle minted before the field existed is a
/// sealed bundle — and is therefore refused at load.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BundleMode {
    #[serde(rename = "a")]
    A,
    #[default]
    #[serde(rename = "b")]
    B,
}

impl BundleMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::A => "a (public attestation bundle; key born host-side)",
            Self::B => "b (sealed key transit — removed)",
        }
    }
}

/// Public, operator-editable agent configuration carried by the manifest.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ManifestConfig {
    /// Inbound author gate the harness runs with (`owner-only`, `anyone`,
    /// `allowlist`).
    pub respond_to: String,
    /// Channels the owner published kind:9000 bot adds for at attest time.
    pub channels: Vec<Uuid>,
    /// Harness command for the host-side unit (`BUZZ_ACP_AGENT_COMMAND`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub harness_cmd: Option<String>,
    /// Relay the agent connects to (`BUZZ_RELAY_URL`). Optional: provision
    /// accepts `--relay-url` instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relay_url: Option<String>,
}

/// The public bundle manifest (`manifest.json`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub v: u32,
    pub name: String,
    pub agent_pubkey: PublicKey,
    pub owner_pubkey: PublicKey,
    pub host_pubkey: PublicKey,
    /// NIP-OA auth tag JSON (public data, same as native's agent record).
    pub attestation: String,
    pub created_at: u64,
    /// Which ceremony minted this bundle. Absent in pre-mode manifests,
    /// which are all sealed (B) bundles — refused at load.
    #[serde(default)]
    pub mode: BundleMode,
    pub config: ManifestConfig,
}

/// Validate an agent name for use as a directory name and systemd instance
/// name: 1-64 chars of `[A-Za-z0-9_-]`, starting alphanumeric.
///
/// Enforced at mint (the name becomes the bundle dir) AND at provision (the
/// manifest is operator-editable, so the consumer re-validates).
pub fn validate_agent_name(name: &str) -> Result<(), String> {
    let ok = !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        && name.as_bytes()[0].is_ascii_alphanumeric();
    if ok {
        Ok(())
    } else {
        Err(format!(
            "invalid name {name:?}: expected 1-64 chars of [A-Za-z0-9_-], starting with an \
             alphanumeric (it becomes a directory and systemd instance name)"
        ))
    }
}

/// A structurally loaded (not yet validated) bundle.
#[derive(Debug)]
pub struct LoadedBundle {
    pub dir: PathBuf,
    pub manifest: Manifest,
    /// Problems found while loading (stray sealed-era artifacts). Folded
    /// into [`validate`]'s output.
    pub load_problems: Vec<String>,
}

/// Fatal load failures: without these pieces there is no bundle to validate.
#[derive(Debug, thiserror::Error)]
pub enum BundleError {
    #[error("cannot read {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("{path} does not parse as {what}: {source}")]
    Parse {
        path: String,
        what: &'static str,
        #[source]
        source: serde_json::Error,
    },
    #[error(
        "{path} declares a sealed (B-mode) bundle — sealed key transit was removed. \
         Re-mint with the A ceremony: `buzz-provision keygen` on the host, then \
         `buzz-mint attest` on the owner machine"
    )]
    SealedModeRemoved { path: String },
}

fn read_json<T: serde::de::DeserializeOwned>(
    path: &Path,
    what: &'static str,
) -> Result<T, BundleError> {
    let raw = fs::read_to_string(path).map_err(|source| BundleError::Read {
        path: path.display().to_string(),
        source,
    })?;
    serde_json::from_str(&raw).map_err(|source| BundleError::Parse {
        path: path.display().to_string(),
        what,
        source,
    })
}

/// Load a bundle directory: `manifest.json`, plus stray-artifact detection.
pub fn load(dir: &Path) -> Result<LoadedBundle, BundleError> {
    let manifest_path = dir.join("manifest.json");
    let manifest: Manifest = read_json(&manifest_path, "a manifest")?;
    if manifest.mode == BundleMode::B {
        return Err(BundleError::SealedModeRemoved {
            path: manifest_path.display().to_string(),
        });
    }

    let mut load_problems: Vec<String> = Vec::new();
    // A-mode bundles are 100% public data: a sealed.json or pre-signed
    // events/ dir means the bundle does not match its declared ceremony.
    if dir.join("sealed.json").exists() {
        load_problems.push(
            "sealed.json present in an A-mode bundle (A-mode carries manifest.json only; \
             the agent key is born host-side and never sealed)"
                .to_owned(),
        );
    }
    if dir.join("events").is_dir() {
        load_problems.push(
            "events/ present in an A-mode bundle (the owner publishes the kind:9000 batch \
             directly at attest time; A-mode bundles are manifest-only)"
                .to_owned(),
        );
    }

    Ok(LoadedBundle {
        dir: dir.to_path_buf(),
        manifest,
        load_problems,
    })
}

/// Validate a loaded bundle's public-field consistency.
///
/// Returns every problem found (empty = structurally valid). The check it
/// cannot make — that the host's credential derives to the attested pubkey —
/// runs host-side at provision time.
pub fn validate(bundle: &LoadedBundle) -> Vec<String> {
    let mut problems: Vec<String> = bundle.load_problems.clone();
    let manifest = &bundle.manifest;

    if manifest.v != MANIFEST_VERSION {
        problems.push(format!(
            "manifest.v is {} (expected {MANIFEST_VERSION})",
            manifest.v
        ));
    }

    // Attestation is public — verify it and bind attester == owner.
    match buzz_sdk::nip_oa::verify_auth_tag(&manifest.attestation, &manifest.agent_pubkey) {
        Ok(attester) => {
            if attester != manifest.owner_pubkey {
                problems.push(format!(
                    "attestation attester {} is not manifest.owner_pubkey {}",
                    attester.to_hex(),
                    manifest.owner_pubkey.to_hex()
                ));
            }
        }
        Err(e) => problems.push(format!("attestation does not verify: {e}")),
    }

    problems
}
