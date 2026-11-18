//! Endpoint codec. Verify outer signature and pinned peer before decrypting.
//! Successful opening authenticates a packet; lifecycle transitions and matching
//! outstanding parent, nonce, local epoch and monotonic time remain caller checks.
use crate::kind::KIND_PAIRED_HOST_V1;
use nostr::{nips::nip44, EventBuilder, Keys, Kind, PublicKey, Tags, Timestamp};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use zeroize::Zeroizing;

mod body;
mod create;
pub use body::{display_name, runtime_id, Body};
pub use create::{sign_possession, verify_possession, Policy, Possession};

/// Deliberately opaque error (never print secrets, input or relay responses).
#[derive(Debug, thiserror::Error)]
#[error("invalid paired-host packet or scope")]
pub struct Error;
type Result<T> = std::result::Result<T, Error>;

/// An exact locally trusted host-owner binding.
#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scope {
    pub community: String,
    pub generation: String,
    pub owner: PublicKey,
    pub host: PublicKey,
}

/// Clear routing fields authenticated by NIP-98 on transport and cross-checked on decryption.
#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Route {
    #[serde(deserialize_with = "Deserialize::deserialize")]
    pub r: Option<String>,
    #[serde(deserialize_with = "Deserialize::deserialize")]
    pub prev: Option<String>,
    pub expires_at: u64,
}

/// Preserve actual event bytes until strict verification.
#[allow(missing_docs)]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Packet {
    pub event: Box<RawValue>,
    pub route: Route,
}

/// Authenticated message and exact event ID. Historical receipts do not confer fresh authority.
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub struct Message {
    pub id: String,
    pub created_at: u64,
    pub expires_at: u64,
    pub body: Body,
}

fn canonical_id(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn peers(scope: &Scope, from_owner: bool) -> Result<(PublicKey, PublicKey)> {
    let url = url::Url::parse(&scope.community).map_err(|_| Error)?;
    if url.scheme() != "https"
        || url.origin().ascii_serialization() != scope.community
        || !matches!(url.host(), Some(url::Host::Domain(host)) if host.len() <= 253 && host.split('.').all(|l| !l.is_empty() && l.len() <= 63 && !l.starts_with('-') && !l.ends_with('-') && l.bytes().all(|b| b.is_ascii_alphanumeric() || b==b'-')))
        || !canonical_id(&scope.generation)
        || scope.owner == scope.host
        || scope.owner.xonly().is_err()
        || scope.host.xonly().is_err()
    {
        return Err(Error);
    }
    Ok(if from_owner {
        (scope.owner, scope.host)
    } else {
        (scope.host, scope.owner)
    })
}

fn tags(scope: &Scope, recipient: PublicKey, op: &str) -> Result<Tags> {
    Tags::parse([
        ["p", &recipient.to_hex()],
        ["community", &scope.community],
        ["g", &scope.generation],
        ["op", op],
    ])
    .map_err(|_| Error)
}

#[derive(Serialize, Deserialize)]
struct Frame {
    v: u8,
    community: String,
    generation: String,
    owner: String,
    host: String,
    expires_at: u64,
    #[serde(flatten)]
    body: Body,
}

fn route(body: &Body, expires_at: u64) -> Route {
    let (r, prev) = body.references();
    Route {
        r: r.map(str::to_owned),
        prev: Some(prev.to_string()),
        expires_at,
    }
}

/// Create one immutable encrypted business packet. Persist the result before retrying.
pub fn seal(keys: &Keys, scope: &Scope, body: Body, now: u64, expires_at: u64) -> Result<Packet> {
    let (author, recipient) = peers(scope, body.from_owner())?;
    body.validate()?;
    create::verify_offer(scope, &body, expires_at)?;
    if keys.public_key() != author || expires_at <= now || expires_at - now > body.ttl() {
        return Err(Error);
    }
    let route = route(&body, expires_at);
    let tags = tags(scope, recipient, body.operation())?;
    let frame = Frame {
        v: 1,
        community: scope.community.clone(),
        generation: scope.generation.clone(),
        owner: scope.owner.to_hex(),
        host: scope.host.to_hex(),
        expires_at,
        body,
    };
    let bytes = Zeroizing::new(serde_json::to_vec(&frame).map_err(|_| Error)?);
    if bytes.len() > 16384 {
        return Err(Error);
    }
    let content = nip44::encrypt(keys.secret_key(), &recipient, &*bytes, nip44::Version::V2)
        .map_err(|_| Error)?;
    let event = EventBuilder::new(Kind::Custom(KIND_PAIRED_HOST_V1 as u16), content)
        .tags(tags)
        .custom_created_at(Timestamp::from(now))
        .sign_with_keys(keys)
        .map_err(|_| Error)?;
    let event = serde_json::value::to_raw_value(&event).map_err(|_| Error)?;
    Ok(Packet { event, route })
}

/// Read a peer-authored message. Durable evidence may be historical; all new
/// authorizing operations must pass the embedded expiry at the actual endpoint.
pub fn open(keys: &Keys, scope: &Scope, packet: &Packet, now: u64) -> Result<Message> {
    decode(keys, scope, packet, now, None, false)
}

/// Recover a locally persisted authored message by its exact saved ID. Verifies
/// its original time window, signature, sidecar, peer and content. The returned
/// history alone never proves a current challenge or a still-active grant.
pub fn restore_sent(
    keys: &Keys,
    scope: &Scope,
    packet: &Packet,
    expected_id: &str,
) -> Result<Message> {
    decode(keys, scope, packet, 0, Some(expected_id), false)
}

/// Evidence for refusing a genuinely signed but late activation. This type
/// cannot enter the live create state machine.
#[allow(missing_docs)]
pub struct ExpiredActivation {
    pub request_id: String,
    pub intent_id: String,
    pub offer_id: String,
    pub agent: String,
    pub nonce: String,
}
/// Verify an expired activate only, with all other normal checks preserved.
pub fn expired_activation(
    keys: &Keys,
    scope: &Scope,
    packet: &Packet,
    now: u64,
) -> Result<ExpiredActivation> {
    let m = decode(keys, scope, packet, now, None, true)?;
    match m.body {
        Body::Activate {
            request_id,
            intent_id,
            offer_id,
            agent,
            nonce,
        } if now >= m.expires_at => Ok(ExpiredActivation {
            request_id,
            intent_id,
            offer_id,
            agent,
            nonce,
        }),
        _ => Err(Error),
    }
}

fn decode(
    keys: &Keys,
    scope: &Scope,
    packet: &Packet,
    now: u64,
    saved: Option<&str>,
    late_activation: bool,
) -> Result<Message> {
    let local = keys.public_key();
    let peer = if local == scope.owner {
        scope.host
    } else if local == scope.host {
        scope.owner
    } else {
        return Err(Error);
    };
    let author = if saved.is_some() { local } else { peer };
    let (event, recipient, _) = crate::paired_host::transport::verify_outer(
        packet.event.get().as_bytes(),
        &scope.community,
        &scope.generation,
        author,
    )
    .map_err(|_| Error)?;
    let (_, expected_recipient) = peers(scope, author == scope.owner)?;
    if recipient != expected_recipient || saved.is_some_and(|id| event.id.to_hex() != id) {
        return Err(Error);
    }
    let bytes = Zeroizing::new(
        nip44::decrypt_to_bytes(keys.secret_key(), &peer, &event.content).map_err(|_| Error)?,
    );
    if bytes.len() > 16384
        || !bytes
            .iter()
            .copied()
            .find(|b| !b.is_ascii_whitespace())
            .is_some_and(|b| b == b'{')
    {
        return Err(Error);
    }
    let frame: Frame = serde_json::from_slice(&bytes).map_err(|_| Error)?;
    frame.body.validate()?;
    create::verify_offer(scope, &frame.body, frame.expires_at)?;
    let created_at = event.created_at.as_secs();
    let ttl = frame.body.ttl();
    if frame.v != 1
        || frame.community != scope.community
        || frame.generation != scope.generation
        || frame.owner != scope.owner.to_hex()
        || frame.host != scope.host.to_hex()
        || frame.body.from_owner() != (author == scope.owner)
        || event.tags != tags(scope, recipient, frame.body.operation())?
        || packet.route != route(&frame.body, frame.expires_at)
        || frame.expires_at <= created_at
        || frame.expires_at - created_at > ttl
        || (saved.is_none() && created_at > now.saturating_add(30))
        || (saved.is_none() && !late_activation && ttl != 86400 && now >= frame.expires_at)
    {
        return Err(Error);
    }
    Ok(Message {
        id: event.id.to_hex(),
        created_at,
        expires_at: frame.expires_at,
        body: frame.body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn queries_are_bound_to_exact_sidecar_recipient_and_deadline() {
        let (owner, host) = (Keys::generate(), Keys::generate());
        let scope = Scope {
            community: "https://team.test".into(),
            generation: "ab".repeat(32),
            owner: owner.public_key(),
            host: host.public_key(),
        };
        let query = Body::InventoryQuery {
            pair_ack_id: "cd".repeat(32),
            nonce: "ef".repeat(32),
        };
        let mut p = seal(&owner, &scope, query.clone(), 100, 160).unwrap();
        assert_eq!(open(&host, &scope, &p, 159).unwrap().body, query);
        assert!(open(&host, &scope, &p, 160).is_err());
        assert!(open(&owner, &scope, &p, 120).is_err());
        p.route.r = Some("11".repeat(32));
        assert!(open(&host, &scope, &p, 120).is_err());
        assert!(serde_json::from_str::<Route>(r#"{"prev":null,"expires_at":123}"#).is_err());
    }
    #[test]
    fn no_hidden_plaintext_members() {
        let raw = r#"{"v":1,"community":"c","generation":"g","owner":"o","host":"h","expires_at":100,"op":"unpair","pair_ack_id":"a","revocation_id":"n"}"#;
        assert!(serde_json::from_str::<Frame>(raw).is_ok());
        for key in ["revocation_id", "op", "community", "command"] {
            let s = format!("{},\"{key}\":\"other\"}}", &raw[..raw.len() - 1]);
            assert!(serde_json::from_str::<Frame>(&s).is_err(), "accepted {key}");
        }
    }
}
