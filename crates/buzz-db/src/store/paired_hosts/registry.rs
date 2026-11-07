//! V2 adapters share the existing writer, owner policy and transcript tables.
use super::*;
use buzz_core::{host_registry as wire, PublicKey};

/// Admission ceiling, never a cached authorization grant.
#[derive(Clone, Debug)]
pub struct RegistrySession {
    /// Owner dependency for removal/disconnect tracking; absent before selection.
    pub owner: Option<[u8; 32]>,
    /// Removed-owner mode returns only binding state and permits tombstones.
    pub recovery_only: bool,
    /// Expired candidate may only retry a previously signed timely ack.
    pub ack_retry_only: bool,
}
/// Compact private read, never used by generic event storage.
pub struct RegistryRead<'a> {
    /// Validated fork query.
    pub query: &'a wire::Query,
    /// Exclusive generation or sequence position from an authenticated cursor.
    pub after: Option<&'a str>,
    /// First page snapshot high-water, if continuing.
    pub snapshot: Option<i64>,
    /// Host connection scope ceiling; normal members have no such ceiling.
    pub host_generation: Option<&'a [u8; 32]>,
}
/// Bounded data and opaque cursor inputs, sealed by the relay adapter.
pub struct RegistryPage {
    /// Compact current state or exact evidence.
    pub data: Value,
    /// Principal-scoped committed high-water.
    pub high_water: i64,
    /// Exclusive next position if incomplete.
    pub next: Option<String>,
}
impl Db {
    /// Scoped host admission under the same fence as registration mutations.
    pub async fn registry_session(
        &self,
        c: CommunityId,
        host: &[u8; 32],
        g: &[u8; 32],
        allow: bool,
    ) -> Result<RegistrySession> {
        let mut tx = self.paired_tx(c).await?;
        let result = session(&mut tx, host, g, allow).await?;
        tx.commit().await?;
        Ok(result)
    }
    /// Translate strict signed v2 routing to the existing authoritative writer.
    pub async fn post_registry_event(
        &self,
        c: CommunityId,
        origin: &str,
        signer: PublicKey,
        raw: &[u8],
        allow: bool,
        source: &[u8; 32],
    ) -> Result<()> {
        let (e, h) = wire::verify(raw, origin, signer).map_err(|_| denied())?;
        let p = Packet {
            id: e.id.to_bytes(),
            generation: wire::bytes(&h.generation).map_err(|_| denied())?,
            owner: wire::bytes(&h.owner).map_err(|_| denied())?,
            host: wire::bytes(&h.host).map_err(|_| denied())?,
            op: h.op.clone(),
            prev: h
                .parent
                .as_deref()
                .map(wire::bytes)
                .transpose()
                .map_err(|_| denied())?,
            request: Some(wire::bytes(&h.operation_id).map_err(|_| denied())?),
            created: e.created_at.as_secs().try_into().map_err(|_| denied())?,
            expiry: h.expires_at.try_into().map_err(|_| denied())?,
            event: serde_json::value::RawValue::from_string(
                std::str::from_utf8(raw).map_err(|_| denied())?.into(),
            )?,
            route: serde_json::to_value(h)?,
        };
        self.post_paired_packet(c, allow, &p, source).await
    }
    /// Protected v2 projection, reauthorized on the primary for every page/send.
    pub async fn read_registry(
        &self,
        c: CommunityId,
        reader: &[u8; 32],
        allow: bool,
        r: RegistryRead<'_>,
    ) -> Result<RegistryPage> {
        r.query.validate().map_err(|_| denied())?;
        let mut tx = self.paired_tx(c).await?;
        let g = r
            .query
            .generation
            .as_deref()
            .map(wire::bytes)
            .transpose()
            .map_err(|_| denied())?;
        let mut recovery = false;
        if let Some(ceiling) = r.host_generation {
            require(
                g.as_ref() == Some(ceiling)
                    && !matches!(r.query.view.as_str(), "hosts" | "history"),
            )?;
            let scoped_session = session(&mut tx, reader, ceiling, allow).await?;
            require(!scoped_session.ack_retry_only)?;
            recovery = scoped_session.recovery_only;
            require(!recovery || matches!(r.query.view.as_str(), "binding" | "live"))?;
        } else {
            owner_access(&mut tx, reader, allow).await?;
        }
        let t = c.as_uuid();
        let high:i64=sqlx::query_scalar("SELECT coalesce(max(seq),0) FROM paired_host_events WHERE community_id=$1 AND (owner_key=$2 OR host_key=$2) AND ($3::bytea IS NULL OR generation=$3) AND (event_json::jsonb->>'kind')::int=43103")
            .bind(t).bind(&reader[..]).bind(g.as_ref().map(|b|&b[..])).fetch_one(tx.conn()).await?;
        if let Some(old) = r.snapshot {
            require(old == high)?;
        }
        let mut next = None;
        let data = match r.query.view.as_str() {
            "hosts" | "history" => {
                let after = r.after.map(wire::bytes).transpose().map_err(|_| denied())?;
                let rows=sqlx::query("SELECT b.generation,b.owner_key,b.host_key,b.state,b.claim_id,b.ack_id,b.tombstone_id FROM paired_host_bindings b JOIN paired_host_events e ON e.community_id=b.community_id AND e.event_id=b.claim_id WHERE b.community_id=$1 AND b.owner_key=$2 AND ($3 OR b.state<>'dead') AND ($4::bytea IS NULL OR b.generation>$4) AND (e.event_json::jsonb->>'kind')::int=43103 ORDER BY b.generation LIMIT 17")
                    .bind(t).bind(&reader[..]).bind(r.query.view=="history").bind(after.as_ref().map(|b|&b[..])).fetch_all(tx.conn()).await?;
                let more = rows.len() > 16;
                let mut hosts = Vec::new();
                for b in rows.iter().take(16) {
                    let h: Vec<u8> = b.get("host_key");
                    if !principal_restricted(tx.conn(), c, &hex::encode(&h)).await? {
                        hosts.push(summary(b));
                    }
                }
                if more {
                    next = Some(hex::encode(rows[15].get::<Vec<u8>, _>("generation")));
                }
                json!({"hosts":hosts})
            }
            "offers" => {
                let g = g.ok_or_else(denied)?;
                require(!recovery)?;
                let after = r
                    .after
                    .unwrap_or("0")
                    .parse::<i64>()
                    .map_err(|_| denied())?;
                let rows=sqlx::query("SELECT owner_key,host_key,event_id,seq FROM paired_host_events e WHERE community_id=$1 AND generation=$2 AND (owner_key=$3 OR host_key=$3) AND operation='pair-claim' AND expires_at>=floor(extract(epoch FROM now()))-86400 AND seq>$4 AND (event_json::jsonb->>'kind')::int=43103 AND NOT EXISTS(SELECT 1 FROM paired_host_bindings b WHERE b.community_id=e.community_id AND b.generation=e.generation AND (b.state<>'active' OR b.claim_id<>e.event_id)) ORDER BY seq LIMIT 101")
                    .bind(t).bind(&g[..]).bind(&reader[..]).bind(after).fetch_all(tx.conn()).await?;
                let mut refs = Vec::new();
                for row in rows.iter().take(100) {
                    let o: Vec<u8> = row.get("owner_key");
                    let h: Vec<u8> = row.get("host_key");
                    match authorize(&mut tx, &o, &h, allow, false).await {
                        Ok(()) => refs.push(hex::encode(row.get::<Vec<u8>, _>("event_id"))),
                        Err(DbError::AccessDenied(_)) => {}
                        Err(e) => return Err(e),
                    }
                }
                if rows.len() > 100 {
                    next = Some(rows[99].get::<i64, _>("seq").to_string());
                }
                json!({"events":refs})
            }
            "live" if g.is_none() => {
                let generation:Option<Vec<u8>>=sqlx::query_scalar("SELECT generation FROM paired_host_events WHERE community_id=$1 AND owner_key=$2 AND (event_json::jsonb->>'kind')::int=43103 ORDER BY seq DESC LIMIT 1").bind(t).bind(&reader[..]).fetch_optional(tx.conn()).await?;
                json!({"generation":generation.map(hex::encode)})
            }
            _ => {
                let g = g.ok_or_else(denied)?;
                let b=sqlx::query("SELECT b.generation,b.owner_key,b.host_key,b.state,b.claim_id,b.ack_id,b.tombstone_id,b.retention_floor FROM paired_host_bindings b JOIN paired_host_events e ON e.community_id=b.community_id AND e.event_id=b.claim_id WHERE b.community_id=$1 AND b.generation=$2 AND (b.owner_key=$3 OR b.host_key=$3) AND (e.event_json::jsonb->>'kind')::int=43103")
                    .bind(t).bind(&g[..]).bind(&reader[..]).fetch_optional(tx.conn()).await?;
                // A candidate host may listen before the ack commits.
                if b.is_none() && r.query.view == "live" {
                    json!({})
                } else if b.is_none() && r.query.view == "evidence" {
                    let id = wire::bytes(r.query.event_id.as_deref().ok_or_else(denied)?)
                        .map_err(|_| denied())?;
                    let row=sqlx::query("SELECT owner_key,host_key,event_json FROM paired_host_events WHERE community_id=$1 AND generation=$2 AND (owner_key=$3 OR host_key=$3) AND event_id=$4 AND operation='pair-claim' AND expires_at>=floor(extract(epoch FROM now()))-86400 AND (event_json::jsonb->>'kind')::int=43103")
                        .bind(t).bind(&g[..]).bind(&reader[..]).bind(&id[..]).fetch_optional(tx.conn()).await?.ok_or_else(denied)?;
                    authorize(
                        &mut tx,
                        &row.get::<Vec<u8>, _>("owner_key"),
                        &row.get::<Vec<u8>, _>("host_key"),
                        allow,
                        false,
                    )
                    .await?;
                    json!({"event":serde_json::from_str::<Value>(row.get("event_json"))?})
                } else {
                    let b = b.ok_or_else(denied)?;
                    if !recovery {
                        authorize(
                            &mut tx,
                            &b.get::<Vec<u8>, _>("owner_key"),
                            &b.get::<Vec<u8>, _>("host_key"),
                            allow,
                            false,
                        )
                        .await?;
                    }
                    match r.query.view.as_str() {
                        "binding" | "live" => {
                            if recovery {
                                json!({"generation":hex::encode(g),"state":b.get::<&str,_>("state"),"recovery_only":true})
                            } else {
                                let latest:Option<Vec<u8>>=sqlx::query_scalar("SELECT event_id FROM paired_host_events WHERE community_id=$1 AND generation=$2 AND operation='status' ORDER BY seq DESC LIMIT 1").bind(t).bind(&g[..]).fetch_optional(tx.conn()).await?;
                                let mut data = summary(&b);
                                data["latest_status_id"] = json!(latest.map(hex::encode));
                                data
                            }
                        }
                        "evidence" => {
                            let id = wire::bytes(r.query.event_id.as_deref().ok_or_else(denied)?)
                                .map_err(|_| denied())?;
                            let raw:String=sqlx::query_scalar("SELECT event_json FROM paired_host_events WHERE community_id=$1 AND generation=$2 AND owner_key=$3 AND host_key=$4 AND event_id=$5 AND (event_json::jsonb->>'kind')::int=43103 AND (operation<>'pair-claim' OR event_id=$6)")
                                .bind(t).bind(&g[..]).bind(b.get::<Vec<u8>,_>("owner_key")).bind(b.get::<Vec<u8>,_>("host_key")).bind(&id[..]).bind(b.get::<Vec<u8>,_>("claim_id")).fetch_optional(tx.conn()).await?.ok_or_else(denied)?;
                            json!({"event":serde_json::from_str::<Value>(&raw)?})
                        }
                        "messages" => {
                            let after = r
                                .after
                                .unwrap_or("0")
                                .parse::<i64>()
                                .map_err(|_| denied())?;
                            // A null cursor starts a new snapshot of retained evidence.
                            // Only a continuation promises coverage from its old position.
                            if r.after.is_some() && after < b.get::<i64, _>("retention_floor") {
                                return Err(DbError::PairedCursorPruned);
                            }
                            let rows=sqlx::query("SELECT event_id,seq,operation,expires_at FROM paired_host_events WHERE community_id=$1 AND generation=$2 AND owner_key=$3 AND host_key=$4 AND seq>$5 AND (operation<>'pair-claim' OR event_id=$6) AND (event_json::jsonb->>'kind')::int=43103 ORDER BY seq LIMIT 101")
                                .bind(t).bind(&g[..]).bind(b.get::<Vec<u8>,_>("owner_key")).bind(b.get::<Vec<u8>,_>("host_key")).bind(after).bind(b.get::<Vec<u8>,_>("claim_id")).fetch_all(tx.conn()).await?;
                            if rows.len() > 100 {
                                next = Some(rows[99].get::<i64, _>("seq").to_string());
                            }
                            json!({"events":rows.iter().take(100).map(|r|json!({"id":hex::encode(r.get::<Vec<u8>,_>("event_id")),"op":r.get::<&str,_>("operation"),"expires_at":r.get::<i64,_>("expires_at")})).collect::<Vec<_>>()})
                        }
                        _ => return Err(denied()),
                    }
                }
            }
        };
        tx.commit().await?;
        Ok(RegistryPage {
            data,
            high_water: high,
            next,
        })
    }
}
async fn session(
    tx: &mut AdmittedTx,
    host: &[u8; 32],
    g: &[u8; 32],
    allow: bool,
) -> Result<RegistrySession> {
    let c = tx.community();
    require(!principal_restricted(tx.conn(), c, &hex::encode(host)).await?)?;
    let b=sqlx::query("SELECT b.owner_key FROM paired_host_bindings b JOIN paired_host_events e ON e.community_id=b.community_id AND e.event_id=b.claim_id WHERE b.community_id=$1 AND b.generation=$2 AND b.host_key=$3 AND (e.event_json::jsonb->>'kind')::int=43103")
        .bind(c.as_uuid()).bind(&g[..]).bind(&host[..]).fetch_optional(tx.conn()).await?;
    if let Some(b) = b {
        let owner: Vec<u8> = b.get("owner_key");
        let recovery = match owner_access(tx, &owner, allow).await {
            Ok(()) => false,
            Err(DbError::AccessDenied(_)) => true,
            Err(e) => return Err(e),
        };
        return Ok(RegistrySession {
            owner: Some(owner.try_into().map_err(|_| denied())?),
            recovery_only: recovery,
            ack_retry_only: false,
        });
    }
    let rows=sqlx::query("SELECT owner_key, expires_at>floor(extract(epoch from now())) AS live FROM paired_host_events WHERE community_id=$1 AND generation=$2 AND host_key=$3 AND operation='pair-claim' AND expires_at>=floor(extract(epoch from now()))-86400 AND (event_json::jsonb->>'kind')::int=43103 ORDER BY seq LIMIT 64")
        .bind(c.as_uuid()).bind(&g[..]).bind(&host[..]).fetch_all(tx.conn()).await?;
    for r in rows {
        let o: Vec<u8> = r.get("owner_key");
        match owner_access(tx, &o, allow).await {
            Ok(()) => {
                return Ok(RegistrySession {
                    owner: None,
                    recovery_only: false,
                    ack_retry_only: !r.get::<bool, _>("live"),
                })
            }
            Err(DbError::AccessDenied(_)) => {}
            Err(e) => return Err(e),
        }
    }
    Err(denied())
}
fn summary(b: &sqlx::postgres::PgRow) -> Value {
    let id = |n| hex::encode(b.get::<Vec<u8>, _>(n));
    json!({"generation":id("generation"),"owner":id("owner_key"),"host":id("host_key"),"state":b.get::<&str,_>("state"),"claim_id":id("claim_id"),"ack_id":id("ack_id"),"tombstone_id":b.get::<Option<Vec<u8>>,_>("tombstone_id").map(hex::encode),"latest_status_id":null})
}
