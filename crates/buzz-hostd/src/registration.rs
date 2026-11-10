//! Registration-only facade over the daemon's existing SQLite identity,
//! invitation, attempts and exact signed outbox. No second journal or keys.
use crate::{
    clock::Clock,
    store::{self, Store},
};
use anyhow::{ensure, Result};
use buzz_core::{
    host_registry::{self as wire, Body},
    paired_message::{Packet, Route, Scope},
};
use nostr::{Event, PublicKey};
use rusqlite::params;
use std::path::Path;
use zeroize::Zeroizing;

/// The daemon clock, including boot identity and monotonic invitation deadline.
pub use crate::clock::Clock as RegistrationClock;
/// Host registration state backed by the existing host.sqlite store.
pub struct Host {
    pub(crate) store: Store,
    live: std::cell::RefCell<Option<live::Live>>,
}
mod live;
impl Host {
    /// Initialize or reopen a private canonical state directory.
    pub fn at(dir: &Path, init: bool) -> Result<Self> {
        Ok(Self {
            store: Store::load(dir, init)?,
            live: std::cell::RefCell::new(None),
        })
    }
    /// Persistent public control identity. The private key never leaves hostd.
    pub fn public_key(&self) -> Result<PublicKey> {
        Ok(self.store.keys()?.public_key())
    }
    /// Public local registration states. This never returns control-key material.
    pub fn registrations(&self) -> Result<Vec<(String, String, String)>> {
        let mut q = self
            .store
            .0
            .prepare("SELECT c,g,state FROM grants ORDER BY rowid")?;
        let rows = q.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }
    /// Bounded work for one operator-selected relay. Expired offers are spent
    /// locally; dead generations stay eligible only while their outbox is pending.
    pub fn work(&mut self, community: &str, clock: &Clock) -> Result<Vec<String>> {
        self.store.0.execute("UPDATE grants SET state='spent',digest=NULL WHERE state='offered' AND (expires<=? OR boot<>? OR deadline<=?)", params![clock.wall, clock.boot, clock.millis])?;
        let mut q = self.store.0.prepare("SELECT g FROM grants WHERE c=? AND (state IN ('offered','active') OR (state='dead' AND EXISTS(SELECT 1 FROM sent WHERE sent.g=grants.g AND pending=1 AND json_extract(packet,'$.event.kind')=43103))) ORDER BY rowid LIMIT 33")?;
        let rows = q.query_map([community], |r| r.get::<_, String>(0))?;
        let work = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        ensure!(work.len() <= 32, "registration work bound exceeded");
        Ok(work)
    }
    /// Issue the existing ten-minute single-use invitation, explicitly version2.
    pub fn offer(&mut self, community: String, clock: &Clock) -> Result<Zeroizing<String>> {
        let old = self.store.offer(community, clock)?;
        let mut inv: serde_json::Value = serde_json::from_str(&old)?;
        inv["v"] = 2.into();
        Ok(Zeroizing::new(serde_json::to_string(&inv)?))
    }
    /// Authenticate a v2-only connection with the host key kept in this store.
    pub async fn authenticate(
        &self,
        ws: &mut buzz_ws_client::NostrWsConnection,
        g: &str,
    ) -> Result<()> {
        ensure!(wire::identifier(g), "invalid generation");
        let tag = nostr::Tag::parse(["host-registry", "2", g])?;
        ws.authenticate(&self.store.keys()?, Some(&tag)).await?;
        Ok(())
    }
    /// Verify proof and atomically consume the invitation with exact ack outbox.
    pub fn claim(&mut self, g: &str, e: &Event, clock: &Clock) -> Result<()> {
        let keys = self.store.keys()?;
        let tx = self.store.tx()?;
        let (c, expires, boot, deadline, digest, state, saved): (
            String,
            u64,
            String,
            u64,
            Option<Vec<u8>>,
            String,
            Option<String>,
        ) = tx.query_row(
            "SELECT c,expires,boot,deadline,digest,state,claim FROM grants WHERE g=?",
            [g],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                ))
            },
        )?;
        let (_, h) = wire::verify(&serde_json::to_vec(e)?, &c, e.pubkey)
            .map_err(|_| store::InvalidPacket)?;
        ensure!(
            h.generation == g
                && h.host == keys.public_key().to_hex()
                && h.op == "pair-claim"
                && h.expires_at == expires,
            store::InvalidPacket
        );
        if state == "active" {
            let old: Packet = serde_json::from_str(
                &saved.ok_or_else(|| anyhow::anyhow!("missing selected claim"))?,
            )?;
            ensure!(
                store::packet_id(&old)? == e.id.to_hex(),
                store::InvalidPacket
            );
            tx.execute("UPDATE sent SET pending=1 WHERE g=? AND op='pair-ack'", [g])?;
            tx.commit()?;
            return Ok(());
        }
        ensure!(state == "offered", "spent offer");
        if store::check_clock(&tx, clock).is_err()
            || boot != clock.boot
            || deadline <= clock.millis
            || expires <= clock.wall
        {
            tx.execute("UPDATE grants SET state='spent',digest=NULL WHERE g=?", [g])?;
            tx.commit()?;
            anyhow::bail!("local offer expired");
        }
        ensure!(
            expires > e.created_at.as_secs() && expires - e.created_at.as_secs() <= 630,
            store::InvalidPacket
        );
        let (h, body) =
            wire::open(&keys, &h.scope()?, e, clock.wall).map_err(|_| store::InvalidPacket)?;
        let Body::PairClaim { proof } = body else {
            return Err(store::InvalidPacket.into());
        };
        let digest: [u8; 32] = digest
            .ok_or_else(|| anyhow::anyhow!("missing proof"))?
            .try_into()
            .map_err(|_| anyhow::anyhow!("invalid digest"))?;
        if !wire::proof_matches(&proof, &digest)? {
            tx.execute(
                "INSERT OR IGNORE INTO attempts VALUES(?,?)",
                params![g, e.id.to_hex()],
            )?;
            tx.execute("UPDATE grants SET state='spent',digest=NULL WHERE g=? AND (SELECT count(*) FROM attempts WHERE g=?)>=5",params![g,g])?;
            tx.commit()?;
            return Err(store::InvalidPacket.into());
        }
        let scope = h.scope()?;
        let ack = wire::seal(
            &keys,
            &scope,
            Body::PairAck {
                claim_id: e.id.to_hex(),
            },
            store::random()?,
            Some(e.id.to_hex()),
            clock.wall,
            expires,
        )?;
        let packet = packet(&ack)?;
        let encoded = store::save(
            &tx,
            g,
            "pair-ack",
            &e.id.to_hex(),
            &ack.id.to_hex(),
            &packet,
        )?;
        tx.execute(
            "UPDATE grants SET state='active',digest=NULL,owner=?,claim=?,ack=?,ack_id=? WHERE g=?",
            params![
                scope.owner.to_hex(),
                serde_json::to_string(&packet_from(e, &h)?)?,
                encoded,
                ack.id.to_hex(),
                g
            ],
        )?;
        tx.commit()?;
        Ok(())
    }
    /// Apply a nonce query or restrictive unpair, saving the exact response first.
    pub fn receive(&mut self, g: &str, e: &Event, label: &str, clock: &Clock) -> Result<()> {
        let keys = self.store.keys()?;
        let tx = self.store.tx()?;
        let (scope, ack, state) = store::binding(&tx, g, keys.public_key())?;
        let (h, body) = wire::open(&keys, &scope, e, clock.wall)?;
        ensure!(e.pubkey == scope.owner, "invalid author");
        store::check_clock(&tx, clock)?;
        match body {
            Body::Unpair { ack_id } if ack_id == ack => {
                revoke(&tx, &keys, &scope, &ack, &state, clock)?
            }
            Body::StatusQuery { ack_id, nonce } if ack_id == ack && state == "active" => {
                if tx.execute(
                    "UPDATE sent SET pending=1 WHERE g=? AND parent=? AND op='status'",
                    params![g, e.id.to_hex()],
                )? == 0
                {
                    let sequence:u64=tx.query_row("UPDATE grants SET inventory_seq=inventory_seq+1 WHERE g=? RETURNING inventory_seq",[g],|r|r.get(0))?;
                    let response = wire::seal(
                        &keys,
                        &scope,
                        Body::Status {
                            ack_id: ack,
                            nonce,
                            sequence,
                            state: "active".into(),
                            label: label.into(),
                        },
                        store::random()?,
                        Some(e.id.to_hex()),
                        clock.wall,
                        h.expires_at,
                    )?;
                    store::save(
                        &tx,
                        g,
                        "status",
                        &e.id.to_hex(),
                        &response.id.to_hex(),
                        &packet(&response)?,
                    )?;
                }
            }
            _ => anyhow::bail!("inactive or invalid operation"),
        }
        tx.commit()?;
        Ok(())
    }
    /// Persist local revocation before any network delivery; no service changes.
    pub fn revoke(&mut self, g: &str, clock: &Clock) -> Result<()> {
        let keys = self.store.keys()?;
        let tx = self.store.tx()?;
        let (scope, ack, state) = store::binding(&tx, g, keys.public_key())?;
        revoke(&tx, &keys, &scope, &ack, &state, clock)?;
        tx.commit()?;
        Ok(())
    }
    /// Exact pending signed events, bounded to eight for fair reconciliation.
    pub fn pending(&self) -> Result<Vec<Event>> {
        self.pending_generation(None)
    }
    fn pending_generation(&self, generation: Option<&str>) -> Result<Vec<Event>> {
        // Readiness replies confer no authority after expiry. Expire their
        // retry obligation explicitly; immutable ack/tombstone evidence stays.
        self.store.0.execute("UPDATE sent SET pending=0 WHERE op='status' AND pending=1 AND json_extract(packet,'$.event.kind')=43103 AND json_extract(packet,'$.route.expires_at')<=unixepoch()",[])?;
        self.store.0.execute("DELETE FROM sent WHERE op='status' AND pending=0 AND json_extract(packet,'$.event.kind')=43103 AND json_extract(packet,'$.route.expires_at')<unixepoch()-300",[])?;

        let mut q=self.store.0.prepare("SELECT packet FROM sent WHERE pending=1 AND json_extract(packet,'$.event.kind')=43103 AND (?1 IS NULL OR g=?1) ORDER BY CASE op WHEN 'unpaired' THEN 0 ELSE 1 END,rowid LIMIT 8")?;
        let rows = q.query_map([generation], |r| r.get::<_, String>(0))?;
        rows.map(|r| {
            let p: Packet = serde_json::from_str(&r?)?;
            Ok(serde_json::from_str(p.event.get())?)
        })
        .collect()
    }
    /// Retire an outbox entry only after a positive relay OK for that exact ID.
    pub fn delivered(&self, id: &str) -> Result<()> {
        self.store
            .0
            .execute("UPDATE sent SET pending=0 WHERE id=?", [id])?;
        Ok(())
    }
    /// Read one protected page using this host's scoped key and exact generation.
    pub async fn page(
        &self,
        ws: &mut buzz_ws_client::NostrWsConnection,
        relay: PublicKey,
        q: wire::Query,
    ) -> Result<wire::Reply> {
        use buzz_ws_client::RelayMessage;
        use std::time::{Duration, Instant};
        q.validate()?;
        let keys = self.store.keys()?;
        let community: String = self.store.0.query_row(
            "SELECT c FROM grants WHERE g=?",
            [q.generation
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("missing generation"))?],
            |r| r.get(0),
        )?;
        let sub = format!("host-{}", store::random()?);
        ws.send_raw(&serde_json::json!(["REQ",sub,{"kinds":[43103],"#p":[keys.public_key().to_hex()],"host_registry":q}])).await?;
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut page = None;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            ensure!(!remaining.is_zero(), "registry unavailable");
            match ws.next_event(remaining).await? {
                RelayMessage::Event {
                    subscription_id,
                    event,
                } if subscription_id == sub => {
                    let r = wire::open_reply(&keys, relay, &community, &q, &event)?;
                    ensure!(
                        r.op == "registry-page" && page.is_none(),
                        "invalid registry page"
                    );
                    page = Some(r);
                }
                RelayMessage::Eose { subscription_id } if subscription_id == sub => {
                    return page.ok_or_else(|| anyhow::anyhow!("missing registry page"))
                }
                RelayMessage::Closed {
                    subscription_id, ..
                } if subscription_id == sub => anyhow::bail!("registry unavailable"),
                other => {
                    self.observe_live(&other)?;
                }
            }
        }
    }
    /// One bounded registration reconciliation. A daemon schedules this on
    /// verified change/reconnect, with a bounded periodic backstop.
    pub async fn reconcile(
        &mut self,
        ws: &mut buzz_ws_client::NostrWsConnection,
        relay: PublicKey,
        g: &str,
        label: &str,
        clock: &Clock,
    ) -> Result<()> {
        let query = |view: &str, event_id: Option<String>| -> Result<wire::Query> {
            Ok(wire::Query {
                v: 2,
                view: view.into(),
                nonce: store::random()?,
                cursor: None,
                generation: Some(g.into()),
                event_id,
            })
        };
        if self.state(g)? == "offered" {
            let offers = self.page(ws, relay, query("offers", None)?).await?;
            for id in offers.data["events"]
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("missing offers"))?
                .iter()
                .take(100)
            {
                let evidence = self
                    .page(
                        ws,
                        relay,
                        query(
                            "evidence",
                            Some(
                                id.as_str()
                                    .ok_or_else(|| anyhow::anyhow!("invalid reference"))?
                                    .into(),
                            ),
                        )?,
                    )
                    .await?;
                let e = wire::parse_event(
                    &serde_json::to_vec(&evidence.data["event"])?,
                    wire::MAX_EVENT,
                )?;
                match self.claim(g, &e, &clock.refresh()?) {
                    Ok(()) => break,
                    Err(error) if error.is::<store::InvalidPacket>() => {}
                    Err(error) => return Err(error),
                }
            }
        }
        self.flush_pending(ws, g).await?;
        if matches!(self.state(g)?.as_str(), "offered" | "dead") {
            return Ok(());
        }
        let binding = self.page(ws, relay, query("binding", None)?).await?;
        if binding.data["state"] != "active" || binding.data["recovery_only"] == true {
            self.revoke(g, &clock.refresh()?)?;
        } else {
            let mut mail = query("messages", None)?;
            let mut complete = false;
            // The shared mailbox quota is2048 rows: at most21 pages. One
            // additional iteration detects any violated server bound explicitly.
            for _ in 0..22 {
                let messages = self.page(ws, relay, mail.clone()).await?;
                let current = clock.refresh()?;
                for reference in messages.data["events"]
                    .as_array()
                    .ok_or_else(|| anyhow::anyhow!("missing messages"))?
                    .iter()
                    .take(100)
                {
                    if !matches!(reference["op"].as_str(), Some("status-query" | "unpair"))
                        || reference["expires_at"]
                            .as_u64()
                            .is_none_or(|expiry| expiry <= current.wall)
                    {
                        continue;
                    }
                    let id = reference["id"]
                        .as_str()
                        .ok_or_else(|| anyhow::anyhow!("invalid reference"))?;
                    let evidence = self
                        .page(ws, relay, query("evidence", Some(id.into()))?)
                        .await?;
                    let e = wire::parse_event(
                        &serde_json::to_vec(&evidence.data["event"])?,
                        wire::MAX_EVENT,
                    )?;
                    // References only reduce needless crypto; exact evidence is
                    // independently verified before any durable local effect.
                    self.receive(g, &e, label, &clock.refresh()?)?;
                }
                if messages.complete {
                    complete = true;
                    break;
                }
                mail.cursor = messages.next_cursor;
            }
            ensure!(complete, "registration mailbox page bound exceeded");
        }
        self.flush_pending(ws, g).await?;
        Ok(())
    }
    /// Deliver only this generation's exact journal entries before protected
    /// reads, including expired timely acknowledgements and restrictive writes.
    pub async fn flush_pending(
        &self,
        ws: &mut buzz_ws_client::NostrWsConnection,
        g: &str,
    ) -> Result<()> {
        for e in self.pending_generation(Some(g))? {
            // AUTH is generation-scoped; another generation keeps its own retry.
            let (_, header) = wire::verify(
                &serde_json::to_vec(&e)?,
                e.tags
                    .iter()
                    .find(|t| t.as_slice()[0] == "community")
                    .and_then(|t| t.content())
                    .ok_or_else(|| anyhow::anyhow!("missing community"))?,
                e.pubkey,
            )?;
            if header.generation != g {
                continue;
            }
            let id = e.id.to_hex();
            let ok = ws.send_event(e).await?;
            ensure!(ok.accepted, "registry denied");
            self.delivered(&id)?;
        }
        Ok(())
    }
    /// Current local grant state; an absent generation is an error.
    pub fn state(&self, g: &str) -> Result<String> {
        Ok(self
            .store
            .0
            .query_row("SELECT state FROM grants WHERE g=?", [g], |r| r.get(0))?)
    }
}
fn packet(e: &Event) -> Result<Packet> {
    let (_, h) = wire::verify(
        &serde_json::to_vec(e)?,
        e.tags
            .iter()
            .find(|t| t.as_slice()[0] == "community")
            .and_then(|t| t.content())
            .ok_or_else(|| anyhow::anyhow!("missing community"))?,
        e.pubkey,
    )?;
    packet_from(e, &h)
}
fn packet_from(e: &Event, h: &wire::Header) -> Result<Packet> {
    Ok(Packet {
        event: serde_json::value::to_raw_value(e)?,
        route: Route {
            r: Some(h.operation_id.clone()),
            prev: h.parent.clone(),
            expires_at: h.expires_at,
        },
    })
}
fn revoke(
    tx: &rusqlite::Transaction<'_>,
    keys: &nostr::Keys,
    scope: &Scope,
    ack: &str,
    state: &str,
    clock: &Clock,
) -> Result<()> {
    if state == "dead" {
        tx.execute(
            "UPDATE sent SET pending=1 WHERE g=? AND op='unpaired'",
            [&scope.generation],
        )?;
        return Ok(());
    }
    ensure!(state == "active", "no active binding");
    // Restrictive completion remains possible with a bad local wall clock.
    let last: u64 = tx.query_row("SELECT last_wall FROM identity", [], |r| r.get(0))?;
    let now = clock.wall.max(last);
    let e = wire::seal(
        keys,
        scope,
        Body::Unpaired { ack_id: ack.into() },
        store::random()?,
        Some(ack.into()),
        now,
        now + 86400,
    )?;
    store::save(
        tx,
        &scope.generation,
        "unpaired",
        ack,
        &e.id.to_hex(),
        &packet(&e)?,
    )?;
    tx.execute(
        "UPDATE grants SET state='dead',tomb=? WHERE g=?",
        params![e.id.to_hex(), scope.generation],
    )?;
    tx.execute(
        "UPDATE sent SET pending=0 WHERE g=? AND op IN ('status','pair-ack')",
        [&scope.generation],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nostr::Keys;
    use std::os::unix::fs::PermissionsExt;
    fn fixture() -> (tempfile::TempDir, Host, Clock) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let h = Host::at(dir.path(), true).unwrap();
        let c = Clock {
            source: None,
            wall: 1000,
            boot: "synthetic-boot".into(),
            millis: 100,
        };
        (dir, h, c)
    }
    fn claim(h: &mut Host, c: &Clock, o: &Keys, proof: Option<&str>) -> (String, Event) {
        let invitation = h.offer("https://registry.test".into(), c).unwrap();
        let inv: serde_json::Value = serde_json::from_str(&invitation).unwrap();
        let s = Scope {
            community: "https://registry.test".into(),
            generation: inv["generation"].as_str().unwrap().into(),
            owner: o.public_key(),
            host: h.public_key().unwrap(),
        };
        let e = wire::seal(
            o,
            &s,
            Body::PairClaim {
                proof: proof.unwrap_or(inv["proof"].as_str().unwrap()).into(),
            },
            "11".repeat(32),
            None,
            c.wall,
            inv["expires_at"].as_u64().unwrap(),
        )
        .unwrap();
        (s.generation, e)
    }
    #[test]
    fn generation_outbox_is_not_starved_by_older_restrictive_work() {
        let (_dir, mut h, c) = fixture();
        let owner = Keys::generate();
        let mut generations = Vec::new();
        for _ in 0..10 {
            let (g, e) = claim(&mut h, &c, &owner, None);
            h.claim(&g, &e, &c).unwrap();
            h.revoke(&g, &c).unwrap();
            generations.push(g);
        }
        assert_eq!(h.pending().unwrap().len(), 8);
        let newest = generations.last().unwrap();
        let selected = h.pending_generation(Some(newest)).unwrap();
        assert_eq!(selected.len(), 1);
        let (_, header) = wire::verify(
            &serde_json::to_vec(&selected[0]).unwrap(),
            "https://registry.test",
            h.public_key().unwrap(),
        )
        .unwrap();
        assert_eq!(&header.generation, newest);
        assert_eq!(header.op, "unpaired");
        assert_eq!(h.work("https://registry.test", &c).unwrap().len(), 10);
    }
    #[test]
    fn exact_retry_and_revocation_outbox_survive_restart() {
        let (dir, mut h, c) = fixture();
        let o = Keys::generate();
        let (g, e) = claim(&mut h, &c, &o, None);
        h.claim(&g, &e, &c).unwrap();
        let ack = h.pending().unwrap().remove(0);
        assert_eq!(h.state(&g).unwrap(), "active");
        h.delivered(&ack.id.to_hex()).unwrap();
        drop(h);
        let mut h = Host::at(dir.path(), false).unwrap();
        h.claim(&g, &e, &c).unwrap();
        assert_eq!(h.pending().unwrap()[0].id, ack.id);
        assert!(h.offer("https://registry.test".into(), &c).is_err());
        h.revoke(&g, &c).unwrap();
        let tomb = h.pending().unwrap().remove(0);
        drop(h);
        let mut h = Host::at(dir.path(), false).unwrap();
        assert_eq!(h.state(&g).unwrap(), "dead");
        assert_eq!(h.pending().unwrap()[0].id, tomb.id);
        h.revoke(&g, &c).unwrap();
        assert_eq!(h.pending().unwrap()[0].id, tomb.id);
        let fresh = h.offer("https://registry.test".into(), &c).unwrap();
        let inv: serde_json::Value = serde_json::from_str(&fresh).unwrap();
        assert_ne!(inv["generation"], g);
    }
    #[test]
    fn wrong_proof_attempts_are_distinct_and_bounded() {
        let (_dir, mut h, c) = fixture();
        let o = Keys::generate();
        let (g, e) = claim(&mut h, &c, &o, Some(&"ff".repeat(32)));
        for _ in 0..5 {
            assert!(h.claim(&g, &e, &c).is_err());
        }
        assert_eq!(h.state(&g).unwrap(), "offered");
        let (_, header) = wire::verify(
            &serde_json::to_vec(&e).unwrap(),
            "https://registry.test",
            o.public_key(),
        )
        .unwrap();
        for n in 1..5 {
            let distinct = wire::seal(
                &o,
                &header.scope().unwrap(),
                Body::PairClaim {
                    proof: "ff".repeat(32),
                },
                format!("{n:064x}"),
                None,
                c.wall,
                header.expires_at,
            )
            .unwrap();
            assert!(h.claim(&g, &distinct, &c).is_err());
        }
        assert_eq!(h.state(&g).unwrap(), "spent");
        assert!(h.pending().unwrap().is_empty());
    }
    #[test]
    fn expired_rebooted_or_rollback_offer_never_first_consumes() {
        for failure in ["expiry", "boot", "rollback", "monotonic"] {
            let (_dir, mut h, c) = fixture();
            let o = Keys::generate();
            let (g, e) = claim(&mut h, &c, &o, None);
            let mut changed = c.clone();
            match failure {
                "expiry" => changed.wall = 1600,
                "boot" => changed.boot = "different".into(),
                "rollback" => changed.wall = 999,
                _ => changed.millis = 600100,
            };
            assert!(h.claim(&g, &e, &changed).is_err());
            assert_eq!(h.state(&g).unwrap(), "spent");
            assert!(h.pending().unwrap().is_empty());
        }
    }
}
