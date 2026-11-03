//! Valid public correlations for the encrypted private mailbox. Possession,
//! inner bindings and host-local commit authority are verified at endpoints.
use super::*;
use sqlx::postgres::PgRow;

pub(super) async fn advance(tx: &mut AdmittedTx, b: &PgRow, p: &Packet, now: i64) -> Result<()> {
    let op = p.op.as_str();
    let t = *tx.community().as_uuid();
    let active = b.get::<&str, _>("state") == "active";
    let prev = p.prev.ok_or_else(denied)?;
    let parent = sqlx::query(
        r#"SELECT operation, request_id, expires_at, seq FROM paired_host_events
           WHERE community_id = $1 AND generation = $2 AND owner_key = $3
           AND host_key = $4 AND event_id = $5"#,
    )
    .bind(t)
    .bind(&p.generation[..])
    .bind(&p.owner[..])
    .bind(&p.host[..])
    .bind(&prev[..])
    .fetch_optional(tx.conn())
    .await?
    .ok_or_else(denied)?;
    let parent_op = parent.get::<&str, _>("operation");
    let parent_r = parent.get::<Option<Vec<u8>>, _>("request_id");
    let durable = matches!(op, "committed" | "credentials" | "result");
    let ttl = match op {
        "create" => 900,
        "committed" | "credentials" | "result" => 86400,
        _ => 60,
    };
    require(p.expiry - p.created <= ttl && (durable || now < p.expiry))?;
    if !durable {
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
    }
    match op {
        "inventory-query" | "inventory" => {
            require(active && p.request.is_none() && parent_r.is_none())?;
            if op == "inventory-query" {
                require(parent_op == "pair-ack" && prev[..] == b.get::<Vec<u8>, _>("ack_id"))?
            } else {
                require(
                    parent_op == "inventory-query"
                        && now < parent.get::<i64, _>("expires_at")
                        && p.expiry <= parent.get::<i64, _>("expires_at"),
                )?
            }
        }
        "create" => {
            let r = p.request.ok_or_else(denied)?;
            require(
                active
                    && parent_op == "inventory"
                    && parent_r.is_none()
                    && now < parent.get::<i64, _>("expires_at"),
            )?;
            let room: bool = sqlx::query_scalar(
                r#"SELECT count(*)<512 AND count(*) FILTER(WHERE terminal_id IS NULL)<8
                   AND count(*) FILTER(WHERE request_id = $5) = 0
                   FROM paired_host_requests WHERE community_id = $1
                   AND generation = $2 AND owner_key = $3 AND host_key = $4"#,
            )
            .bind(t)
            .bind(&p.generation[..])
            .bind(&p.owner[..])
            .bind(&p.host[..])
            .bind(&r[..])
            .fetch_one(tx.conn())
            .await?;
            require(room)?;
            sqlx::query(
                r#"INSERT INTO paired_host_requests (community_id, generation, owner_key, host_key, request_id, intent_id,
                   head_phase, intent_expiry) VALUES($1, $2, $3, $4, $5, $6, 'create', $7)"#,
            )
                .bind(t).bind(&p.generation[..]).bind(&p.owner[..]).bind(&p.host[..]).bind(&r[..]).bind(&p.id[..]).bind(p.expiry).execute(tx.conn()).await?;
        }
        _ => {
            let r = p.request.ok_or_else(denied)?;
            require(parent_r.as_deref() == Some(&r[..]))?;
            let req = sqlx::query(
                r#"SELECT * FROM paired_host_requests WHERE community_id = $1
                   AND generation = $2 AND owner_key = $3 AND host_key = $4
                   AND request_id = $5"#,
            )
            .bind(t)
            .bind(&p.generation[..])
            .bind(&p.owner[..])
            .bind(&p.host[..])
            .bind(&r[..])
            .fetch_optional(tx.conn())
            .await?
            .ok_or_else(denied)?;
            let id_matches =
                |col: &str| req.get::<Option<Vec<u8>>, _>(col).as_deref() == Some(&prev[..]);
            let head = req.get::<&str, _>("head_phase");
            let intent = req.get::<Vec<u8>, _>("intent_id");
            let parent_live = now < parent.get::<i64, _>("expires_at");
            let precommit = matches!(op, "challenge" | "confirm" | "key-offer" | "activate");
            if precommit {
                require(active && p.expiry <= req.get::<i64, _>("intent_expiry"))?
            }
            let column = match op {
                "challenge" if matches!(head, "create" | "challenge") && prev[..] == intent => {
                    "generation_challenge_id"
                }
                "confirm"
                    if head == "challenge"
                        && id_matches("generation_challenge_id")
                        && parent_live
                        && p.expiry <= parent.get::<i64, _>("expires_at") =>
                {
                    "confirm_id"
                }
                "key-offer"
                    if matches!(head, "confirm" | "key-offer") && id_matches("confirm_id") =>
                {
                    "offer_id"
                }
                "activate"
                    if head == "key-offer"
                        && id_matches("offer_id")
                        && parent_live
                        && p.expiry <= parent.get::<i64, _>("expires_at") =>
                {
                    "activate_id"
                }
                "committed"
                    if head == "activate"
                        && id_matches("activate_id")
                        && b.get::<Option<i64>, _>("fence_seq")
                            .is_none_or(|seq| parent.get::<i64, _>("seq") < seq) =>
                {
                    "commit_id"
                }
                "credentials" if head == "committed" && id_matches("commit_id") => "credentials_id",
                "result"
                    if head != "result"
                        && (id_matches("commit_id")
                            || (req.get::<Option<Vec<u8>>, _>("commit_id").is_none()
                                && prev[..] == intent)) =>
                {
                    "terminal_id"
                }
                "status-query" if prev[..] == intent => "",
                "status"
                    if parent_op == "status-query"
                        && parent_live
                        && p.expiry <= parent.get::<i64, _>("expires_at") =>
                {
                    ""
                }
                _ => return Err(denied()),
            };
            if !column.is_empty() {
                // The column comes only from the closed transition table above.
                let sql=format!("UPDATE paired_host_requests SET {column}=$6,head_phase=$7 WHERE community_id=$1 AND generation=$2 AND owner_key=$3 AND host_key=$4 AND request_id=$5");
                sqlx::query(sqlx::AssertSqlSafe(sql))
                    .bind(t)
                    .bind(&p.generation[..])
                    .bind(&p.owner[..])
                    .bind(&p.host[..])
                    .bind(&r[..])
                    .bind(&p.id[..])
                    .bind(op)
                    .execute(tx.conn())
                    .await?;
            }
        }
    }
    insert(tx, p, None).await?;
    Ok(())
}

async fn prune(tx: &mut AdmittedTx, p: &Packet) -> Result<()> {
    // Retain immutable enrollment, current inventory and every referenced
    // proof. Unreferenced superseded challenges and status polls age out.
    let t = *tx.community().as_uuid();
    sqlx::query(
        r#"WITH removed AS ( DELETE FROM paired_host_events e
           WHERE e.community_id = $1 AND e.generation = $2
           AND e.owner_key = $3 AND e.host_key = $4 AND e.received_at < now()-interval '7 days'
           AND e.operation IN ('inventory-query', 'inventory', 'status-query', 'status', 'challenge', 'key-offer')
           AND NOT EXISTS (SELECT 1 FROM paired_host_requests r
           WHERE r.community_id = e.community_id AND r.generation = e.generation
           AND e.event_id IN(r.generation_challenge_id, r.offer_id))
           AND NOT EXISTS (SELECT 1 FROM paired_host_events child
           WHERE child.community_id = e.community_id AND child.generation = e.generation
           AND child.prev_id = e.event_id) AND (e.operation<>'inventory'
           OR EXISTS (SELECT 1 FROM paired_host_events newer
           WHERE newer.community_id = e.community_id AND newer.generation = e.generation
           AND newer.operation = 'inventory' AND newer.seq>e.seq))
           RETURNING seq) UPDATE paired_host_bindings SET retention_floor = greatest(retention_floor, (SELECT
           max(seq) FROM removed)) WHERE community_id = $1
           AND generation = $2 AND EXISTS (SELECT 1 FROM removed)"#,
    )
        .bind(t).bind(&p.generation[..]).bind(&p.owner[..]).bind(&p.host[..]).execute(tx.conn()).await?;
    Ok(())
}
