//! Private registration adapter. Ordinary socket lifecycle, existing paired writer.
use crate::{
    connection::{AuthState, ConnectionState},
    metrics::AuthOutcome,
    protocol::RelayMessage,
    state::AppState,
};
use buzz_core::{host_registry as wire, kind::KIND_HOST_REGISTRY_V2};
use buzz_db::paired_hosts::RegistryRead;
use hmac::{Hmac, KeyInit, Mac};
use nostr::{Event, EventBuilder, Kind, Tag};
use serde::{Deserialize, Serialize};
use serde_json::{json, value::RawValue};
use sha2::{Digest, Sha256};
use std::sync::{Arc, OnceLock};
use tokio::sync::Semaphore;
mod support;

/// Returns true for frames that need private routing before ordinary filters discard extensions.
pub(crate) fn private_frame(frame: &serde_json::Value, scoped: bool) -> bool {
    let Some(parts) = frame.as_array() else {
        return false;
    };
    match parts.first().and_then(serde_json::Value::as_str) {
        Some("AUTH" | "CLOSE") => false,
        _ if scoped => true,
        Some("EVENT") => parts
            .get(1)
            .and_then(|e| e["kind"].as_u64())
            .is_some_and(|k| matches!(k, 43103 | 43104)),
        Some("REQ" | "COUNT") => private_filters(parts.get(2..).unwrap_or_default()),
        _ => false,
    }
}

pub(crate) fn private_filters(filters: &[serde_json::Value]) -> bool {
    filters.iter().any(|f| {
        f.get("host_registry").is_some()
            || f.get("host_deployment").is_some()
            || f["kinds"].as_array().is_some_and(|kinds| {
                kinds
                    .iter()
                    .any(|kind| matches!(kind.as_u64(), Some(43103 | 43104)))
            })
    })
}

const DENIED: &str = "restricted: host registry request denied";
#[derive(Debug)]
pub(crate) struct Connection {
    generation: OnceLock<[u8; 32]>,
    pub(crate) ordinary: Arc<Semaphore>,
    recovery: Arc<Semaphore>,
}
impl Default for Connection {
    fn default() -> Self {
        Self {
            generation: OnceLock::new(),
            ordinary: Arc::new(Semaphore::new(3)),
            recovery: Arc::new(Semaphore::new(1)),
        }
    }
}
impl Connection {
    pub(crate) fn generation(&self) -> Option<[u8; 32]> {
        self.generation.get().copied()
    }
}

pub(crate) async fn authenticate(
    e: &Event,
    challenge: &str,
    c: &ConnectionState,
    s: &AppState,
) -> bool {
    if !e
        .tags
        .iter()
        .any(|t| t.as_slice().first().is_some_and(|n| n == "host-registry"))
    {
        return false;
    }
    let id = e.id.to_hex();
    if let Err(outcome) = scope_auth(e, challenge, c, s).await {
        if !crate::access::AuthAttempt::nip42(c, e.pubkey, outcome != AuthOutcome::Invalid)
            .reject(c, outcome)
            || !c.community_control.is_open()
        {
            return true;
        }
        if outcome == AuthOutcome::ConcurrencyLimit {
            c.cancel.cancel();
            return true;
        }
        c.send(RelayMessage::ok(&id, false, DENIED));
    }
    true
}
async fn scope_auth(
    e: &Event,
    challenge: &str,
    c: &ConnectionState,
    s: &AppState,
) -> Result<(), AuthOutcome> {
    if !s.config.serves_host_registry()
        || c.nip_fi_assertion.is_some()
        || !e.content.is_empty()
        || e.tags.len() != 3
    {
        return Err(AuthOutcome::Invalid);
    }
    let mut generation = None;
    let mut relay = 0;
    let mut challenges = 0;
    for tag in e.tags.iter() {
        match tag.as_slice() {
            [name, v, g] if name == "host-registry" && v == "2" && generation.is_none() => {
                generation = Some(wire::bytes(g).map_err(|_| AuthOutcome::Invalid)?)
            }
            [name, _] if name == "relay" => relay += 1,
            [name, _] if name == "challenge" => challenges += 1,
            _ => return Err(AuthOutcome::Invalid),
        }
    }
    if relay != 1 || challenges != 1 {
        return Err(AuthOutcome::Invalid);
    }
    let g = generation.ok_or(AuthOutcome::Invalid)?;
    let url = crate::api::bridge::nip42_expected_relay_url(&s.config.relay_url, &c.tenant);
    let mut ctx = s
        .auth
        .verify_auth_event(e.clone(), challenge, &url)
        .await
        .map_err(|_| AuthOutcome::Invalid)?;
    let _auth_permit = s
        .auth_gate
        .admit(super::auth::AUTH_PERMIT_WAIT)
        .await
        .ok_or(AuthOutcome::ConcurrencyLimit)?;
    let host = ctx.pubkey.to_bytes();
    let allow = s.config.pubkey_allowlist_enabled;
    let initial =
        s.db.registry_session(c.tenant.community(), &host, &g, allow)
            .await
            .map_err(|_| AuthOutcome::HostScopeDenied)?;
    if let Some(owner) = initial.owner {
        s.conn_manager.set_admitted_owner(c.conn_id, owner);
    }
    let _permit = c
        .nip_fi_gate
        .acquire_effect()
        .await
        .map_err(|_| AuthOutcome::HostScopeDenied)?;
    // Bind before final writer recheck: a concurrent host ban sees this socket.
    s.conn_manager
        .set_authenticated_identity(c.conn_id, host.to_vec(), None);
    let session =
        s.db.registry_session(c.tenant.community(), &host, &g, allow)
            .await
            .map_err(|_| AuthOutcome::HostScopeDenied)?;
    if let Some(owner) = session.owner {
        s.conn_manager.set_admitted_owner(c.conn_id, owner);
    }
    if session.owner != initial.owner {
        s.db.registry_session(c.tenant.community(), &host, &g, allow)
            .await
            .map_err(|_| AuthOutcome::HostScopeDenied)?;
    }

    if c.cancel.is_cancelled() {
        return Err(AuthOutcome::HostScopeDenied);
    }
    ctx.scopes.clear();
    ctx.channel_ids = None;
    ctx.agent_owner_pubkey = None;
    c.registry
        .generation
        .set(g)
        .map_err(|_| AuthOutcome::HostScopeDenied)?;
    if !c.authenticate(ctx) {
        return Err(AuthOutcome::HostScopeDenied);
    }
    if c.cancel.is_cancelled() || !c.community_control.is_open() {
        return Ok(());
    }
    // Host-only admission does not participate in community presence.
    crate::access::AuthAttempt::nip42(c, e.pubkey, true).log(crate::access::AuthResult::Success);
    c.send(RelayMessage::ok(&e.id.to_hex(), true, ""));
    Ok(())
}

// Decode extension intent before generic Filter can discard unknown members.
pub(crate) async fn dispatch(raw: &str, c: Arc<ConnectionState>, s: Arc<AppState>) -> bool {
    let Ok(a) = serde_json::from_str::<Vec<Box<RawValue>>>(raw) else {
        return false;
    };
    let verb = a
        .first()
        .and_then(|r| serde_json::from_str::<String>(r.get()).ok())
        .unwrap_or_default();
    if matches!(verb.as_str(), "AUTH" | "CLOSE") {
        return false;
    }
    let registry_req = matches!(verb.as_str(), "REQ" | "COUNT")
        && a.iter().skip(2).any(|r| {
            serde_json::from_str::<serde_json::Value>(r.get())
                .ok()
                .is_some_and(|f| {
                    f.get("host_registry").is_some()
                        || f.get("kinds")
                            .and_then(|k| k.as_array())
                            .is_some_and(|k| k.contains(&json!(KIND_HOST_REGISTRY_V2)))
                })
        });
    let ev = a
        .get(1)
        .and_then(|r| serde_json::from_str::<serde_json::Value>(r.get()).ok());
    let registry_event = verb == "EVENT"
        && ev
            .as_ref()
            .is_some_and(|e| e["kind"] == KIND_HOST_REGISTRY_V2);
    if !registry_req && !registry_event && c.registry.generation().is_none() {
        return false;
    }
    let sub = a
        .get(1)
        .and_then(|v| serde_json::from_str::<String>(v.get()).ok())
        .unwrap_or_default();
    let id = ev
        .as_ref()
        .and_then(|e| e["id"].as_str())
        .unwrap_or("")
        .to_owned();
    if !registry_req && !registry_event {
        deny(&verb, &sub, &id, &c, &s).await;
        return true;
    }
    let AuthState::Authenticated(ctx) = c.auth_state_snapshot() else {
        deny(&verb, &sub, &id, &c, &s).await;
        return true;
    };
    if !s.config.serves_host_registry()
        || registry_req && (sub.is_empty() || sub.len() > 256)
        || verb == "COUNT"
        || c.nip_fi_assertion.is_some()
        || ctx.agent_owner_pubkey.is_some()
        || ctx.channel_ids.is_some()
    {
        deny(&verb, &sub, &id, &c, &s).await;
        return true;
    }
    let binding_recovery = registry_req
        && a.get(2)
            .and_then(|r| serde_json::from_str::<serde_json::Value>(r.get()).ok())
            .is_some_and(|f| f["host_registry"]["view"] == "binding");
    let restrictive = binding_recovery
        || registry_event
            && ev
                .as_ref()
                .and_then(|e| e["tags"].as_array())
                .is_some_and(|ts| {
                    ts.iter()
                        .any(|t| t == &json!(["op", "unpair"]) || t == &json!(["op", "unpaired"]))
                });
    let slots = if restrictive {
        &c.registry.recovery
    } else {
        &c.registry.ordinary
    };
    let Ok(permit) = slots.clone().try_acquire_owned() else {
        deny(&verb, &sub, &id, &c, &s).await;
        return true;
    };
    // Restrictive recovery has its own connection slot and skips the ordinary global queue.
    let global = if restrictive {
        None
    } else {
        match s.handler_semaphore.clone().try_acquire_owned() {
            Ok(p) => Some(p),
            Err(_) => {
                deny(&verb, &sub, &id, &c, &s).await;
                return true;
            }
        }
    };
    let event = registry_event.then(|| a[1].get().to_owned());
    let filter = registry_req.then(|| a.get(2).map(|v| v.get().to_owned()).unwrap_or_default());
    let exact = if registry_event {
        a.len() == 2
    } else {
        a.len() == 3
    };
    let owner = if registry_req {
        match super::close::admit_with_limit(&sub, &c, 32).await {
            Ok(owner) => Some(owner),
            Err(_) => return true,
        }
    } else {
        None
    };
    tokio::spawn(async move {
        let result = if !exact {
            Err(())
        } else if let Some(event) = event {
            tokio::time::timeout(std::time::Duration::from_secs(5), post(&event, &c, &s))
                .await
                .unwrap_or(Err(()))
        } else if let Some(owner) = owner {
            // REQ bounds database work internally. Never cancel a lifecycle
            // transition halfway through its index/topic ownership effects.
            request(&sub, owner, filter.as_deref().unwrap_or_default(), &c, &s).await
        } else {
            Err(())
        };
        if result.is_err() {
            if let Some(owner) = owner {
                super::close::close_if_owner(
                    &sub,
                    owner,
                    Some("resync_required: host registry unavailable"),
                    &c,
                    &s,
                )
                .await;
            } else {
                deny(&verb, &sub, &id, &c, &s).await;
            }
        }
        drop(global);
        drop(permit);
    });
    true
}
async fn deny(verb: &str, sub: &str, id: &str, c: &ConnectionState, s: &AppState) {
    match verb {
        "COUNT" => {
            c.send(RelayMessage::closed(sub, DENIED));
        }
        "REQ" => {
            super::close::refuse(sub, "resync_required: host registry unavailable", c, s).await
        }
        "EVENT" => {
            c.send(RelayMessage::ok(id, false, DENIED));
        }
        _ => {
            c.send(RelayMessage::notice(DENIED));
        }
    }
}
pub(super) fn actor(c: &ConnectionState) -> Result<nostr::PublicKey, ()> {
    match c.auth_state_snapshot() {
        AuthState::Authenticated(a)
            if c.nip_fi_assertion.is_none()
                && a.agent_owner_pubkey.is_none()
                && a.channel_ids.is_none() =>
        {
            Ok(a.pubkey)
        }
        _ => Err(()),
    }
}
fn origin(c: &ConnectionState) -> String {
    format!("https://{}", c.tenant.host())
}
pub(crate) async fn budget(
    c: &ConnectionState,
    s: &AppState,
    who: nostr::PublicKey,
    recovery: bool,
    g: Option<&[u8; 32]>,
) -> Result<(), ()> {
    let bucket: [u8; 32] = Sha256::digest(c.remote_addr.ip().to_string().as_bytes()).into();
    if !support::source_capacity(s, &c.tenant, &bucket).await?
        && !s
            .db
            .known_paired_principal(c.tenant.community(), who.as_bytes(), g)
            .await
            .map_err(|_| ())?
    {
        return Err(());
    }
    let lane = if recovery { "recovery" } else { "normal" };
    let mut db = s.redis_pool.clone();
    let ok:i64=redis::cmd("EVAL").arg("local a=redis.call('INCR',KEYS[1]);if a==1 then redis.call('EXPIRE',KEYS[1],60) end;local b=redis.call('INCR',KEYS[2]);if b==1 then redis.call('EXPIRE',KEYS[2],60) end;if a>tonumber(ARGV[1]) or b>tonumber(ARGV[2]) then return 0 end;return 1").arg(2).arg(format!("buzz:{}:host-registry:{lane}:{}",c.tenant.community(),who)).arg(format!("buzz:{}:host-registry:{lane}:tenant",c.tenant.community())).arg(if recovery{60}else{600}).arg(if recovery{600}else{6000}).query_async(&mut db).await.map_err(|_|())?;
    if ok == 1 {
        Ok(())
    } else {
        Err(())
    }
}
async fn post(raw: &str, c: &ConnectionState, s: &AppState) -> Result<(), ()> {
    let who = actor(c)?;
    let community = origin(c);
    let bytes = raw.as_bytes().to_vec();
    let parsed = tokio::task::spawn_blocking(move || wire::verify(&bytes, &community, who))
        .await
        .map_err(|_| ())?
        .map_err(|_| ())?;
    let (event, h) = parsed;
    let g = wire::bytes(&h.generation).map_err(|_| ())?;
    let restrictive = matches!(h.op.as_str(), "unpair" | "unpaired");
    if let Some(ceiling) = c.registry.generation() {
        if g != ceiling
            || h.host != who.to_hex()
            || !matches!(h.op.as_str(), "pair-ack" | "status" | "models" | "unpaired")
        {
            return Err(());
        }
        let session =
            s.db.registry_session(
                c.tenant.community(),
                who.as_bytes(),
                &g,
                s.config.pubkey_allowlist_enabled,
            )
            .await
            .map_err(|_| ())?;
        if (session.recovery_only && h.op != "unpaired")
            || (session.ack_retry_only && h.op != "pair-ack")
        {
            return Err(());
        }
        if let Some(owner) = session.owner {
            s.conn_manager.set_admitted_owner(c.conn_id, owner);
        }
    } else if !h.owner_sends().map_err(|_| ())? || h.owner != who.to_hex() {
        return Err(());
    }
    budget(c, s, who, restrictive, Some(&g)).await?;
    let _permit = c.nip_fi_gate.acquire_effect().await.map_err(|_| ())?;
    if c.cancel.is_cancelled() {
        return Err(());
    }
    let bucket: [u8; 32] = Sha256::digest(c.remote_addr.ip().to_string().as_bytes()).into();
    s.db.post_registry_event(
        c.tenant.community(),
        &origin(c),
        who,
        raw.as_bytes(),
        s.config.pubkey_allowlist_enabled,
        &bucket,
    )
    .await
    .map_err(|_| ())?;
    c.send(RelayMessage::ok(&event.id.to_hex(), true, ""));
    // Post-commit best effort invalidation. Exact private ciphertext never enters
    // the ordinary event pipeline. Generic delivery rejects this private kind.
    if let Ok(signal) = EventBuilder::new(Kind::Custom(KIND_HOST_REGISTRY_V2 as u16), "")
        .tags(Tag::parse(["g", &h.generation]))
        .sign_with_keys(&s.relay_keypair)
    {
        let _ = s
            .pubsub
            .publish_event(&c.tenant, buzz_pubsub::EventTopic::Global, &signal)
            .await;
    }
    Ok(())
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Filter {
    kinds: Vec<u32>,
    #[serde(rename = "#p")]
    recipient: Vec<String>,
    host_registry: wire::Query,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cursor {
    position: String,
    high: i64,
    expires: u64,
}
fn now() -> u64 {
    nostr::Timestamp::now().as_secs()
}
fn scope(c: &ConnectionState, who: &str, q: &wire::Query) -> Vec<String> {
    vec![
        "registry-v2".into(),
        c.tenant.community().to_string(),
        who.into(),
        q.view.clone(),
        q.generation.clone().unwrap_or_default(),
        q.nonce.clone(),
    ]
}
async fn read(
    c: &ConnectionState,
    s: &AppState,
    q: &wire::Query,
) -> Result<buzz_db::paired_hosts::RegistryPage, ()> {
    let who = actor(c)?;
    let parts = scope(c, &who.to_hex(), q);
    let parts: Vec<_> = parts.iter().map(String::as_str).collect();
    let cursor = if let Some(raw) = &q.cursor {
        let plain = support::open_cursor(s, &parts, raw, 160)?;
        let cursor: Cursor = serde_json::from_str(&plain).map_err(|_| ())?;
        if now() >= cursor.expires {
            return Err(());
        }
        Some(cursor)
    } else {
        None
    };
    let g = c.registry.generation();
    s.db.read_registry(
        c.tenant.community(),
        who.as_bytes(),
        s.config.pubkey_allowlist_enabled,
        RegistryRead {
            query: q,
            after: cursor.as_ref().map(|c| c.position.as_str()),
            snapshot: cursor.as_ref().map(|c| c.high),
            host_generation: g.as_ref(),
        },
    )
    .await
    .map_err(|_| ())
}
fn page_event(
    c: &ConnectionState,
    s: &AppState,
    q: &wire::Query,
    p: buzz_db::paired_hosts::RegistryPage,
    change: bool,
) -> Result<Event, ()> {
    let who = actor(c)?;
    let parts = scope(c, &who.to_hex(), q);
    let parts: Vec<_> = parts.iter().map(String::as_str).collect();
    let next = if let Some(position) = p.next {
        let cursor = serde_json::to_string(&Cursor {
            position,
            high: p.high_water,
            expires: now() + 60,
        })
        .map_err(|_| ())?;
        Some(support::seal_cursor(s, &parts, &cursor, 160)?)
    } else {
        None
    };
    wire::reply(
        &s.relay_keypair,
        who,
        wire::Reply {
            v: 2,
            community: origin(c),
            recipient: who.to_hex(),
            op: if change {
                "registry-change"
            } else {
                "registry-page"
            }
            .into(),
            query: q.clone(),
            high_water: high_token(s, c.tenant.community(), who, q, p.high_water)?,
            data: if change {
                json!({"generation":p.data.get("generation").cloned().or_else(||q.generation.clone().map(serde_json::Value::String))})
            } else {
                p.data
            },
            complete: next.is_none(),
            next_cursor: next,
        },
        now(),
    )
    .map_err(|_| ())
}
async fn request(
    sub: &str,
    owner: u64,
    raw: &str,
    c: &Arc<ConnectionState>,
    s: &Arc<AppState>,
) -> Result<(), ()> {
    if sub.is_empty() || sub.len() > 256 {
        return Err(());
    }
    let f: Filter = wire::object(raw.as_bytes(), 4096).map_err(|_| ())?;
    f.host_registry.validate().map_err(|_| ())?;
    let q = f.host_registry;
    let who = actor(c)?;
    if f.kinds != [KIND_HOST_REGISTRY_V2]
        || f.recipient != [who.to_hex()]
        || q.view == "live" && q.cursor.is_some()
    {
        return Err(());
    }
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        budget(
            c,
            s,
            who,
            q.view == "binding",
            c.registry.generation().as_ref(),
        ),
    )
    .await
    .map_err(|_| ())??;
    let _permit = c.nip_fi_gate.acquire_effect().await.map_err(|_| ())?;
    // Claim with the existing lifecycle map; all CLOSE/replacement/cleanup paths
    // retire this owner token, so background live delivery cannot outlive it.
    let events = s.pubsub.subscribe_local();
    if !claim(sub, owner, q.view == "live", c, s).await {
        return Ok(());
    }
    request_owned(sub, owner, &q, events, c, s).await
}

/// Atomically replace the old holder. A live private reader needs the global
/// wake topic, but no ordinary events. Upstream derives retained topics from
/// each filter's scope, so register a single filter whose empty kinds match nothing.
pub(super) async fn claim(
    sub: &str,
    owner: u64,
    live: bool,
    c: &ConnectionState,
    s: &AppState,
) -> bool {
    let mut subs = c.subscriptions.lock().await;
    if !super::close::claim_locked(&mut subs, sub, owner, c) {
        return false;
    }
    super::close::retire_holder_locked(sub, c, s).await;
    if live {
        let no_events = nostr::Filter {
            kinds: Some(Default::default()),
            ..Default::default()
        };
        s.sub_registry.register_scoped(
            c.tenant.community(),
            c.conn_id,
            sub.into(),
            vec![no_events],
            None,
        );
        s.pubsub
            .retain_topic(&c.tenant, buzz_pubsub::EventTopic::Global)
            .await;
    }
    true
}

async fn request_owned(
    sub: &str,
    owner: u64,
    q: &wire::Query,
    mut events: tokio::sync::broadcast::Receiver<buzz_pubsub::ChannelEvent>,
    c: &Arc<ConnectionState>,
    s: &Arc<AppState>,
) -> Result<(), ()> {
    let p = tokio::time::timeout(std::time::Duration::from_secs(5), read(c, s, q))
        .await
        .map_err(|_| ())??;
    let mut high = p.high_water;
    let e = page_event(c, s, q, p, false)?;
    if !super::close::send_if_owner(sub, owner, RelayMessage::event(sub, &e), c).await {
        return Ok(());
    }
    super::close::send_if_owner(sub, owner, RelayMessage::eose(sub), c).await;
    if q.view != "live" {
        super::close::close_if_owner(sub, owner, None, c, s).await;
        return Ok(());
    }
    let (sub, c, s) = (sub.to_owned(), Arc::clone(c), Arc::clone(s));
    let q = q.clone();
    tokio::spawn(async move {
        let mut periodic = tokio::time::interval(std::time::Duration::from_secs(60));
        periodic.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        periodic.tick().await;
        loop {
            tokio::select! {biased;_ = c.cancel.cancelled()=>break,_=periodic.tick()=>{},received=events.recv()=>{match received {Ok(e) if e.community_id==c.tenant.community()&&u32::from(e.event.kind.as_u16())==KIND_HOST_REGISTRY_V2=>{},Ok(_)=>continue,Err(tokio::sync::broadcast::error::RecvError::Closed)=>break,Err(_)=>{}}}}
            if !super::close::owns(&*c.subscriptions.lock().await, &sub, owner) {
                break;
            }
            // Load-bearing live reauthorization on the sending node, including
            // cross-node Redis invalidations and periodic loss recovery.
            let p = match read(&c, &s, &q).await {
                Ok(p) => p,
                Err(_) => {
                    super::close::close_if_owner(
                        &sub,
                        owner,
                        Some("resync_required: registry unavailable"),
                        &c,
                        &s,
                    )
                    .await;
                    break;
                }
            };
            if p.high_water != high {
                high = p.high_water;
                let Ok(e) = page_event(&c, &s, &q, p, true) else {
                    super::close::close_if_owner(
                        &sub,
                        owner,
                        Some("resync_required: registry unavailable"),
                        &c,
                        &s,
                    )
                    .await;
                    break;
                };
                if !super::close::send_if_owner(&sub, owner, RelayMessage::event(&sub, &e), &c)
                    .await
                {
                    break;
                }
            }
        }
        super::close::close_if_owner(&sub, owner, None, &c, &s).await;
    });
    Ok(())
}

// Opaque positions prevent a participant inferring unrelated tenants' traffic
// from the shared paired_host_events identity sequence. Cursor contents keep the
// numeric position encrypted; both tokens derive from the existing relay key.
fn high_token(
    s: &AppState,
    tenant: buzz_core::CommunityId,
    reader: nostr::PublicKey,
    q: &wire::Query,
    high: i64,
) -> Result<String, ()> {
    let mut mac = Hmac::<Sha256>::new_from_slice(s.relay_keypair.secret_key().as_secret_bytes())
        .map_err(|_| ())?;
    mac.update(b"buzz-host-registry-v2-high-water\0");
    for part in [
        tenant.to_string(),
        reader.to_hex(),
        q.view.clone(),
        q.generation.clone().unwrap_or_default(),
        high.to_string(),
    ] {
        mac.update(part.as_bytes());
        mac.update(&[0]);
    }
    Ok(hex::encode(mac.finalize().into_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::tests::{authenticated_state, test_conn_with_auth};
    use buzz_pubsub::EventTopic;

    #[test]
    fn private_routing_recognizes_mixed_and_count_filters_before_generic_parsing() {
        for verb in ["REQ", "COUNT"] {
            for private in [
                json!({"kinds":[43103]}),
                json!({"kinds":[1,43104]}),
                json!({"host_registry":null}),
                json!({"host_deployment":{}}),
            ] {
                assert!(private_frame(
                    &json!([verb, "s", {"kinds":[1]}, private]),
                    false
                ));
            }
            assert!(!private_frame(&json!([verb, "s", {"kinds":[1]}]), false));
            assert!(private_frame(&json!([verb, "s", {"kinds":[1]}]), true));
        }
        for kind in [43103, 43104] {
            assert!(private_frame(&json!(["EVENT", {"kind":kind}]), false));
        }
        for verb in ["AUTH", "CLOSE"] {
            assert!(!private_frame(&json!([verb, "s"]), true));
        }
    }

    #[tokio::test]
    async fn private_live_replacement_fences_late_read_and_releases_global_topic() {
        let state = crate::state::tests::test_state().await;
        let (conn, _recv) = test_conn_with_auth(authenticated_state());
        let first = super::super::close::admit_with_limit("s", &conn, 32)
            .await
            .unwrap();
        assert!(claim("s", first, true, &conn, &state).await);
        assert_eq!(
            state
                .pubsub
                .topic_refcount(&conn.tenant, EventTopic::Global)
                .await,
            1
        );
        let next = super::super::close::admit_with_limit("s", &conn, 32)
            .await
            .unwrap();
        assert!(!claim("s", first, true, &conn, &state).await);
        assert!(claim("s", next, true, &conn, &state).await);
        assert_eq!(
            state
                .pubsub
                .topic_refcount(&conn.tenant, EventTopic::Global)
                .await,
            1
        );
        let filters = state.sub_registry.get_filters(conn.conn_id, "s").unwrap();
        assert_eq!(filters.len(), 1);
        assert!(filters[0].kinds.as_ref().unwrap().is_empty());
        super::super::close::handle_close("s".into(), conn.clone(), state.clone()).await;
        assert!(!claim("s", next, true, &conn, &state).await);
        assert_eq!(
            state
                .pubsub
                .topic_refcount(&conn.tenant, EventTopic::Global)
                .await,
            0
        );
    }
}
