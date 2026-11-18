//! Build and freeze the post-commit artifacts once. The host publishes only
//! agent-authored records. Registration and channel additions belong to Windows.
use super::{
    clock::Clock,
    request::Request,
    store::{Catalog, Store},
    transport::Http,
};
use crate::store::InvalidPacket;
use anyhow::{anyhow, ensure, Result};
use buzz_core::paired_message::Scope;
use nostr::{Event, EventBuilder, Kind, PublicKey, Tags, Timestamp};
use rusqlite::{params, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const SCHEMA:&str="CREATE TABLE onboarding(g TEXT NOT NULL,r TEXT NOT NULL,oa TEXT NOT NULL,registration TEXT NOT NULL,profile TEXT NOT NULL,directory TEXT NOT NULL,PRIMARY KEY(g,r));";

/// Public 30177 content is a projection, and has no private/local extensions.
#[derive(Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Registration {
    name: String,
    parallelism: u32,
    respond_to: String,
    #[serde(default)]
    respond_to_allowlist: Vec<String>,
}

pub fn accept(
    tx: &Transaction<'_>,
    scope: &Scope,
    q: &Request,
    oa: &[String; 4],
    reg: &Event,
    clock: &Clock,
) -> Result<()> {
    let agent = q.agent_keys()?;
    ensure!(
        oa[0] == "auth" && oa[1] == scope.owner.to_hex() && oa[2].is_empty(),
        InvalidPacket
    );
    let attestation = serde_json::to_string(oa)?;
    let verified = buzz_sdk::nip_oa::verify_auth_tag_for_auth_event(
        &attestation,
        &agent.public_key(),
        clock.wall,
    )
    .map_err(|_| InvalidPacket)?;
    ensure!(verified == scope.owner, InvalidPacket);
    let expected = Registration {
        name: q.name.clone(),
        parallelism: 1,
        respond_to: q.policy.respond_to.clone(),
        respond_to_allowlist: q.policy.respond_to_allowlist.clone(),
    };
    ensure!(
        buzz_core::verify_event(reg).is_ok()
            && reg.pubkey == scope.owner
            && reg.kind == Kind::Custom(30177)
            && reg.tags
                == Tags::parse([["d", &agent.public_key().to_hex()], ["runtime", &q.runtime]])?
            && serde_json::from_str::<Registration>(&reg.content).map_err(|_| InvalidPacket)?
                == expected,
        InvalidPacket
    );
    let auth = buzz_sdk::nip_oa::parse_auth_tag(&attestation)?;
    let profile =
        buzz_sdk::builders::build_profile(Some(&q.name), Some(&q.name), None, None, None)?
            .tags([auth.clone()])
            .custom_created_at(Timestamp::from(clock.wall))
            .sign_with_keys(&agent)?;
    let directory=EventBuilder::new(Kind::Custom(10100),json!({
        "name":q.name,"display_name":q.name,"agent_type":"agent","channels":[],"channel_ids":q.channels,"capabilities":[],
        "status":"offline","respond_to":q.policy.respond_to,"respond_to_allowlist":q.policy.respond_to_allowlist,
        "channel_add_policy":q.policy.channel_add_policy,"host":scope.host.to_hex(),
    }).to_string()).tags([auth]).custom_created_at(Timestamp::from(clock.wall)).sign_with_keys(&agent)?;
    tx.execute(
        "INSERT INTO onboarding VALUES(?,?,?,?,?,?)",
        params![
            scope.generation,
            q.r,
            attestation,
            serde_json::to_string(reg)?,
            serde_json::to_string(&profile)?,
            serde_json::to_string(&directory)?
        ],
    )?;
    Ok(())
}

#[derive(Deserialize)]
struct Receipt {
    accepted: bool,
}

async fn head(
    net: &Http,
    keys: &nostr::Keys,
    c: &str,
    oa: &str,
    mut filter: Value,
) -> Result<Option<Event>> {
    filter["consistency"] = json!("strong");
    let query = serde_json::to_string(&[filter])?;
    let rows: Vec<Event> = net
        .authed(keys, &format!("{c}/query"), Some(query), Some(oa))
        .await?;
    ensure!(rows.len() <= 1, "ambiguous event head");
    Ok(rows.into_iter().next())
}

#[derive(Debug)]
pub(crate) struct Permanent(pub String);
impl std::fmt::Display for Permanent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Permanent {}

pub async fn reconcile(
    store: &mut Store,
    g: &str,
    r: &str,
    net: &Http,
    cat: &Catalog,
    revision: &str,
) -> Result<()> {
    let keys = store.keys()?;
    let (scope, mut q) = {
        let tx = store.tx()?;
        let (scope, _, _) = super::store::binding(&tx, g, keys.public_key())?;
        (
            scope,
            Request::load(&tx, g, r)?.ok_or_else(|| anyhow!("missing request"))?,
        )
    };
    if q.state == "ready" {
        super::unit::settle(store, &q, &scope).await?;
        q.sealed = true;
        let tx = store.tx()?;
        q.update(&tx, g)?;
        tx.commit()?;
        return Ok(());
    }
    if q.failure.is_none() {
        match attempt(store, g, r, net, cat, revision).await {
            Ok(()) => return Ok(()),
            Err(err) => {
                let code = if let Some(code) = err.downcast_ref::<Permanent>() {
                    code.0.clone()
                } else if err
                    .downcast_ref::<crate::transport::HttpStatus>()
                    .is_some_and(|s| matches!(s.0, 401 | 403))
                {
                    "admission-denied".into()
                } else {
                    return Err(err);
                };
                let tx = store.tx()?;
                q = Request::load(&tx, g, r)?.ok_or_else(|| anyhow!("missing request"))?;
                q.failure = Some(code);
                q.update(&tx, g)?;
                tx.commit()?;
            }
        }
    }
    finish_failure(store, q, &scope, &keys).await
}

async fn finish_failure(
    store: &mut Store,
    mut q: Request,
    scope: &Scope,
    keys: &nostr::Keys,
) -> Result<()> {
    // Record the decision before quiescing. A lost helper reply repeats only
    // cleanup, never another start. Publish a terminal result after read-back.
    if q.start_attempted {
        super::unit::quiesce(store, &q, &scope).await?;
    }
    let tx = store.tx()?;
    let code = q
        .failure
        .clone()
        .ok_or_else(|| anyhow!("missing failure"))?;
    q.finish(&tx, &keys, &scope, "failed", &code, &Clock::read()?)?;
    tx.commit()?;
    Ok(())
}

/// An unreadable/empty replacement configuration cannot leave an unsettled
/// enabled process behind. Committed and previously completed agents are
/// distinguished by the persisted start-intent and terminal receipt.
pub async fn unavailable_catalog(store: &mut Store) -> Result<()> {
    let rows = {
        let mut stmt=store.0.prepare("SELECT g,r FROM requests WHERE json_extract(record,'$.state')='committed' AND json_extract(record,'$.start_attempted')=1")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    let keys = store.keys()?;
    for (g, r) in rows {
        let tx = store.tx()?;
        let (scope, _, _) = super::store::binding(&tx, &g, keys.public_key())?;
        let mut q = Request::load(&tx, &g, &r)?.ok_or_else(|| anyhow!("missing request"))?;
        q.failure.get_or_insert_with(|| "catalog-changed".into());
        q.update(&tx, &g)?;
        tx.commit()?;
        finish_failure(store, q, &scope, &keys).await?;
    }
    Ok(())
}
async fn attempt(
    store: &mut Store,
    g: &str,
    r: &str,
    net: &Http,
    cat: &Catalog,
    revision: &str,
) -> Result<()> {
    let h = store.keys()?;
    let (scope, q, oa, reg, profile, directory) = {
        let tx = store.tx()?;
        let (scope, _, _) = super::store::binding(&tx, g, h.public_key())?;
        let q = Request::load(&tx, g, r)?.ok_or_else(|| anyhow!("unknown request"))?;
        ensure!(
            q.commit.is_some() && q.credentials.is_some() && q.terminal.is_none(),
            "missing committed credentials"
        );
        let (oa, reg, p, d): (String, String, String, String) = tx.query_row(
            "SELECT oa,registration,profile,directory FROM onboarding WHERE g=? AND r=?",
            params![g, r],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?;
        (
            scope,
            q,
            oa,
            serde_json::from_str::<Event>(&reg)?,
            serde_json::from_str::<Event>(&p)?,
            serde_json::from_str::<Event>(&d)?,
        )
    };
    ensure!(
        q.revision == revision && cat.current(revision)?,
        Permanent("catalog-changed".into())
    );
    let agent = q.agent_keys()?;
    let c = &scope.community;
    for event in [&profile, &directory] {
        let published: Result<Receipt> = net
            .authed(
                &agent,
                &format!("{c}/events"),
                Some(serde_json::to_string(event)?),
                Some(&oa),
            )
            .await;
        match published {
            Ok(Receipt { accepted: true }) => (),
            Err(e)
                if !e
                    .downcast_ref::<crate::transport::HttpStatus>()
                    .is_some_and(|s| matches!(s.0, 401 | 403)) =>
            {
                return Err(e)
            }
            _ => return Err(Permanent("admission-denied".into()).into()),
        }
        let live=head(net,&agent,c,&oa,json!({"kinds":[event.kind.as_u16()],"authors":[agent.public_key().to_hex()],"limit":1})).await?;
        let live = live.ok_or_else(|| anyhow!("awaiting agent record"))?;
        ensure!(
            buzz_core::verify_event(&live).is_ok()
                && live.kind == event.kind
                && live.pubkey == event.pubkey,
            "invalid head evidence"
        );
        ensure!(live.id == event.id, Permanent("profile-conflict".into()));
        record_step(
            store,
            g,
            r,
            if event.kind == Kind::Metadata {
                "profile-verified"
            } else {
                "directory-verified"
            },
        )?;
    }
    let live=head(net,&agent,c,&oa,json!({"kinds":[30177],"authors":[scope.owner.to_hex()],"#d":[agent.public_key().to_hex()],"limit":1})).await?;
    let live = live.ok_or_else(|| anyhow!("awaiting owner registration"))?;
    ensure!(
        buzz_core::verify_event(&live).is_ok()
            && live.pubkey == scope.owner
            && live.kind == reg.kind
            && live.tags.identifier() == reg.tags.identifier(),
        "invalid registration evidence"
    );
    if live.id != reg.id {
        ensure!(
            live.created_at < reg.created_at
                || (live.created_at == reg.created_at && live.id > reg.id),
            Permanent("profile-conflict".into())
        );
        return Err(anyhow!("awaiting current owner registration"));
    }
    record_step(store, g, r, "registration-verified")?;
    let relay = PublicKey::from_hex(&cat.relay_key)?;
    for channel in &q.channels {
        let roster = head(
            net,
            &agent,
            c,
            &oa,
            json!({"kinds":[39002],"authors":[relay.to_hex()],"#d":[channel],"limit":1}),
        )
        .await
        .map_err(|e| {
            if e.downcast_ref::<crate::transport::HttpStatus>()
                .is_some_and(|s| matches!(s.0, 401 | 403))
            {
                anyhow::Error::from(Permanent(format!("membership-denied:{channel}")))
            } else {
                e
            }
        })?
        .ok_or_else(|| anyhow!("awaiting channel roster"))?;
        ensure!(
            roster.kind == Kind::Custom(39002)
                && roster.pubkey == relay
                && buzz_core::verify_event(&roster).is_ok()
                && roster.tags.identifier() == Some(channel.as_str()),
            "invalid roster evidence"
        );
        let found = roster.tags.iter().any(|t| {
            t.as_slice().first().is_some_and(|v| v == "p")
                && t.as_slice().get(1) == Some(&agent.public_key().to_hex())
        });
        if !found {
            ensure!(
                !q.progress
                    .contains(&format!("membership-verified:{channel}")),
                Permanent(format!("membership-denied:{channel}"))
            );
            return Err(anyhow!("awaiting owner channel publication"));
        }
        record_step(store, g, r, &format!("membership-verified:{channel}"))?;
    }
    super::unit::activate(store, &q, &scope, &oa, cat, revision).await?;
    record_step(store, g, r, "unit-active")?;
    if q.started_at.is_none() {
        let tx = store.tx()?;
        let mut local = Request::load(&tx, g, r)?.ok_or_else(|| anyhow!("missing request"))?;
        local.started_at = Some(Clock::read()?.wall);
        local.update(&tx, g)?;
        tx.commit()?;
        // Recheck authoritative heads/rosters after the bounded startup wait.
        return Ok(());
    }
    let presence = head(
        net,
        &agent,
        c,
        &oa,
        json!({"kinds":[20001],"authors":[agent.public_key().to_hex()],"limit":1}),
    )
    .await?;
    if presence.as_ref().is_none_or(|p| p.content != "online") {
        ensure!(
            Clock::read()?
                .wall
                .saturating_sub(q.started_at.unwrap_or_default())
                < 90,
            Permanent("unit-failed".into())
        );
        return Err(anyhow!("awaiting worker presence"));
    }
    let presence = presence.ok_or_else(|| anyhow!("awaiting worker presence"))?;
    let now = Clock::read()?.wall;
    ensure!(
        buzz_core::verify_event(&presence).is_ok()
            && presence.pubkey == relay
            && presence.kind == Kind::Custom(20001)
            && presence.tags == Tags::parse([["p", &agent.public_key().to_hex()]])?
            && presence.content == "online"
            && presence.created_at.as_secs().abs_diff(now) <= 60,
        "worker presence not confirmed"
    );
    ensure!(cat.current(revision)?, Permanent("catalog-changed".into()));
    record_step(store, g, r, "presence-confirmed")?;
    let tx = store.tx()?;
    let mut q = Request::load(&tx, g, r)?.ok_or_else(|| anyhow!("request disappeared"))?;
    if q.terminal.is_none() {
        q.finish(&tx, &h, &scope, "ready", "verified", &Clock::read()?)?;
    }
    tx.commit()?;
    Ok(())
}

fn record_step(store: &mut Store, g: &str, r: &str, step: &str) -> Result<()> {
    let tx = store.tx()?;
    let mut q = Request::load(&tx, g, r)?.ok_or_else(|| anyhow!("missing request"))?;
    if !q.progress.iter().any(|s| s == step) {
        q.progress.push(step.into());
        q.update(&tx, g)?;
    }
    tx.commit()?;
    Ok(())
}
