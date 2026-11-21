use super::*;
use anyhow::{anyhow, Context};
use buzz_core::{
    paired_host::{self, Acknowledgement, Offer, Proof},
    paired_message::{open, seal, Body, Message, Route},
};
use nostr::{EventId, PublicKey};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::Path,
    time::{Duration, Instant},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Invitation<'a> {
    v: u8,
    community: &'a str,
    generation: &'a str,
    host: &'a str,
    proof: &'a str,
    expires_at: u64,
}

/// Non-secret, verified fields suitable for the UI. Freshness is per process.
#[derive(Clone, Serialize, Deserialize)]
pub struct HostView {
    /// Exact key and generation, independent of display label.
    pub scope: Scope,
    /// Enrollment, verified, revoking or unpaired.
    pub state: String,
    /// Most recent signed inventory (cached across restarts).
    pub label: Option<String>,
    /// Installed runtimes from the same verified inventory.
    pub runtimes: Vec<String>,
    /// Operator-signed capacity and catalog content revision, when available.
    pub capacity: Option<u32>,
    /// Revision shown when choosing an installed runtime.
    pub catalog_revision: Option<String>,
    /// Only a currently live nonce response permits a create.
    pub fresh: bool,
    /// Exact signed event for both cached and live views.
    pub inventory_id: Option<String>,
    /// Remaining native wall/monotonic lifetime measured just before returning.
    pub fresh_for_ms: u64,
    /// Requests whose original tap is stored on this device.
    pub requests: Vec<crate::RequestView>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Grant {
    pub scope: Scope,
    pub claim: Packet,
    #[serde(default)]
    pub previous_claims: Vec<Packet>,
    pub ack: Option<Packet>,
    pub tombstone: Option<Packet>,
    pub revocation: Option<Packet>,
    pub fenced: bool,
    pub inventory: Option<Packet>,
    pub sequence: u64,
    pub capacity: Option<u32>,
    pub catalog_revision: Option<String>,
    pub label: Option<String>,
    pub runtimes: Vec<String>,
}

impl Grant {
    pub(crate) fn active(&self) -> bool {
        self.ack.is_some() && !self.fenced && self.revocation.is_none() && self.tombstone.is_none()
    }
}

pub(crate) struct StatusQuery {
    pub id: String,
    pub nonce: String,
    pub since: Instant,
    pub after: String,
    pub consumed: bool,
}

/// Per-device durable state. Keep one instance for the unlocked native client.
/// The directory belongs to the OS user. The DB contains private inventory and ciphertext.
pub struct Device {
    pub(crate) db: Connection,
    pub(crate) local_keys: Option<Box<dyn Fn() -> Result<Vec<PublicKey>> + Send>>,
    pub(crate) queries: HashMap<String, (String, String, Instant)>,
    pub(crate) fresh: HashMap<String, (String, Instant, u64)>,
    pub(crate) status: HashMap<(String, String), StatusQuery>,
}
pub(crate) fn coordinate(s: &Scope) -> Result<String> {
    Ok(serde_json::to_string(s)?)
}
pub(crate) fn grant(db: &Connection, s: &Scope) -> Result<Grant> {
    let raw: String = db.query_row(
        "SELECT data FROM grants WHERE scope=?",
        [coordinate(s)?],
        |r| r.get(0),
    )?;
    let g: Grant = serde_json::from_str(&raw)?;
    ensure!(g.scope == *s, "wrong journal scope");
    Ok(g)
}
pub(crate) fn save(db: &Connection, g: &Grant) -> Result<()> {
    db.execute("INSERT INTO grants(scope,data) VALUES(?,?) ON CONFLICT(scope) DO UPDATE SET data=excluded.data",params![coordinate(&g.scope)?,serde_json::to_string(g)?])?;
    Ok(())
}

pub(super) fn verify_binding(
    keys: &Keys,
    scope: &Scope,
    claim: &Packet,
    ack: &Packet,
    now: u64,
) -> Result<()> {
    let expires_at = claim.route.expires_at;
    let offer = Offer {
        community: scope.community.clone(),
        generation: scope.generation.clone(),
        host: scope.host,
        expires_at,
    };
    let claim_id = EventId::from_hex(&id(claim)?)?;
    ensure!(
        claim.route.r.is_none()
            && claim.route.prev.is_none()
            && ack.route.r.is_none()
            && ack.route.prev == Some(claim_id.to_hex())
            && ack.route.expires_at == expires_at,
        "incorrect enrollment sidecar"
    );
    let saved =
        paired_host::restore_claim(claim.event.get().as_bytes(), keys, &offer, claim_id, now)?;
    match paired_host::verify_ack(ack.event.get().as_bytes(), keys, &saved, now)? {
        Acknowledgement::Fresh(_) | Acknowledgement::Historical(_) => Ok(()),
    }
}

impl Device {
    /// Open or initialize a device-local journal.
    pub fn at(path: &Path) -> Result<Self> {
        let db = Connection::open(path)?;
        db.execute_batch("PRAGMA synchronous=FULL; CREATE TABLE IF NOT EXISTS grants(scope TEXT PRIMARY KEY,data TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS requests(scope TEXT NOT NULL,r TEXT NOT NULL,data TEXT NOT NULL,PRIMARY KEY(scope,r));
            CREATE INDEX IF NOT EXISTS requests_by_id ON requests(r);
            CREATE TABLE IF NOT EXISTS known_identities(key TEXT PRIMARY KEY);
            CREATE TABLE IF NOT EXISTS delivery_cursor(community TEXT,owner TEXT,last_scope TEXT NOT NULL,PRIMARY KEY(community,owner));
            CREATE TABLE IF NOT EXISTS clock(id INTEGER PRIMARY KEY,wall INTEGER NOT NULL);")?;
        Ok(Self {
            db,
            local_keys: None,
            queries: HashMap::new(),
            fresh: HashMap::new(),
            status: HashMap::new(),
        })
    }

    pub(crate) fn check_time(&mut self, wall: u64) -> Result<()> {
        let n=self.db.execute("INSERT INTO clock VALUES(1,?) ON CONFLICT(id) DO UPDATE SET wall=excluded.wall WHERE excluded.wall>=wall",[wall])?;
        ensure!(n == 1, "local clock moved backwards");
        Ok(())
    }

    /// Persist the exact OOB claim before making it available for transmission.
    pub fn pair(
        &mut self,
        selected: &str,
        keys: &Keys,
        invitation: &str,
    ) -> Result<(Scope, Packet)> {
        ensure!(invitation.len() <= 4096, "invalid invitation");
        let inv: Invitation =
            serde_json::from_str(invitation).map_err(|_| anyhow!("invalid invitation"))?;
        ensure!(
            inv.v == 1 && inv.community == selected,
            "invitation community mismatch"
        );
        let offer = Offer {
            community: selected.into(),
            generation: inv.generation.into(),
            host: PublicKey::from_hex(inv.host)?,
            expires_at: inv.expires_at,
        };
        ensure!(offer.host.to_hex() == inv.host, "invalid invitation host");
        let scope = Scope {
            community: selected.into(),
            generation: offer.generation.clone(),
            owner: keys.public_key(),
            host: offer.host,
        };
        let now = now()?;
        self.check_time(now)?;
        let proof = Proof::from_hex(inv.proof)?;
        let (event, _) = paired_host::build_claim(keys, &offer, &proof, now)?;
        let claim = Packet {
            event: serde_json::value::to_raw_value(&event)?,
            route: Route {
                r: None,
                prev: None,
                expires_at: inv.expires_at,
            },
        };
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing: Option<String> = tx
            .query_row(
                "SELECT data FROM grants WHERE scope=?",
                [coordinate(&scope)?],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(raw) = existing {
            let mut old: Grant = serde_json::from_str(&raw)?;
            ensure!(
                old.ack.is_none() && old.claim.route.expires_at == inv.expires_at,
                "generation already paired"
            );
            let mut claims = old.previous_claims.clone();
            claims.push(old.claim.clone());
            let mut matching = None;
            for (idx, p) in claims.iter().enumerate() {
                let saved = paired_host::restore_claim(
                    p.event.get().as_bytes(),
                    keys,
                    &offer,
                    EventId::from_hex(&id(p)?)?,
                    now,
                )?;
                if saved.matches_proof(&proof) {
                    matching = Some(idx);
                    break;
                }
            }
            let current = if let Some(idx) = matching {
                claims.remove(idx)
            } else {
                ensure!(
                    claims.len() < 5,
                    "attempt budget exhausted; obtain a fresh host invitation"
                );
                claim
            };
            old.claim = current.clone();
            old.previous_claims = claims;
            save(&tx, &old)?;
            tx.commit()?;
            return Ok((scope, current));
        }
        save(
            &tx,
            &Grant {
                scope: scope.clone(),
                claim: claim.clone(),
                previous_claims: Vec::new(),
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
            },
        )?;
        tx.commit()?;
        Ok((scope, claim))
    }

    /// Restore both peers' exact signed evidence. Relay state can only remove authority.
    pub fn restore(&mut self, selected: &str, keys: &Keys, b: &Binding) -> Result<Scope> {
        let scope = Scope {
            community: selected.into(),
            generation: b.generation.clone(),
            owner: PublicKey::from_hex(&b.owner)?,
            host: PublicKey::from_hex(&b.host)?,
        };
        session(&scope, selected, keys)?;
        let now = now()?;
        self.check_time(now)?;
        verify_binding(keys, &scope, &b.claim, &b.ack, now)?;
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing = tx
            .query_row(
                "SELECT data FROM grants WHERE scope=?",
                [coordinate(&scope)?],
                |r| r.get::<_, String>(0),
            )
            .optional()?;
        let mut g: Grant = if let Some(raw) = existing {
            serde_json::from_str(&raw)?
        } else {
            Grant {
                scope: scope.clone(),
                claim: b.claim.clone(),
                previous_claims: Vec::new(),
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
        };
        ensure!(
            (id(&g.claim)? == id(&b.claim)?
                || g.previous_claims
                    .iter()
                    .any(|p| id(p).is_ok_and(|i| Some(i) == id(&b.claim).ok())))
                && g.ack
                    .as_ref()
                    .map(id)
                    .transpose()?
                    .is_none_or(|a| Some(a) == id(&b.ack).ok()),
            "conflicting enrollment"
        );
        g.claim = b.claim.clone();
        g.previous_claims.clear();
        g.ack = Some(b.ack.clone());
        // A fresh installation may display authenticated historical inventory, but
        // only accept_inventory() can set the process-local freshness marker.
        if let Some(packet) = &b.inventory {
            let event: Event = serde_json::from_str(packet.event.get())?;
            ensure!(
                event.created_at.as_secs() <= now.saturating_add(30),
                "future inventory"
            );
            let msg = open(keys, &scope, packet, event.created_at.as_secs())?;
            let Body::Inventory {
                sequence,
                label,
                runtimes,
                capacity,
                catalog_revision,
                ..
            } = msg.body
            else {
                anyhow::bail!("wrong inventory record")
            };
            if sequence >= g.sequence {
                if sequence == g.sequence {
                    ensure!(
                        g.inventory.as_ref().map(id).transpose()?.as_ref() == Some(&msg.id),
                        "conflicting inventory sequence"
                    );
                }
                g.capacity = Some(capacity);
                g.catalog_revision = Some(catalog_revision);
                g.sequence = sequence;
                g.inventory = Some(packet.clone());
                g.label = Some(label);
                g.runtimes = runtimes;
            }
        }
        if !b.state.eq_ignore_ascii_case("active") {
            g.fenced = true;
        }
        if let Some(t) = &b.tombstone {
            Self::tombstone(&mut g, keys, t, now)?;
        }
        save(&tx, &g)?;
        tx.commit()?;
        Ok(scope)
    }

    fn tombstone(g: &mut Grant, keys: &Keys, p: &Packet, now: u64) -> Result<()> {
        let msg = open(keys, &g.scope, p, now)?;
        let ack = id(g.ack.as_ref().context("missing enrollment")?)?;
        ensure!(
            matches!(msg.body,Body::Unpaired{pair_ack_id,..} if pair_ack_id==ack),
            "invalid tombstone"
        );
        ensure!(
            g.tombstone
                .as_ref()
                .map(id)
                .transpose()?
                .is_none_or(|old| old == msg.id),
            "conflicting tombstones"
        );
        g.tombstone = Some(p.clone());
        Ok(())
    }

    /// Live query, single nonce, bounded by the process monotonic clock.
    pub fn inventory_query(&mut self, selected: &str, keys: &Keys, s: &Scope) -> Result<Packet> {
        session(s, selected, keys)?;
        let now = now()?;
        self.check_time(now)?;
        let g = grant(&self.db, s)?;
        ensure!(g.active(), "host inactive");
        let nonce = random()?;
        let p = seal(
            keys,
            s,
            Body::InventoryQuery {
                pair_ack_id: id(g.ack.as_ref().context("missing enrollment")?)?,
                nonce: nonce.clone(),
            },
            now,
            now + 60,
        )?;
        let coord = coordinate(s)?;
        self.fresh.remove(&coord);
        self.queries.insert(coord, (id(&p)?, nonce, Instant::now()));
        Ok(p)
    }

    /// Verify a fresh response before accepting the label/runtimes or enabling Create.
    pub fn accept_inventory(
        &mut self,
        selected: &str,
        keys: &Keys,
        s: &Scope,
        p: &Packet,
    ) -> Result<()> {
        session(s, selected, keys)?;
        let now = now()?;
        self.check_time(now)?;
        let Message {
            id,
            body,
            expires_at,
            ..
        } = open(keys, s, p, now)?;
        let Body::Inventory {
            query_id,
            nonce,
            sequence,
            label,
            runtimes,
            capacity,
            catalog_revision,
            ..
        } = body
        else {
            anyhow::bail!("wrong inventory response")
        };
        let coord = coordinate(s)?;
        let (q, n, t) = self
            .queries
            .get(&coord)
            .context("no live inventory query")?;
        ensure!(
            q == &query_id && n == &nonce && t.elapsed() < Duration::from_secs(60),
            "stale inventory"
        );
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut g = grant(&tx, s)?;
        ensure!(
            g.active()
                && (sequence > g.sequence
                    || (sequence == g.sequence
                        && g.inventory.as_ref().map(crate::id).transpose()?.as_ref() == Some(&id))),
            "stale inventory or inactive host"
        );
        g.sequence = sequence;
        g.capacity = Some(capacity);
        g.catalog_revision = Some(catalog_revision);
        g.inventory = Some(p.clone());
        g.label = Some(label);
        g.runtimes = runtimes;
        save(&tx, &g)?;
        tx.commit()?;
        self.fresh.insert(coord.clone(), (id, *t, expires_at));
        self.queries.remove(&coord);
        Ok(())
    }
    pub(crate) fn catalog(&self, s: &Scope, now: u64) -> Result<&str> {
        let (id, t, expires) = self
            .fresh
            .get(&coordinate(s)?)
            .context("refresh host inventory")?;
        ensure!(
            now < *expires && t.elapsed() < Duration::from_secs(60),
            "refresh host inventory"
        );
        Ok(id)
    }
    /// Refresh the installed native client's private identity inventory at each
    /// offer/commit boundary. The staging driver has only its isolated journal.
    pub fn with_local_identities(
        mut self,
        reader: impl Fn() -> Result<Vec<PublicKey>> + Send + 'static,
    ) -> Self {
        self.local_keys = Some(Box::new(reader));
        self
    }
    /// Record public-only local identity inventory. Previously used keys cannot become new agents.
    pub fn remember_identities(&mut self, keys: Vec<PublicKey>) -> Result<()> {
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        for key in keys {
            tx.execute(
                "INSERT OR IGNORE INTO known_identities VALUES(?)",
                [key.to_hex()],
            )?;
        }
        tx.commit()?;
        Ok(())
    }
    /// Take a fair bounded batch of pending local packets across all grants of this owner.
    /// Persist the last selected coordinate so reconnects and process restarts cannot starve later hosts.
    pub fn next_deliveries(
        &mut self,
        selected: &str,
        keys: &Keys,
    ) -> Result<Vec<(Scope, Packet, bool)>> {
        let owner = keys.public_key().to_hex();
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let last: String = tx
            .query_row(
                "SELECT last_scope FROM delivery_cursor WHERE community=? AND owner=?",
                params![selected, owner],
                |r| r.get(0),
            )
            .optional()?
            .unwrap_or_default();
        let mut batch = Vec::new();
        {
            let mut q=tx.prepare("SELECT data FROM grants
                WHERE json_extract(scope,'$.community')=? AND json_extract(scope,'$.owner')=?
                  AND ((json_extract(data,'$.ack') IS NULL AND json_extract(data,'$.claim.route.expires_at')>?)
                     OR (json_extract(data,'$.revocation') IS NOT NULL AND json_extract(data,'$.tombstone') IS NULL))
                ORDER BY scope<=?,scope LIMIT 8")?;
            let rows = q.query_map(params![selected, owner, now()?, last], |r| {
                r.get::<_, String>(0)
            })?;
            for row in rows {
                let grant: Grant = serde_json::from_str(&row?)?;
                if let Some(revocation) = grant.revocation {
                    batch.push((grant.scope, revocation, false));
                } else {
                    batch.push((grant.scope, grant.claim, true));
                }
            }
        }
        if let Some((scope, ..)) = batch.last() {
            tx.execute("INSERT INTO delivery_cursor VALUES(?,?,?) ON CONFLICT(community,owner) DO UPDATE SET last_scope=excluded.last_scope",params![selected,owner,coordinate(scope)?])?;
        }
        tx.commit()?;
        Ok(batch)
    }

    /// Restore local cards, including pending enrollment whose HTTP receipt was lost.
    pub fn hosts(&self, selected: &str, keys: &Keys) -> Result<Vec<HostView>> {
        let mut query=self.db.prepare("SELECT scope FROM grants WHERE json_extract(scope,'$.community')=? AND json_extract(scope,'$.owner')=? ORDER BY rowid")?;
        let rows = query.query_map(params![selected, keys.public_key().to_hex()], |row| {
            row.get::<_, String>(0)
        })?;
        rows.map(|row| {
            let mut view = self.view(selected, keys, &serde_json::from_str(&row?)?)?;
            view.requests = self.requests(selected, keys, &view.scope)?;
            Ok(view)
        })
        .collect()
    }
    /// Retrieve transport-ready enrollment after a lost claim response.
    pub fn pending_claim(&self, selected: &str, keys: &Keys, s: &Scope) -> Result<Option<Packet>> {
        session(s, selected, keys)?;
        let g = grant(&self.db, s)?;
        Ok((g.ack.is_none() && now()? < g.claim.route.expires_at).then_some(g.claim))
    }
    /// Return the local display state for a selected binding.
    pub fn view(&self, selected: &str, keys: &Keys, s: &Scope) -> Result<HostView> {
        session(s, selected, keys)?;
        let g = grant(&self.db, s)?;
        let state = if g.tombstone.is_some() {
            "unpaired"
        } else if g.revocation.is_some() || g.fenced {
            "unpair-pending"
        } else if g.ack.is_none() {
            if now()? >= g.claim.route.expires_at {
                "pair-expired"
            } else {
                "pair-pending"
            }
        } else {
            "paired"
        };
        let inventory_id = g.inventory.as_ref().map(id).transpose()?;
        let mut fresh_for_ms = 0;
        if g.active() {
            if let Some((_, since, expires)) = self
                .fresh
                .get(&coordinate(s)?)
                .filter(|(live, ..)| Some(live) == inventory_id.as_ref())
            {
                let wall = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?;
                let remaining = Duration::from_secs(60)
                    .saturating_sub(since.elapsed())
                    .min(Duration::from_secs(*expires).saturating_sub(wall));
                fresh_for_ms = remaining.as_millis().try_into()?;
            }
        }
        Ok(HostView {
            scope: s.clone(),
            fresh: fresh_for_ms > 0,
            inventory_id,
            fresh_for_ms,
            state: state.into(),
            requests: vec![],
            capacity: g.capacity,
            catalog_revision: g.catalog_revision,
            label: g.label,
            runtimes: g.runtimes,
        })
    }
    /// Retry only a previously requested local unpair, until host-signed completion.
    pub fn pending_unpair(&self, selected: &str, keys: &Keys, s: &Scope) -> Result<Option<Packet>> {
        session(s, selected, keys)?;
        let g = grant(&self.db, s)?;
        Ok(if g.tombstone.is_none() {
            g.revocation
        } else {
            None
        })
    }
    /// Fence locally before exposing the signed revocation for delivery.
    pub fn unpair(&mut self, selected: &str, keys: &Keys, s: &Scope) -> Result<Packet> {
        session(s, selected, keys)?;
        let now = now()?;
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut g = grant(&tx, s)?;
        if let Some(p) = g.revocation {
            return Ok(p);
        }
        let ack = id(g.ack.as_ref().context("missing enrollment")?)?;
        let p = seal(
            keys,
            s,
            Body::Unpair {
                pair_ack_id: ack,
                revocation_id: random()?,
            },
            now,
            now + 86400,
        )?;
        g.revocation = Some(p.clone());
        save(&tx, &g)?;
        tx.commit()?;
        Ok(p)
    }
}
