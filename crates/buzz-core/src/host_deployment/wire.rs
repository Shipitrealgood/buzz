//! Private deployment extension. Registration v2 remains an independent protocol.
use super::*;
use crate::{host_registry as registry, kind::KIND_HOST_DEPLOYMENT_V1};
use nostr::{
    nips::nip44::{self, Version},
    Event, EventBuilder, Kind, Tag, Timestamp,
};
use serde::de::DeserializeOwned;
use zeroize::Zeroizing;

/// All frames stay below the ordinary WebSocket ceiling, including encryption.
pub const MAX_FRAME: usize = 60 * 1024;
/// A command is returned inside an encrypted private response.
pub const MAX_EVENT: usize = 32 * 1024;
/// Maximum lifetime of one immediately consumed execution permit.
pub const PERMIT_SECONDS: u64 = 10;
/// Reserved restrictive-operation namespace; ordinary Deploy IDs must exclude it.
pub const CLEANUP_OPERATION_PREFIX: &str = "c1ea0001";
fn validate_command_id(a: &Authority, created: u64) -> Result<()> {
    if a.operation_id.starts_with(CLEANUP_OPERATION_PREFIX)
        && (a.action != Action::Cleanup || a.operation_id != cleanup_operation_id(a, created)?)
    {
        return Err(Error);
    }
    Ok(())
}

/// Signed routing metadata. Content remains encrypted to the other participant.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Header {
    /// Owner-signed immutable operation.
    Command {
        /// Exact operation authority and ciphertext commitment.
        authority: Authority,
    },
    /// Host-signed correlated result.
    Result {
        /// Correlated observed state; never inferred from presence.
        result: DeploymentResult,
    },
    /// Host-signed, expiring installed-catalog observation.
    Status {
        /// Existing private enrollment.
        scope: Scope,
        /// Exclusive observation freshness deadline.
        expires_at: u64,
    },
}
impl Header {
    /// Exact participant binding.
    pub fn scope(&self) -> &Scope {
        match self {
            Self::Command { authority } => &authority.scope,
            Self::Result { result } => &result.request.scope,
            Self::Status { scope, .. } => scope,
        }
    }
    fn sender(&self) -> PublicKey {
        match self {
            Self::Command { .. } => self.scope().owner,
            _ => self.scope().host,
        }
    }
    fn recipient(&self) -> PublicKey {
        match self {
            Self::Command { .. } => self.scope().host,
            _ => self.scope().owner,
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEvent {
    id: String,
    pubkey: String,
    created_at: u64,
    kind: u32,
    tags: Vec<Tag>,
    content: String,
    sig: String,
}
fn parse(raw: &[u8], maximum: usize) -> Result<Event> {
    let r: RawEvent = registry::object(raw, maximum).map_err(|_| Error)?;
    if r.kind != KIND_HOST_DEPLOYMENT_V1
        || !registry::identifier(&r.id)
        || !registry::identifier(&r.pubkey)
        || r.sig.len() != 128
        || !r
            .sig
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        || r.content.is_empty()
        || r.tags.len() != 2
        || r.tags
            .iter()
            .any(|t| t.as_slice().len() != 2 || t.as_slice()[1].len() > 2048)
    {
        return Err(Error);
    }
    let e: Event = serde_json::from_slice(raw).map_err(|_| Error)?;
    crate::verify_event(&e).map_err(|_| Error)?;
    if e.created_at.as_secs() != r.created_at {
        return Err(Error);
    }
    Ok(e)
}
fn tag_values(e: &Event) -> Result<(&str, &str)> {
    let t: Vec<_> = e.tags.iter().map(|t| t.as_slice()).collect();
    if t.len() != 2
        || t[0].len() != 2
        || t[1].len() != 2
        || t[0][0] != "deployment"
        || t[1][0] != "p"
    {
        return Err(Error);
    }
    Ok((&t[0][1], &t[1][1]))
}
fn encrypted<T: Serialize>(keys: &Keys, recipient: PublicKey, value: &T) -> Result<String> {
    let raw = Zeroizing::new(serde_json::to_vec(value).map_err(|_| Error)?);
    nip44::encrypt(keys.secret_key(), &recipient, &*raw, Version::V2).map_err(|_| Error)
}
fn decrypted<T: DeserializeOwned>(keys: &Keys, e: &Event, maximum: usize) -> Result<T> {
    let raw = Zeroizing::new(
        nip44::decrypt(keys.secret_key(), &e.pubkey, &e.content).map_err(|_| Error)?,
    );
    registry::object(raw.as_bytes(), maximum).map_err(|_| Error)
}
fn signed(
    keys: &Keys,
    recipient: PublicKey,
    header: &impl Serialize,
    content: String,
    now: u64,
    maximum: usize,
) -> Result<Event> {
    let header = serde_json::to_string(header).map_err(|_| Error)?;
    let e = EventBuilder::new(Kind::Custom(KIND_HOST_DEPLOYMENT_V1 as u16), content)
        .tags([
            Tag::parse(["deployment", &header]).map_err(|_| Error)?,
            Tag::parse(["p", &recipient.to_hex()]).map_err(|_| Error)?,
        ])
        .custom_created_at(Timestamp::from(now))
        .sign_with_keys(keys)
        .map_err(|_| Error)?;
    parse(&serde_json::to_vec(&e).map_err(|_| Error)?, maximum)?;
    Ok(e)
}
/// Sign an exact previously sealed command without changing its ciphertext.
pub fn sign_intent(keys: &Keys, intent: &ProtectedCreateIntent, created: u64) -> Result<Event> {
    sign_command(keys, &intent.authority, &intent.ciphertext, created)
}
/// Sign the immutable ciphertext commitment for either supported action.
pub fn sign_command(
    keys: &Keys,
    authority: &Authority,
    ciphertext: &[u8],
    created: u64,
) -> Result<Event> {
    authority.validate(
        &authority.scope,
        authority.expected_revision,
        created,
        ciphertext,
    )?;
    validate_command_id(authority, created)?;
    if keys.public_key() != authority.scope.owner {
        return Err(Error);
    }
    signed(
        keys,
        authority.scope.host,
        &Header::Command {
            authority: authority.clone(),
        },
        String::from_utf8(ciphertext.to_vec()).map_err(|_| Error)?,
        created,
        MAX_EVENT,
    )
}
/// Content-bound restrictive ID. Retained envelopes still require exact byte replay;
/// after supersession this ID cannot be reused with fresh authority/revision.
pub fn cleanup_operation_id(authority: &Authority, created: u64) -> Result<String> {
    if authority.action != Action::Cleanup {
        return Err(Error);
    }
    let mut canonical = authority.clone();
    canonical.operation_id.clear();
    let bytes = serde_json::to_vec(&(
        "buzz-host-cleanup-replacement-v1",
        canonical,
        created,
        KIND_HOST_DEPLOYMENT_V1,
    ))
    .map_err(|_| Error)?;
    let digest = hex::encode(Sha256::digest(bytes));
    Ok(format!("{CLEANUP_OPERATION_PREFIX}{}", &digest[..56]))
}
/// Encrypt a resolved native command once; caller journals before effects.
pub fn seal_command(
    keys: &Keys,
    mut authority: Authority,
    command: &Command,
    now: u64,
) -> Result<(Authority, Event)> {
    command.validate(&authority)?;
    let content = encrypted(keys, authority.scope.host, command)?;
    authority.content_sha256 = hex::encode(Sha256::digest(content.as_bytes()));
    if authority.action == Action::Cleanup {
        authority.operation_id = cleanup_operation_id(&authority, now)?;
    }
    let event = sign_command(keys, &authority, content.as_bytes(), now)?;
    Ok((authority, event))
}
/// Verify exact signed routing; current permissions and time are writer checks.
pub fn verify(raw: &[u8], origin: &str, signer: PublicKey) -> Result<(Event, Header)> {
    let e = parse(raw, MAX_EVENT)?;
    let (h, recipient) = tag_values(&e)?;
    let h: Header = registry::object(h.as_bytes(), 2048).map_err(|_| Error)?;
    validate_scope(h.scope())?;
    if h.scope().community != origin
        || e.pubkey != signer
        || signer != h.sender()
        || recipient != h.recipient().to_hex()
    {
        return Err(Error);
    }
    match &h {
        Header::Command { authority } => {
            authority.validate(
                h.scope(),
                authority.expected_revision,
                e.created_at.as_secs(),
                e.content.as_bytes(),
            )?;
            validate_command_id(authority, e.created_at.as_secs())?;
        }
        Header::Status { expires_at, .. }
            if *expires_at <= e.created_at.as_secs()
                || expires_at.saturating_sub(e.created_at.as_secs()) > 120 =>
        {
            return Err(Error)
        }
        Header::Result { result } => {
            if e.content.len() > 4096 {
                return Err(Error);
            }
            result.validate(&result.request)?;
        }
        _ => {}
    }
    Ok((e, h))
}
/// Open a verified command for the enrolled host, including NIP-OA identity validation.
pub fn open_command(keys: &Keys, e: &Event, authority: &Authority) -> Result<Command> {
    let (_, header) = verify(
        &serde_json::to_vec(e).map_err(|_| Error)?,
        &authority.scope.community,
        authority.scope.owner,
    )?;
    if keys.public_key() != authority.scope.host
        || header
            != (Header::Command {
                authority: authority.clone(),
            })
    {
        return Err(Error);
    }
    let c: Command = decrypted(keys, e, 32 * 1024)?;
    c.validate(authority)?;
    Ok(c)
}
/// Recover the native owner's own ciphertext without exporting their key.
pub fn restore_command(keys: &Keys, authority: &Authority, ciphertext: &[u8]) -> Result<Command> {
    if keys.public_key() != authority.scope.owner
        || hex::encode(Sha256::digest(ciphertext)) != authority.content_sha256
    {
        return Err(Error);
    }
    let raw = Zeroizing::new(
        nip44::decrypt(
            keys.secret_key(),
            &authority.scope.host,
            std::str::from_utf8(ciphertext).map_err(|_| Error)?,
        )
        .map_err(|_| Error)?,
    );
    let c: Command = registry::object(raw.as_bytes(), 32 * 1024).map_err(|_| Error)?;
    c.validate(authority)?;
    Ok(c)
}
/// Sign an observed host result. The relay separately checks the accepted operation.
pub fn result(keys: &Keys, value: &DeploymentResult, now: u64) -> Result<Event> {
    value.validate(&value.request)?;
    if keys.public_key() != value.request.scope.host {
        return Err(Error);
    }
    signed(
        keys,
        value.request.scope.owner,
        &Header::Result {
            result: value.clone(),
        },
        encrypted(keys, value.request.scope.owner, value)?,
        now,
        MAX_EVENT,
    )
}
/// Sign a bounded private observation without changing the public result header.
pub fn result_body(keys: &Keys, body: &observation::ResultBody, now: u64) -> Result<Event> {
    body.validate(&body.result.request)?;
    if keys.public_key() != body.result.request.scope.host
        || body
            .observation
            .as_ref()
            .is_some_and(|o| o.observed_at > now)
    {
        return Err(Error);
    }
    let content = encrypted(keys, body.result.request.scope.owner, body)?;
    if content.len() > 4096 {
        return Err(Error);
    }
    signed(
        keys,
        body.result.request.scope.owner,
        &Header::Result {
            result: body.result.clone(),
        },
        content,
        now,
        MAX_EVENT,
    )
}
/// Verify and open a private result, preserving legacy result-only bodies.
pub fn open_result_body(
    keys: &Keys,
    event: &Event,
    request: &Authority,
) -> Result<observation::ResultBody> {
    let (_, h) = verify(
        &serde_json::to_vec(event).map_err(|_| Error)?,
        &request.scope.community,
        request.scope.host,
    )?;
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Body {
        Current(Box<observation::ResultBody>),
        Legacy(Box<DeploymentResult>),
    }
    let body = match decrypted::<Body>(keys, event, 4096)? {
        Body::Current(b) => *b,
        Body::Legacy(result) => observation::ResultBody {
            result: *result,
            observation: None,
        },
    };
    if keys.public_key() != request.scope.owner
        || h != (Header::Result {
            result: body.result.clone(),
        })
        || body
            .observation
            .as_ref()
            .is_some_and(|o| o.observed_at > event.created_at.as_secs())
    {
        return Err(Error);
    }
    body.validate(request)?;
    Ok(body)
}
/// Compatibility accessor for the exact private host outcome.
pub fn open_result(keys: &Keys, event: &Event, request: &Authority) -> Result<DeploymentResult> {
    Ok(open_result_body(keys, event, request)?.result)
}
/// Publish only actual installed catalog evidence while host management is enabled.
pub fn status(keys: &Keys, value: &Status, now: u64) -> Result<Event> {
    value.capability.validate()?;
    if keys.public_key() != value.scope.host
        || value.expires_at <= now
        || value.expires_at > now + 120
    {
        return Err(Error);
    }
    signed(
        keys,
        value.scope.owner,
        &Header::Status {
            scope: value.scope.clone(),
            expires_at: value.expires_at,
        },
        encrypted(keys, value.scope.owner, value)?,
        now,
        MAX_EVENT,
    )
}
/// Open a private host capability observation, never generic presence.
pub fn open_status(keys: &Keys, e: &Event, scope: &Scope, now: u64) -> Result<Status> {
    let (_, h) = verify(
        &serde_json::to_vec(e).map_err(|_| Error)?,
        &scope.community,
        scope.host,
    )?;
    let r: Status = decrypted(keys, e, 8192)?;
    r.capability.validate()?;
    if keys.public_key() != scope.owner
        || r.scope != *scope
        || h != (Header::Status {
            scope: scope.clone(),
            expires_at: r.expires_at,
        })
        || now >= r.expires_at
    {
        return Err(Error);
    }
    Ok(r)
}
/// Bounded private request. Nonce also identifies one execution attempt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Query {
    /// Version independent of registration v2.
    pub v: u8,
    /// Exact enrollment scope.
    pub scope: Scope,
    /// status, pending, operation, permit, or live.
    pub view: String,
    /// Fresh unpredictable one-attempt challenge.
    pub nonce: String,
    /// Exact operation for operation/permit; optional exclusive rotation point for pending.
    pub operation_id: Option<String>,
}
impl Query {
    /// Refuse ambiguous, unbounded and cross-protocol reads.
    pub fn validate(&self) -> Result<()> {
        validate_scope(&self.scope)?;
        if self.v != 1
            || !registry::identifier(&self.nonce)
            || !matches!(
                self.view.as_str(),
                "status" | "pending" | "operation" | "permit" | "live"
            )
            || (matches!(self.view.as_str(), "operation" | "permit") && self.operation_id.is_none())
            || (!matches!(self.view.as_str(), "operation" | "permit" | "pending")
                && self.operation_id.is_some())
            || self
                .operation_id
                .as_deref()
                .is_some_and(|s| !registry::identifier(s))
        {
            return Err(Error);
        }
        Ok(())
    }
}
/// Relay response encrypted to the authenticated reader; no generic event storage.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reply {
    /// Exact nonce-bearing query.
    pub query: Query,
    /// Exclusive short validity deadline.
    pub expires_at: u64,
    /// Bounded projection or exact signed peer evidence.
    pub data: serde_json::Value,
}
/// Seal a query response under the pinned relay identity.
pub fn reply(keys: &Keys, reader: PublicKey, r: &Reply, now: u64) -> Result<Event> {
    r.query.validate()?;
    signed(
        keys,
        reader,
        &r.query,
        encrypted(keys, reader, r)?,
        now,
        MAX_FRAME,
    )
}
/// Validate pinned relay, recipient, challenge, scope, signature and freshness.
pub fn open_reply(keys: &Keys, relay: PublicKey, q: &Query, e: &Event, now: u64) -> Result<Reply> {
    let e = parse(&serde_json::to_vec(e).map_err(|_| Error)?, MAX_FRAME)?;
    let (header, recipient) = tag_values(&e)?;
    let echoed: Query = registry::object(header.as_bytes(), 2048).map_err(|_| Error)?;
    q.validate()?;
    if e.pubkey != relay
        || recipient != keys.public_key().to_hex()
        || echoed != *q
        || e.created_at.as_secs() > now + 30
    {
        return Err(Error);
    }
    let r: Reply = decrypted(keys, &e, 45 * 1024)?;
    if r.query != *q || now >= r.expires_at || r.expires_at > now + PERMIT_SECONDS {
        return Err(Error);
    }
    Ok(r)
}
