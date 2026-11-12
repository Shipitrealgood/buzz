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
    /// Registration-only cards; legacy provisioning journals are not v2 hosts.
    pub fn registry_hosts(&self, selected: &str, keys: &Keys) -> Result<Vec<HostView>> {
        let mut q = self.db.prepare("SELECT scope FROM grants WHERE json_extract(scope,'$.community')=? AND json_extract(scope,'$.owner')=? AND json_extract(data,'$.claim.event.kind')=43103 ORDER BY rowid")?;
        let rows = q.query_map(
            rusqlite::params![selected, keys.public_key().to_hex()],
            |r| r.get::<_, String>(0),
        )?;
        rows.map(|r| self.view(selected, keys, &serde_json::from_str(&r?)?))
            .collect()
    }
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
    /// Mark absent cached cards stale; absence alone cannot certify revocation.
    /// A protected binding and host-signed tombstone retire a durable outbox.
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
            let g: Grant = serde_json::from_str(&raw)?;
            if g.ack.is_some()
                && event(&g.claim)?.kind.as_u16() == 43103
                && !current.contains(&g.scope)
            {
                self.fresh.remove(&coordinate(&g.scope)?);
            }
        }
        tx.commit()?;
        Ok(())
    }
    fn pending_registry_scopes(&self, selected: &str, keys: &Keys) -> Result<Vec<Scope>> {
        let mut q=self.db.prepare("SELECT scope FROM grants WHERE json_extract(scope,'$.community')=? AND json_extract(scope,'$.owner')=? AND json_extract(data,'$.claim.event.kind')=43103 AND json_extract(data,'$.ack') IS NOT NULL AND json_extract(data,'$.tombstone') IS NULL ORDER BY scope LIMIT 513")?;
        let rows = q.query_map(
            rusqlite::params![selected, keys.public_key().to_hex()],
            |r| r.get::<_, String>(0),
        )?;
        let scopes = rows
            .map(|r| Ok(serde_json::from_str(&r?)?))
            .collect::<Result<Vec<Scope>>>()?;
        ensure!(scopes.len() <= 512, "registration cache bound exceeded");
        Ok(scopes)
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
/// Selected relay explicitly omits registration support; transport failures are
/// a different state and must not be presented as unsupported or an empty list.
#[derive(Debug)]
pub struct Unsupported;
impl std::fmt::Display for Unsupported {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("host registration unsupported")
    }
}
impl std::error::Error for Unsupported {}

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
        Self::connect_guarded(selected, &|| Ok(keys.clone())).await
    }
    /// Discover and authenticate with the native scope checked after every wait,
    /// including the AUTH challenge, before using its signing authority.
    pub async fn connect_guarded<F>(selected: &str, signer: &F) -> Result<Self>
    where
        F: Fn() -> Result<Keys>,
    {
        let owner = signer()?.public_key();
        let endpoint = Endpoint::new(selected, owner)?;
        let metadata = endpoint.metadata().await?;
        ensure!(signer()?.public_key() == owner, "active signer changed");
        if metadata["host_registry"].is_null() {
            return Err(Unsupported.into());
        }
        let relay = Self::capability(&metadata)?;
        let mut ws = tokio::time::timeout(
            Duration::from_secs(10),
            NostrWsConnection::connect(&selected.replacen("https://", "wss://", 1)),
        )
        .await??;
        ws.authenticate_with_signer(|challenge, url| {
            let keys = signer().map_err(|_| {
                buzz_ws_client::WsClientError::AuthFailed("active session changed".into())
            })?;
            if keys.public_key() != owner {
                return Err(buzz_ws_client::WsClientError::AuthFailed(
                    "active signer changed".into(),
                ));
            }
            buzz_ws_client::build_auth_event(challenge, url, &keys, None)
        })
        .await?;
        ensure!(signer()?.public_key() == owner, "active signer changed");
        Self::from_authenticated(selected, owner, relay, ws)
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
    /// Reconcile authoritative evidence before retrying exact durable deliveries.
    /// Host-local revocation can retire an owner intent that the relay never saw;
    /// retrying that rejected intent first would strand its terminal evidence.
    pub async fn synchronize_guarded<F>(&mut self, device: &mut Device, signer: &F) -> Result<()>
    where
        F: Fn() -> Result<Keys>,
    {
        self.recover_guarded(device, signer).await?;
        for (_, packet, _) in device.next_deliveries(&self.community, &signer()?)? {
            let event = event(&packet)?;
            if event.kind.as_u16() == 43103 {
                self.publish(&signer()?, event).await?;
                ensure!(
                    signer()?.public_key() == self.owner,
                    "active signer changed"
                );
            }
        }
        Ok(())
    }
    /// Recover complete authoritative lists with at most three snapshot retries.
    /// Cached cards survive an unavailable/partial snapshot. Freshness requires a
    /// separately answered status nonce for each host.
    pub async fn recover(&mut self, device: &mut Device, keys: &Keys) -> Result<Vec<Scope>> {
        self.recover_guarded(device, &|| Ok(keys.clone())).await
    }
    /// Recover with a native session fence before every durable journal effect.
    pub async fn recover_guarded<F>(
        &mut self,
        device: &mut Device,
        signer: &F,
    ) -> Result<Vec<Scope>>
    where
        F: Fn() -> Result<Keys>,
    {
        let keys = signer()?;
        let owner = self.owner;
        let guard = || -> Result<()> {
            ensure!(signer()?.public_key() == owner, "active signer changed");
            Ok(())
        };
        self.subscribe(&keys).await?;
        guard()?;

        let mut snapshot = None;
        for _ in 0..3 {
            let mut q = query("hosts", None, None)?;
            let mut summaries = Vec::new();
            let mut failed = false;
            for _ in 0..32 {
                let page = match self.page(&keys, q.clone()).await {
                    Ok(p) => p,
                    Err(_) => {
                        failed = true;
                        break;
                    }
                };
                guard()?;
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
        let known = device.pending_registry_scopes(&self.community, &keys)?;
        let listed: std::collections::HashSet<_> =
            snapshot.iter().map(|b| b.generation.clone()).collect();
        let mut verified = Vec::new();
        for b in snapshot {
            let (scope, live) = self.restore_one(device, &keys, &b, &guard).await?;
            if live {
                verified.push(scope);
            }
        }
        guard()?;
        device.registry_reconcile_list(&self.community, &keys, &verified)?;
        // DEAD grants leave the current list. A cached pending operation still
        // needs exact protected evidence before it can retire its durable retry.
        for scope in known
            .into_iter()
            .filter(|s| !listed.contains(&s.generation))
        {
            let page = self
                .page(&keys, query("binding", Some(&scope.generation), None)?)
                .await?;
            guard()?;
            let b: RegistryBinding = serde_json::from_value(page.data)?;
            ensure!(
                b.generation == scope.generation
                    && b.owner == scope.owner.to_hex()
                    && b.host == scope.host.to_hex(),
                "binding changed"
            );
            self.restore_one(device, &keys, &b, &guard).await?;
        }
        Ok(verified)
    }
    async fn restore_one(
        &mut self,
        device: &mut Device,
        keys: &Keys,
        b: &RegistryBinding,
        guard: &impl Fn() -> Result<()>,
    ) -> Result<(Scope, bool)> {
        let binding = self
            .page(keys, query("binding", Some(&b.generation), None)?)
            .await?;
        guard()?;
        let current: RegistryBinding = serde_json::from_value(binding.data)?;
        ensure!(
            current.claim_id == b.claim_id
                && current.ack_id == b.ack_id
                && current.host == b.host
                && current.owner == b.owner,
            "binding changed"
        );
        let claim = self
            .evidence(keys, &b.generation, &current.claim_id)
            .await?;
        let ack = self.evidence(keys, &b.generation, &current.ack_id).await?;
        let status = if let Some(id) = &current.latest_status_id {
            Some(self.evidence(keys, &b.generation, id).await?)
        } else {
            None
        };
        guard()?;
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
            guard()?;
            device.accept_registry_tombstone(&self.community, keys, &scope, &tomb)?;
        }
        Ok((scope, current.state != "dead"))
    }
    /// Query one installed runtime without creating an agent or durable create intent.
    pub async fn models<F>(
        &mut self,
        signer: &F,
        scope: &Scope,
        runtime: &str,
        revision: &str,
    ) -> Result<wire::HostModels>
    where
        F: Fn() -> Result<Keys>,
    {
        session(scope, &self.community, &signer()?)?;
        self.subscribe(&signer()?).await?;
        let page = self
            .page(&signer()?, query("binding", Some(&scope.generation), None)?)
            .await?;
        let binding: RegistryBinding = serde_json::from_value(page.data)?;
        ensure!(
            binding.state == "active"
                && binding.owner == scope.owner.to_hex()
                && binding.host == scope.host.to_hex()
                && binding.generation == scope.generation,
            "host unavailable"
        );
        let ack = self
            .evidence(&signer()?, &scope.generation, &binding.ack_id)
            .await?;
        let (_, Body::PairAck { claim_id }) =
            wire::open(&signer()?, scope, &ack, ack.created_at.as_secs())?
        else {
            bail!("invalid enrollment")
        };
        ensure!(
            ack.pubkey == scope.host && claim_id == binding.claim_id,
            "invalid enrollment"
        );
        let request = wire::ModelsQuery {
            ack_id: binding.ack_id,
            nonce: random()?,
            runtime_id: runtime.into(),
            catalog_revision: revision.into(),
        };
        let issued = now()?;
        let event = wire::seal(
            &signer()?,
            scope,
            Body::ModelsQuery {
                request: request.clone(),
            },
            random()?,
            Some(request.ack_id.clone()),
            issued,
            issued + 60,
        )?;
        let parent = event.id.to_hex();
        self.publish(&signer()?, event).await?;
        tokio::time::timeout(
            Duration::from_secs((issued + 60).saturating_sub(now()?)),
            async {
                loop {
                    let mut q = query("messages", Some(&scope.generation), None)?;
                    let mut complete = false;
                    for _ in 0..22 {
                        let page = self.page(&signer()?, q.clone()).await?;
                        let wall = now()?;
                        for entry in page.data["events"].as_array().context("missing messages")? {
                            if entry["op"] != "models"
                                || entry["expires_at"].as_u64().is_none_or(|t| t <= wall)
                            {
                                continue;
                            }
                            let e = self
                                .evidence(
                                    &signer()?,
                                    &scope.generation,
                                    entry["id"].as_str().context("missing event ID")?,
                                )
                                .await?;
                            let (h, body) = wire::open(&signer()?, scope, &e, now()?)?;
                            if h.parent.as_deref() != Some(&parent) {
                                continue;
                            }
                            let Body::Models { result } = body else {
                                bail!("invalid models response")
                            };
                            ensure!(
                                e.pubkey == scope.host
                                    && result.request == request
                                    && h.expires_at == issued + 60
                                    && result.observed_at.saturating_add(30) >= issued,
                                "models response mismatch"
                            );
                            session(scope, &self.community, &signer()?)?;
                            return Ok(result);
                        }
                        if page.complete {
                            complete = true;
                            break;
                        }
                        q.cursor = page.next_cursor;
                    }
                    ensure!(complete, "models mailbox page bound exceeded");
                    ensure!(now()? < issued + 60, "host models timed out");
                    self.wait_for_refresh(&signer()?, Duration::from_secs(5))
                        .await?;
                }
            },
        )
        .await
        .context("host models timed out")?
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

#[cfg(test)]
mod registration_adapter_tests {
    use super::*;
    #[test]
    fn invitation_validation_and_exact_retry_keep_registration_separate() {
        let dir = tempfile::tempdir().unwrap();
        let mut device = Device::at(&dir.path().join("owner.sqlite")).unwrap();
        let owner = Keys::generate();
        let host = Keys::generate();
        let origin = "https://fixture.test";
        let mut invitation = json!({"v":2,"community":origin,"host":host.public_key().to_hex(),"generation":"aa".repeat(32),"proof":"bb".repeat(32),"expires_at":now().unwrap()+600});
        for (field, value) in [
            ("v", json!(1)),
            ("community", json!("https://other.test")),
            ("expires_at", json!(now().unwrap() - 1)),
            ("proof", json!("invalid")),
        ] {
            let old = invitation[field].clone();
            invitation[field] = value;
            assert!(device
                .pair_registry(origin, &owner, &invitation.to_string())
                .is_err());
            assert!(device.registry_hosts(origin, &owner).unwrap().is_empty());
            invitation[field] = old;
        }
        let (scope, first) = device
            .pair_registry(origin, &owner, &invitation.to_string())
            .unwrap();
        let (_, retry) = device
            .pair_registry(origin, &owner, &invitation.to_string())
            .unwrap();
        assert_eq!(first.id, retry.id);
        assert_eq!(
            device.registry_hosts(origin, &owner).unwrap()[0].state,
            "pair-pending"
        );
        let other = Keys::generate();
        assert!(device.registry_hosts(origin, &other).unwrap().is_empty());
        assert!(device.view(origin, &other, &scope).is_err());
    }
    #[tokio::test]
    async fn models_wire_adapter_correlates_host_runtime_revision_nonce_and_parent() {
        use futures_util::{SinkExt, StreamExt};
        for fault in [
            "ok",
            "runtime",
            "revision",
            "nonce",
            "parent",
            "host",
            "generation",
        ] {
            let owner = Keys::generate();
            let host = Keys::generate();
            let relay = Keys::generate();
            let scope = Scope {
                community: "https://models.test".into(),
                owner: owner.public_key(),
                host: host.public_key(),
                generation: "aa".repeat(32),
            };
            let issued = now().unwrap();
            let claim = "bb".repeat(32);
            let ack = wire::seal(
                &host,
                &scope,
                Body::PairAck {
                    claim_id: claim.clone(),
                },
                "cc".repeat(32),
                Some(claim.clone()),
                issued,
                issued + 600,
            )
            .unwrap();
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let relay_key = relay.public_key();
            let served_scope = scope.clone();
            let peer = tokio::spawn(async move {
                let (tcp, _) = listener.accept().await.unwrap();
                let mut ws = tokio_tungstenite::accept_async(tcp).await.unwrap();
                let mut response: Option<Event> = None;
                while let Some(Ok(frame)) = ws.next().await {
                    if !frame.is_text() {
                        continue;
                    }
                    let Ok(text) = frame.to_text() else { continue };
                    let message: Value = serde_json::from_str(text).unwrap();
                    if message[0] == "EVENT" {
                        let e: Event = serde_json::from_value(message[1].clone()).unwrap();
                        let (request_header, Body::ModelsQuery { mut request }) =
                            wire::open(&host, &served_scope, &e, issued).unwrap()
                        else {
                            panic!("request")
                        };
                        match fault {
                            "runtime" => request.runtime_id = "other/harness".into(),
                            "revision" => request.catalog_revision = "ee".repeat(32),
                            "nonce" => request.nonce = "ee".repeat(32),
                            _ => {}
                        }
                        let mut reply_scope = served_scope.clone();
                        if fault == "generation" {
                            reply_scope.generation = "ff".repeat(32);
                        }
                        let other = Keys::generate();
                        if fault == "host" {
                            reply_scope.host = other.public_key();
                        }
                        let result = wire::HostModels {
                            request,
                            observed_at: issued,
                            models: buzz_core::host_deployment::observation::Models::Empty,
                            readiness: buzz_core::harness::Readiness::LoggedIn,
                            availability: buzz_core::harness::AcpAvailabilityStatus::Available,
                        };
                        response = Some(
                            wire::seal(
                                if fault == "host" { &other } else { &host },
                                &reply_scope,
                                Body::Models { result },
                                "dd".repeat(32),
                                Some(if fault == "parent" {
                                    "ff".repeat(32)
                                } else {
                                    e.id.to_hex()
                                }),
                                issued,
                                request_header.expires_at,
                            )
                            .unwrap(),
                        );
                        ws.send(tokio_tungstenite::tungstenite::Message::Text(
                            json!(["OK", e.id, true, ""]).to_string().into(),
                        ))
                        .await
                        .unwrap();
                    } else if message[0] == "REQ" {
                        let q: Query =
                            serde_json::from_value(message[2]["host_registry"].clone()).unwrap();
                        let data = match q.view.as_str() {
                            "live" => json!({}),
                            "binding" => {
                                json!({"generation":served_scope.generation,"owner":served_scope.owner,"host":served_scope.host,"state":"active","claim_id":claim,"ack_id":ack.id,"tombstone_id":null,"latest_status_id":null})
                            }
                            "evidence" => {
                                json!({"event":if q.event_id.as_deref()==Some(&ack.id.to_hex()) {&ack} else {response.as_ref().unwrap()}})
                            }
                            "messages" => {
                                json!({"events":[{"op":"models","id":response.as_ref().unwrap().id,"expires_at":issued+60}]})
                            }
                            _ => panic!("unexpected view"),
                        };
                        let page = wire::Reply {
                            v: 2,
                            community: served_scope.community.clone(),
                            recipient: served_scope.owner.to_hex(),
                            op: "registry-page".into(),
                            query: q,
                            high_water: "00".repeat(32),
                            data,
                            next_cursor: None,
                            complete: true,
                        };
                        let event = wire::reply(&relay, served_scope.owner, page, issued).unwrap();
                        for frame in [
                            json!(["EVENT", message[1], event]),
                            json!(["EOSE", message[1]]),
                        ] {
                            ws.send(tokio_tungstenite::tungstenite::Message::Text(
                                frame.to_string().into(),
                            ))
                            .await
                            .unwrap();
                        }
                    }
                }
            });
            let ws = NostrWsConnection::connect(&format!("ws://{address}"))
                .await
                .unwrap();
            let mut registry =
                Registry::from_authenticated(&scope.community, owner.public_key(), relay_key, ws)
                    .unwrap();
            let result = tokio::time::timeout(
                Duration::from_secs(2),
                registry.models(
                    &|| Ok(owner.clone()),
                    &scope,
                    "main/claude",
                    &"11".repeat(32),
                ),
            )
            .await;
            if fault == "ok" {
                assert!(matches!(&result, Ok(Ok(_))), "{fault}: {result:?}");
            } else {
                assert!(!matches!(result, Ok(Ok(_))), "{fault}");
            }
            registry.disconnect().await.unwrap();
            peer.await.unwrap();
        }
    }
}
