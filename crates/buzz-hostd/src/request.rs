//! Durable request state. Every entry point runs inside the grant's SQLite
//! IMMEDIATE transaction, including key creation, commit and local revocation.
use crate::{
    clock::Clock,
    store::{issue, random, Catalog, InvalidPacket},
};
use anyhow::{anyhow, ensure, Result};
use buzz_core::{
    paired_message::{sign_possession, Body, Message, Packet, Policy, Possession, Scope},
    Keys,
};
use nostr::SecretKey;
use rusqlite::{params, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};

pub const SCHEMA:&str="
CREATE TABLE requests(g TEXT NOT NULL,r TEXT NOT NULL, intent_id TEXT NOT NULL UNIQUE,name TEXT UNIQUE,record TEXT NOT NULL,PRIMARY KEY(g,r));
";

#[derive(Deserialize, Serialize)]
pub(crate) struct Request {
    pub r: String,
    pub intent: String,
    pub expiry: u64,
    pub runtime: String,
    pub name: String,
    pub policy: Policy,
    pub channels: Vec<String>,
    pub revision: String,
    pub state: String,
    pub phase_id: String,
    pub nonce: String,
    pub boot: String,
    pub mono: u64,
    pub deadline: u64,
    pub agent: Option<String>,
    pub agent_secret: Option<String>,
    pub confirm: Option<String>,
    pub offer: Option<String>,
    pub activate: Option<String>,
    pub commit: Option<String>,
    pub terminal: Option<String>,
    pub credentials: Option<String>,
    pub original: Packet,
    pub activation: Option<Packet>,
    #[serde(default)]
    pub progress: Vec<String>,
    #[serde(default)]
    pub start_attempted: bool,
    #[serde(default)]
    pub sealed: bool,
    #[serde(default)]
    pub started_at: Option<u64>,
    #[serde(default)]
    pub failure: Option<String>,
}

impl Request {
    pub(crate) fn load(tx: &Transaction<'_>, g: &str, r: &str) -> Result<Option<Self>> {
        let s: Option<String> = tx
            .query_row(
                "SELECT record FROM requests WHERE g=? AND r=?",
                params![g, r],
                |row| row.get(0),
            )
            .optional()?;
        s.map(|raw| serde_json::from_str(&raw).map_err(Into::into))
            .transpose()
    }
    pub(crate) fn update(&self, tx: &Transaction<'_>, g: &str) -> Result<()> {
        tx.execute(
            "UPDATE requests SET record=? WHERE g=? AND r=?",
            params![serde_json::to_string(&self)?, g, self.r],
        )?;
        Ok(())
    }
    pub(crate) fn agent_keys(&self) -> Result<Keys> {
        let keys = Keys::new(SecretKey::from_hex(
            self.agent_secret
                .as_deref()
                .ok_or_else(|| anyhow!("no candidate"))?,
        )?);
        ensure!(
            self.agent.as_deref() == Some(keys.public_key().to_hex().as_str()),
            "stored agent identity mismatch"
        );
        Ok(keys)
    }
    fn set_nonce(&mut self, clock: &Clock) -> Result<()> {
        ensure!(self.expiry > clock.wall, "intent expired");
        self.nonce = random()?;
        self.boot = clock.boot.clone();
        self.deadline = self.expiry.min(clock.wall + 60);
        self.mono = clock.millis + (self.deadline - clock.wall) * 1000;
        Ok(())
    }
    fn fresh(&self, clock: &Clock) -> bool {
        self.deadline > clock.wall && self.mono > clock.millis && self.boot == clock.boot
    }
    fn challenge(
        &mut self,
        tx: &Transaction<'_>,
        keys: &Keys,
        scope: &Scope,
        clock: &Clock,
    ) -> Result<()> {
        self.set_nonce(clock)?;
        self.phase_id = issue(
            tx,
            keys,
            scope,
            Body::Challenge {
                request_id: self.r.clone(),
                intent_id: self.intent.clone(),
                phase: "generate".into(),
                nonce: self.nonce.clone(),
            },
            clock.wall,
            self.deadline,
        )?;
        self.update(tx, &scope.generation)
    }
    fn key_offer(
        &mut self,
        tx: &Transaction<'_>,
        keys: &Keys,
        scope: &Scope,
        clock: &Clock,
    ) -> Result<()> {
        self.set_nonce(clock)?;
        let agent = self.agent_keys()?;
        let confirm_id = self
            .confirm
            .clone()
            .ok_or_else(|| anyhow!("missing confirm"))?;
        let proof = sign_possession(
            scope,
            &Possession {
                request_id: &self.r,
                intent_id: &self.intent,
                confirm_id: &confirm_id,
                agent: agent.public_key(),
                nonce: &self.nonce,
                expires_at: self.deadline,
            },
            &agent,
        )?;
        self.phase_id = issue(
            tx,
            keys,
            scope,
            Body::KeyOffer {
                request_id: self.r.clone(),
                intent_id: self.intent.clone(),
                confirm_id,
                agent: agent.public_key().to_hex(),
                nonce: self.nonce.clone(),
                agent_sig: proof,
            },
            clock.wall,
            self.deadline,
        )?;
        self.offer = Some(self.phase_id.clone());
        self.update(tx, &scope.generation)
    }
    pub fn finish(
        &mut self,
        tx: &Transaction<'_>,
        keys: &Keys,
        scope: &Scope,
        state: &str,
        step: &str,
        clock: &Clock,
    ) -> Result<()> {
        ensure!(self.terminal.is_none(), "already terminal");
        let mut steps = self.progress.clone();
        steps.push(step.into());
        let id = issue(
            tx,
            keys,
            scope,
            Body::Result {
                request_id: self.r.clone(),
                intent_id: self.intent.clone(),
                commit_id: self.commit.clone(),
                agent: self.agent.clone(),
                state: state.into(),
                steps,
            },
            clock.wall,
            clock.wall + 86400,
        )?;
        self.state = state.into();
        self.terminal = Some(id);
        self.update(tx, &scope.generation)
    }
}

fn requeue(tx: &Transaction<'_>, id: &str) -> Result<()> {
    ensure!(
        tx.execute("UPDATE sent SET pending=1 WHERE id=?", [id])? == 1,
        "missing local outbox event"
    );
    Ok(())
}

pub fn handle(
    tx: &Transaction<'_>,
    keys: &Keys,
    scope: &Scope,
    active: bool,
    msg: &Message,
    packet: &Packet,
    cat: &Catalog,
    revision: &str,
    clock: &Clock,
) -> Result<()> {
    let g = &scope.generation;
    let r = packet.route.r.as_deref().ok_or(InvalidPacket)?;
    let existing = Request::load(tx, g, r)?;
    if let Body::StatusQuery {
        intent_id, nonce, ..
    } = &msg.body
    {
        let request = existing.filter(|q| q.intent == *intent_id);
        let commit_id = request.as_ref().and_then(|q| q.commit.clone());
        let terminal_id = request.as_ref().and_then(|q| q.terminal.clone());
        for id in [commit_id.as_ref(), terminal_id.as_ref()]
            .into_iter()
            .flatten()
        {
            requeue(tx, id)?;
        }
        issue(
            tx,
            keys,
            scope,
            Body::Status {
                request_id: r.into(),
                intent_id: intent_id.clone(),
                query_id: msg.id.clone(),
                nonce: nonce.clone(),
                state: request
                    .as_ref()
                    .map_or("unknown", |q| q.state.as_str())
                    .into(),
                commit_id,
                terminal_id,
            },
            clock.wall,
            msg.expires_at,
        )?;
        return Ok(());
    }
    let mut q = if let Some(existing) = existing {
        existing
    } else {
        let Body::Create {
            catalog_id,
            runtime,
            name,
            policy,
            channels,
            ..
        } = &msg.body
        else {
            return Err(InvalidPacket.into());
        };
        ensure!(active, InvalidPacket);
        let inventory:Option<(String,String)>=tx.query_row("SELECT s.packet,i.revision FROM sent s JOIN inventories i ON i.id=s.id WHERE s.id=? AND s.g=? AND s.op='inventory'",params![catalog_id,g],|row|Ok((row.get(0)?,row.get(1)?))).optional()?;
        let matches_catalog = match inventory {
            Some((p, rev)) => {
                rev == revision
                    && serde_json::from_str::<Packet>(&p)?.route.expires_at > clock.wall
                    && cat.runtimes.contains_key(runtime)
            }
            None => false,
        };
        let pending: u64 = tx.query_row(
            "SELECT COUNT(*) FROM requests WHERE g=? AND json_extract(record,'$.state')='pending'",
            [g],
            |row| row.get(0),
        )?;

        let mut q = Request {
            r: r.into(),
            intent: msg.id.clone(),
            expiry: msg.expires_at,
            runtime: runtime.clone(),
            name: name.clone(),
            policy: policy.clone(),
            channels: channels.clone(),
            revision: revision.into(),
            state: "pending".into(),
            phase_id: String::new(),
            nonce: String::new(),
            boot: String::new(),
            mono: 0,
            deadline: 0,
            agent: None,
            agent_secret: None,
            confirm: None,
            offer: None,
            activate: None,
            commit: None,
            terminal: None,
            credentials: None,
            original: packet.clone(),
            activation: None,
            progress: Vec::new(),
            start_attempted: false,
            sealed: false,
            started_at: None,
            failure: None,
        };
        tx.execute(
            "INSERT INTO requests(g,r,intent_id,record) VALUES(?,?,?,?)",
            params![g, r, msg.id, serde_json::to_string(&q)?],
        )?;
        if !matches_catalog {
            return q.finish(tx, keys, scope, "failed", "catalog-changed", clock);
        }
        if pending >= 8 {
            return q.finish(tx, keys, scope, "failed", "capacity-or-name", clock);
        }
        q.challenge(tx, keys, scope, clock)?;
        return Ok(());
    };
    let intent = match &msg.body {
        Body::Create { .. } => &msg.id,
        Body::Confirm { intent_id, .. }
        | Body::Activate { intent_id, .. }
        | Body::Credentials { intent_id, .. } => intent_id,
        _ => return Err(InvalidPacket.into()),
    };
    ensure!(intent == &q.intent, InvalidPacket);
    if let Some(terminal) = &q.terminal {
        return requeue(tx, terminal);
    }
    if let Body::Credentials {
        commit_id,
        agent,
        oa,
        registration,
        ..
    } = &msg.body
    {
        ensure!(
            q.commit.as_ref() == Some(commit_id) && q.agent.as_ref() == Some(agent),
            InvalidPacket
        );
        if let Some(saved) = &q.credentials {
            ensure!(saved == &msg.id, InvalidPacket);
            return Ok(());
        }
        super::onboard::accept(tx, scope, &q, oa, registration, clock)?;
        q.credentials = Some(msg.id.clone());
        return q.update(tx, g);
    }
    if let Some(id) = &q.commit {
        return requeue(tx, id);
    }
    if !active {
        return q.finish(tx, keys, scope, "revoked-before-commit", "revoked", clock);
    }
    if q.expiry <= clock.wall {
        return q.finish(tx, keys, scope, "expired", "expired", clock);
    }
    if q.revision != revision || !cat.runtimes.contains_key(&q.runtime) {
        return q.finish(tx, keys, scope, "failed", "catalog-changed", clock);
    }
    match &msg.body {
        Body::Create { .. } => {
            if q.fresh(clock) {
                return requeue(tx, &q.phase_id);
            }
            if q.agent.is_some() {
                q.key_offer(tx, keys, scope, clock)
            } else {
                q.challenge(tx, keys, scope, clock)
            }
        }
        Body::Confirm {
            challenge_id,
            nonce,
            ..
        } => {
            if q.confirm.as_ref() == Some(&msg.id) {
                if q.fresh(clock) {
                    return requeue(tx, &q.phase_id);
                }
                return q.key_offer(tx, keys, scope, clock);
            }
            ensure!(
                q.confirm.is_none()
                    && *challenge_id == q.phase_id
                    && *nonce == q.nonce
                    && q.fresh(clock)
                    && msg.expires_at <= q.deadline,
                InvalidPacket
            );
            let allocated: u32 = tx.query_row(
                "SELECT COUNT(*) FROM requests WHERE name IS NOT NULL",
                [],
                |r| r.get(0),
            )?;
            let busy: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM requests WHERE name=?)",
                [&q.name],
                |r| r.get(0),
            )?;
            if allocated >= cat.capacity || busy {
                return q.finish(tx, keys, scope, "failed", "capacity-or-name", clock);
            }
            tx.execute(
                "UPDATE requests SET name=? WHERE g=? AND r=?",
                params![q.name, g, r],
            )?;
            let key = Keys::generate();
            q.agent = Some(key.public_key().to_hex());
            q.agent_secret = Some(key.secret_key().to_secret_hex());
            q.confirm = Some(msg.id.clone());
            q.key_offer(tx, keys, scope, clock)
        }
        Body::Activate {
            offer_id,
            agent,
            nonce,
            ..
        } => {
            ensure!(
                q.offer.as_ref() == Some(offer_id)
                    && q.agent.as_ref() == Some(agent)
                    && &q.nonce == nonce,
                InvalidPacket
            );
            if !q.fresh(clock) || msg.expires_at > q.deadline {
                return q.finish(tx, keys, scope, "expired", "expired", clock);
            }
            let rt = cat
                .runtimes
                .get(&q.runtime)
                .ok_or_else(|| anyhow!("runtime removed"))?;
            if rt.harness.verify().is_err()
                || rt.adapter.verify().is_err()
                || buzz_hostd::provider_path(&rt.credential_profile).is_err()
                || !cat.current(&q.revision)?
            {
                return q.finish(tx, keys, scope, "failed", "catalog-changed", clock);
            }
            let clock = &clock.refresh()?;
            crate::store::check_clock(tx, clock)?;
            if !q.fresh(clock) || msg.expires_at <= clock.wall {
                return q.finish(tx, keys, scope, "expired", "expired", clock);
            }
            q.agent_keys()?;
            q.activation = Some(packet.clone());
            q.activate = Some(msg.id.clone());
            q.commit = Some(issue(
                tx,
                keys,
                scope,
                Body::Committed {
                    request_id: r.into(),
                    intent_id: q.intent.clone(),
                    activate_id: msg.id.clone(),
                    offer_id: offer_id.clone(),
                    agent: agent.clone(),
                },
                clock.wall,
                clock.wall + 86400,
            )?);
            q.state = "committed".into();
            q.update(tx, g)
        }
        _ => Err(InvalidPacket.into()),
    }
}

/// Expire locally even during a partition. After receiving the mailbox, renew
/// the outstanding challenge using the same persisted confirm and candidate.
/// The relay deduplicates original intents, so replay cannot be our wakeup.
pub fn tick(
    store: &mut crate::store::Store,
    g: &str,
    clock: &Clock,
    revision: &str,
    renew: bool,
) -> Result<()> {
    let keys = store.keys()?;
    let tx = store.tx()?;
    let rows = {
        let mut stmt = tx.prepare(
            "SELECT record FROM requests WHERE g=? AND json_extract(record,'$.state')='pending'",
        )?;
        let rows = stmt.query_map([g], |r| r.get::<_, String>(0))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    if rows.is_empty() {
        return Ok(());
    }
    let (scope, _, state) = crate::store::binding(&tx, g, keys.public_key())?;
    crate::store::check_clock(&tx, clock)?;
    for raw in rows {
        let mut q: Request = serde_json::from_str(&raw)?;
        if state != "active" {
            q.finish(
                &tx,
                &keys,
                &scope,
                "revoked-before-commit",
                "revoked",
                clock,
            )?;
        } else if q.expiry <= clock.wall {
            q.finish(&tx, &keys, &scope, "expired", "expired", clock)?;
        } else if q.revision != revision {
            q.finish(&tx, &keys, &scope, "failed", "catalog-changed", clock)?;
        } else if renew && !q.fresh(clock) {
            if q.confirm.is_some() {
                q.key_offer(&tx, &keys, &scope, clock)?;
            } else {
                q.challenge(&tx, &keys, &scope, clock)?;
            }
        }
    }
    tx.commit()?;
    Ok(())
}

pub fn expire_delivery(
    tx: &Transaction<'_>,
    keys: &Keys,
    scope: &Scope,
    m: buzz_core::paired_message::ExpiredActivation,
    clock: &Clock,
) -> Result<()> {
    let g = &scope.generation;
    let mut q = Request::load(tx, g, &m.request_id)?.ok_or(InvalidPacket)?;
    ensure!(
        q.intent == m.intent_id && q.agent.as_ref() == Some(&m.agent),
        InvalidPacket
    );
    let raw: String = tx
        .query_row(
            "SELECT packet FROM sent WHERE id=? AND g=? AND op='key-offer'",
            params![m.offer_id, g],
            |r| r.get(0),
        )
        .optional()?
        .ok_or(InvalidPacket)?;
    let signed = buzz_core::paired_message::restore_sent(
        keys,
        scope,
        &serde_json::from_str(&raw)?,
        &m.offer_id,
    )?;
    ensure!(
        matches!(signed.body,Body::KeyOffer{request_id,intent_id,nonce,agent,..} if request_id==m.request_id && intent_id==m.intent_id && nonce==m.nonce && agent==m.agent),
        InvalidPacket
    );
    if let Some(id) = q.terminal.as_ref().or(q.commit.as_ref()) {
        return requeue(tx, id);
    }
    q.finish(tx, keys, scope, "expired", "expired", clock)
}
