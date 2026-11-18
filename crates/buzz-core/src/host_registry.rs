//! Fork protocol v2: signed, private host registration on the root WebSocket.
//! This capability grants no community membership or agent-management rights.
use crate::{
    kind::KIND_HOST_REGISTRY_V2,
    paired_host::{Error, Proof},
    paired_message::Scope,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use nostr::{
    nips::nip44::{self, Version},
    Event, EventBuilder, Keys, Kind, PublicKey, Tag, Timestamp,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

/// Serialized peer event limit; separate from immutable v1 limits.
pub const MAX_EVENT: usize = 16 * 1024;
/// Decrypted peer payload ceiling.
pub const MAX_PAYLOAD: usize = 8 * 1024;
/// Plaintext response ceiling; encrypted EVENT frame must also fit MAX_FRAME.
pub const MAX_PAGE: usize = 24 * 1024;
/// Strict outbound frame ceiling, below the normal WS 64 KiB ceiling.
pub const MAX_FRAME: usize = 60 * 1024;

/// Canonical 32-byte lowercase identifier.
pub fn identifier(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
/// Decode a canonical identifier.
pub fn bytes(s: &str) -> Result<[u8; 32], Error> {
    if !identifier(s) {
        return Err(Error::Message);
    }
    let mut b = [0; 32];
    hex::decode_to_slice(s, &mut b).map_err(|_| Error::Message)?;
    Ok(b)
}
/// Canonical selected HTTPS community origin, shared with v1 invitation validation.
pub fn origin(s: &str) -> bool {
    crate::paired_host::canonical_origin(s)
}
/// Strict object decoding retains duplicate-field rejection and bounded JSON depth.
pub fn object<T: DeserializeOwned>(raw: &[u8], limit: usize) -> Result<T, Error> {
    if raw.len() > limit || raw.iter().copied().find(|b| !b.is_ascii_whitespace()) != Some(b'{') {
        return Err(Error::Message);
    }
    serde_json::from_slice(raw).map_err(|_| Error::Message)
}

/// Exact installed runtime query; no agent identity or caller-controlled execution data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelsQuery {
    /// Accepted enrollment acknowledgment.
    pub ack_id: String,
    /// Fresh request challenge.
    pub nonce: String,
    /// Exact approved installed runtime.
    pub runtime_id: String,
    /// Current installed catalog revision.
    pub catalog_revision: String,
}
impl ModelsQuery {
    fn valid(&self) -> bool {
        identifier(&self.ack_id)
            && identifier(&self.nonce)
            && crate::paired_message::runtime_id(&self.runtime_id)
            && identifier(&self.catalog_revision)
    }
}
/// Independent, bounded host discovery evidence without an agent envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostModels {
    /// Exact request context.
    pub request: ModelsQuery,
    /// Host observation time.
    pub observed_at: u64,
    /// Shared main model projection.
    pub models: crate::host_deployment::observation::Models,
    /// Native account readiness, independent of model enumeration.
    pub readiness: crate::harness::Readiness,
    /// Installed runtime classification.
    pub availability: crate::harness::AcpAvailabilityStatus,
}
/// Operation-specific encrypted data. Every variant has a closed schema.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Body {
    /// Console proof is visible only to the host.
    PairClaim {
        /// Private single-use console proof.
        proof: String,
    },
    /// Exact selected claim, never an invitation replay.
    PairAck {
        /// Exact selected claim ID.
        claim_id: String,
    },
    /// Fresh readiness query; no execution authority.
    StatusQuery {
        /// Accepted enrollment acknowledgment.
        ack_id: String,
        /// Fresh random challenge.
        nonce: String,
    },
    /// Authenticated readiness response; labels are private display data.
    Status {
        /// Accepted enrollment acknowledgment.
        ack_id: String,
        /// Exact query nonce.
        nonce: String,
        /// Durable increasing host sequence.
        sequence: u64,
        /// Active or locally revoked.
        state: String,
        /// Short private display label.
        label: String,
    },
    /// On-demand discovery before agent creation.
    ModelsQuery {
        /// Exact selected runtime and enrollment.
        request: ModelsQuery,
    },
    /// Correlated host response, never execution authority.
    Models {
        /// Sanitized discovery evidence.
        result: HostModels,
    },
    /// Owner's restrictive intent.
    Unpair {
        /// Accepted enrollment acknowledgment.
        ack_id: String,
    },
    /// Host's durable restrictive completion.
    Unpaired {
        /// Accepted enrollment acknowledgment.
        ack_id: String,
    },
}
impl Body {
    /// Signed operation corresponding to this payload.
    pub fn operation(&self) -> &'static str {
        match self {
            Self::PairClaim { .. } => "pair-claim",
            Self::PairAck { .. } => "pair-ack",
            Self::StatusQuery { .. } => "status-query",
            Self::Status { .. } => "status",
            Self::ModelsQuery { .. } => "models-query",
            Self::Models { .. } => "models",
            Self::Unpair { .. } => "unpair",
            Self::Unpaired { .. } => "unpaired",
        }
    }
    fn valid(&self) -> bool {
        match self {
            Self::PairClaim { proof } => identifier(proof),
            Self::PairAck { claim_id } => identifier(claim_id),
            Self::StatusQuery { ack_id, nonce } => identifier(ack_id) && identifier(nonce),
            Self::Status {
                ack_id,
                nonce,
                sequence,
                state,
                label,
            } => {
                identifier(ack_id)
                    && identifier(nonce)
                    && *sequence > 0
                    && matches!(state.as_str(), "active" | "revoked")
                    && crate::paired_message::display_name(label)
            }
            Self::ModelsQuery { request } => request.valid(),
            Self::Models { result } => {
                result.request.valid()
                    && match &result.models {
                        crate::host_deployment::observation::Models::Available { catalog } => {
                            crate::host_deployment::observation::validate_models(catalog).is_ok()
                        }
                        _ => true,
                    }
            }
            Self::Unpair { ack_id } | Self::Unpaired { ack_id } => identifier(ack_id),
        }
    }
}

/// Public routing authority derived exclusively from verified signed tags.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Header {
    /// Explicit fork version.
    pub v: u8,
    /// Canonical origin of the bound tenant.
    pub community: String,
    /// Immutable owner key.
    pub owner: String,
    /// Immutable host key.
    pub host: String,
    /// Enrollment generation.
    pub generation: String,
    /// Signed operation name.
    pub op: String,
    /// Stable operation identity, persisted with the signed event before sending.
    pub operation_id: String,
    /// Exact parent ID; null in decrypted JSON corresponds only to signed '-'.
    pub parent: Option<String>,
    /// Exclusive expiry, duplicated in encrypted data.
    pub expires_at: u64,
}
impl Header {
    /// Trusted peer scope.
    pub fn scope(&self) -> Result<Scope, Error> {
        Ok(Scope {
            community: self.community.clone(),
            generation: self.generation.clone(),
            owner: PublicKey::from_hex(&self.owner).map_err(|_| Error::Message)?,
            host: PublicKey::from_hex(&self.host).map_err(|_| Error::Message)?,
        })
    }
    /// Author role for an operation; unknown operations are rejected.
    pub fn owner_sends(&self) -> Result<bool, Error> {
        match self.op.as_str() {
            "pair-claim" | "status-query" | "models-query" | "unpair" => Ok(true),
            "pair-ack" | "status" | "models" | "unpaired" => Ok(false),
            _ => Err(Error::Message),
        }
    }
    fn tags(&self, recipient: PublicKey) -> Result<Vec<Tag>, Error> {
        let expiry = self.expires_at.to_string();
        let recipient = recipient.to_hex();
        [
            ("v", "2"),
            ("community", self.community.as_str()),
            ("o", self.owner.as_str()),
            ("host", self.host.as_str()),
            ("g", self.generation.as_str()),
            ("p", recipient.as_str()),
            ("op", self.op.as_str()),
            ("i", self.operation_id.as_str()),
            ("prev", self.parent.as_deref().unwrap_or("-")),
            ("expiration", expiry.as_str()),
        ]
        .into_iter()
        .map(|(k, v)| Tag::parse([k, v]).map_err(|_| Error::Message))
        .collect()
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Payload {
    header: Header,
    body: Body,
}
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

/// Check raw bytes before the normal SDK can normalize spelling or drop fields.
pub fn parse_event(raw: &[u8], limit: usize) -> Result<Event, Error> {
    let w: Wire = object(raw, limit)?;
    if w.kind != KIND_HOST_REGISTRY_V2
        || !identifier(&w.id)
        || !identifier(&w.pubkey)
        || w.sig.len() != 128
        || !w
            .sig
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(Error::Message);
    }
    let e: Event = serde_json::from_slice(raw).map_err(|_| Error::Message)?;
    crate::verify_event(&e).map_err(|_| Error::Message)?;
    // The bounded strict wire decoder intentionally consumes every field.
    if w.tags.len() > 10
        || w.tags
            .iter()
            .any(|t| t.as_slice().len() != 2 || t.as_slice().iter().any(|s| s.len() > 300))
        || w.content.is_empty()
        || w.created_at != e.created_at.as_secs()
    {
        return Err(Error::Message);
    }
    Ok(e)
}

/// Verify immutable signed routing. Current authority is checked by the DB writer.
pub fn verify(raw: &[u8], community: &str, signer: PublicKey) -> Result<(Event, Header), Error> {
    let e = parse_event(raw, MAX_EVENT)?;
    if e.pubkey != signer || e.tags.len() != 10 {
        return Err(Error::Message);
    }
    let tags: Vec<_> = e.tags.iter().map(|t| t.as_slice()).collect();
    let names = [
        "v",
        "community",
        "o",
        "host",
        "g",
        "p",
        "op",
        "i",
        "prev",
        "expiration",
    ];
    if tags.iter().zip(names).any(|(t, n)| t[0] != n)
        || tags[0][1] != "2"
        || tags[1][1] != community
        || !origin(community)
    {
        return Err(Error::Message);
    }
    for i in [2, 3, 4, 5, 7] {
        bytes(&tags[i][1])?;
    }
    let expiry = tags[9][1].parse::<u64>().map_err(|_| Error::Message)?;
    if expiry.to_string() != tags[9][1] {
        return Err(Error::Message);
    }
    let parent = if tags[8][1] == "-" {
        None
    } else {
        bytes(&tags[8][1])?;
        Some(tags[8][1].clone())
    };
    let h = Header {
        v: 2,
        community: community.into(),
        owner: tags[2][1].clone(),
        host: tags[3][1].clone(),
        generation: tags[4][1].clone(),
        op: tags[6][1].clone(),
        operation_id: tags[7][1].clone(),
        parent,
        expires_at: expiry,
    };
    let s = h.scope()?;
    let (author, recipient) = if h.owner_sends()? {
        (s.owner, s.host)
    } else {
        (s.host, s.owner)
    };
    if s.owner == s.host
        || author != signer
        || recipient.to_hex() != tags[5][1]
        || s.owner.xonly().is_err()
        || s.host.xonly().is_err()
        || expiry <= e.created_at.as_secs()
    {
        return Err(Error::Message);
    }
    let data = STANDARD.decode(&e.content).map_err(|_| Error::Message)?;
    if data.len() < 99 || data.first() != Some(&2) {
        return Err(Error::Message);
    }
    Ok((e, h))
}

/// Build a peer event with a caller-persisted operation identity.
pub fn seal(
    keys: &Keys,
    scope: &Scope,
    body: Body,
    operation_id: String,
    parent: Option<String>,
    now: u64,
    expires_at: u64,
) -> Result<Event, Error> {
    let h = Header {
        v: 2,
        community: scope.community.clone(),
        owner: scope.owner.to_hex(),
        host: scope.host.to_hex(),
        generation: scope.generation.clone(),
        op: body.operation().into(),
        operation_id,
        parent,
        expires_at,
    };
    if !body.valid() {
        return Err(Error::Message);
    }
    let recipient = if h.owner_sends()? {
        scope.host
    } else {
        scope.owner
    };
    let payload = Zeroizing::new(
        serde_json::to_vec(&Payload {
            header: h.clone(),
            body,
        })
        .map_err(|_| Error::Create)?,
    );
    if payload.len() > MAX_PAYLOAD {
        return Err(Error::Message);
    }
    let content = nip44::encrypt(keys.secret_key(), &recipient, &*payload, Version::V2)
        .map_err(|_| Error::Create)?;
    let e = EventBuilder::new(Kind::Custom(KIND_HOST_REGISTRY_V2 as u16), content)
        .tags(h.tags(recipient)?)
        .custom_created_at(Timestamp::from(now))
        .sign_with_keys(keys)
        .map_err(|_| Error::Create)?;
    verify(
        &serde_json::to_vec(&e).map_err(|_| Error::Create)?,
        &scope.community,
        keys.public_key(),
    )?;
    Ok(e)
}

/// Open an exact peer event, checking every encrypted duplicate against local scope.
/// Historical claim/ack evidence is checked at its signed time by the caller.
pub fn open(keys: &Keys, scope: &Scope, e: &Event, now: u64) -> Result<(Header, Body), Error> {
    let (e, h) = verify(
        &serde_json::to_vec(e).map_err(|_| Error::Message)?,
        &scope.community,
        e.pubkey,
    )?;
    if h.scope()? != *scope
        || (keys.public_key() != scope.owner && keys.public_key() != scope.host)
        || e.created_at.as_secs() > now.saturating_add(30)
        || now >= h.expires_at
    {
        return Err(Error::Message);
    }
    let peer = if keys.public_key() == scope.owner {
        scope.host
    } else {
        scope.owner
    };
    let plain = Zeroizing::new(
        nip44::decrypt(keys.secret_key(), &peer, &e.content).map_err(|_| Error::Message)?,
    );
    let p: Payload = object(plain.as_bytes(), MAX_PAYLOAD)?;
    if p.header != h || p.body.operation() != h.op || !p.body.valid() {
        return Err(Error::Message);
    }
    match &p.body {
        Body::PairClaim { .. } if h.parent.is_some() => return Err(Error::Message),
        Body::PairAck { claim_id } if h.parent.as_ref() != Some(claim_id) => {
            return Err(Error::Message)
        }
        Body::ModelsQuery { request } if h.parent.as_ref() != Some(&request.ack_id) => {
            return Err(Error::Message)
        }
        Body::Models { result }
            if result.observed_at > now.saturating_add(30)
                || result.observed_at >= h.expires_at =>
        {
            return Err(Error::Message)
        }
        Body::StatusQuery { ack_id, .. } | Body::Unpair { ack_id } | Body::Unpaired { ack_id }
            if h.parent.as_ref() != Some(ack_id) =>
        {
            return Err(Error::Message)
        }
        _ => {}
    }
    Ok((h, p.body))
}
/// Constant-time proof-digest comparison using the existing secret primitive.
pub fn proof_matches(proof: &str, digest: &[u8; 32]) -> Result<bool, Error> {
    Ok(bool::from(Proof::from_hex(proof)?.digest().ct_eq(digest)))
}

/// Closed fork REQ extension. Cursor is opaque and never conveys permission.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Query {
    /// Must be version2.
    pub v: u8,
    /// One supported private view.
    pub view: String,
    /// Correlation nonce independent of mutation identities.
    pub nonce: String,
    /// Scoped authenticated continuation, or explicit null.
    #[serde(deserialize_with = "required_cursor")]
    pub cursor: Option<String>,
    /// Required for scoped views, forbidden for owner lists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation: Option<String>,
    /// Exact event for the evidence view only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
}
impl Query {
    /// Validate the complete closed view shape.
    pub fn validate(&self) -> Result<(), Error> {
        let scoped = matches!(
            self.view.as_str(),
            "binding" | "offers" | "evidence" | "messages"
        );
        if self.v != 2
            || !identifier(&self.nonce)
            || !matches!(
                self.view.as_str(),
                "hosts" | "history" | "binding" | "offers" | "evidence" | "messages" | "live"
            )
            || self.cursor.as_ref().is_some_and(|s| s.len() > 512)
            || self.generation.as_ref().is_some_and(|s| !identifier(s))
            || self.event_id.as_ref().is_some_and(|s| !identifier(s))
            || (scoped && self.generation.is_none())
            || (!scoped && self.view != "live" && self.generation.is_some())
            || ((self.view == "evidence") != self.event_id.is_some())
        {
            return Err(Error::Message);
        }
        Ok(())
    }
}

/// Relay-signed encrypted snapshot/invalidation, never a durable inbound event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reply {
    /// Fork version.
    pub v: u8,
    /// Bound community.
    pub community: String,
    /// Exact authenticated recipient.
    pub recipient: String,
    /// registry-page or registry-change.
    pub op: String,
    /// Original REQ extension; correlates nonce, view and generation.
    pub query: Query,
    /// Opaque tenant/principal/view-bound commit high-water; never a global DB sequence.
    pub high_water: String,
    /// Compact projection, references or one exact event.
    pub data: serde_json::Value,
    /// Next opaque cursor, if incomplete.
    pub next_cursor: Option<String>,
    /// Complete result only when no further cursor is needed.
    pub complete: bool,
}
/// Seal a bounded transient reply to the authenticated reader.
pub fn reply(keys: &Keys, reader: PublicKey, r: Reply, now: u64) -> Result<Event, Error> {
    r.query.validate()?;
    if r.v != 2
        || !identifier(&r.high_water)
        || !origin(&r.community)
        || r.recipient != reader.to_hex()
        || !matches!(r.op.as_str(), "registry-page" | "registry-change")
        || r.complete != r.next_cursor.is_none()
    {
        return Err(Error::Message);
    }
    let plain = Zeroizing::new(serde_json::to_vec(&r).map_err(|_| Error::Create)?);
    if plain.len() > MAX_PAGE {
        return Err(Error::Message);
    }
    let content = nip44::encrypt(keys.secret_key(), &reader, &*plain, Version::V2)
        .map_err(|_| Error::Create)?;
    let mut tags = vec![
        Tag::parse(["v", "2"]),
        Tag::parse(["community", &r.community]),
        Tag::parse(["p", &r.recipient]),
        Tag::parse(["op", &r.op]),
        Tag::parse(["q", &r.query.nonce]),
        Tag::parse(["view", &r.query.view]),
    ]
    .into_iter()
    .collect::<Result<Vec<_>, _>>()
    .map_err(|_| Error::Create)?;
    if let Some(g) = &r.query.generation {
        tags.push(Tag::parse(["g", g]).map_err(|_| Error::Create)?);
    }
    let e = EventBuilder::new(Kind::Custom(KIND_HOST_REGISTRY_V2 as u16), content)
        .tags(tags)
        .custom_created_at(Timestamp::from(now))
        .sign_with_keys(keys)
        .map_err(|_| Error::Create)?;
    if serde_json::to_vec(&e).map_err(|_| Error::Create)?.len() + 300 >= MAX_FRAME {
        return Err(Error::Message);
    }
    Ok(e)
}
/// Verify selected-tenant relay identity and correlated private reply before use.
pub fn open_reply(
    keys: &Keys,
    relay: PublicKey,
    community: &str,
    q: &Query,
    e: &Event,
) -> Result<Reply, Error> {
    let e = parse_event(
        &serde_json::to_vec(e).map_err(|_| Error::Message)?,
        MAX_FRAME,
    )?;
    if e.pubkey != relay {
        return Err(Error::Message);
    }
    let plain = Zeroizing::new(
        nip44::decrypt(keys.secret_key(), &relay, &e.content).map_err(|_| Error::Message)?,
    );
    let r: Reply = object(plain.as_bytes(), MAX_PAGE)?;
    if r.v != 2
        || !identifier(&r.high_water)
        || r.community != community
        || r.recipient != keys.public_key().to_hex()
        || serde_json::to_value(&r.query).map_err(|_| Error::Message)?
            != serde_json::to_value(q).map_err(|_| Error::Message)?
        || r.complete != r.next_cursor.is_none()
    {
        return Err(Error::Message);
    }
    let expected = reply_tags(&r)?;
    if e.tags
        .iter()
        .map(|t| t.as_slice().to_vec())
        .collect::<Vec<_>>()
        != expected
    {
        return Err(Error::Message);
    }
    Ok(r)
}
fn reply_tags(r: &Reply) -> Result<Vec<Vec<String>>, Error> {
    if !matches!(r.op.as_str(), "registry-page" | "registry-change") {
        return Err(Error::Message);
    }
    let mut tags = vec![
        vec!["v".into(), "2".into()],
        vec!["community".into(), r.community.clone()],
        vec!["p".into(), r.recipient.clone()],
        vec!["op".into(), r.op.clone()],
        vec!["q".into(), r.query.nonce.clone()],
        vec!["view".into(), r.query.view.clone()],
    ];
    if let Some(g) = &r.query.generation {
        tags.push(vec!["g".into(), g.clone()]);
    }
    Ok(tags)
}

fn required_cursor<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    Option::<String>::deserialize(d)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn scope() -> (Keys, Keys, Scope) {
        let o = Keys::generate();
        let h = Keys::generate();
        let s = Scope {
            community: "https://registry.test:8443".into(),
            generation: "ab".repeat(32),
            owner: o.public_key(),
            host: h.public_key(),
        };
        (o, h, s)
    }
    #[test]
    fn model_query_is_agent_free_closed_scoped_and_correlated() {
        let (o, h, s) = scope();
        let request = ModelsQuery {
            ack_id: "11".repeat(32),
            nonce: "22".repeat(32),
            runtime_id: "main/claude".into(),
            catalog_revision: "33".repeat(32),
        };
        let body = Body::ModelsQuery {
            request: request.clone(),
        };
        let query = seal(
            &o,
            &s,
            body.clone(),
            "44".repeat(32),
            Some(request.ack_id.clone()),
            100,
            160,
        )
        .unwrap();
        assert_eq!(open(&h, &s, &query, 101).unwrap().1, body);
        let wrong = seal(
            &o,
            &s,
            body.clone(),
            "45".repeat(32),
            Some("ff".repeat(32)),
            100,
            160,
        )
        .unwrap();
        assert!(open(&h, &s, &wrong, 101).is_err());
        for field in ["owner", "host", "community", "generation"] {
            let mut foreign = s.clone();
            match field {
                "owner" => foreign.owner = Keys::generate().public_key(),
                "host" => foreign.host = Keys::generate().public_key(),
                "community" => foreign.community = "https://other.test".into(),
                _ => foreign.generation = "ff".repeat(32),
            }
            assert!(open(&h, &foreign, &query, 101).is_err(), "{field}");
        }
        let mut raw = serde_json::to_value(&body).unwrap();
        raw["request"]["command"] = serde_json::json!("arbitrary");
        assert!(serde_json::from_value::<Body>(raw).is_err());
        let result = HostModels {
            request,
            observed_at: 102,
            models: crate::host_deployment::observation::Models::Empty,
            readiness: crate::harness::Readiness::LoggedIn,
            availability: crate::harness::AcpAvailabilityStatus::Available,
        };
        let reply = seal(
            &h,
            &s,
            Body::Models {
                result: result.clone(),
            },
            "55".repeat(32),
            Some(query.id.to_hex()),
            102,
            160,
        )
        .unwrap();
        let (header, decoded) = open(&o, &s, &reply, 103).unwrap();
        assert_eq!(header.parent, Some(query.id.to_hex()));
        assert_eq!(decoded, Body::Models { result });
        assert!(open(&o, &s, &reply, 160).is_err());
        assert!(seal(
            &o,
            &s,
            decoded,
            "66".repeat(32),
            Some(query.id.to_hex()),
            102,
            160
        )
        .is_err());
    }
    #[test]
    fn strict_signed_v2_roundtrip_and_v1_separation() {
        let (o, h, s) = scope();
        let e = seal(
            &o,
            &s,
            Body::PairClaim {
                proof: "cd".repeat(32),
            },
            "ef".repeat(32),
            None,
            100,
            700,
        )
        .unwrap();
        let (header, body) = open(&h, &s, &e, 101).unwrap();
        assert_eq!(header.operation_id, "ef".repeat(32));
        assert!(matches!(body, Body::PairClaim { .. }));
        assert!(crate::paired_host::transport::verify_outer(
            &serde_json::to_vec(&e).unwrap(),
            &s.community,
            &s.generation,
            o.public_key()
        )
        .is_err());
        let mut foreign = s.clone();
        foreign.community = "https://other.test".into();
        assert!(open(&h, &foreign, &e, 101).is_err());
        let raw = serde_json::to_string(&e).unwrap();
        let duplicate = raw.replacen("{", "{\"kind\":43103,", 1);
        assert!(parse_event(duplicate.as_bytes(), MAX_EVENT).is_err());
        let mut tags = e.tags.clone().to_vec();
        tags.push(Tag::parse(["h", "private-channel"]).unwrap());
        let duplicate = EventBuilder::new(e.kind, e.content.clone())
            .tags(tags)
            .custom_created_at(e.created_at)
            .sign_with_keys(&o)
            .unwrap();
        assert!(verify(
            &serde_json::to_vec(&duplicate).unwrap(),
            &s.community,
            o.public_key()
        )
        .is_err());
        assert!(open(&h, &s, &e, 700).is_err());
        assert!(!origin("https://localhost/path"));
        assert!(!origin("https://127.0.0.1"));
        assert!(!origin("https://u@registry.test"));
        assert!(!identifier(&"AB".repeat(32)));
    }
    #[test]
    fn correlated_reply_bounds_and_closed_query_schema() {
        let (o, _, s) = scope();
        let relay = Keys::generate();
        let q = Query {
            v: 2,
            view: "hosts".into(),
            nonce: "12".repeat(32),
            cursor: None,
            generation: None,
            event_id: None,
        };
        let r = Reply {
            v: 2,
            community: s.community.clone(),
            recipient: o.public_key().to_hex(),
            op: "registry-page".into(),
            query: q.clone(),
            high_water: "00".repeat(32),
            data: serde_json::json!({"hosts":[]}),
            next_cursor: None,
            complete: true,
        };
        let e = reply(&relay, o.public_key(), r.clone(), 100).unwrap();
        assert!(open_reply(&o, relay.public_key(), &s.community, &q, &e).is_ok());
        let mut wrong = q.clone();
        wrong.nonce = "34".repeat(32);
        assert!(open_reply(&o, relay.public_key(), &s.community, &wrong, &e).is_err());
        assert!(open_reply(&o, Keys::generate().public_key(), &s.community, &q, &e).is_err());
        let mut huge = r;
        huge.data = serde_json::json!({"unbounded":"x".repeat(MAX_PAGE)});
        assert!(reply(&relay, o.public_key(), huge, 100).is_err());
        for raw in [
            r#"{"v":2,"view":"hosts","nonce":"aa","cursor":null,"owner":"x"}"#,
            r#"{"v":2,"view":"hosts","nonce":"aa"}"#,
            r#"{"v":2,"v":2,"view":"hosts","nonce":"aa","cursor":null}"#,
        ] {
            assert!(object::<Query>(raw.as_bytes(), 4096).is_err());
        }
        let mut long = q;
        long.cursor = Some("x".repeat(513));
        assert!(long.validate().is_err());
        assert!(object::<Query>(
            format!("{}0{}", "[".repeat(129), "]".repeat(129)).as_bytes(),
            4096
        )
        .is_err());
    }
}
