//! Secret-handling primitives and the sealed-envelope seal/unseal pair.
//!
//! **The seal/unseal machinery has no CLI caller**: the sealed-key mint mode
//! (B-mode) was removed — its provision path could never converge against
//! the relay, and the key-born-on-host ceremony covers every supported
//! locus. The envelope primitive is retained as a library because it is the
//! right tool for problems that still exist: sealed API-key delivery (real
//! secrets that must reach a host), host→host re-homing, and sealed
//! noded-spec content. Its requirement: move exactly one secret bundle
//! owner→host, once, over an untrusted channel, such that only the target
//! host can open it, the host can prove it came from the owner who signed
//! the attestation, and the pieces inside cannot be mixed-and-matched
//! across agents or hosts.
//!
//! Primitive: **NIP-44 v2 with a static sender key** — the same primitive the
//! Tamarin-proved NIP-AB pairing flow uses for `PayloadType::Nsec`, minus the
//! interactive session (our transit is offline/one-shot). The poly1305 MAC
//! over the ECDH conversation key is the sender-authentication step: a
//! successful decrypt with `(host_sk, envelope.owner_pubkey)` proves the
//! sealer knew `ECDH(owner_sk, host_pk)`.
//!
//! Key-handling rules (binding): the agent secret is canonical
//! 64-lowercase-hex, held in [`Zeroizing`] buffers, never `Debug`/`Display`
//! formatted, and never echoed by any error path. Every fallible step of
//! [`unseal`] is fail-closed with a distinct error variant, in the exact
//! order the spec numbers them.

use std::collections::BTreeMap;
use std::fmt;

use nostr::nips::nip44;
use nostr::{Keys, PublicKey, SecretKey};
use serde::de::Error as SerdeDeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use zeroize::Zeroizing;

/// The `type` discriminator every envelope must carry.
pub const ENVELOPE_TYPE: &str = "buzz-sealed-agent";

/// Envelope / inner payload schema version this crate produces and accepts.
pub const VERSION: u32 = 1;

/// Step-6 hygiene threshold: warn when a bundle is older than this (30 days).
pub const STALENESS_WARN_SECS: u64 = 30 * 24 * 60 * 60;

/// True iff `s` is exactly 64 ASCII lowercase-hex characters.
///
/// This is the canonical-form check for secret-key material: it rejects
/// uppercase, whitespace, CR/LF, bech32 (`nsec…`), and any trailing byte —
/// killing the newline/CRLF class of malformed-secret bugs at the source.
pub fn is_canonical_64_hex(s: &[u8]) -> bool {
    s.len() == 64 && s.iter().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// Redact key-shaped material from a message before it reaches stderr.
///
/// clap's own error rendering echoes argv values verbatim ("unexpected
/// argument 'nsec1…' found") — and a secret pasted without its flag prefix
/// would land in shell history and session transcripts (a measured leak
/// channel). Every token that looks like key material is replaced
/// wholesale: whole-token hex, a ≥32-char hex run fused inside a longer
/// token (`x<64hex>`), or a bech32 `nsec`/`npub` string anywhere in the
/// token. Shared by every mint binary that parses argv — one redactor, all
/// consumers.
pub fn redact_key_material(message: &str) -> String {
    let mut out = String::with_capacity(message.len());
    let mut chars = message.char_indices().peekable();
    while let Some(&(start, c)) = chars.peek() {
        if c.is_ascii_alphanumeric() {
            let mut end = start;
            while let Some(&(i, c2)) = chars.peek() {
                if c2.is_ascii_alphanumeric() {
                    end = i + c2.len_utf8();
                    chars.next();
                } else {
                    break;
                }
            }
            let token = &message[start..end];
            // A ≥32-hex RUN anywhere in the token (not whole-token hex):
            // catches a key fused to argv junk, e.g. `x<64hex>`.
            let hex_run = {
                let mut run = 0usize;
                let mut max = 0usize;
                for b in token.bytes() {
                    run = if b.is_ascii_hexdigit() { run + 1 } else { 0 };
                    max = max.max(run);
                }
                max >= 32
            };
            let lower = token.to_ascii_lowercase();
            let bech32ish = lower.contains("nsec1") || lower.contains("npub1");
            if hex_run || bech32ish {
                out.push_str("<redacted-key-material>");
            } else {
                out.push_str(token);
            }
        } else {
            out.push(c);
            chars.next();
        }
    }
    out
}

/// Strictly validate a 64-lowercase-hex PUBLIC key argument.
///
/// The value is NEVER echoed back: a mistakenly pasted secret key is
/// indistinguishable from a public key (both are 64 hex chars), so no error
/// path may reproduce the input. `what` names the argument; `hint`, when
/// given, says where the right value comes from.
pub fn validate_pubkey_arg(
    s: &str,
    what: &str,
    hint: Option<&str>,
) -> Result<nostr::PublicKey, String> {
    if !is_canonical_64_hex(s.as_bytes()) {
        let hint = hint.map(|h| format!(", {h}")).unwrap_or_default();
        return Err(format!(
            "invalid {what} value (got {} chars): expected a PUBLIC key as exactly \
             64 lowercase hex characters{hint} (value withheld from this message on purpose)",
            s.len()
        ));
    }
    nostr::PublicKey::from_hex(s).map_err(|_| {
        format!(
            "invalid {what} value: not a valid x-only secp256k1 public key \
             (value withheld from this message on purpose)"
        )
    })
}

// ─── Secret newtypes ─────────────────────────────────────────────────────────

/// An agent secret key in canonical form: exactly 64 lowercase hex chars,
/// validated to be a real secp256k1 secret key at construction.
///
/// The class of malformed-secret bugs (uppercase, 65 chars, trailing `\n`,
/// CRLF, `nsec` bech32) is killed at this type's boundary: a value that
/// parses IS canonical. The buffer zeroizes on drop. There is no `Display`
/// impl and `Debug` is redacted; the only way to read the bytes is the
/// deliberately named [`AgentSecretKey::expose_canonical_hex`].
#[derive(Clone)]
pub struct AgentSecretKey(Zeroizing<String>);

impl AgentSecretKey {
    /// Parse from canonical 64-lowercase-hex. Rejects everything else.
    ///
    /// The input is NEVER echoed in errors (it is secret material).
    pub fn from_canonical_hex(s: &str) -> Result<Self, SealError> {
        if !is_canonical_64_hex(s.as_bytes()) {
            return Err(SealError::NonCanonicalSecretKey { len: s.len() });
        }
        // Curve validity: parse now so downstream derivation cannot surprise.
        if SecretKey::from_hex(s).is_err() {
            return Err(SealError::InvalidSecretKey);
        }
        Ok(Self(Zeroizing::new(s.to_owned())))
    }

    /// Build from freshly generated [`Keys`] (e.g. `Keys::generate()`).
    ///
    /// `to_secret_hex()` is canonical lowercase hex by construction; the
    /// canonical check is kept as defense in depth.
    pub fn from_keys(keys: &Keys) -> Result<Self, SealError> {
        let hex = Zeroizing::new(keys.secret_key().to_secret_hex());
        Self::from_canonical_hex(&hex)
    }

    /// Deliberate, grep-able accessor for the secret bytes.
    ///
    /// Callers must only route this to sealed plaintext or a 0600
    /// credential file — never to a log, error, or stdout surface.
    pub fn expose_canonical_hex(&self) -> &str {
        &self.0
    }

    /// Derive the public key this secret key corresponds to.
    pub fn derive_public_key(&self) -> Result<PublicKey, SealError> {
        let sk = SecretKey::from_hex(&self.0).map_err(|_| SealError::InvalidSecretKey)?;
        Ok(Keys::new(sk).public_key())
    }
}

impl fmt::Debug for AgentSecretKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AgentSecretKey(<redacted>)")
    }
}

impl PartialEq for AgentSecretKey {
    fn eq(&self, other: &Self) -> bool {
        // Test/equality convenience; offline artifacts, no timing channel.
        *self.0 == *other.0
    }
}
impl Eq for AgentSecretKey {}

impl Serialize for AgentSecretKey {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for AgentSecretKey {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // Residual: serde's intermediate String is not zeroized (inherent to
        // serde internals; same documented residual as the pairing crate).
        let raw = Zeroizing::new(String::deserialize(deserializer)?);
        AgentSecretKey::from_canonical_hex(&raw).map_err(|_| {
            // Deliberately opaque: the value is secret-tier and must not be
            // echoed into serde's error chain.
            D::Error::custom(
                "agent_sk is not canonical (expected exactly 64 lowercase hex chars, \
                 valid secp256k1; value withheld)",
            )
        })
    }
}

/// A secret string value (provider API keys). Zeroizes on drop; redacted
/// `Debug`; no `Display`.
#[derive(Clone)]
pub struct SecretString(Zeroizing<String>);

impl SecretString {
    pub fn new(value: String) -> Self {
        Self(Zeroizing::new(value))
    }

    /// Deliberate, grep-able accessor. Same routing rules as
    /// [`AgentSecretKey::expose_canonical_hex`].
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretString(<redacted>)")
    }
}

impl PartialEq for SecretString {
    fn eq(&self, other: &Self) -> bool {
        *self.0 == *other.0
    }
}
impl Eq for SecretString {}

impl Serialize for SecretString {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for SecretString {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self(Zeroizing::new(String::deserialize(deserializer)?)))
    }
}

// ─── Wire types ──────────────────────────────────────────────────────────────

/// The authoritative, encrypted payload (spec §Inner payload).
///
/// Ships the attestation *inside* so the blob is self-contained and cannot
/// be paired with a different agent's tag; ships `host_pubkey` inside as the
/// anti-splice binding (envelope fields can be rewritten, this cannot).
///
/// No `Debug` impl on purpose: the struct carries secret material and must
/// never be debug-formatted wholesale.
#[derive(Clone, Serialize, Deserialize)]
pub struct InnerPayload {
    /// Schema version; must be [`VERSION`].
    pub v: u32,
    /// Agent secret key, canonical 64-lowercase-hex (validated newtype).
    pub agent_sk: AgentSecretKey,
    /// Public key `agent_sk` must derive to.
    pub agent_pubkey: PublicKey,
    /// NIP-OA auth tag JSON: `["auth","<owner>","<conditions>","<sig>"]`.
    pub attestation: String,
    /// The host this payload is bound to (authoritative anti-splice copy).
    pub host_pubkey: PublicKey,
    /// Seal time (unix seconds) — staleness hygiene only, not a validity bound.
    pub created_at: u64,
    /// Optional provider API keys (same secret tier as `agent_sk`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_keys: Option<BTreeMap<String, SecretString>>,
}

/// The public envelope written to `sealed.json` (spec §Envelope).
///
/// Every field except `ciphertext` is an **unauthenticated routing hint**;
/// the inner payload is authoritative on all of them.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Envelope {
    /// Schema version; must be [`VERSION`].
    pub v: u32,
    /// Discriminator; must be [`ENVELOPE_TYPE`].
    #[serde(rename = "type")]
    pub bundle_type: String,
    /// The claimed sealer — the static NIP-44 sender key used to decrypt.
    pub owner_pubkey: PublicKey,
    /// The claimed seal target.
    pub host_pubkey: PublicKey,
    /// Routing hint so provision can name things without decrypting.
    pub agent_pubkey: PublicKey,
    /// NIP-44 v2 base64 payload.
    pub ciphertext: String,
}

/// Successful unseal: the authoritative inner payload plus non-fatal
/// warnings (spec steps 3 and 6 warn rather than reject).
pub struct Unsealed {
    pub inner: InnerPayload,
    pub warnings: Vec<UnsealWarning>,
}

impl fmt::Debug for Unsealed {
    /// Manual, redacting impl: names the agent by pubkey only. The inner
    /// payload deliberately has no `Debug` at all.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Unsealed")
            .field("agent_pubkey", &self.inner.agent_pubkey)
            .field("warnings", &self.warnings)
            .finish_non_exhaustive()
    }
}

/// Non-fatal findings from [`unseal`]. `Display` carries pubkey-tier data only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnsealWarning {
    /// Envelope routing hint disagrees with the authoritative inner copy —
    /// inner wins (spec step 3).
    EnvelopeAgentPubkeyMismatch {
        envelope: PublicKey,
        inner: PublicKey,
    },
    /// Bundle older than [`STALENESS_WARN_SECS`] (spec step 6, hygiene only).
    Stale { age_secs: u64 },
}

impl fmt::Display for UnsealWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EnvelopeAgentPubkeyMismatch { envelope, inner } => write!(
                f,
                "envelope agent_pubkey {} disagrees with inner (authoritative) {} — inner wins",
                envelope.to_hex(),
                inner.to_hex()
            ),
            Self::Stale { age_secs } => write!(
                f,
                "bundle is {} days old (> 30-day hygiene threshold); the blob is inert \
                 without the host key, but consider re-minting",
                age_secs / 86_400
            ),
        }
    }
}

// ─── Errors ──────────────────────────────────────────────────────────────────

/// Seal-side failures. No variant carries secret material.
#[derive(Debug, thiserror::Error)]
pub enum SealError {
    #[error(
        "secret key is not canonical (got {len} chars): expected exactly 64 lowercase hex \
         characters — no uppercase, no whitespace/CRLF, no nsec bech32 (value withheld)"
    )]
    NonCanonicalSecretKey { len: usize },
    #[error("secret key is not a valid secp256k1 secret key (value withheld)")]
    InvalidSecretKey,
    #[error("inner payload version {got} unsupported (expected {VERSION})")]
    Version { got: u32 },
    #[error("inner.host_pubkey does not match the seal-target host pubkey")]
    HostMismatch,
    #[error(
        "agent_sk does not derive to inner.agent_pubkey — refusing to seal an inconsistent payload"
    )]
    AgentPubkeyMismatch,
    #[error("attestation does not verify for inner.agent_pubkey: {0}")]
    AttestationInvalid(String),
    #[error("attestation attester differs from the sealing owner key — refusing to wrap someone else's attestation")]
    AttesterMismatch,
    #[error("inner payload serialization failed (detail withheld: plaintext is secret-tier)")]
    Serialize,
    #[error("NIP-44 encryption failed (detail withheld: plaintext is secret-tier)")]
    Encrypt,
}

/// Unseal-side failures — one distinct variant per fail-closed spec step,
/// in spec order. No variant carries secret material; parse failures are
/// deliberately opaque because the plaintext is secret-tier.
#[derive(Debug, thiserror::Error)]
pub enum UnsealError {
    // Step 1 — envelope checks.
    #[error("step 1: envelope version {got} unsupported (expected {VERSION})")]
    EnvelopeVersion { got: u32 },
    #[error("step 1: envelope type {got:?} is not {ENVELOPE_TYPE:?}")]
    EnvelopeType { got: String },
    #[error("step 1: envelope host_pubkey {envelope} is not this host {own} — bundle was sealed for a different host")]
    EnvelopeHostMismatch { envelope: String, own: String },
    // Step 2 — decrypt / sender-auth MAC.
    #[error(
        "step 2: NIP-44 decrypt failed (MAC): either the blob was not sealed to this host \
         key or envelope.owner_pubkey is not the real sealer"
    )]
    Decrypt,
    // Step 3 — inner authoritative checks.
    #[error(
        "step 3: inner payload does not parse as the expected shape at line {line} column \
         {column} (content withheld: plaintext is secret-tier)"
    )]
    InnerParse { line: usize, column: usize },
    #[error("step 3: inner payload version {got} unsupported (expected {VERSION})")]
    InnerVersion { got: u32 },
    #[error(
        "step 3: inner.host_pubkey (authoritative) is not this host — anti-splice reject: \
         this payload was bound to a different host"
    )]
    InnerHostMismatch,
    #[error("step 3: inner.agent_sk is not a valid secp256k1 secret key (value withheld)")]
    InvalidSecretKey,
    #[error(
        "step 3: inner.agent_sk does not derive to inner.agent_pubkey — payload is \
         internally inconsistent"
    )]
    AgentPubkeyMismatch,
    // Step 4 — attestation binding attester == sender.
    #[error("step 4: attestation does not verify for inner.agent_pubkey: {0}")]
    AttestationInvalid(String),
    #[error(
        "step 4: attestation attester {attester} is not the NIP-44 sender \
         {sender} — refusing a wrapped foreign attestation"
    )]
    AttesterNotSender { attester: String, sender: String },
    // Step 5 — owner pinning.
    #[error(
        "step 5: attester {attester} is not a pinned owner — refusing a self-consistent \
         but foreign bundle (extend the pin list explicitly to trust this owner)"
    )]
    OwnerNotPinned { attester: String },
}

// ─── seal / unseal ───────────────────────────────────────────────────────────

/// Seal `inner` to `host_pubkey`, sender-authenticated by `owner_keys`.
///
/// Refuses to produce an inconsistent blob: the inner payload must already
/// be self-consistent (version, host binding, sk→pubkey derivation,
/// attestation verifying with the sealing owner as attester). This is the
/// seal-time half of spec test 6 — mint refuses at source.
pub fn seal(
    owner_keys: &Keys,
    host_pubkey: &PublicKey,
    inner: &InnerPayload,
) -> Result<Envelope, SealError> {
    if inner.v != VERSION {
        return Err(SealError::Version { got: inner.v });
    }
    if inner.host_pubkey != *host_pubkey {
        return Err(SealError::HostMismatch);
    }
    let derived = inner.agent_sk.derive_public_key()?;
    if derived != inner.agent_pubkey {
        return Err(SealError::AgentPubkeyMismatch);
    }
    let attester = buzz_sdk::nip_oa::verify_auth_tag(&inner.attestation, &inner.agent_pubkey)
        .map_err(|e| SealError::AttestationInvalid(e.to_string()))?;
    if attester != owner_keys.public_key() {
        return Err(SealError::AttesterMismatch);
    }

    // Residuals that cannot be zeroized: serde_json intermediate buffers and
    // nip44's internal plaintext copy (same documented residuals as the
    // pairing crate). The buffer we control zeroizes on drop.
    let plaintext = Zeroizing::new(serde_json::to_string(inner).map_err(|_| SealError::Serialize)?);
    let ciphertext = nip44::encrypt(
        owner_keys.secret_key(),
        host_pubkey,
        plaintext.as_str(),
        nip44::Version::V2,
    )
    .map_err(|_| SealError::Encrypt)?;

    Ok(Envelope {
        v: VERSION,
        bundle_type: ENVELOPE_TYPE.to_owned(),
        owner_pubkey: owner_keys.public_key(),
        host_pubkey: *host_pubkey,
        agent_pubkey: inner.agent_pubkey,
        ciphertext,
    })
}

/// Unseal an envelope with this host's keys, enforcing the spec's
/// fail-closed verification order exactly (steps 1–6; step 7, the
/// credential write, is the caller's job).
///
/// `pinned_owners` is the host's allowed-owners list written at host-init;
/// an empty list rejects every bundle (fail-closed, spec step 5).
pub fn unseal(
    host_keys: &Keys,
    envelope: &Envelope,
    pinned_owners: &[PublicKey],
) -> Result<Unsealed, UnsealError> {
    unseal_at(
        host_keys,
        envelope,
        pinned_owners,
        nostr::Timestamp::now().as_secs(),
    )
}

/// [`unseal`] with an injectable clock (step-6 staleness), for tests.
pub fn unseal_at(
    host_keys: &Keys,
    envelope: &Envelope,
    pinned_owners: &[PublicKey],
    now_unix: u64,
) -> Result<Unsealed, UnsealError> {
    // ── Step 1: envelope checks (unauthenticated routing hints — cheap
    // rejects only; nothing here is trusted further than "worth decrypting").
    if envelope.v != VERSION {
        return Err(UnsealError::EnvelopeVersion { got: envelope.v });
    }
    if envelope.bundle_type != ENVELOPE_TYPE {
        return Err(UnsealError::EnvelopeType {
            got: envelope.bundle_type.clone(),
        });
    }
    let own = host_keys.public_key();
    if envelope.host_pubkey != own {
        return Err(UnsealError::EnvelopeHostMismatch {
            envelope: envelope.host_pubkey.to_hex(),
            own: own.to_hex(),
        });
    }

    // ── Step 2: decrypt with (host_sk, envelope.owner_pubkey). The NIP-44
    // MAC failing here IS the sender-authentication reject.
    let plaintext = Zeroizing::new(
        nip44::decrypt(
            host_keys.secret_key(),
            &envelope.owner_pubkey,
            &envelope.ciphertext,
        )
        .map_err(|_| UnsealError::Decrypt)?,
    );

    // ── Step 3: inner authoritative checks.
    let inner: InnerPayload =
        serde_json::from_str(&plaintext).map_err(|e| UnsealError::InnerParse {
            line: e.line(),
            column: e.column(),
        })?;
    drop(plaintext); // zeroizes
    if inner.v != VERSION {
        return Err(UnsealError::InnerVersion { got: inner.v });
    }
    if inner.host_pubkey != own {
        // Anti-splice: envelope fields could be rewritten; this one cannot.
        return Err(UnsealError::InnerHostMismatch);
    }
    let derived = inner
        .agent_sk
        .derive_public_key()
        .map_err(|_| UnsealError::InvalidSecretKey)?;
    if derived != inner.agent_pubkey {
        return Err(UnsealError::AgentPubkeyMismatch);
    }
    let mut warnings = Vec::new();
    if envelope.agent_pubkey != inner.agent_pubkey {
        // Envelope hint disagrees — inner wins, warn only (spec step 3).
        warnings.push(UnsealWarning::EnvelopeAgentPubkeyMismatch {
            envelope: envelope.agent_pubkey,
            inner: inner.agent_pubkey,
        });
    }

    // ── Step 4: attestation must verify for THIS agent pubkey, and the
    // attester must be the NIP-44 sender — binds sealer == attester.
    let attester = buzz_sdk::nip_oa::verify_auth_tag(&inner.attestation, &inner.agent_pubkey)
        .map_err(|e| UnsealError::AttestationInvalid(e.to_string()))?;
    if attester != envelope.owner_pubkey {
        return Err(UnsealError::AttesterNotSender {
            attester: attester.to_hex(),
            sender: envelope.owner_pubkey.to_hex(),
        });
    }

    // ── Step 5: owner pinning — step 4 proved internal consistency; this
    // proves it is OUR owner. Empty pin list rejects everything.
    if !pinned_owners.contains(&attester) {
        return Err(UnsealError::OwnerNotPinned {
            attester: attester.to_hex(),
        });
    }

    // ── Step 6: staleness hygiene (warn, never reject — the blob is not a
    // bearer credential; without host_sk it is inert).
    let age = now_unix.saturating_sub(inner.created_at);
    if age > STALENESS_WARN_SECS {
        warnings.push(UnsealWarning::Stale { age_secs: age });
    }

    Ok(Unsealed { inner, warnings })
}

#[cfg(test)]
mod redact_tests {
    use super::*;

    #[test]
    fn redactor_catches_bare_fused_and_bech32_key_material() {
        let hex64 = "f5c79eab2f44284ff278780c34aab621aafb149be4d7dba8f9625bfd99419733";
        // Bare 64-hex token.
        assert!(!redact_key_material(&format!("unexpected argument {hex64}")).contains(hex64));
        // Key material FUSED into a mixed token (argv junk glued to the key).
        let fused = format!("x{hex64}");
        assert!(!redact_key_material(&format!("got {fused} here")).contains(hex64));
        let fused = format!("key{hex64}end");
        assert!(!redact_key_material(&fused).contains(hex64));
        // bech32 fused mid-token.
        let nsec = "nsec1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq";
        assert!(!redact_key_material(&format!("argnsec={nsec}x")).contains(nsec));
        // Ordinary words and short hex survive.
        let msg = "error: invalid value cafe1234 for --channel";
        assert_eq!(redact_key_material(msg), msg);
    }
}
