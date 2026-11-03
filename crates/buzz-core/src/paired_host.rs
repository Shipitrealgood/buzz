//! Private paired-host v1 enrollment. The caller owns the local offer, monotonic
//! expiry, attempt budget and atomic consumption. A verified packet alone does
//! not establish durable inventory or authorize a process launch.

pub mod transport;

use std::fmt;

use nostr::nips::nip44::{self, Version};
use nostr::secp256k1::schnorr::Signature;
use nostr::{Event, EventBuilder, EventId, Keys, Kind, PublicKey, Tag, Tags, Timestamp};
use rand::{rngs::SysRng, TryRng};
use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use thiserror::Error;
use url::{Host, Url};
use zeroize::{Zeroize, Zeroizing};

use crate::kind::KIND_PAIRED_HOST_V1;

const KIND: Kind = Kind::Custom(KIND_PAIRED_HOST_V1 as u16);
const TTL: u64 = 600;
const SKEW: u64 = 30;
/// Maximum size of a raw signed JSON event. Transport readers must cap reads too.
pub const MAX_EVENT_BYTES: usize = 32 * 1024;
const MAX_PLAINTEXT_BYTES: usize = 16 * 1024;

/// Errors expose only the authenticated ID of a wrong-proof claim for attempt accounting.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    /// Local configuration or invitation data is invalid.
    #[error("invalid paired-host enrollment")]
    Enrollment,
    /// Malformed or incorrectly scoped event, invalid signature, ciphertext, or payload.
    #[error("invalid paired-host message")]
    Message,
    /// Event time or offer lifetime outside the enrollment window.
    #[error("paired-host message outside enrollment window")]
    Time,
    /// Correctly signed and scoped claim with an incorrect proof.
    #[error("incorrect paired-host proof")]
    Proof(EventId),
    /// Randomness, encryption or signing failed.
    #[error("could not create paired-host message")]
    Create,
}

fn canonical_hex(input: &str, length: usize) -> bool {
    input.len() == length
        && input
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn canonical_origin(input: &str) -> bool {
    let Ok(url) = Url::parse(input) else {
        return false;
    };
    let Some(Host::Domain(host)) = url.host() else {
        return false;
    };
    url.scheme() == "https"
        && url.origin().ascii_serialization() == input
        && host.len() <= 253
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
}

/// A private single-use console secret. Its debug representation is always redacted.
pub struct Proof(Zeroizing<[u8; 32]>);

impl fmt::Debug for Proof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Proof(<redacted>)")
    }
}

impl Proof {
    /// Decode canonical private QR data.
    pub fn from_hex(input: &str) -> Result<Self, Error> {
        if !canonical_hex(input, 64) {
            return Err(Error::Enrollment);
        }
        let mut bytes = Zeroizing::new([0; 32]);
        hex::decode_to_slice(input, bytes.as_mut()).map_err(|_| Error::Enrollment)?;
        Ok(Self(bytes))
    }

    /// Reveal only for private console transfer or encrypted claim construction.
    pub fn expose_hex(&self) -> Zeroizing<String> {
        Zeroizing::new(hex::encode(self.0.as_ref()))
    }

    /// Digest to store with the host's local offer, computed over raw bytes.
    pub fn digest(&self) -> [u8; 32] {
        Sha256::digest(self.0.as_ref()).into()
    }
}

/// Trusted scope from local enrollment state or the owner's imported invitation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer {
    /// Selected community's canonical HTTPS origin.
    pub community: String,
    /// Independent one-time generation, in lowercase hex.
    pub generation: String,
    /// Host identity pinned by console exchange.
    pub host: PublicKey,
    /// Exclusive wall-clock expiry in Unix seconds.
    pub expires_at: u64,
}

impl Offer {
    /// Issue a fresh ten-minute offer and independent proof using OS randomness.
    pub fn issue(community: String, host: PublicKey, now: u64) -> Result<(Self, Proof), Error> {
        let mut generation = [0; 32];
        let mut proof = Proof(Zeroizing::new([0; 32]));
        SysRng
            .try_fill_bytes(&mut generation)
            .map_err(|_| Error::Create)?;
        SysRng
            .try_fill_bytes(proof.0.as_mut())
            .map_err(|_| Error::Create)?;
        let offer = Self {
            community,
            generation: hex::encode(generation),
            host,
            expires_at: now.checked_add(TTL).ok_or(Error::Time)?,
        };
        offer.validate()?;
        Ok((offer, proof))
    }

    fn validate(&self) -> Result<(), Error> {
        if !canonical_origin(&self.community)
            || !canonical_hex(&self.generation, 64)
            || self.host.xonly().is_err()
        {
            return Err(Error::Enrollment);
        }
        Ok(())
    }

    fn check_signed_time(&self, created_at: u64, now: u64) -> Result<(), Error> {
        if self.expires_at > now.saturating_add(TTL + SKEW)
            || created_at >= self.expires_at
            || created_at > now.saturating_add(SKEW)
            || created_at < self.expires_at.saturating_sub(TTL + SKEW)
        {
            return Err(Error::Time);
        }
        Ok(())
    }
}

/// The owner's original outbound claim, freshly signed or restored from local
/// ciphertext. This type can verify a receipt but cannot issue one.
#[derive(Clone, Debug)]
pub struct SavedClaim {
    offer: Offer,
    owner: PublicKey,
    id: EventId,
    created_at: u64,
    checked_at: u64,
}

impl SavedClaim {
    fn from_event(event: &Event, offer: &Offer, now: u64) -> Self {
        Self {
            offer: offer.clone(),
            owner: event.pubkey,
            id: event.id,
            created_at: event.created_at.as_secs(),
            checked_at: now,
        }
    }

    /// Exact event ID to persist independently alongside the trusted invitation scope.
    pub fn id(&self) -> EventId {
        self.id
    }
}

/// A fresh claim whose console proof the host has verified under its local offer.
#[derive(Clone, Debug)]
pub struct VerifiedClaim(SavedClaim);

impl VerifiedClaim {
    /// Exact event ID used for consumption and the receipt outbox.
    pub fn id(&self) -> EventId {
        self.0.id
    }
    /// Authenticated claimant to pin atomically when consuming an unpinned offer.
    pub fn owner(&self) -> PublicKey {
        self.0.owner
    }
}

/// Classification of a verified host receipt at the owner's verification time.
#[derive(Debug, PartialEq, Eq)]
#[must_use = "historical receipts require a fresh host inventory challenge before use"]
pub enum Acknowledgement {
    /// The original claim and matching receipt were verified within the offer lifetime.
    Fresh(EventId),
    /// Signed history only. A fresh host-signed inventory challenge must confirm current state.
    Historical(EventId),
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Operation {
    PairClaim,
    PairAck,
}

impl Operation {
    fn name(self) -> &'static str {
        match self {
            Self::PairClaim => "pair-claim",
            Self::PairAck => "pair-ack",
        }
    }
}

// Missing is permitted for the other operation's field; explicit null never is.
fn present_string<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    String::deserialize(d).map(Some)
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Body {
    v: u8,
    op: Operation,
    community: String,
    generation: String,
    owner: String,
    host: String,
    expires_at: u64,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_string"
    )]
    proof: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_string"
    )]
    claim_id: Option<String>,
}

impl Drop for Body {
    fn drop(&mut self) {
        self.proof.zeroize();
    }
}

impl Body {
    fn new(op: Operation, offer: &Offer, owner: PublicKey) -> Self {
        Self {
            v: 1,
            op,
            community: offer.community.clone(),
            generation: offer.generation.clone(),
            owner: owner.to_hex(),
            host: offer.host.to_hex(),
            expires_at: offer.expires_at,
            proof: None,
            claim_id: None,
        }
    }

    fn check(&self, op: Operation, offer: &Offer, owner: PublicKey) -> Result<(), Error> {
        if self.v != 1
            || self.op != op
            || self.community != offer.community
            || self.generation != offer.generation
            || self.host != offer.host.to_hex()
            || self.owner != owner.to_hex()
            || self.expires_at != offer.expires_at
            || self.proof.is_some() != (op == Operation::PairClaim)
            || self
                .proof
                .as_deref()
                .is_some_and(|proof| !canonical_hex(proof, 64))
            || self.claim_id.is_some() != (op == Operation::PairAck)
        {
            return Err(Error::Message);
        }
        Ok(())
    }
}

fn tags(op: Operation, offer: &Offer, recipient: PublicKey) -> Result<Tags, Error> {
    Tags::parse([
        ["p", &recipient.to_hex()],
        ["community", &offer.community],
        ["g", &offer.generation],
        ["op", op.name()],
    ])
    .map_err(|_| Error::Message)
}

fn seal(
    keys: &Keys,
    recipient: PublicKey,
    offer: &Offer,
    body: &Body,
    now: u64,
) -> Result<Event, Error> {
    offer.validate()?;
    offer.check_signed_time(now, now)?;
    if recipient == keys.public_key() || recipient.xonly().is_err() {
        return Err(Error::Enrollment);
    }
    let plaintext = Zeroizing::new(serde_json::to_vec(body).map_err(|_| Error::Create)?);
    let ciphertext = nip44::encrypt(keys.secret_key(), &recipient, &*plaintext, Version::V2)
        .map_err(|_| Error::Create)?;
    EventBuilder::new(KIND, ciphertext)
        .tags(tags(body.op, offer, recipient)?)
        .custom_created_at(Timestamp::from(now))
        .sign_with_keys(keys)
        .map_err(|_| Error::Create)
}

/// Sign the owner's claim and remember its exact identity for acknowledgement.
pub fn build_claim(
    owner: &Keys,
    offer: &Offer,
    proof: &Proof,
    now: u64,
) -> Result<(Event, SavedClaim), Error> {
    let mut body = Body::new(Operation::PairClaim, offer, owner.public_key());
    body.proof = Some(hex::encode(proof.0.as_ref()));
    let event = seal(owner, offer.host, offer, &body, now)?;
    let claim = SavedClaim::from_event(&event, offer, now);
    Ok((event, claim))
}

// Parse the actual HTTP bytes. The generic Nostr decoder normalizes key/id spellings
// and ignores unknown members before the caller can check the private v1 contract.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    id: String,
    pubkey: String,
    created_at: u64,
    kind: u32,
    tags: Vec<Tag>,
    content: String,
    sig: String,
}

// Serde also accepts positional arrays for structs. The protocol requires an object.
fn object<T: DeserializeOwned>(raw: &[u8]) -> Result<T, Error> {
    if raw.iter().copied().find(|b| !b.is_ascii_whitespace()) != Some(b'{') {
        return Err(Error::Message);
    }
    serde_json::from_slice(raw).map_err(|_| Error::Message)
}

fn parse(raw: &[u8]) -> Result<Event, Error> {
    if raw.len() > MAX_EVENT_BYTES {
        return Err(Error::Message);
    }
    let wire: Wire = object(raw)?;
    if wire.kind != KIND_PAIRED_HOST_V1
        || !canonical_hex(&wire.id, 64)
        || !canonical_hex(&wire.pubkey, 64)
        || !canonical_hex(&wire.sig, 128)
    {
        return Err(Error::Message);
    }
    Ok(Event::new(
        EventId::from_hex(&wire.id).map_err(|_| Error::Message)?,
        PublicKey::from_hex(&wire.pubkey).map_err(|_| Error::Message)?,
        Timestamp::from(wire.created_at),
        KIND,
        wire.tags,
        wire.content,
        wire.sig.parse::<Signature>().map_err(|_| Error::Message)?,
    ))
}

enum Reader<'a> {
    Owner(&'a Keys),
    Host(&'a Keys),
}

fn open(
    event: &Event,
    reader: Reader<'_>,
    owner: PublicKey,
    offer: &Offer,
    op: Operation,
    now: u64,
) -> Result<Body, Error> {
    offer.validate()?;
    let (from, to) = match op {
        Operation::PairClaim => (owner, offer.host),
        Operation::PairAck => (offer.host, owner),
    };
    let (keys, local, peer) = match reader {
        Reader::Owner(keys) => (keys, owner, offer.host),
        Reader::Host(keys) => (keys, offer.host, owner),
    };
    if from == to
        || event.pubkey != from
        || keys.public_key() != local
        || event.tags != tags(op, offer, to)?
    {
        return Err(Error::Message);
    }
    offer.check_signed_time(event.created_at.as_secs(), now)?;
    crate::verify_event(event).map_err(|_| Error::Message)?;
    let plaintext = Zeroizing::new(
        nip44::decrypt_to_bytes(keys.secret_key(), &peer, &event.content)
            .map_err(|_| Error::Message)?,
    );
    if plaintext.len() > MAX_PLAINTEXT_BYTES {
        return Err(Error::Message);
    }
    let body: Body = object(&plaintext)?;
    body.check(op, offer, owner)?;
    Ok(body)
}

/// Recover the original owner-signed claim after restart, without keeping
/// plaintext proof. Load the offer and expected claim ID from trusted local
/// state. An expired claim permits only historical acknowledgement verification.
pub fn restore_claim(
    raw: &[u8],
    owner: &Keys,
    offer: &Offer,
    expected_id: EventId,
    now: u64,
) -> Result<SavedClaim, Error> {
    let event = parse(raw)?;
    if event.id != expected_id {
        return Err(Error::Message);
    }
    open(
        &event,
        Reader::Owner(owner),
        owner.public_key(),
        offer,
        Operation::PairClaim,
        now,
    )?;
    Ok(SavedClaim::from_event(&event, offer, now))
}

/// Verify a fresh claim against local offer data. Explicitly pass `None` for
/// first enrollment when any owner with the console proof may claim the host.
/// Only `Error::Proof` represents an authenticated, correctly scoped wrong proof.
pub fn verify_claim(
    raw: &[u8],
    host: &Keys,
    offer: &Offer,
    pinned_owner: Option<PublicKey>,
    expected_digest: &[u8; 32],
    now: u64,
) -> Result<VerifiedClaim, Error> {
    let event = parse(raw)?;
    let owner = pinned_owner.unwrap_or(event.pubkey);
    if now >= offer.expires_at {
        return Err(Error::Time);
    }
    let body = open(
        &event,
        Reader::Host(host),
        owner,
        offer,
        Operation::PairClaim,
        now,
    )?;
    let proof = Proof::from_hex(body.proof.as_deref().ok_or(Error::Message)?)
        .map_err(|_| Error::Message)?;
    if !bool::from(proof.digest().ct_eq(expected_digest)) {
        return Err(Error::Proof(event.id));
    }
    Ok(VerifiedClaim(SavedClaim::from_event(&event, offer, now)))
}

/// Sign an acknowledgement referencing the exact proof-verified claim.
/// An owner-saved outbound packet is not sufficient authority:
///
/// ```compile_fail
/// use buzz_core::{Keys, paired_host::{SavedClaim, build_ack}};
/// fn bypass(host: &Keys, saved: &SavedClaim, now: u64) {
///     let _ = build_ack(host, saved, now);
/// }
/// ```
pub fn build_ack(host: &Keys, verified: &VerifiedClaim, now: u64) -> Result<Event, Error> {
    let claim = &verified.0;
    if host.public_key() != claim.offer.host {
        return Err(Error::Enrollment);
    }
    if now < claim.checked_at || now < claim.created_at.saturating_sub(SKEW) {
        return Err(Error::Time);
    }
    let mut body = Body::new(Operation::PairAck, &claim.offer, claim.owner);
    body.claim_id = Some(claim.id.to_hex());
    seal(host, claim.owner, &claim.offer, &body, now)
}

/// Verify the host acknowledgement against the owner's saved, exact claim.
/// At or after expiry the signed receipt is historical evidence only.
pub fn verify_ack(
    raw: &[u8],
    owner: &Keys,
    claim: &SavedClaim,
    now: u64,
) -> Result<Acknowledgement, Error> {
    let event = parse(raw)?;
    if now < claim.checked_at {
        return Err(Error::Time);
    }
    let body = open(
        &event,
        Reader::Owner(owner),
        claim.owner,
        &claim.offer,
        Operation::PairAck,
        now,
    )?;
    if event.created_at.as_secs() < claim.created_at.saturating_sub(SKEW) {
        return Err(Error::Time);
    }
    if body.claim_id.as_deref() != Some(claim.id.to_hex().as_str()) {
        return Err(Error::Message);
    }
    Ok(if now < claim.offer.expires_at {
        Acknowledgement::Fresh(event.id)
    } else {
        Acknowledgement::Historical(event.id)
    })
}

#[cfg(test)]
mod tests;
