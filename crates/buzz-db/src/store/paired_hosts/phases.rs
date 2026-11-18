//! Bound retained private registration traffic.
use super::*;

async fn prune(tx: &mut AdmittedTx, p: &Packet) -> Result<()> {
    // Retain immutable enrollment, current inventory and every referenced
    // proof. Unreferenced superseded challenges and status polls age out.
    let t = *tx.community().as_uuid();
    sqlx::query(
        r#"WITH removed AS ( DELETE FROM paired_host_events e
           WHERE e.community_id = $1 AND e.generation = $2
           AND e.owner_key = $3 AND e.host_key = $4 AND e.received_at < now()-
           CASE WHEN (e.event_json::jsonb->>'kind')::int=43103 THEN interval '5 minutes' ELSE interval '7 days' END
           AND e.operation IN ('inventory-query', 'inventory', 'status-query', 'status', 'models-query', 'models', 'challenge', 'key-offer')
           AND NOT EXISTS (SELECT 1 FROM paired_host_requests r
           WHERE r.community_id = e.community_id AND r.generation = e.generation
           AND e.event_id IN(r.generation_challenge_id, r.offer_id))
           AND NOT EXISTS (SELECT 1 FROM paired_host_events child
           WHERE child.community_id = e.community_id AND child.generation = e.generation
           AND child.prev_id = e.event_id) AND (e.operation<>'inventory'
           OR EXISTS (SELECT 1 FROM paired_host_events newer
           WHERE newer.community_id = e.community_id AND newer.generation = e.generation
           AND newer.operation = 'inventory' AND newer.seq>e.seq))
           AND (e.operation<>'status' OR (e.event_json::jsonb->>'kind')::int<>43103
           OR EXISTS(SELECT 1 FROM paired_host_events newer WHERE newer.community_id=e.community_id
           AND newer.generation=e.generation AND newer.operation='status' AND newer.seq>e.seq))
           RETURNING seq) UPDATE paired_host_bindings SET retention_floor = greatest(retention_floor, (SELECT
           max(seq) FROM removed)) WHERE community_id = $1
           AND generation = $2 AND EXISTS (SELECT 1 FROM removed)"#,
    )
        .bind(t).bind(&p.generation[..]).bind(&p.owner[..]).bind(&p.host[..]).execute(tx.conn()).await?;
    Ok(())
}

pub(super) async fn admit_transient(tx: &mut AdmittedTx, p: &Packet) -> Result<()> {
    let t = *tx.community().as_uuid();
    prune(tx, p).await?;
    // Bounded row and ciphertext budget. Reserve capacity for the
    // finite immutable finishing transitions of previously accepted R's.
    let room: bool = sqlx::query_scalar(
        r#"SELECT count(*) < 2048 AND coalesce(sum(pg_column_size(event_json)), 0) < 16744448
               FROM paired_host_events WHERE community_id = $1
               AND generation = $2 AND owner_key = $3 AND host_key = $4"#,
    )
    .bind(t)
    .bind(&p.generation[..])
    .bind(&p.owner[..])
    .bind(&p.host[..])
    .fetch_one(tx.conn())
    .await?;
    require(room)?;
    Ok(())
}
