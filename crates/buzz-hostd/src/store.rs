use crate::clock::Clock;
use anyhow::{anyhow, bail, ensure, Result};
use buzz_core::{
    paired_host::{self, Offer},
    paired_message::{self, Body, Packet, Route, Scope},
    Keys, PublicKey,
};
use nostr::SecretKey;
use rand::{rngs::SysRng, TryRng};
use rusqlite::{params, Connection, Transaction, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
    time::Duration,
};
use zeroize::Zeroizing;

pub(crate) const SCHEMA: &str = "
CREATE TABLE identity(secret TEXT NOT NULL, pubkey TEXT NOT NULL, last_wall INTEGER NOT NULL);
CREATE TABLE grants(g TEXT PRIMARY KEY,c TEXT NOT NULL,expires INTEGER NOT NULL,boot TEXT NOT NULL,deadline INTEGER NOT NULL, digest BLOB,
 state TEXT NOT NULL,owner TEXT, claim TEXT,ack TEXT,ack_id TEXT,tomb TEXT,inventory_seq INTEGER NOT NULL DEFAULT 0,cursor TEXT);
CREATE UNIQUE INDEX one_grant ON grants((1)) WHERE state IN ('offered','active');
CREATE TABLE attempts(g TEXT NOT NULL,id TEXT NOT NULL,PRIMARY KEY(g,id));
CREATE TABLE sent(id TEXT PRIMARY KEY,g TEXT NOT NULL, parent TEXT NOT NULL, op TEXT NOT NULL, packet TEXT NOT NULL, pending INTEGER NOT NULL DEFAULT 1);
CREATE INDEX outbox ON sent(pending,g);
";

#[derive(Debug)]
pub struct InvalidPacket;
impl std::fmt::Display for InvalidPacket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid paired-host input")
    }
}
impl std::error::Error for InvalidPacket {}
pub struct Store(
    pub(crate) Connection,
    pub(crate) usize,
    #[cfg(test)] pub(crate) Option<std::path::PathBuf>,
);

pub use buzz_hostd::Catalog;

pub(crate) fn random() -> Result<String> {
    let mut bytes = [0; 32];
    SysRng
        .try_fill_bytes(&mut bytes)
        .map_err(|_| anyhow!("entropy unavailable"))?;
    Ok(hex::encode(bytes))
}

impl Store {
    // The service's dedicated uid and 0700 directory exclude other local principals.
    // Opening a missing store never silently creates a new signing identity.
    pub fn load(dir: &Path, init: bool) -> Result<Self> {
        let meta = fs::symlink_metadata(dir)?;
        ensure!(
            dir.is_absolute()
                && dir.canonicalize()? == dir
                && meta.is_dir()
                && meta.uid() == rustix::process::geteuid().as_raw()
                && meta.mode() & 0o077 == 0,
            "state directory must be canonical, owned and private"
        );
        let path = dir.join("host.sqlite");
        if init {
            fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .mode(0o600)
                .open(&path)?
                .sync_all()?;
        }
        let meta = fs::symlink_metadata(&path)?;
        ensure!(
            meta.is_file()
                && meta.nlink() == 1
                && meta.uid() == rustix::process::geteuid().as_raw()
                && meta.mode() & 0o077 == 0,
            "unsafe state file"
        );
        let db = Connection::open_with_flags(
            path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE | rusqlite::OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )?;
        db.busy_timeout(Duration::from_secs(5))?;
        db.execute_batch(
            "PRAGMA synchronous=FULL; PRAGMA secure_delete=ON; PRAGMA trusted_schema=OFF;",
        )?;
        let mut this = Self(
            db,
            0,
            #[cfg(test)]
            None,
        );
        if init {
            let tx = this.tx()?;
            tx.execute_batch(SCHEMA)?;
            tx.execute_batch("PRAGMA user_version=1")?;
            tx.execute_batch(crate::request::SCHEMA)?;
            tx.execute_batch(crate::onboard::SCHEMA)?;
            tx.execute_batch(
                "CREATE TABLE inventories(id TEXT PRIMARY KEY,revision TEXT NOT NULL)",
            )?;
            let keys = Keys::generate();
            let sec = Zeroizing::new(keys.secret_key().to_secret_hex());
            tx.execute(
                "INSERT INTO identity VALUES(?,?,0)",
                params![&*sec, keys.public_key().to_hex()],
            )?;
            tx.commit()?;
            fs::File::open(dir)?.sync_all()?;
        }
        this.keys()?;
        this.0.execute_batch("CREATE TABLE IF NOT EXISTS work_cursor(g TEXT PRIMARY KEY,r TEXT NOT NULL); CREATE TABLE IF NOT EXISTS resync(g TEXT PRIMARY KEY,state TEXT NOT NULL)")?;
        let tx = this.tx()?;
        let version: u32 = tx.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version == 0 {
            // Upgrade the enrollment-only journal in place, including already active grants.
            tx.execute_batch(crate::request::SCHEMA)?;
            tx.execute_batch(crate::onboard::SCHEMA)?;
            tx.execute_batch("CREATE TABLE inventories(id TEXT PRIMARY KEY,revision TEXT NOT NULL); PRAGMA user_version=1")?;
        } else {
            ensure!(version == 1, "unsupported host journal version");
        }
        tx.commit()?;
        Ok(this)
    }

    pub(crate) fn keys(&self) -> Result<Keys> {
        let (s, p): (String, String) =
            self.0
                .query_row("SELECT secret,pubkey FROM identity", [], |r| {
                    Ok((r.get(0)?, r.get(1)?))
                })?;
        let s = Zeroizing::new(s);
        let keys = Keys::new(SecretKey::from_hex(&s)?);
        ensure!(keys.public_key().to_hex() == p, "host identity mismatch");
        Ok(keys)
    }

    pub(crate) fn tx(&mut self) -> Result<Transaction<'_>> {
        Ok(self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)?)
    }

    pub fn offer(&mut self, c: String, clock: &Clock) -> Result<Zeroizing<String>> {
        let h = self.keys()?.public_key();
        let (offer, proof) = Offer::issue(c, h, clock.wall)?;
        let tx = self.tx()?;
        check_clock(&tx, clock)?;
        tx.execute(
            "UPDATE grants SET state='spent',digest=NULL WHERE state='offered'",
            [],
        )?;
        tx.execute("INSERT INTO grants(g,c,expires,boot,deadline,digest,state) VALUES(?,?,?,?,?,?,'offered')",
          params![offer.generation,offer.community,offer.expires_at,clock.boot,clock.millis+600000,proof.digest()])?;
        tx.commit()?;
        #[derive(Serialize)]
        struct Invitation<'a> {
            v: u8,
            community: &'a str,
            generation: &'a str,
            host: String,
            proof: &'a str,
            expires_at: u64,
        }
        Ok(Zeroizing::new(serde_json::to_string(&Invitation {
            v: 1,
            community: &offer.community,
            generation: &offer.generation,
            host: h.to_hex(),
            proof: &proof.expose_hex(),
            expires_at: offer.expires_at,
        })?))
    }

    pub fn claim(&mut self, g: &str, packet: &Packet, clock: &Clock) -> Result<()> {
        let keys = self.keys()?;
        let tx = self.tx()?;
        let (c, expires, boot, deadline, digest, state): (
            String,
            u64,
            String,
            u64,
            Option<Vec<u8>>,
            String,
        ) = tx.query_row(
            "SELECT c,expires,boot,deadline,digest,state FROM grants WHERE g=?",
            [g],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                ))
            },
        )?;
        ensure!(
            packet.route
                == (Route {
                    r: None,
                    prev: None,
                    expires_at: expires
                }),
            "incorrect claim sidecar"
        );
        if state == "active" {
            let saved: String =
                tx.query_row("SELECT claim FROM grants WHERE g=?", [g], |r| r.get(0))?;
            let old: Packet = serde_json::from_str(&saved)?;
            ensure!(
                crate::store::packet_id(&old)? == crate::store::packet_id(packet)?
                    && paired_host::transport::verify_outer(
                        packet.event.get().as_bytes(),
                        &c,
                        g,
                        PublicKey::from_hex(&tx.query_row::<String, _, _>(
                            "SELECT owner FROM grants WHERE g=?",
                            [g],
                            |r| r.get(0)
                        )?)?
                    )
                    .is_ok(),
                "host already bound"
            );
            tx.execute("UPDATE sent SET pending=1 WHERE g=? AND op='pair-ack'", [g])?;
            tx.commit()?;
            return Ok(());
        }
        ensure!(state == "offered", "spent offer");
        if check_clock(&tx, clock).is_err()
            || boot != clock.boot
            || deadline <= clock.millis
            || expires <= clock.wall
        {
            tx.execute("UPDATE grants SET state='spent',digest=NULL WHERE g=?", [g])?;
            tx.commit()?;
            bail!("local offer expired");
        }
        let offer = Offer {
            community: c,
            generation: g.to_owned(),
            host: keys.public_key(),
            expires_at: expires,
        };
        let digest: [u8; 32] = digest
            .ok_or_else(|| anyhow!("missing proof digest"))?
            .try_into()
            .map_err(|_| anyhow!("invalid proof digest"))?;
        match paired_host::verify_claim(
            packet.event.get().as_bytes(),
            &keys,
            &offer,
            None,
            &digest,
            clock.wall,
        ) {
            Ok(claim) => {
                let ack = paired_host::build_ack(&keys, &claim, clock.wall)?;
                let parent = claim.id().to_hex();
                let id = ack.id.to_hex();
                let ack = Packet {
                    event: serde_json::value::to_raw_value(&ack)?,
                    route: Route {
                        r: None,
                        prev: Some(parent.clone()),
                        expires_at: expires,
                    },
                };
                let encoded = save(&tx, g, "pair-ack", &parent, &id, &ack)?;
                tx.execute("UPDATE grants SET state='active',digest=NULL,owner=?,claim=?,ack=?,ack_id=? WHERE g=?",params![claim.owner().to_hex(),serde_json::to_string(packet)?,encoded,id,g])?;
            }
            Err(paired_host::Error::Proof(id)) => {
                tx.execute(
                    "INSERT OR IGNORE INTO attempts VALUES(?,?)",
                    params![g, id.to_hex()],
                )?;
                tx.execute("UPDATE grants SET state='spent',digest=NULL WHERE g=? AND (SELECT COUNT(*) FROM attempts WHERE g=?)>=5",params![g,g])?;
                tx.commit()?;
                bail!("claim rejected");
            }
            Err(_) => bail!("claim rejected"),
        }
        tx.commit()?;
        Ok(())
    }

    pub fn revoke(&mut self, g: &str, clock: &Clock) -> Result<()> {
        let keys = self.keys()?;
        let tx = self.tx()?;
        let (scope, ack, state) = binding(&tx, g, keys.public_key())?;
        revoke(&tx, &keys, &scope, &ack, &state, clock)?;
        tx.commit()?;
        Ok(())
    }

    pub fn receive(
        &mut self,
        g: &str,
        packet: &Packet,
        catalog: &Catalog,
        revision: &str,
        clock: &Clock,
    ) -> Result<()> {
        let keys = self.keys()?;
        let tx = self.tx()?;
        let (scope, ack, state) = binding(&tx, g, keys.public_key())?;
        let msg = match paired_message::open(&keys, &scope, packet, clock.wall) {
            Ok(m) => m,
            Err(_) => {
                check_clock(&tx, clock)?;
                let late = paired_message::expired_activation(&keys, &scope, packet, clock.wall)
                    .map_err(|_| InvalidPacket)?;
                crate::request::expire_delivery(&tx, &keys, &scope, late, clock)?;
                tx.commit()?;
                return Ok(());
            }
        };
        match msg.body {
            Body::Unpair { pair_ack_id, .. } if pair_ack_id == ack => {
                revoke(&tx, &keys, &scope, &ack, &state, clock)?
            }
            Body::InventoryQuery { pair_ack_id, nonce }
                if pair_ack_id == ack && state == "active" =>
            {
                check_clock(&tx, clock)?;
                if tx.execute(
                    "UPDATE sent SET pending=1 WHERE g=? AND parent=? AND op='inventory'",
                    params![g, msg.id],
                )? == 0
                {
                    let sequence:u64=tx.query_row("UPDATE grants SET inventory_seq=inventory_seq+1 WHERE g=? RETURNING inventory_seq",[g],|r|r.get(0))?;
                    let body = Body::Inventory {
                        query_id: msg.id,
                        nonce,
                        sequence,
                        label: catalog.label.clone(),
                        capacity: catalog.capacity,
                        runtimes: catalog.runtimes.keys().cloned().collect(),
                        catalog_revision: revision.into(),
                    };
                    let id = issue(&tx, &keys, &scope, body, clock.wall, msg.expires_at)?;
                    tx.execute("INSERT INTO inventories VALUES(?,?)", params![id, revision])?;
                }
            }
            _ => {
                check_clock(&tx, clock)?;
                crate::request::handle(
                    &tx,
                    &keys,
                    &scope,
                    state == "active",
                    &msg,
                    packet,
                    catalog,
                    revision,
                    clock,
                )?;
            }
        }
        tx.commit()?;
        Ok(())
    }
}

pub(crate) fn binding(
    tx: &Transaction<'_>,
    g: &str,
    host: PublicKey,
) -> Result<(Scope, String, String)> {
    let (community, owner, ack, state): (String, String, String, String) = tx.query_row(
        "SELECT c,owner,ack_id,state FROM grants WHERE g=? AND owner IS NOT NULL",
        [g],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    )?;
    Ok((
        Scope {
            community,
            generation: g.into(),
            owner: PublicKey::from_hex(&owner)?,
            host,
        },
        ack,
        state,
    ))
}

pub(crate) fn check_clock(tx: &Transaction<'_>, clock: &Clock) -> Result<()> {
    ensure!(
        tx.execute(
            "UPDATE identity SET last_wall=? WHERE last_wall<=?",
            params![clock.wall, clock.wall]
        )? == 1,
        "wall clock rollback"
    );
    Ok(())
}

pub(crate) fn issue(
    tx: &Transaction<'_>,
    keys: &Keys,
    scope: &Scope,
    body: Body,
    now: u64,
    expires: u64,
) -> Result<String> {
    let op = body.operation();
    let packet = paired_message::seal(keys, scope, body, now, expires)?;
    let id = packet_id(&packet)?;
    save(
        tx,
        &scope.generation,
        op,
        packet.route.prev.as_deref().unwrap_or(""),
        &id,
        &packet,
    )?;
    Ok(id)
}

pub(crate) fn packet_id(p: &Packet) -> Result<String> {
    #[derive(Deserialize)]
    struct Header {
        id: String,
    }
    Ok(serde_json::from_str::<Header>(p.event.get())?.id)
}

pub(crate) fn save(
    tx: &Transaction<'_>,
    g: &str,
    op: &str,
    parent: &str,
    id: &str,
    p: &Packet,
) -> Result<String> {
    let encoded = serde_json::to_string(p)?;
    tx.execute(
        "INSERT INTO sent(id,g,parent,op,packet) VALUES(?,?,?,?,?)",
        params![id, g, parent, op, encoded],
    )?;
    Ok(encoded)
}

fn revoke(
    tx: &Transaction<'_>,
    keys: &Keys,
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
    } else {
        ensure!(state == "active", "no active binding");
        let revoking = {
            let mut stmt=tx.prepare("SELECT record FROM requests WHERE g=? AND json_extract(record,'$.state')='pending'")?;
            let rows = stmt.query_map([&scope.generation], |r| r.get::<_, String>(0))?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
        };
        for row in revoking {
            let mut q: crate::request::Request = serde_json::from_str(&row)?;
            q.finish(tx, keys, scope, "revoked-before-commit", "revoked", clock)?;
        }
        let id = issue(
            tx,
            keys,
            scope,
            Body::Unpaired {
                pair_ack_id: ack.into(),
                tombstone_nonce: random()?,
            },
            clock.wall,
            clock.wall + 86400,
        )?;
        tx.execute(
            "UPDATE grants SET state='dead',tomb=? WHERE g=?",
            params![id, scope.generation],
        )?;
        tx.execute(
            "UPDATE sent SET pending=0 WHERE g=? AND op='inventory'",
            [&scope.generation],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
