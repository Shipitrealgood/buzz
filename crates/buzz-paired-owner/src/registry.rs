//! V2 native registration using the existing Device journal and shared WS client.
use super::*;
use crate::device::{coordinate, grant, save, Grant};
use anyhow::{bail, Context};
use buzz_core::host_registry::{self as wire, Body, Query};
use buzz_core::paired_message::Route;
use buzz_ws_client::{NostrWsConnection, RelayMessage};
use nostr::PublicKey;
use rusqlite::{OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::{Duration, Instant};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Invitation {
    v: u8,
    community: String,
    generation: String,
    host: String,
    proof: String,
    expires_at: u64,
}
/// Compact relay projection; peer evidence must be independently verified.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryBinding {
    /// Exact enrollment generation.
    pub generation: String,
    /// Relay-enforced immutable owner.
    pub owner: String,
    /// Persistent host key.
    pub host: String,
    /// Active, fenced or dead; never used to invent peer consent.
    pub state: String,
    /// Exact selected owner's claim.
    pub claim_id: String,
    /// Exact host acknowledgment.
    pub ack_id: String,
    /// Durable restrictive completion if present.
    pub tombstone_id: Option<String>,
    /// Latest private status for an authenticated cached label.
    pub latest_status_id: Option<String>,
}
fn packet(e: &Event, h: &wire::Header) -> Result<Packet> {
    Ok(Packet {
        event: serde_json::value::to_raw_value(e)?,
        route: Route {
            r: Some(h.operation_id.clone()),
            prev: h.parent.clone(),
            expires_at: h.expires_at,
        },
    })
}
fn event(p: &Packet) -> Result<Event> {
    Ok(serde_json::from_str(p.event.get())?)
}
fn new_grant(scope: Scope, claim: Packet) -> Grant {
    Grant {
        scope,
        claim,
        previous_claims: vec![],
        ack: None,
        tombstone: None,
        revocation: None,
        fenced: false,
        inventory: None,
        sequence: 0,
        capacity: None,
        catalog_revision: None,
        label: None,
        runtimes: vec![],
    }
}
impl Device {
    /// Persist an exact v2 claim in the existing outbox before publishing it.
    pub fn pair_registry(
        &mut self,
        selected: &str,
        keys: &Keys,
        invitation: &str,
    ) -> Result<(Scope, Event)> {
        let inv: Invitation = wire::object(invitation.as_bytes(), 4096)?;
        ensure!(
            inv.v == 2
                && inv.community == selected
                && wire::origin(selected)
                && wire::identifier(&inv.generation)
                && wire::identifier(&inv.host)
                && wire::identifier(&inv.proof),
            "invalid invitation"
        );
        let wall = now()?;
        self.check_time(wall)?;
        ensure!(
            wall < inv.expires_at && inv.expires_at <= wall + 630,
            "invitation expired"
        );
        let scope = Scope {
            community: selected.into(),
            owner: keys.public_key(),
            host: PublicKey::from_hex(&inv.host)?,
            generation: inv.generation,
        };
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let old: Option<String> = tx
            .query_row(
                "SELECT data FROM grants WHERE scope=?",
                [coordinate(&scope)?],
                |r| r.get(0),
            )
            .optional()?;
        let mut g = if let Some(raw) = old {
            let mut g: Grant = serde_json::from_str(&raw)?;
            ensure!(
                g.ack.is_none() && g.claim.route.expires_at == inv.expires_at,
                "already paired"
            );
            let mut previous = g.previous_claims.clone();
            previous.push(g.claim.clone());
            for p in &previous {
                let e = event(p)?;
                if let Ok((_, Body::PairClaim { proof })) = wire::open(keys, &scope, &e, wall) {
                    if proof == inv.proof {
                        return Ok((scope, e));
                    }
                }
            }
            ensure!(previous.len() < 5, "attempt budget exhausted");
            g.previous_claims = previous;
            g
        } else {
            new_grant(
                scope.clone(),
                Packet {
                    event: serde_json::value::RawValue::from_string("null".into())?,
                    route: Route {
                        r: None,
                        prev: None,
                        expires_at: inv.expires_at,
                    },
                },
            )
        };
        let e = wire::seal(
            keys,
            &scope,
            Body::PairClaim { proof: inv.proof },
            random()?,
            None,
            wall,
            inv.expires_at,
        )?;
        let (_, h) = wire::verify(&serde_json::to_vec(&e)?, selected, keys.public_key())?;
        g.claim = packet(&e, &h)?;
        save(&tx, &g)?;
        tx.commit()?;
        Ok((scope, e))
    }
    /// Recover dual consent into an independent journal without the console proof.
    pub fn restore_registry(
        &mut self,
        selected: &str,
        keys: &Keys,
        b: &RegistryBinding,
        claim: &Event,
        ack: &Event,
        status: Option<&Event>,
    ) -> Result<Scope> {
        let scope = Scope {
            community: selected.into(),
            generation: b.generation.clone(),
            owner: PublicKey::from_hex(&b.owner)?,
            host: PublicKey::from_hex(&b.host)?,
        };
        session(&scope, selected, keys)?;
        ensure!(
            wire::identifier(&b.generation)
                && matches!(b.state.as_str(), "active" | "fenced" | "dead")
                && claim.id.to_hex() == b.claim_id
                && ack.id.to_hex() == b.ack_id,
            "invalid binding evidence"
        );
        let wall = now()?;
        self.check_time(wall)?;
        // Historical consent is never a readiness result. Validate its original
        // signed lifetime and exact claim reference even after invitation expiry.
        let (ch, cb) = wire::open(keys, &scope, claim, claim.created_at.as_secs())?;
        let (ah, ab) = wire::open(keys, &scope, ack, ack.created_at.as_secs())?;
        ensure!(
            claim.pubkey == scope.owner
                && ack.pubkey == scope.host
                && matches!(cb, Body::PairClaim { .. })
                && matches!(ab,Body::PairAck{claim_id} if claim_id==b.claim_id)
                && ah.parent.as_ref() == Some(&b.claim_id)
                && ch.expires_at == ah.expires_at
                && ch.expires_at - claim.created_at.as_secs() <= 630
                && ah.expires_at - ack.created_at.as_secs() <= 630
                && claim.created_at.as_secs() <= wall + 30
                && ack.created_at.as_secs() <= wall + 30,
            "invalid dual consent"
        );
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let old: Option<String> = tx
            .query_row(
                "SELECT data FROM grants WHERE scope=?",
                [coordinate(&scope)?],
                |r| r.get(0),
            )
            .optional()?;
        let old = old
            .map(|raw| serde_json::from_str::<Grant>(&raw))
            .transpose()?;
        if let Some(old) = &old {
            ensure!(
                old.ack
                    .as_ref()
                    .map(id)
                    .transpose()?
                    .is_none_or(|id| id == b.ack_id),
                "conflicting enrollment"
            );
        }
        let mut g = match old {
            Some(g) => g,
            None => new_grant(scope.clone(), packet(claim, &ch)?),
        };
        g.claim = packet(claim, &ch)?;
        g.previous_claims.clear();
        g.ack = Some(packet(ack, &ah)?);
        if b.state != "active" {
            g.fenced = true;
        }
        if let Some(e) = status {
            let (h, body) = wire::open(keys, &scope, e, e.created_at.as_secs())?;
            let Body::Status {
                ack_id,
                label,
                sequence,
                ..
            } = body
            else {
                bail!("invalid historical status")
            };
            ensure!(
                e.pubkey == scope.host
                    && e.created_at.as_secs() <= wall + 30
                    && ack_id == b.ack_id
                    && sequence >= g.sequence,
                "invalid historical status"
            );
            g.sequence = sequence;
            g.label = Some(label);
            g.inventory = Some(packet(e, &h)?);
        }
        save(&tx, &g)?;
        tx.commit()?;
        Ok(scope)
    }
    /// Save a process-local freshness challenge for a recovered registration.
    pub fn registry_status_query(
        &mut self,
        selected: &str,
        keys: &Keys,
        s: &Scope,
    ) -> Result<Event> {
        session(s, selected, keys)?;
        let wall = now()?;
        self.check_time(wall)?;
        let g = grant(&self.db, s)?;
        ensure!(g.active(), "inactive host");
        let nonce = random()?;
        let ack = id(g.ack.as_ref().context("missing enrollment")?)?;
        let e = wire::seal(
            keys,
            s,
            Body::StatusQuery {
                ack_id: ack,
                nonce: nonce.clone(),
            },
            random()?,
            Some(id(g.ack.as_ref().context("missing enrollment")?)?),
            wall,
            wall + 60,
        )?;
        let coord = coordinate(s)?;
        self.fresh.remove(&coord);
        self.queries
            .insert(coord, (e.id.to_hex(), nonce, Instant::now()));
        Ok(e)
    }
    /// Verify a fresh exact nonce response before marking the host available.
    pub fn accept_registry_status(
        &mut self,
        selected: &str,
        keys: &Keys,
        s: &Scope,
        e: &Event,
    ) -> Result<()> {
        session(s, selected, keys)?;
        let wall = now()?;
        self.check_time(wall)?;
        let (h, body) = wire::open(keys, s, e, wall)?;
        let Body::Status {
            ack_id,
            nonce,
            sequence,
            state,
            label,
        } = body
        else {
            bail!("invalid status")
        };
        let coord = coordinate(s)?;
        let (q, n, t) = self.queries.get(&coord).context("no live query")?;
        ensure!(
            h.parent.as_ref() == Some(q)
                && n == &nonce
                && t.elapsed() < Duration::from_secs(60)
                && e.pubkey == s.host,
            "stale status"
        );
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut g = grant(&tx, s)?;
        ensure!(
            g.active()
                && ack_id == id(g.ack.as_ref().context("missing enrollment")?)?
                && (sequence > g.sequence
                    || (sequence == g.sequence
                        && g.inventory.as_ref().map(id).transpose()?.as_ref()
                            == Some(&e.id.to_hex()))),
            "inactive or stale status"
        );
        g.sequence = sequence;
        g.label = Some(label);
        g.inventory = Some(packet(e, &h)?);
        if state != "active" {
            g.fenced = true;
        }
        save(&tx, &g)?;
        tx.commit()?;
        if state == "active" {
            self.fresh
                .insert(coord.clone(), (e.id.to_hex(), *t, h.expires_at));
        }
        self.queries.remove(&coord);
        Ok(())
    }
    /// Fence cached v2 cards absent from a completely verified authoritative list.
    /// Partial/unavailable snapshots never call this method or erase cached evidence.
    pub fn registry_reconcile_list(
        &mut self,
        selected: &str,
        keys: &Keys,
        current: &[Scope],
    ) -> Result<()> {
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let rows = {
            let mut q=tx.prepare("SELECT data FROM grants WHERE json_extract(scope,'$.community')=? AND json_extract(scope,'$.owner')=?")?;
            let rows = q.query_map(
                rusqlite::params![selected, keys.public_key().to_hex()],
                |r| r.get::<_, String>(0),
            )?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
        };
        for raw in rows {
            let mut g: Grant = serde_json::from_str(&raw)?;
            if g.ack.is_some()
                && event(&g.claim)?.kind.as_u16() == 43103
                && !current.contains(&g.scope)
            {
                g.fenced = true;
                save(&tx, &g)?;
                self.fresh.remove(&coordinate(&g.scope)?);
            }
        }
        tx.commit()?;
        Ok(())
    }
    /// Verify host-signed terminal evidence without restoring readiness.
    pub fn accept_registry_tombstone(
        &mut self,
        selected: &str,
        keys: &Keys,
        s: &Scope,
        e: &Event,
    ) -> Result<()> {
        session(s, selected, keys)?;
        let (h, body) = wire::open(keys, s, e, e.created_at.as_secs())?;
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut g = grant(&tx, s)?;
        ensure!(
            e.pubkey == s.host
                && matches!(body,Body::Unpaired{ack_id} if Some(ack_id.as_str())==g.ack.as_ref().map(id).transpose()?.as_deref()),
            "invalid tombstone"
        );
        g.tombstone = Some(packet(e, &h)?);
        g.fenced = true;
        save(&tx, &g)?;
        tx.commit()?;
        self.fresh.remove(&coordinate(s)?);
        Ok(())
    }
    /// Restrict locally and persist exact owner intent before any network send.
    pub fn unpair_registry(&mut self, selected: &str, keys: &Keys, s: &Scope) -> Result<Event> {
        session(s, selected, keys)?;
        let wall = now()?;
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut g = grant(&tx, s)?;
        if let Some(p) = &g.revocation {
            return event(p);
        }
        let ack = id(g.ack.as_ref().context("missing enrollment")?)?;
        let e = wire::seal(
            keys,
            s,
            Body::Unpair {
                ack_id: ack.clone(),
            },
            random()?,
            Some(ack),
            wall,
            wall + 86400,
        )?;
        let (_, h) = wire::verify(&serde_json::to_vec(&e)?, selected, keys.public_key())?;
        g.revocation = Some(packet(&e, &h)?);
        save(&tx, &g)?;
        tx.commit()?;
        self.fresh.remove(&coordinate(s)?);
        Ok(e)
    }
}
/// Native WS transport pinned to the selected community, relay key and signer.
pub struct Registry {
    ws: NostrWsConnection,
    community: String,
    relay: PublicKey,
    owner: PublicKey,
    live: Option<(String, Query)>,
    dirty: bool,
}
impl Registry {
    /// Discover explicit v2 support over selected-tenant HTTPS, then authenticate.
    pub async fn connect(selected: &str, keys: &Keys) -> Result<Self> {
        let endpoint = Endpoint::new(selected, keys.public_key())?;
        let metadata = endpoint.metadata().await?;
        let relay = Self::capability(&metadata)?;
        let mut ws =
            NostrWsConnection::connect(&selected.replacen("https://", "wss://", 1)).await?;
        ws.authenticate(keys, None).await?;
        Self::from_authenticated(selected, keys.public_key(), relay, ws)
    }
    /// Validate a selected-tenant NIP-11 response. No v1/HTTP fallback.
    pub fn capability(metadata: &Value) -> Result<PublicKey> {
        ensure!(
            metadata["host_registry"]
                == json!({"versions":[2],"kind":43103,"admission":"host-scoped","reads":"req-v2"}),
            "host registration unsupported"
        );
        let raw = metadata["pubkey"].as_str().context("missing relay key")?;
        ensure!(wire::identifier(raw), "invalid relay key");
        Ok(PublicKey::from_hex(raw)?)
    }
    /// Bind an already authenticated shared WS client, including cluster/fixture
    /// direct-address connections. Relay identity must come from trusted metadata.
    pub fn from_authenticated(
        selected: &str,
        owner: PublicKey,
        relay: PublicKey,
        ws: NostrWsConnection,
    ) -> Result<Self> {
        ensure!(wire::origin(selected), "invalid selected community");
        Ok(Self {
            ws,
            community: selected.into(),
            relay,
            owner,
            live: None,
            dirty: false,
        })
    }
    /// Publish one exact durable peer event and require a positive correlated OK.
    pub async fn publish(&mut self, keys: &Keys, e: Event) -> Result<()> {
        ensure!(
            keys.public_key() == self.owner && e.pubkey == self.owner,
            "active signer changed"
        );
        let (_, h) = wire::verify(&serde_json::to_vec(&e)?, &self.community, self.owner)?;
        ensure!(h.owner == self.owner.to_hex(), "wrong owner");
        let ok = self.ws.send_event(e).await?;
        ensure!(ok.accepted, "registration denied: {}", ok.message);
        Ok(())
    }
    /// One protected finite page; EOSE without a valid correlated page is failure.
    pub async fn page(&mut self, keys: &Keys, q: Query) -> Result<wire::Reply> {
        ensure!(keys.public_key() == self.owner, "active signer changed");
        q.validate()?;
        let sub = format!("registry-{}", uuid::Uuid::new_v4().simple());
        self.ws
            .send_raw(
                &json!(["REQ",sub,{"kinds":[43103],"#p":[self.owner.to_hex()],"host_registry":q}]),
            )
            .await?;
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut page = None;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            ensure!(!remaining.is_zero(), "registry page unavailable");
            match self.ws.next_event(remaining).await? {
                RelayMessage::Event {
                    subscription_id,
                    event,
                } if subscription_id == sub => {
                    let r = wire::open_reply(keys, self.relay, &self.community, &q, &event)?;
                    ensure!(r.op == "registry-page" && page.is_none(), "invalid page");
                    page = Some(r);
                }
                RelayMessage::Eose { subscription_id } if subscription_id == sub => {
                    return page.context("missing registry page")
                }
                RelayMessage::Closed {
                    subscription_id, ..
                } if subscription_id == sub => bail!("registry unavailable"),
                RelayMessage::Event {
                    subscription_id,
                    event,
                } if self
                    .live
                    .as_ref()
                    .is_some_and(|(id, _)| id == &subscription_id) =>
                {
                    let (_, live) = self.live.as_ref().context("missing live query")?;
                    let change = wire::open_reply(keys, self.relay, &self.community, live, &event)?;
                    ensure!(change.op == "registry-change", "invalid live change");
                    self.dirty = true;
                }
                RelayMessage::Closed {
                    subscription_id, ..
                } if self
                    .live
                    .as_ref()
                    .is_some_and(|(id, _)| id == &subscription_id) =>
                {
                    bail!("registry live unavailable")
                }
                _ => {}
            }
        }
    }
    /// Subscribe before snapshots. Invalidation is only a refetch hint.
    pub async fn subscribe(&mut self, keys: &Keys) -> Result<()> {
        ensure!(keys.public_key() == self.owner, "active signer changed");
        if let Some((id, _)) = self.live.take() {
            self.ws.send_raw(&json!(["CLOSE", id])).await?;
        }
        let q = query("live", None, None)?;
        let sub = format!("registry-live-{}", uuid::Uuid::new_v4().simple());
        self.ws
            .send_raw(
                &json!(["REQ",sub,{"kinds":[43103],"#p":[self.owner.to_hex()],"host_registry":q}]),
            )
            .await?;
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut page = false;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            ensure!(!remaining.is_zero(), "registry live unavailable");
            match self.ws.next_event(remaining).await? {
                RelayMessage::Event {
                    subscription_id,
                    event,
                } if subscription_id == sub => {
                    let r = wire::open_reply(keys, self.relay, &self.community, &q, &event)?;
                    ensure!(r.op == "registry-page" && !page, "invalid live page");
                    page = true;
                }
                RelayMessage::Eose { subscription_id } if subscription_id == sub => {
                    ensure!(page, "missing live page");
                    self.live = Some((sub, q));
                    self.dirty = false;
                    return Ok(());
                }
                RelayMessage::Closed {
                    subscription_id, ..
                } if subscription_id == sub => bail!("registry live unavailable"),
                _ => {}
            }
        }
    }
    /// Wait for a verified invalidation or a bounded periodic resync deadline.
    /// Native callers refresh after this returns and on reconnect/resume. The
    /// periodic path repairs notifications lost before local delivery.
    pub async fn wait_for_refresh(&mut self, keys: &Keys, maximum_wait: Duration) -> Result<()> {
        ensure!(
            keys.public_key() == self.owner
                && maximum_wait >= Duration::from_secs(1)
                && maximum_wait <= Duration::from_secs(60),
            "invalid refresh interval or signer"
        );
        if self.dirty {
            self.dirty = false;
            return Ok(());
        }
        let (sub, q) = self.live.clone().context("registry not subscribed")?;
        let deadline = Instant::now() + maximum_wait;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Ok(());
            }
            match self.ws.next_event(remaining).await {
                Err(buzz_ws_client::WsClientError::Timeout) => return Ok(()),
                Err(error) => return Err(error.into()),
                Ok(RelayMessage::Event {
                    subscription_id,
                    event,
                }) if subscription_id == sub => {
                    let r = wire::open_reply(keys, self.relay, &self.community, &q, &event)?;
                    ensure!(r.op == "registry-change", "invalid change");
                    return Ok(());
                }
                Ok(RelayMessage::Closed {
                    subscription_id, ..
                }) if subscription_id == sub => {
                    bail!("registry unavailable; cached evidence retained")
                }
                _ => {}
            }
        }
    }
    /// Recover complete authoritative lists with at most three snapshot retries.
    /// Cached cards survive an unavailable/partial snapshot. Freshness requires a
    /// separately answered status nonce for each host.
    pub async fn recover(&mut self, device: &mut Device, keys: &Keys) -> Result<Vec<Scope>> {
        self.subscribe(keys).await?;
        let mut snapshot = None;
        for _ in 0..3 {
            let mut q = query("hosts", None, None)?;
            let mut summaries = Vec::new();
            let mut failed = false;
            for _ in 0..32 {
                let page = match self.page(keys, q.clone()).await {
                    Ok(p) => p,
                    Err(_) => {
                        failed = true;
                        break;
                    }
                };
                let rows = page.data["hosts"].as_array().context("missing host list")?;
                summaries.extend(
                    rows.iter()
                        .map(|r| serde_json::from_value::<RegistryBinding>(r.clone()))
                        .collect::<Result<Vec<_>, _>>()?,
                );
                if page.complete {
                    snapshot = Some(summaries);
                    break;
                }
                q.cursor = page.next_cursor;
            }
            if snapshot.is_some() {
                break;
            }
            if !failed {
                bail!("registry page bound exceeded");
            }
        }
        let snapshot = snapshot.context("registry snapshot unavailable; cached data retained")?;
        let mut verified = Vec::new();
        for b in snapshot {
            let claim = self.evidence(keys, &b.generation, &b.claim_id).await?;
            let ack = self.evidence(keys, &b.generation, &b.ack_id).await?;
            let binding = self
                .page(keys, query("binding", Some(&b.generation), None)?)
                .await?;
            let current: RegistryBinding = serde_json::from_value(binding.data)?;
            ensure!(
                current.claim_id == b.claim_id
                    && current.ack_id == b.ack_id
                    && current.host == b.host
                    && current.owner == b.owner,
                "binding changed"
            );
            // Restore labels from authenticated historical status, without freshness.
            let status = if let Some(id) = &current.latest_status_id {
                Some(self.evidence(keys, &b.generation, id).await?)
            } else {
                None
            };
            let scope = device.restore_registry(
                &self.community,
                keys,
                &current,
                &claim,
                &ack,
                status.as_ref(),
            )?;
            if let Some(id) = &current.tombstone_id {
                let tomb = self.evidence(keys, &b.generation, id).await?;
                device.accept_registry_tombstone(&self.community, keys, &scope, &tomb)?;
            }
            verified.push(scope);
        }
        device.registry_reconcile_list(&self.community, keys, &verified)?;
        Ok(verified)
    }
    /// Fetch one exact retained event; knowing its ID never supplies authorization.
    pub async fn evidence(&mut self, keys: &Keys, g: &str, id: &str) -> Result<Event> {
        let reply = self
            .page(keys, query("evidence", Some(g), Some(id))?)
            .await?;
        let raw = serde_json::to_vec(&reply.data["event"])?;
        let e = wire::parse_event(&raw, wire::MAX_EVENT)?;
        ensure!(e.id.to_hex() == id, "wrong evidence");
        Ok(e)
    }
    /// Explicit close releases the root socket and live subscription.
    pub async fn disconnect(self) -> Result<()> {
        self.ws.disconnect().await?;
        Ok(())
    }
}
/// Build an exact v2 protected query with a new nonce and explicit null cursor.
pub fn query(view: &str, g: Option<&str>, event_id: Option<&str>) -> Result<Query> {
    let q = Query {
        v: 2,
        view: view.into(),
        nonce: random()?,
        cursor: None,
        generation: g.map(str::to_owned),
        event_id: event_id.map(str::to_owned),
    };
    q.validate()?;
    Ok(q)
}
