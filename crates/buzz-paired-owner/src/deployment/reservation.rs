//! Durable pre-attestation reservation, custody, desired profile, and tombstone state.
use super::*;
use buzz_core::host_deployment::validate_scope;
use buzz_core::host_registry::{identifier as valid_id, object as decode_obj};
use buzz_core::paired_message::{display_name, runtime_id, Policy, Scope};
use buzz_core::private_managed_agent::validate_auth_tag;
use nostr::{
    nips::nip44::{self, Version},
    PublicKey, ToBech32,
};
use rusqlite::{params, OptionalExtension, Params, Row, Transaction, TransactionBehavior};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, Zeroizing};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReservationState {
    Reserved,
    Prepared,
    Active,
    Retiring,
    Retired,
}
use ReservationState::*;

impl ReservationState {
    pub fn as_str(self) -> &'static str {
        match self {
            Reserved => "reserved",
            Prepared => "prepared",
            Active => "active",
            Retiring => "retiring",
            Retired => "retired",
        }
    }
    fn retired(self) -> bool {
        matches!(self, Retiring | Retired)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReservationConfig {
    pub instructions: Option<String>,
    pub effort: Option<String>,
    pub parallelism: u32,
    pub respond_to: String,
    pub respond_to_allowlist: Vec<String>,
    pub channel_add_policy: String,
}

impl ReservationConfig {
    pub fn validate(&self) -> Result<()> {
        let p = Policy {
            respond_to: self.respond_to.clone(),
            respond_to_allowlist: self.respond_to_allowlist.clone(),
            channel_add_policy: self.channel_add_policy.clone(),
        };
        ensure!(p.valid(), "invalid agent policy");
        ensure!((1..=32).contains(&self.parallelism), "invalid parallelism");
        ensure!(
            self.instructions
                .as_deref()
                .is_none_or(|s| s.len() <= 16 * 1024),
            "instructions too large"
        );
        let eff_ok = self
            .effort
            .as_deref()
            .is_none_or(|s| !s.is_empty() && s.len() <= 256 && !s.chars().any(char::is_control));
        ensure!(eff_ok, "invalid thinking effort");
        Ok(())
    }
    fn frozen_matches(&self, o: &Self) -> bool {
        self.instructions == o.instructions
            && self.parallelism == o.parallelism
            && self.respond_to == o.respond_to
            && self.respond_to_allowlist == o.respond_to_allowlist
            && self.channel_add_policy == o.channel_add_policy
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesiredProfile {
    pub name: String,
    pub picture: Option<String>,
    pub updated_at: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReservationSecret {
    v: u8,
    row: [String; 7],
    state: ReservationState,
    operation_id: Option<String>,
    agent_nsec: Option<String>,
    nip_oa: Option<Vec<String>>,
    picture: Option<String>,
    config: ReservationConfig,
    desired_profile: Option<DesiredProfile>,
    last_profile_created_at: Option<u64>,
    profile_synced: bool,
    retired_at: Option<u64>,
    retired_via: Option<String>,
}
impl Drop for ReservationSecret {
    fn drop(&mut self) {
        self.agent_nsec.zeroize();
    }
}

pub struct Reservation {
    pub scope: Scope,
    pub runtime_id: String,
    pub name_scope: String,
    pub pending_name: Option<String>,
    pub agent: PublicKey,
    pub state: ReservationState,
    pub operation_id: Option<String>,
    pub agent_nsec: Option<Zeroizing<String>>,
    pub nip_oa: Option<Vec<String>>,
    pub picture: Option<String>,
    pub config: ReservationConfig,
    pub desired_profile: Option<DesiredProfile>,
    pub last_profile_created_at: Option<u64>,
    pub profile_synced: bool,
    pub retired_at: Option<u64>,
    pub retired_via: Option<String>,
}

pub struct ReserveAgent<'a> {
    pub scope: &'a Scope,
    pub runtime_id: &'a str,
    pub name: &'a str,
    pub picture: Option<String>,
    pub config: ReservationConfig,
}

const COLS: &str = "community,owner,agent,host,generation,runtime_id,name_scope,pending_name,state,operation_id,desired_profile_ts,ciphertext";

fn check_pic(pic: Option<&str>) -> Result<()> {
    let ok = pic.is_none_or(|u| {
        u.starts_with("https://") && u.len() <= 2048 && !u.chars().any(char::is_control)
    });
    ensure!(ok, "invalid picture URL");
    Ok(())
}

fn seal(k: &Keys, r: &Reservation) -> Result<String> {
    let row = [
        r.scope.community.clone(),
        r.scope.owner.to_hex(),
        r.agent.to_hex(),
        r.scope.host.to_hex(),
        r.scope.generation.clone(),
        r.runtime_id.clone(),
        r.name_scope.clone(),
    ];
    let sec = ReservationSecret {
        v: 1,
        row,
        state: r.state,
        operation_id: r.operation_id.clone(),
        agent_nsec: r.agent_nsec.as_ref().map(|s| s.as_str().to_owned()),
        nip_oa: r.nip_oa.clone(),
        picture: r.picture.clone(),
        config: r.config.clone(),
        desired_profile: r.desired_profile.clone(),
        last_profile_created_at: r.last_profile_created_at,
        profile_synced: r.profile_synced,
        retired_at: r.retired_at,
        retired_via: r.retired_via.clone(),
    };
    let plain = Zeroizing::new(serde_json::to_vec(&sec)?);
    ensure!(plain.len() <= 48 * 1024, "reservation payload too large");
    Ok(nip44::encrypt(
        k.secret_key(),
        &k.public_key(),
        &*plain,
        Version::V2,
    )?)
}

fn open(k: &Keys, r: &Row<'_>) -> Result<(Reservation, u64)> {
    let plain = Zeroizing::new(nip44::decrypt(
        k.secret_key(),
        &k.public_key(),
        &r.get::<_, String>(11)?,
    )?);
    let mut s: ReservationSecret = decode_obj(plain.as_bytes(), 48 * 1024)?;
    let g = |i| r.get::<_, String>(i);
    let ok = s.v == 1
        && s.row == [g(0)?, g(1)?, g(2)?, g(3)?, g(4)?, g(5)?, g(6)?]
        && s.state.as_str() == g(8)?
        && s.operation_id == r.get::<_, Option<String>>(9)?;
    ensure!(ok, "reservation metadata mismatch");
    let [comm, oh, ah, hh, gen, rt, ns] = std::mem::take(&mut s.row);
    let scope = Scope {
        community: comm,
        generation: gen,
        owner: PublicKey::from_hex(&oh)?,
        host: PublicKey::from_hex(&hh)?,
    };
    session(&scope, &scope.community, k)?;
    let res = Reservation {
        scope,
        runtime_id: rt,
        name_scope: ns,
        pending_name: r.get(7)?,
        agent: PublicKey::from_hex(&ah)?,
        state: s.state,
        operation_id: s.operation_id.take(),
        agent_nsec: s.agent_nsec.take().map(Zeroizing::new),
        nip_oa: s.nip_oa.take(),
        picture: s.picture.take(),
        config: s.config.clone(),
        desired_profile: s.desired_profile.take(),
        last_profile_created_at: s.last_profile_created_at,
        profile_synced: s.profile_synced,
        retired_at: s.retired_at,
        retired_via: s.retired_via.take(),
    };
    Ok((res, r.get(10)?))
}

fn q_one(
    db: &rusqlite::Connection,
    k: &Keys,
    col: &str,
    p: impl Params,
) -> Result<Option<(Reservation, u64)>> {
    let sql =
        format!("SELECT {COLS} FROM agent_reservations WHERE community=? AND owner=? AND {col}=?");
    db.prepare(&sql)?
        .query(p)?
        .next()?
        .map(|r| open(k, r))
        .transpose()
}

fn save_row(
    tx: &rusqlite::Connection,
    k: &Keys,
    r: &Reservation,
    ts: u64,
    wall: u64,
) -> Result<()> {
    let ct = seal(k, r)?;
    tx.execute(
        "UPDATE agent_reservations SET pending_name=?,state=?,operation_id=?,desired_profile_ts=?,ciphertext=?,updated_at=? WHERE community=? AND owner=? AND agent=?",
        params![r.pending_name, r.state.as_str(), r.operation_id, ts, ct, wall, r.scope.community, r.scope.owner.to_hex(), r.agent.to_hex()],
    )?;
    Ok(())
}

/// Called in the transaction retaining these exact Deploy bytes. A legacy client
/// without an agent reservation still owns its own materialization sequence.
/// Returns true when a first reservation link needs to exclude older, unlinked Deploys.
pub(super) fn link_deployment(
    tx: &Transaction<'_>,
    keys: &Keys,
    authority: &Authority,
    runtime: &str,
    previous: Option<&str>,
    wall: u64,
) -> Result<bool> {
    let scope = &authority.scope;
    let Some((mut reservation, profile_ts)) = q_one(
        tx,
        keys,
        "agent",
        params![
            scope.community,
            keys.public_key().to_hex(),
            authority.agent.to_hex()
        ],
    )?
    else {
        return Ok(false);
    };
    ensure!(
        authority.action == buzz_core::host_deployment::Action::Deploy
            && reservation.scope == *scope
            && reservation.agent == authority.agent
            && reservation.runtime_id == runtime,
        "reservation placement mismatch"
    );
    if reservation.operation_id.as_deref() == Some(&authority.operation_id) {
        ensure!(
            matches!(reservation.state, Prepared | Active),
            "invalid reservation state"
        );
        return Ok(false);
    }
    ensure!(
        reservation.operation_id.as_deref() == previous
            && match previous {
                None => reservation.state == Reserved && authority.expected_revision == 0,
                Some(_) => reservation.state == Prepared,
            },
        "reservation deployment predecessor changed"
    );
    reservation.state = Prepared;
    reservation.operation_id = Some(authority.operation_id.clone());
    save_row(tx, keys, &reservation, profile_ts, wall)?;
    Ok(previous.is_none())
}

impl Device {
    pub(super) fn reservation_schema(&self) -> Result<()> {
        self.db.execute_batch(
            "CREATE TABLE IF NOT EXISTS agent_reservations(community TEXT NOT NULL,owner TEXT NOT NULL,agent TEXT NOT NULL,host TEXT NOT NULL,generation TEXT NOT NULL,runtime_id TEXT NOT NULL,name_scope TEXT NOT NULL,pending_name TEXT,state TEXT NOT NULL CHECK(state IN('reserved','prepared','active','retiring','retired')),operation_id TEXT,desired_profile_ts INTEGER NOT NULL DEFAULT 0,ciphertext TEXT NOT NULL,updated_at INTEGER NOT NULL,PRIMARY KEY(community,owner,agent));
            CREATE UNIQUE INDEX IF NOT EXISTS agent_reservations_pending_name ON agent_reservations(community,owner,pending_name) WHERE pending_name IS NOT NULL;
            CREATE INDEX IF NOT EXISTS agent_reservations_by_scope ON agent_reservations(community,owner,host,generation,runtime_id,state);",
        )?;
        Ok(())
    }

    fn mutate(
        &mut self,
        s: &str,
        k: &Keys,
        a: &PublicKey,
        edit: impl FnOnce(&Transaction<'_>, &mut Reservation, &mut u64, u64) -> Result<bool>,
    ) -> Result<Reservation> {
        self.deployment_schema()?;
        let wall = now()?;
        self.check_time(wall)?;
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (mut res, mut ts) = q_one(
            &tx,
            k,
            "agent",
            params![s, &k.public_key().to_hex(), &a.to_hex()],
        )?
        .context("missing agent reservation")?;
        if edit(&tx, &mut res, &mut ts, wall)? {
            save_row(&tx, k, &res, ts, wall)?;
            tx.commit()?;
        }
        Ok(res)
    }

    pub fn reserve_agent(
        &mut self,
        s: &str,
        k: &Keys,
        req: ReserveAgent<'_>,
    ) -> Result<Reservation> {
        self.deployment_schema()?;
        let (scope, rt, cfg, trimmed) = (req.scope, req.runtime_id, req.config, req.name.trim());
        session(scope, s, k)?;
        validate_scope(scope)?;
        ensure!(runtime_id(rt), "invalid runtime");
        ensure!(display_name(trimmed), "invalid agent name");
        check_pic(req.picture.as_deref())?;
        cfg.validate()?;
        let wall = now()?;
        self.check_time(wall)?;
        let (oh, hh) = (k.public_key().to_hex(), scope.host.to_hex());
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure!(
            crate::device::grant(&tx, scope)?.active(),
            "host registration inactive"
        );

        if let Some((mut res, ts)) = q_one(&tx, k, "pending_name", params![s, &oh, trimmed])? {
            let same = res.scope.host == scope.host
                && res.scope.generation == scope.generation
                && res.runtime_id == rt;
            ensure!(same, "Pending setup for '{trimmed}' is already bound to another host or harness; finish or delete it first");
            return match res.state {
                Reserved => {
                    if res.config != cfg {
                        res.config = cfg;
                        save_row(&tx, k, &res, ts, wall)?;
                        tx.commit()?;
                    }
                    Ok(res)
                }
                Prepared => {
                    ensure!(
                        res.config.frozen_matches(&cfg),
                        "Pending setup configuration is already prepared and locked"
                    );
                    Ok(res)
                }
                _ => bail!("Pending setup is retiring or already active"),
            };
        }

        let ak = Keys::generate();
        let (agent, ah) = (ak.public_key(), ak.public_key().to_hex());
        ensure!(
            agent != scope.owner && agent != scope.host,
            "generated key collides with owner or host"
        );
        let known = tx
            .query_row("SELECT 1 FROM known_identities WHERE key=?", [&ah], |_| {
                Ok(())
            })
            .optional()?;
        ensure!(known.is_none(), "generated key already known");
        ensure!(
            q_one(&tx, k, "agent", params![s, &oh, &ah])?.is_none(),
            "generated key already reserved"
        );
        let mut stmt = tx.prepare(
            "SELECT data FROM deployment_intents WHERE community=? AND owner=? LIMIT 1025",
        )?;
        for raw in stmt.query_map(params![s, &oh], |r| r.get::<_, String>(0))? {
            ensure!(
                serde_json::from_str::<Intent>(&raw?)?
                    .protected
                    .authority
                    .agent
                    != agent,
                "generated key already has deployment intent"
            );
        }
        drop(stmt);

        let res = Reservation {
            scope: scope.clone(),
            runtime_id: rt.into(),
            name_scope: trimmed.into(),
            pending_name: Some(trimmed.into()),
            agent,
            state: Reserved,
            operation_id: None,
            agent_nsec: Some(Zeroizing::new(ak.secret_key().to_bech32()?)),
            nip_oa: None,
            picture: req.picture.clone(),
            config: cfg,
            desired_profile: Some(DesiredProfile {
                name: trimmed.into(),
                picture: req.picture,
                updated_at: wall,
            }),
            last_profile_created_at: None,
            profile_synced: false,
            retired_at: None,
            retired_via: None,
        };
        let ct = seal(k, &res)?;
        tx.execute(
            "INSERT INTO agent_reservations(community,owner,agent,host,generation,runtime_id,name_scope,pending_name,state,operation_id,desired_profile_ts,ciphertext,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?)",
            params![s, &oh, &ah, &hh, &scope.generation, rt, trimmed, trimmed, Reserved.as_str(), Option::<String>::None, wall, ct, wall],
        )?;
        tx.commit()?;
        Ok(res)
    }

    pub fn reservation(&self, s: &str, k: &Keys, a: &PublicKey) -> Result<Option<Reservation>> {
        self.deployment_schema()?;
        Ok(q_one(
            &self.db,
            k,
            "agent",
            params![s, &k.public_key().to_hex(), &a.to_hex()],
        )?
        .map(|(r, _)| r))
    }

    pub fn bind_reservation_attestation(
        &mut self,
        s: &str,
        k: &Keys,
        a: &PublicKey,
        oa: Vec<String>,
    ) -> Result<Reservation> {
        validate_auth_tag(&serde_json::to_string(&oa)?, &k.public_key().to_hex(), a)?;
        self.mutate(s, k, a, |_, r, _, _| {
            ensure!(!r.state.retired(), "reservation retired");
            if let Some(ex) = &r.nip_oa {
                ensure!(ex.get(..2) == oa.get(..2), "attestation changed");
                return Ok(false);
            }
            r.nip_oa = Some(oa);
            Ok(true)
        })
    }

    pub fn mark_reservation_prepared(
        &mut self,
        s: &str,
        k: &Keys,
        a: &PublicKey,
        op: &str,
    ) -> Result<Reservation> {
        let reservation = self
            .reservation(s, k, a)?
            .context("missing agent reservation")?;
        ensure!(
            reservation.state == Prepared && reservation.operation_id.as_deref() == Some(op),
            "deployment is not linked to this reservation"
        );
        Ok(reservation)
    }

    pub fn mark_reservation_active(
        &mut self,
        s: &str,
        k: &Keys,
        a: &PublicKey,
        op: &str,
        eff: Option<Option<String>>,
    ) -> Result<Reservation> {
        ensure!(valid_id(op), "invalid operation");
        self.mutate(s, k, a, |_, r, _, _| {
            ensure!(
                matches!(r.state, Prepared | Active) && r.operation_id.as_deref() == Some(op),
                "invalid reservation state"
            );
            r.state = Active;
            r.pending_name = None;
            if let Some(e) = eff {
                r.config.effort = e;
                r.config.validate()?;
            }
            Ok(true)
        })
    }

    pub fn set_desired_profile(
        &mut self,
        s: &str,
        k: &Keys,
        a: &PublicKey,
        name: &str,
        pic: Option<String>,
    ) -> Result<Reservation> {
        let trimmed = name.trim();
        ensure!(display_name(trimmed), "invalid agent name");
        check_pic(pic.as_deref())?;
        self.mutate(s, k, a, |_, r, ts, wall| {
            ensure!(r.state == Active, "agent must be active to update profile");
            *ts = wall.max(ts.saturating_add(1));
            r.desired_profile = Some(DesiredProfile {
                name: trimmed.into(),
                picture: pic,
                updated_at: *ts,
            });
            r.profile_synced = false;
            Ok(true)
        })
    }

    pub fn mark_profile_published(
        &mut self,
        s: &str,
        k: &Keys,
        a: &PublicKey,
        ts: u64,
    ) -> Result<Reservation> {
        self.mutate(s, k, a, |_, r, _, _| {
            ensure!(!r.state.retired(), "reservation retired");
            r.last_profile_created_at = Some(r.last_profile_created_at.map_or(ts, |t| t.max(ts)));
            r.profile_synced = true;
            Ok(true)
        })
    }

    pub fn begin_retire_reservation(
        &mut self,
        s: &str,
        k: &Keys,
        a: &PublicKey,
    ) -> Result<Option<Reservation>> {
        if self.reservation(s, k, a)?.is_none() {
            return Ok(None);
        }
        self.mutate(s, k, a, |_, r, _, _| {
            if r.state == Retired {
                return Ok(false);
            }
            (r.state, r.pending_name) = (Retiring, None);
            Ok(true)
        })
        .map(Some)
    }

    pub fn finish_retire_reservation(
        &mut self,
        s: &str,
        k: &Keys,
        a: &PublicKey,
        via: &str,
        op: Option<&str>,
    ) -> Result<Reservation> {
        ensure!(
            matches!(via, "never-deployed" | "cleaned"),
            "invalid retirement reason"
        );
        ensure!(op.is_none_or(valid_id), "invalid operation");
        let ah = a.to_hex();
        self.mutate(s, k, a, |tx, r, _, wall| {
            (r.state, r.pending_name, r.agent_nsec, r.nip_oa) = (Retired, None, None, None);
            r.retired_at.get_or_insert(wall);
            r.retired_via.get_or_insert_with(|| via.into());
            if let Some(id) = op {
                r.operation_id = Some(id.into());
            }
            tx.execute("INSERT OR IGNORE INTO known_identities VALUES(?)", [&ah])?;
            Ok(true)
        })
    }
}
