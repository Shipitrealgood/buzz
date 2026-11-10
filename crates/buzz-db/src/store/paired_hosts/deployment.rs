//! Private placement and execution admission under the registration writer fence.
use super::*;
use buzz_core::{
    host_deployment::{wire, Action, Outcome},
    PublicKey,
};

fn bytes(value: &str) -> Result<[u8; 32]> {
    buzz_core::host_registry::bytes(value).map_err(|_| denied())
}
fn action(a: Action) -> &'static str {
    match a {
        Action::Deploy => "deploy",
        Action::Cleanup => "cleanup",
    }
}
fn outcome(o: &Outcome) -> &'static str {
    match o {
        Outcome::Pending => "pending",
        Outcome::NeedsLogin => "needs-login",
        Outcome::Applied => "applied",
        Outcome::CleanupPending => "cleanup-pending",
        Outcome::Cleaned => "cleaned",
        Outcome::Failed => "failed",
    }
}
async fn wall(tx: &mut AdmittedTx) -> Result<i64> {
    Ok(
        sqlx::query_scalar("SELECT floor(extract(epoch from clock_timestamp()))::bigint")
            .fetch_one(tx.conn())
            .await?,
    )
}
async fn binding(
    tx: &mut AdmittedTx,
    scope: &buzz_core::paired_message::Scope,
    allow: bool,
) -> Result<()> {
    authorize(
        tx,
        scope.owner.as_bytes(),
        scope.host.as_bytes(),
        allow,
        false,
    )
    .await?;
    let g = bytes(&scope.generation)?;
    let c = *tx.community().as_uuid();
    let active: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM paired_host_bindings b JOIN paired_host_events e ON e.community_id=b.community_id AND e.event_id=b.claim_id WHERE b.community_id=$1 AND b.generation=$2 AND b.owner_key=$3 AND b.host_key=$4 AND b.state='active' AND (e.event_json::jsonb->>'kind')::int=43103)")
        .bind(c).bind(&g[..]).bind(scope.owner.as_bytes().as_slice()).bind(scope.host.as_bytes().as_slice()).fetch_one(tx.conn()).await?;
    require(active)
}
impl Db {
    /// Verify and atomically admit one exact operation, status, or host result.
    pub async fn post_host_deployment(
        &self,
        c: CommunityId,
        origin: &str,
        signer: PublicKey,
        raw: &[u8],
        allow: bool,
    ) -> Result<()> {
        let (event, header) = wire::verify(raw, origin, signer).map_err(|_| denied())?;
        let raw = std::str::from_utf8(raw).map_err(|_| denied())?;
        let mut tx = self.paired_tx(c).await?;
        binding(&mut tx, header.scope(), allow).await?;
        let now = wall(&mut tx).await?;
        require(event.created_at.as_secs() <= (now + 30) as u64)?;
        let t = *c.as_uuid();
        let scope = header.scope();
        let g = bytes(&scope.generation)?;
        let owner = scope.owner.as_bytes().as_slice();
        let host = scope.host.as_bytes().as_slice();
        match &header {
            wire::Header::Status { expires_at, .. } => {
                require(*expires_at > now as u64 && *expires_at <= now as u64 + 120)?;
                let old: Option<String> = sqlx::query_scalar("SELECT event_json FROM paired_host_deployment_status WHERE community_id=$1 AND generation=$2").bind(t).bind(&g[..]).fetch_optional(tx.conn()).await?;
                if let Some(old) = old {
                    let old: nostr::Event = serde_json::from_str(&old)?;
                    require(event.created_at > old.created_at || event.id == old.id)?;
                }
                sqlx::query("INSERT INTO paired_host_deployment_status VALUES($1,$2,$3,$4,$5,$6) ON CONFLICT(community_id,generation) DO UPDATE SET expires_at=excluded.expires_at,event_json=excluded.event_json")
                    .bind(t).bind(&g[..]).bind(owner).bind(host).bind(*expires_at as i64).bind(raw).execute(tx.conn()).await?;
            }
            wire::Header::Command { authority: a } => {
                let old: Option<String> = sqlx::query_scalar("SELECT event_json FROM paired_agent_operations WHERE community_id=$1 AND owner_pubkey=$2 AND operation_id=$3")
                    .bind(t).bind(owner).bind(&a.operation_id).fetch_optional(tx.conn()).await?;
                if let Some(old) = old {
                    require(old == raw)?;
                    return tx.commit().await;
                }
                a.validate(
                    scope,
                    a.expected_revision,
                    now as u64,
                    event.content.as_bytes(),
                )
                .map_err(|_| denied())?;
                require(a.expected_revision < i64::MAX as u64)?;
                let p = sqlx::query("SELECT owner_pubkey,host_pubkey,generation,revision,unit_id,cleanup_operation_id FROM paired_agent_placements WHERE community_id=$1 AND agent_pubkey=$2")
                    .bind(t).bind(a.agent.as_bytes().as_slice()).fetch_optional(tx.conn()).await?;
                if let Some(p) = &p {
                    require(
                        p.get::<Vec<u8>, _>("owner_pubkey") == owner
                            && p.get::<Vec<u8>, _>("host_pubkey") == host
                            && p.get::<Vec<u8>, _>("generation") == g
                            && p.get::<i64, _>("revision") == a.expected_revision as i64
                            && p.get::<Option<String>, _>("cleanup_operation_id").is_none(),
                    )?;
                } else {
                    require(a.action == Action::Deploy && a.expected_revision == 0)?;
                }
                let room: bool = sqlx::query_scalar("SELECT count(*)<4096 AND count(*) FILTER(WHERE owner_pubkey=$2)<1024 FROM paired_agent_operations WHERE community_id=$1")
                    .bind(t).bind(owner).fetch_one(tx.conn()).await?;
                require(room)?;
                let state = if a.action == Action::Cleanup {
                    "cleanup-pending"
                } else {
                    "pending"
                };
                let cleanup = (a.action == Action::Cleanup).then_some(&a.operation_id);
                sqlx::query("INSERT INTO paired_agent_operations(community_id,owner_pubkey,operation_id,agent_pubkey,host_pubkey,generation,action,expected_revision,expires_at,content_sha256,event_json,state) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)")
                    .bind(t).bind(owner).bind(&a.operation_id).bind(a.agent.as_bytes().as_slice()).bind(host).bind(&g[..]).bind(action(a.action)).bind(a.expected_revision as i64).bind(a.expires_at as i64).bind(&a.content_sha256).bind(raw).bind(state).execute(tx.conn()).await?;
                sqlx::query("INSERT INTO paired_agent_placements VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) ON CONFLICT(community_id,agent_pubkey) DO UPDATE SET revision=excluded.revision,state=excluded.state,last_operation_id=excluded.last_operation_id,cleanup_operation_id=excluded.cleanup_operation_id")
                    .bind(t).bind(a.agent.as_bytes().as_slice()).bind(owner).bind(host).bind(&g[..]).bind(a.expected_revision as i64 + 1).bind(&a.operation_id).bind(state).bind(&a.operation_id).bind(cleanup).execute(tx.conn()).await?;
            }
            wire::Header::Result { result } => {
                let a = &result.request;
                let row = sqlx::query("SELECT event_json,state,result_json FROM paired_agent_operations WHERE community_id=$1 AND owner_pubkey=$2 AND operation_id=$3")
                    .bind(t).bind(owner).bind(&a.operation_id).fetch_optional(tx.conn()).await?.ok_or_else(denied)?;
                let request: String = row.get("event_json");
                let (_, signed) =
                    wire::verify(request.as_bytes(), origin, scope.owner).map_err(|_| denied())?;
                require(
                    signed
                        == (wire::Header::Command {
                            authority: a.clone(),
                        }),
                )?;
                let old: Option<String> = row.get("result_json");
                if old.as_deref() == Some(raw) {
                    return tx.commit().await;
                }
                require(!matches!(
                    row.get::<&str, _>("state"),
                    "applied" | "needs-login" | "cleaned"
                ))?;
                let state = outcome(&result.outcome);
                sqlx::query("UPDATE paired_agent_operations SET state=$4,result_json=$5 WHERE community_id=$1 AND owner_pubkey=$2 AND operation_id=$3")
                    .bind(t).bind(owner).bind(&a.operation_id).bind(state).bind(raw).execute(tx.conn()).await?;
                sqlx::query("UPDATE paired_agent_placements SET state=$4 WHERE community_id=$1 AND agent_pubkey=$2 AND last_operation_id=$3")
                    .bind(t).bind(a.agent.as_bytes().as_slice()).bind(&a.operation_id).bind(state).execute(tx.conn()).await?;
            }
        }
        tx.commit().await
    }
    /// Current-authority private read. Permit issuance commits under the exact
    /// same lock as revocation: earlier permits may finish one bounded attempt.
    /// No permit survives expiry or grants another retry after revocation.
    pub async fn read_host_deployment(
        &self,
        c: CommunityId,
        reader: PublicKey,
        ceiling: Option<&[u8; 32]>,
        allow: bool,
        q: &wire::Query,
    ) -> Result<wire::Reply> {
        q.validate().map_err(|_| denied())?;
        let scope = &q.scope;
        let g = bytes(&scope.generation)?;
        if let Some(ceiling) = ceiling {
            require(*ceiling == g && reader == scope.host)?;
        } else {
            require(reader == scope.owner && q.view != "permit" && q.view != "pending")?;
        }
        let mut tx = self.paired_tx(c).await?;
        binding(&mut tx, scope, allow).await?;
        let now = wall(&mut tx).await?;
        let t = *c.as_uuid();
        let owner = scope.owner.as_bytes().as_slice();
        let host = scope.host.as_bytes().as_slice();
        let data = match q.view.as_str() {
            "status" => {
                let e: Option<String> = sqlx::query_scalar("SELECT event_json FROM paired_host_deployment_status WHERE community_id=$1 AND generation=$2 AND owner_pubkey=$3 AND host_pubkey=$4 AND expires_at>$5")
                    .bind(t).bind(&g[..]).bind(owner).bind(host).bind(now).fetch_optional(tx.conn()).await?;
                json!({"event":e})
            }
            "pending" | "live" => {
                let rows: Vec<String> = sqlx::query_scalar("SELECT o.operation_id FROM paired_agent_operations o JOIN paired_agent_placements p ON p.community_id=o.community_id AND p.agent_pubkey=o.agent_pubkey AND p.last_operation_id=o.operation_id WHERE o.community_id=$1 AND o.owner_pubkey=$2 AND o.host_pubkey=$3 AND o.generation=$4 AND o.state IN ('pending','failed','cleanup-pending') AND (o.expires_at>$5 OR o.state IN ('pending','cleanup-pending')) ORDER BY o.operation_id LIMIT 33")
                    .bind(t).bind(owner).bind(host).bind(&g[..]).bind(now).fetch_all(tx.conn()).await?;
                json!({"operations":rows.iter().take(32).collect::<Vec<_>>(),"more":rows.len()>32})
            }
            "operation" | "permit" => {
                let op = q.operation_id.as_deref().ok_or_else(denied)?;
                let r = sqlx::query("SELECT o.event_json,o.result_json,o.state,o.expires_at,o.permit_nonce,p.unit_id,p.revision,p.last_operation_id,p.cleanup_operation_id FROM paired_agent_operations o JOIN paired_agent_placements p ON p.community_id=o.community_id AND p.agent_pubkey=o.agent_pubkey WHERE o.community_id=$1 AND o.owner_pubkey=$2 AND o.host_pubkey=$3 AND o.generation=$4 AND o.operation_id=$5")
                    .bind(t).bind(owner).bind(host).bind(&g[..]).bind(op).fetch_optional(tx.conn()).await?.ok_or_else(denied)?;
                if q.view == "permit" {
                    require(
                        reader == scope.host
                            && r.get::<&str, _>("last_operation_id") == op
                            && !matches!(
                                r.get::<&str, _>("state"),
                                "applied" | "needs-login" | "cleaned"
                            )
                            && r.get::<i64, _>("expires_at") > now
                            && r.get::<Option<&str>, _>("permit_nonce") != Some(&q.nonce),
                    )?;
                    let (_, h) = wire::verify(
                        r.get::<String, _>("event_json").as_bytes(),
                        &scope.community,
                        scope.owner,
                    )
                    .map_err(|_| denied())?;
                    let wire::Header::Command { authority } = h else {
                        return Err(denied());
                    };
                    require(
                        authority.action == Action::Cleanup
                            || r.get::<Option<String>, _>("cleanup_operation_id").is_none(),
                    )?;
                    sqlx::query("UPDATE paired_agent_operations SET permit_nonce=$4,permit_expires_at=$5 WHERE community_id=$1 AND owner_pubkey=$2 AND operation_id=$3")
                        .bind(t).bind(owner).bind(op).bind(&q.nonce).bind(now + wire::PERMIT_SECONDS as i64).execute(tx.conn()).await?;
                    json!({"authority":authority,"unit_id":r.get::<String,_>("unit_id"),"revision":r.get::<i64,_>("revision")})
                } else {
                    json!({"event":r.get::<String,_>("event_json"),"result":r.get::<Option<String>,_>("result_json"),"state":r.get::<String,_>("state"),"unit_id":r.get::<String,_>("unit_id"),"revision":r.get::<i64,_>("revision")})
                }
            }
            _ => return Err(denied()),
        };
        tx.commit().await?;
        Ok(wire::Reply {
            query: q.clone(),
            expires_at: now as u64 + wire::PERMIT_SECONDS,
            data,
        })
    }
}

#[cfg(test)]
#[path = "deployment_postgres_tests.rs"]
mod postgres_tests;
