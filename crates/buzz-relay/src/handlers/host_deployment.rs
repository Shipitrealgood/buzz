//! Opt-in private deployment traffic. Never enters generic event storage/fanout.
use crate::{
    connection::{AuthState, ConnectionState},
    protocol::RelayMessage,
    state::AppState,
};
use buzz_core::{host_deployment::wire, kind::KIND_HOST_DEPLOYMENT_V1};
use nostr::{EventBuilder, Kind};
use serde::Deserialize;
use serde_json::{json, value::RawValue};
use std::{sync::Arc, time::Duration};

const DENIED: &str = "restricted: host deployment unavailable";
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Filter {
    kinds: Vec<u32>,
    #[serde(rename = "#p")]
    recipient: Vec<String>,
    host_deployment: wire::Query,
}
fn actor(c: &ConnectionState) -> Result<nostr::PublicKey, ()> {
    match c.auth_state_snapshot() {
        AuthState::Authenticated(a)
            if a.agent_owner_pubkey.is_none() && a.channel_ids.is_none() =>
        {
            Ok(a.pubkey)
        }
        _ => Err(()),
    }
}
fn now() -> u64 {
    nostr::Timestamp::now().as_secs()
}
fn origin(c: &ConnectionState) -> String {
    format!("https://{}", c.tenant.host())
}
/// Intercept explicit management intent before generic Filter drops extensions.
pub(crate) async fn dispatch(raw: &str, c: Arc<ConnectionState>, s: Arc<AppState>) -> bool {
    let Ok(parts) = serde_json::from_str::<Vec<Box<RawValue>>>(raw) else {
        return false;
    };
    let verb = parts
        .first()
        .and_then(|p| serde_json::from_str::<String>(p.get()).ok())
        .unwrap_or_default();
    let ev = parts
        .get(1)
        .and_then(|v| serde_json::from_str::<serde_json::Value>(v.get()).ok());
    let event = verb == "EVENT"
        && ev
            .as_ref()
            .is_some_and(|v| v["kind"] == KIND_HOST_DEPLOYMENT_V1);
    let read = matches!(verb.as_str(), "REQ" | "COUNT")
        && parts.iter().skip(2).any(|v| {
            serde_json::from_str::<serde_json::Value>(v.get())
                .ok()
                .is_some_and(|v| {
                    v.get("host_deployment").is_some()
                        || v["kinds"]
                            .as_array()
                            .is_some_and(|k| k.contains(&json!(KIND_HOST_DEPLOYMENT_V1)))
                })
        });
    if !event && !read {
        return false;
    }
    let sub = parts
        .get(1)
        .and_then(|p| serde_json::from_str::<String>(p.get()).ok())
        .unwrap_or_default();
    let id = ev
        .as_ref()
        .and_then(|e| e["id"].as_str())
        .unwrap_or_default()
        .to_owned();
    let deny = || {
        if event {
            c.send(RelayMessage::ok(&id, false, DENIED));
        } else {
            c.send(RelayMessage::closed(&sub, DENIED));
        }
    };
    if !s.config.serves_host_deployment()
        || actor(&c).is_err()
        || verb == "COUNT"
        || (event && parts.len() != 2)
        || (read && (parts.len() != 3 || sub.is_empty() || sub.len() > 256))
    {
        deny();
        return true;
    }
    let Ok(permit) = c.registry.ordinary.clone().try_acquire_owned() else {
        deny();
        return true;
    };
    let Ok(global) = s.handler_semaphore.clone().try_acquire_owned() else {
        deny();
        return true;
    };
    let raw = if event {
        parts[1].get()
    } else {
        parts[2].get()
    }
    .to_owned();
    tokio::spawn(async move {
        let result = if event {
            tokio::time::timeout(Duration::from_secs(5), post(&raw, &c, &s))
                .await
                .unwrap_or(Err(()))
        } else {
            request(&sub, &raw, &c, &s).await
        };
        if result.is_err() {
            if event {
                c.send(RelayMessage::ok(&id, false, DENIED));
            } else {
                super::close::refuse(&sub, DENIED, &c, &s).await;
            }
        }
        drop(global);
        drop(permit);
    });
    true
}
async fn post(raw: &str, c: &ConnectionState, s: &AppState) -> Result<(), ()> {
    let who = actor(c)?;
    let (e, h) = wire::verify(raw.as_bytes(), &origin(c), who).map_err(|_| ())?;
    if let Some(g) = c.registry.generation() {
        if buzz_core::host_registry::bytes(&h.scope().generation).map_err(|_| ())? != g
            || who != h.scope().host
            || matches!(h, wire::Header::Command { .. })
        {
            return Err(());
        }
    } else if who != h.scope().owner || !matches!(h, wire::Header::Command { .. }) {
        return Err(());
    }
    super::host_registry::budget(c, s, who, false, c.registry.generation().as_ref()).await?;
    let _guard = c.nip_fi_gate.acquire_effect().await.map_err(|_| ())?;
    if c.cancel.is_cancelled() {
        return Err(());
    }
    s.db.post_host_deployment(
        c.tenant.community(),
        &origin(c),
        who,
        raw.as_bytes(),
        s.config.pubkey_allowlist_enabled,
    )
    .await
    .map_err(|_| ())?;
    c.send(RelayMessage::ok(&e.id.to_hex(), true, ""));
    // Non-sensitive wake only. Sending nodes independently reauthorize each view.
    if let Ok(signal) = EventBuilder::new(Kind::Custom(KIND_HOST_DEPLOYMENT_V1 as u16), "")
        .sign_with_keys(&s.relay_keypair)
    {
        let _ = s
            .pubsub
            .publish_event(&c.tenant, buzz_pubsub::EventTopic::Global, &signal)
            .await;
    }
    Ok(())
}
async fn read(q: &wire::Query, c: &ConnectionState, s: &AppState) -> Result<nostr::Event, ()> {
    if !s.config.serves_host_deployment() || q.scope.community != origin(c) {
        return Err(());
    }
    let who = actor(c)?;
    let reply =
        s.db.read_host_deployment(
            c.tenant.community(),
            who,
            c.registry.generation().as_ref(),
            s.config.pubkey_allowlist_enabled,
            q,
        )
        .await
        .map_err(|_| ())?;
    wire::reply(&s.relay_keypair, who, &reply, now()).map_err(|_| ())
}
async fn request(
    sub: &str,
    raw: &str,
    c: &Arc<ConnectionState>,
    s: &Arc<AppState>,
) -> Result<(), ()> {
    let f: Filter = buzz_core::host_registry::object(raw.as_bytes(), 4096).map_err(|_| ())?;
    let q = f.host_deployment;
    q.validate().map_err(|_| ())?;
    let who = actor(c)?;
    if f.kinds != [KIND_HOST_DEPLOYMENT_V1] || f.recipient != [who.to_hex()] {
        return Err(());
    }
    tokio::time::timeout(
        Duration::from_secs(5),
        super::host_registry::budget(c, s, who, false, c.registry.generation().as_ref()),
    )
    .await
    .map_err(|_| ())??;
    let _guard = c.nip_fi_gate.acquire_effect().await.map_err(|_| ())?;
    let mut signals = s.pubsub.subscribe_local();
    let owner = {
        let mut subs = c.subscriptions.lock().await;
        if c.cancel.is_cancelled() || (!subs.contains_key(sub) && subs.len() >= 32) {
            return Err(());
        }
        super::close::retire_locked(&mut subs, sub, c, s).await;
        let owner = super::close::next_owner();
        subs.insert(sub.into(), owner);
        if q.view == "live" {
            s.sub_registry.register_scoped(
                c.tenant.community(),
                c.conn_id,
                sub.into(),
                vec![],
                None,
            );
            s.pubsub
                .retain_topic(&c.tenant, buzz_pubsub::EventTopic::Global)
                .await;
        }
        owner
    };
    let first = tokio::time::timeout(Duration::from_secs(5), read(&q, c, s)).await;
    let Ok(Ok(first)) = first else {
        super::close::close_if_owner(sub, owner, Some(DENIED), c, s).await;
        return Ok(());
    };
    super::close::send_if_owner(sub, owner, RelayMessage::event(sub, &first), c).await;
    super::close::send_if_owner(sub, owner, RelayMessage::eose(sub), c).await;
    if q.view != "live" {
        super::close::close_if_owner(sub, owner, None, c, s).await;
        return Ok(());
    }
    let (sub, c, s) = (sub.to_owned(), c.clone(), s.clone());
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(5));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        tick.tick().await;
        loop {
            tokio::select! { biased; _=c.cancel.cancelled()=>break, _=tick.tick()=>{}, message=signals.recv()=>{match message { Ok(m) if m.community_id==c.tenant.community() && matches!(m.event.kind.as_u16(),43103|43104)=>{}, Ok(_)=>continue, Err(tokio::sync::broadcast::error::RecvError::Closed)=>break, Err(_)=>{} }} }
            if c.subscriptions.lock().await.get(&sub) != Some(&owner) {
                break;
            }
            match tokio::time::timeout(Duration::from_secs(5), read(&q, &c, &s)).await {
                Ok(Ok(e)) => {
                    if !super::close::send_if_owner(&sub, owner, RelayMessage::event(&sub, &e), &c)
                        .await
                    {
                        break;
                    }
                }
                _ => {
                    super::close::close_if_owner(&sub, owner, Some(DENIED), &c, &s).await;
                    break;
                }
            }
        }
        super::close::close_if_owner(&sub, owner, None, &c, &s).await;
    });
    Ok(())
}
