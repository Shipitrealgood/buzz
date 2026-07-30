//! Mint bundle schema + shared structural validation.
//!
//! The bundle directory (`<name>-mint/`) is produced by `buzz-mint new` and
//! consumed by `buzz-provision agent`. Both binaries must agree on its schema
//! and on what "structurally valid" means, so both live here — one authority,
//! two consumers (chuck T1 audit: never grow a second parser for the same
//! artifact).
//!
//! Everything in this module operates on PUBLIC data only: the manifest, the
//! envelope's routing hints, and the owner-signed kind:9000 events (tamper-
//! evident via their own signatures). No secret input, no decrypt — the
//! checks that need the plaintext are exactly [`crate::unseal`]'s steps 2-6.

use std::fs;
use std::path::{Path, PathBuf};

use base64::Engine;
use nostr::{Event, JsonUtil, PublicKey};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::Envelope;

/// Manifest schema version (matches the envelope/inner version).
pub const MANIFEST_VERSION: u32 = 1;

/// Public, operator-editable agent configuration carried by the manifest.
#[derive(Clone, Serialize, Deserialize)]
pub struct ManifestConfig {
    /// Inbound author gate the harness runs with (`owner-only`, `anyone`,
    /// `allowlist`).
    pub respond_to: String,
    /// Channels the owner pre-signed kind:9000 bot adds for.
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
#[derive(Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub v: u32,
    pub name: String,
    pub agent_pubkey: PublicKey,
    pub owner_pubkey: PublicKey,
    pub host_pubkey: PublicKey,
    /// NIP-OA auth tag JSON (public data, same as native's agent record).
    pub attestation: String,
    pub created_at: u64,
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

/// Whether `sealed.json` must be present when loading.
///
/// Provision shreds `sealed.json` after the credential is verified on disk,
/// so a converged re-run legitimately sees a bundle without it.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum SealedPresence {
    Required,
    Optional,
}

/// A structurally loaded (not yet validated) bundle.
pub struct LoadedBundle {
    pub dir: PathBuf,
    pub manifest: Manifest,
    /// `None` only under [`SealedPresence::Optional`] when the file is gone.
    pub envelope: Option<Envelope>,
    /// Parsed `events/9000-<uuid>.json` files, sorted by filename. Files that
    /// exist but do not parse are recorded in `event_problems` instead.
    pub events: Vec<(Uuid, Event)>,
    /// Problems found while enumerating/parsing the events dir (unexpected
    /// filenames, unparseable events). Folded into [`validate`]'s output.
    pub event_problems: Vec<String>,
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

/// Load a bundle directory: `manifest.json`, `sealed.json` (per `sealed`),
/// and every `events/9000-<uuid>.json`.
pub fn load(dir: &Path, sealed: SealedPresence) -> Result<LoadedBundle, BundleError> {
    let manifest: Manifest = read_json(&dir.join("manifest.json"), "a manifest")?;

    let sealed_path = dir.join("sealed.json");
    let envelope: Option<Envelope> = if sealed_path.exists() || sealed == SealedPresence::Required {
        Some(read_json(&sealed_path, "an envelope")?)
    } else {
        None
    };

    let mut events: Vec<(Uuid, Event)> = Vec::new();
    let mut event_problems: Vec<String> = Vec::new();
    let events_dir = dir.join("events");
    if events_dir.is_dir() {
        let mut entries: Vec<PathBuf> = fs::read_dir(&events_dir)
            .map_err(|source| BundleError::Read {
                path: events_dir.display().to_string(),
                source,
            })?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .collect();
        entries.sort();
        for path in entries {
            let fname = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_owned();
            let channel: Option<Uuid> = fname
                .strip_prefix("9000-")
                .and_then(|s| s.strip_suffix(".json"))
                .and_then(|s| s.parse().ok());
            let Some(channel) = channel else {
                event_problems.push(format!(
                    "events/{fname}: unexpected file (want 9000-<uuid>.json)"
                ));
                continue;
            };
            let raw = fs::read_to_string(&path).map_err(|source| BundleError::Read {
                path: path.display().to_string(),
                source,
            })?;
            match Event::from_json(&raw) {
                Ok(ev) => events.push((channel, ev)),
                Err(e) => {
                    event_problems.push(format!("events/{fname}: does not parse as an event: {e}"))
                }
            }
        }
    }

    Ok(LoadedBundle {
        dir: dir.to_path_buf(),
        manifest,
        envelope,
        events,
        event_problems,
    })
}

/// Validate a loaded bundle's public-field consistency.
///
/// Returns every problem found (empty = structurally valid). The checks it
/// cannot make (inner payload authority) are exactly the sealed spec's unseal
/// steps 2-6, run host-side with the host key.
pub fn validate(bundle: &LoadedBundle) -> Vec<String> {
    let mut problems: Vec<String> = bundle.event_problems.clone();
    let manifest = &bundle.manifest;

    if manifest.v != MANIFEST_VERSION {
        problems.push(format!(
            "manifest.v is {} (expected {MANIFEST_VERSION})",
            manifest.v
        ));
    }

    if let Some(envelope) = &bundle.envelope {
        if envelope.v != crate::VERSION {
            problems.push(format!(
                "envelope v is {} (expected {})",
                envelope.v,
                crate::VERSION
            ));
        }
        if envelope.bundle_type != crate::ENVELOPE_TYPE {
            problems.push(format!(
                "envelope type is {:?} (expected {:?})",
                envelope.bundle_type,
                crate::ENVELOPE_TYPE
            ));
        }

        // Manifest ↔ envelope agreement on every shared field.
        for (field, m, e) in [
            ("agent_pubkey", manifest.agent_pubkey, envelope.agent_pubkey),
            ("owner_pubkey", manifest.owner_pubkey, envelope.owner_pubkey),
            ("host_pubkey", manifest.host_pubkey, envelope.host_pubkey),
        ] {
            if m != e {
                problems.push(format!(
                    "{field} disagrees: manifest {} vs envelope {}",
                    m.to_hex(),
                    e.to_hex()
                ));
            }
        }

        // Ciphertext plausibility without decrypting: NIP-44 content bounds
        // and version byte 2 (same bounds the pairing crate enforces
        // pre-decrypt).
        let clen = envelope.ciphertext.len();
        if !(132..=87472).contains(&clen) {
            problems.push(format!(
                "ciphertext length {clen} outside NIP-44 content bounds (132..=87472)"
            ));
        }
        match base64::engine::general_purpose::STANDARD.decode(&envelope.ciphertext) {
            Ok(decoded) if decoded.first() != Some(&2) => problems.push(format!(
                "ciphertext version byte is {:?} (expected 2 = NIP-44 v2)",
                decoded.first()
            )),
            Ok(_) => {}
            Err(_) => problems.push("ciphertext is not valid base64".to_owned()),
        }
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

    // Events: one per manifest channel, none extra, each a valid owner-signed
    // kind:9000 bot add for THIS agent in THAT channel.
    for (channel, event) in &bundle.events {
        let fname = format!("9000-{channel}.json");
        if let Err(e) = event.verify() {
            problems.push(format!("events/{fname}: id/signature invalid: {e}"));
        }
        if event.kind.as_u16() != 9000 {
            problems.push(format!("events/{fname}: kind {} != 9000", event.kind));
        }
        if event.pubkey != manifest.owner_pubkey {
            problems.push(format!(
                "events/{fname}: signer {} is not manifest.owner_pubkey",
                event.pubkey.to_hex()
            ));
        }
        let tag_value = |name: &str| -> Option<String> {
            event.tags.iter().find_map(|t| {
                let s = t.as_slice();
                (s.first().map(String::as_str) == Some(name))
                    .then(|| s.get(1).cloned())
                    .flatten()
            })
        };
        if tag_value("h").as_deref() != Some(channel.to_string().as_str()) {
            problems.push(format!(
                "events/{fname}: h tag does not match filename channel"
            ));
        }
        if tag_value("p").as_deref() != Some(manifest.agent_pubkey.to_hex().as_str()) {
            problems.push(format!(
                "events/{fname}: p tag does not target manifest.agent_pubkey"
            ));
        }
        if tag_value("role").as_deref() != Some("bot") {
            problems.push(format!("events/{fname}: role tag is not \"bot\""));
        }
        if !manifest.config.channels.contains(channel) {
            problems.push(format!(
                "events/{fname}: channel is not listed in manifest config.channels"
            ));
        }
    }
    for channel in &manifest.config.channels {
        if !bundle.events.iter().any(|(c, _)| c == channel) {
            problems.push(format!(
                "missing events/9000-{channel}.json for manifest channel"
            ));
        }
    }

    problems
}
