//! Community-private host enrollment. All operations use the writer, the
//! community lifecycle fence and the same restriction lock as community bans.
use super::relay_invite::{lock_admission, principal_restricted};
use crate::{AdmittedTx, Db, DbError, Result};
use buzz_core::CommunityId;
use serde_json::{json, Value};
use sqlx::Row;

mod phases;
mod reads;
pub use reads::{Fields, Page, Read};

/// An authenticated packet. All IDs represent exactly 32 decoded bytes.
/// The HTTP layer validates the signature, wire shape and sender role.
pub struct Packet {
    /// Signed NIP-01 event ID.
    pub id: [u8; 32],
    /// Console generation.
    pub generation: [u8; 32],
    /// Party in the owner role.
    pub owner: [u8; 32],
    /// Party in the host role.
    pub host: [u8; 32],
    /// Signed operation name.
    pub op: String,
    /// Public correlation field in the HTTP-authenticated sidecar.
    pub prev: Option<[u8; 32]>,
    /// Create request, when applicable.
    pub request: Option<[u8; 32]>,
    /// Signed event time.
    pub created: i64,
    /// Sidecar time.
    pub expiry: i64,
    /// Strictly parsed encrypted event.
    pub event: Box<serde_json::value::RawValue>,
    /// Exact routing fields.
    pub route: Value,
}

/// Fixed denial shared by absent and unauthorized resources.
fn denied() -> DbError {
    DbError::AccessDenied("paired host".into())
}

fn require(ok: bool) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(denied())
    }
}

impl Db {
    /// Eligibility for a rate-limit exception on a busy shared TCP peer.
    /// This grants no access. The actual read or write repeats its full
    /// admission checks inside a lifecycle-admitted transaction.
    pub async fn known_paired_principal(
        &self,
        community: CommunityId,
        signer: &[u8; 32],
        generation: Option<&[u8; 32]>,
    ) -> Result<bool> {
        Ok(sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM relay_members WHERE community_id=$1 AND pubkey=encode($2,'hex'))
                OR EXISTS (SELECT 1 FROM paired_host_bindings WHERE community_id=$1 AND host_key=$2 AND generation=$3)
                OR EXISTS (SELECT 1 FROM paired_host_events WHERE community_id=$1 AND host_key=$2 AND generation=$3
                    AND operation='pair-claim' AND expires_at >= extract(epoch FROM now())-86400)"
        ).bind(community.as_uuid()).bind(&signer[..]).bind(generation.map(|g| &g[..]))
        .fetch_one(&self.pool).await?)
    }

    async fn paired_tx(&self, community: CommunityId) -> Result<AdmittedTx> {
        let mut tx = crate::begin_community_write_transaction(
            &self.pool,
            community,
            crate::observability::WriterOperation::Authorization,
        )
        .await?;
        sqlx::query("SET LOCAL lock_timeout = '3s'")
            .execute(tx.conn())
            .await?;
        // This pilot serializes per community, including quota checks across
        // generations, and allocates cursor seq only after acquiring the lock.
        lock_admission(tx.conn(), community).await?;
        Ok(tx)
    }

    /// Commit a packet and corresponding state transition as one transaction.
    pub async fn post_paired_packet(
        &self,
        community: CommunityId,
        allowlist: bool,
        p: &Packet,
        source_bucket: &[u8; 32],
    ) -> Result<()> {
        let mut tx = self.paired_tx(community).await?;
        require(p.owner != p.host)?;
        let removed_owner_tombstone = p.op == "unpaired";
        authorize(
            &mut tx,
            &p.owner,
            &p.host,
            allowlist,
            removed_owner_tombstone,
        )
        .await?;
        let t = *community.as_uuid();
        let binding = sqlx::query(
            r#"SELECT owner_key, host_key, claim_id, ack_id, state, tombstone_id, fence_seq
               FROM paired_host_bindings WHERE community_id = $1
               AND generation = $2"#,
        )
        .bind(t)
        .bind(&p.generation[..])
        .fetch_optional(tx.conn())
        .await?;
        if let Some(ref b) = binding {
            require(
                b.get::<Vec<u8>, _>("owner_key") == p.owner
                    && b.get::<Vec<u8>, _>("host_key") == p.host,
            )?;
        }
        if let Some(old) = sqlx::query(
            r#"SELECT route_json, event_json FROM paired_host_events
               WHERE community_id = $1 AND event_id = $2 AND generation = $3
               AND owner_key = $4 AND host_key = $5"#,
        )
        .bind(t)
        .bind(&p.id[..])
        .bind(&p.generation[..])
        .bind(&p.owner[..])
        .bind(&p.host[..])
        .fetch_optional(tx.conn())
        .await?
        {
            require(
                old.get::<Value, _>("route_json") == p.route
                    && serde_json::from_str::<Value>(old.get("event_json"))?
                        == serde_json::from_str::<Value>(p.event.get())?,
            )?;
            return tx.commit().await;
        }
        let now: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch from clock_timestamp()))::bigint")
                .fetch_one(tx.conn())
                .await?;
        require(p.created >= 0 && p.created <= now + 30 && p.expiry > p.created)?;
        match p.op.as_str() {
            "pair-claim" => {
                require(
                    binding.is_none()
                        && p.request.is_none()
                        && p.prev.is_none()
                        && p.expiry > now
                        && p.expiry - p.created <= 630
                        && p.expiry <= now + 630,
                )?;
                // Only unselected, expired candidates age out. Bound claim
                // transcripts live as long as their binding, including DEAD.
                sqlx::query(
                    r#"DELETE FROM paired_host_events e WHERE community_id = $1
                       AND operation = 'pair-claim' AND expires_at < $2
                       AND NOT EXISTS (SELECT 1 FROM paired_host_bindings b
                       WHERE b.community_id = e.community_id AND b.generation = e.generation
                       AND b.claim_id = e.event_id)"#,
                )
                .bind(t)
                .bind(now - 86400)
                .execute(tx.conn())
                .await?;
                let allowed: bool = sqlx::query_scalar(
                    r#"SELECT count(*) < 4096 AND count(*) FILTER (WHERE owner_key = $2)<128
                       AND count(*) FILTER (WHERE host_key = $3)<128
                       AND count(*) FILTER (WHERE owner_key = $2 AND host_key = $3)<16
                       AND count(*) FILTER (WHERE generation = $4
                       AND owner_key = $2)<64 AND count(*) FILTER (WHERE generation = $4
                       AND host_key = $3)<64 AND count(*) FILTER (WHERE source_bucket = $5)<256
                       FROM paired_host_events WHERE community_id = $1
                       AND operation = 'pair-claim' AND expires_at >= $6"#,
                )
                .bind(t)
                .bind(&p.owner[..])
                .bind(&p.host[..])
                .bind(&p.generation[..])
                .bind(&source_bucket[..])
                .bind(now - 86400)
                .fetch_one(tx.conn())
                .await?;
                require(allowed)?;
                insert(&mut tx, p, Some(source_bucket)).await?;
            }
            "pair-ack" => {
                require(
                    binding.is_none()
                        && p.request.is_none()
                        && now <= p.expiry.saturating_add(86400)
                        && p.expiry - p.created <= 630,
                )?;
                let claim = p.prev.ok_or_else(denied)?;
                let matches: bool = sqlx::query_scalar(
                    r#"SELECT EXISTS(SELECT 1 FROM paired_host_events
                       WHERE community_id = $1 AND generation = $2 AND owner_key = $3
                       AND host_key = $4 AND event_id = $5 AND operation = 'pair-claim'
                       AND expires_at = $6 AND (event_json::jsonb->>'created_at')::bigint <= $7)"#,
                )
                .bind(t)
                .bind(&p.generation[..])
                .bind(&p.owner[..])
                .bind(&p.host[..])
                .bind(&claim[..])
                .bind(p.expiry)
                .bind(p.created + 30)
                .fetch_one(tx.conn())
                .await?;
                require(matches)?;
                let room: bool = sqlx::query_scalar(
                    r#"SELECT count(*) < 4096 AND count(*) FILTER (WHERE owner_key = $2)<512
                       AND count(*) FILTER (WHERE owner_key = $2 AND state<>'dead')<32
                       AND count(*) FILTER (WHERE host_key = $3 AND state<>'dead') = 0
                       FROM paired_host_bindings WHERE community_id = $1"#,
                )
                .bind(t)
                .bind(&p.owner[..])
                .bind(&p.host[..])
                .fetch_one(tx.conn())
                .await?;
                require(room)?;
                insert(&mut tx, p, None).await?;
                sqlx::query(
                    r#"INSERT INTO paired_host_bindings (community_id, generation, owner_key, host_key, claim_id, ack_id, state)
                       VALUES ($1, $2, $3, $4, $5, $6, 'active')"#,
                )
                    .bind(t).bind(&p.generation[..]).bind(&p.owner[..]).bind(&p.host[..]).bind(&claim[..]).bind(&p.id[..]).execute(tx.conn()).await?;
            }
            "unpair" | "unpaired" => {
                let b = binding.ok_or_else(denied)?;
                require(
                    p.request.is_none()
                        && p.prev.as_ref().map(|id| &id[..])
                            == Some(b.get::<Vec<u8>, _>("ack_id").as_slice())
                        && p.expiry - p.created <= 86400,
                )?;
                let state = b.get::<String, _>("state");
                if p.op == "unpaired" {
                    require(state != "dead")?;
                    let seq = insert(&mut tx, p, None).await?;
                    sqlx::query(
                        r#"UPDATE paired_host_bindings SET state = 'dead', fence_seq = coalesce(fence_seq, $3), tombstone_id = $4
                           WHERE community_id = $1 AND generation = $2"#,
                    )
                        .bind(t).bind(&p.generation[..]).bind(seq).bind(&p.id[..]).execute(tx.conn()).await?;
                } else if state == "active" {
                    let seq = insert(&mut tx, p, None).await?;
                    sqlx::query("UPDATE paired_host_bindings SET state='fenced',fence_seq=$3 WHERE community_id=$1 AND generation=$2")
                        .bind(t).bind(&p.generation[..]).bind(seq).execute(tx.conn()).await?;
                }
            }
            _ => phases::advance(&mut tx, &binding.ok_or_else(denied)?, p, now).await?,
        }
        tx.commit().await
    }
}

async fn authorize(
    tx: &mut AdmittedTx,
    owner: &[u8],
    host: &[u8],
    allowlist: bool,
    tombstone: bool,
) -> Result<()> {
    let c = tx.community();
    require(!principal_restricted(tx.conn(), c, &hex::encode(host)).await?)?;
    // Host-local revocation only removes authority. The caller also matches
    // the existing immutable parties and ack before recording the tombstone.
    if tombstone && !member(tx, owner).await? {
        return Ok(());
    }
    owner_access(tx, owner, allowlist).await
}

async fn member(tx: &mut AdmittedTx, owner: &[u8]) -> Result<bool> {
    let c = tx.community();
    let member: Option<i32> = sqlx::query_scalar(
        "SELECT 1 FROM relay_members WHERE community_id=$1 AND pubkey=encode($2,'hex') FOR SHARE",
    )
    .bind(c.as_uuid())
    .bind(owner)
    .fetch_optional(tx.conn())
    .await?;
    Ok(member.is_some())
}

async fn owner_access(tx: &mut AdmittedTx, owner: &[u8], allowlist: bool) -> Result<()> {
    let c = tx.community();
    require(!principal_restricted(tx.conn(), c, &hex::encode(owner)).await?)?;
    require(member(tx, owner).await?)?;
    if allowlist {
        let allowed: Option<i32> = sqlx::query_scalar(
            "SELECT 1 FROM pubkey_allowlist WHERE community_id=$1 AND pubkey=$2 FOR SHARE",
        )
        .bind(c.as_uuid())
        .bind(owner)
        .fetch_optional(tx.conn())
        .await?;
        require(allowed.is_some())?;
    }
    Ok(())
}

async fn insert(tx: &mut AdmittedTx, p: &Packet, source_bucket: Option<&[u8; 32]>) -> Result<i64> {
    let t = *tx.community().as_uuid();
    Ok(sqlx::query_scalar(
        r#"INSERT INTO paired_host_events (community_id, event_id, generation, owner_key, host_key, operation,
           request_id, prev_id, expires_at, event_json, route_json, source_bucket)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
           RETURNING seq"#,
    )
        .bind(t).bind(&p.id[..]).bind(&p.generation[..]).bind(&p.owner[..]).bind(&p.host[..]).bind(&p.op)
        .bind(p.request.as_ref().map(|v|&v[..])).bind(p.prev.as_ref().map(|v|&v[..])).bind(p.expiry).bind(p.event.get()).bind(&p.route).bind(source_bucket.map(|v|&v[..])).fetch_one(tx.conn()).await?)
}

#[cfg(test)]
mod postgres_tests;
