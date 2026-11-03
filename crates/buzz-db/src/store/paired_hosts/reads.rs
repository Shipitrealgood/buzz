use super::*;
use serde_json::value::{to_raw_value, RawValue};
/// Response fields. Raw values preserve exact signed event bytes inside nested pages.
pub type Fields = std::collections::BTreeMap<String, Box<RawValue>>;

/// Private read target. The HTTP adapter validates cursor authenticity first.
pub enum Read {
    /// Candidate offers known by their console generation.
    Offers([u8; 32]),
    /// Owner's active/fenced hosts, or immutable history, after generation.
    Hosts {
        /// Include closed generations.
        history: bool,
        /// Exclusive generation key.
        after: Option<[u8; 32]>,
    },
    /// Binding and a high-water mark captured under its writer lock.
    Binding([u8; 32]),
    /// Request summaries after an exclusive request ID.
    Requests([u8; 32], Option<[u8; 32]>),
    /// Retained exact request evidence.
    Request([u8; 32], [u8; 32]),
    /// Participant mailbox, after commit sequence.
    Messages([u8; 32], i64),
}

/// Private page. Numeric positions are signed by the HTTP adapter before release.
pub struct Page {
    /// Page response.
    pub data: Fields,
    /// Exclusive next position for host lists.
    pub generation: Option<Vec<u8>>,
    /// Last delivered or snapshot sequence for scoped binding reads.
    pub seq: Option<i64>,
}

impl Db {
    /// Reauthorize the exact principal and generation on the primary at every read.
    pub async fn read_paired(
        &self,
        community: CommunityId,
        signer: &[u8; 32],
        allowlist: bool,
        read: Read,
    ) -> Result<Page> {
        let mut tx = self.paired_tx(community).await?;
        let t = *community.as_uuid();
        let mut result = Page {
            data: Fields::new(),
            generation: None,
            seq: None,
        };
        match read {
            Read::Offers(g) => {
                let rows=sqlx::query(
                    r#"SELECT e.owner_key, e.host_key, e.event_json, e.route_json
                       FROM paired_host_events e WHERE e.community_id = $1
                       AND e.generation = $2 AND e.operation = 'pair-claim'
                       AND (e.host_key = $3 OR e.owner_key = $3) AND e.expires_at >= floor(extract(epoch from now()))-86400
                       AND NOT EXISTS (SELECT 1 FROM paired_host_bindings b
                       WHERE b.community_id = e.community_id AND b.generation = e.generation
                       AND (b.state <> 'active' OR b.claim_id<>e.event_id))
                       ORDER BY e.seq LIMIT 100"#,
                )
                    .bind(t).bind(&g[..]).bind(&signer[..]).fetch_all(tx.conn()).await?;
                let mut messages = Vec::new();
                for row in rows {
                    let owner: Vec<u8> = row.get("owner_key");
                    let host: Vec<u8> = row.get("host_key");
                    match authorize(&mut tx, &owner, &host, allowlist, false).await {
                        Ok(()) => messages.push(wrapper(&row)?),
                        Err(DbError::AccessDenied(_)) => {}
                        Err(e) => return Err(e),
                    }
                }
                result
                    .data
                    .insert("messages".into(), to_raw_value(&messages)?);
            }
            Read::Hosts { history, after } => {
                owner_access(&mut tx, signer, allowlist).await?;
                let rows = sqlx::query(
                    r#"SELECT generation, host_key FROM paired_host_bindings
                       WHERE community_id = $1 AND owner_key = $2 AND ($3
                       OR state <> 'dead') AND ($4::bytea IS NULL OR generation>$4)
                       ORDER BY generation LIMIT 17"#,
                )
                .bind(t)
                .bind(&signer[..])
                .bind(history)
                .bind(after.as_ref().map(|v| &v[..]))
                .fetch_all(tx.conn())
                .await?;
                let more = rows.len() > 16;
                let mut hosts = Vec::new();
                for row in rows.into_iter().take(16) {
                    let g: Vec<u8> = row.get("generation");
                    let host: Vec<u8> = row.get("host_key");
                    if !principal_restricted(tx.conn(), community, &hex::encode(host)).await? {
                        hosts.push(binding(&mut tx, &g, signer).await?.0);
                    }
                    if more {
                        result.generation = Some(g);
                    }
                }
                result.data.insert("hosts".into(), to_raw_value(&hosts)?);
            }
            Read::Binding(g)
            | Read::Messages(g, _)
            | Read::Requests(g, _)
            | Read::Request(g, _) => {
                let b = sqlx::query(
                    r#"SELECT owner_key, host_key, retention_floor FROM paired_host_bindings
                       WHERE community_id = $1 AND generation = $2 AND (owner_key = $3
                       OR host_key = $3)"#,
                )
                .bind(t)
                .bind(&g[..])
                .bind(&signer[..])
                .fetch_optional(tx.conn())
                .await?
                .ok_or_else(denied)?;
                let owner: Vec<u8> = b.get("owner_key");
                let host: Vec<u8> = b.get("host_key");
                authorize(&mut tx, &owner, &host, allowlist, false).await?;
                if let Read::Messages(_, after) = read {
                    if after < b.get::<i64, _>("retention_floor") {
                        return Err(DbError::PairedCursorPruned);
                    }
                    let rows=sqlx::query(
                        r#"SELECT seq, event_json, route_json FROM paired_host_events e
                           JOIN paired_host_bindings b USING(community_id, generation, owner_key, host_key)
                           WHERE e.community_id = $1 AND e.generation = $2
                           AND e.owner_key = $3 AND e.host_key = $4 AND e.seq>$5
                           AND (e.operation<>'pair-claim' OR b.claim_id = e.event_id)
                           AND (b.state = 'active' OR e.operation IN ('pair-claim', 'pair-ack', 'unpair', 'unpaired', 'committed',
                           'credentials', 'result', 'status', 'status-query')
                           OR EXISTS (SELECT 1 FROM paired_host_requests r
                           WHERE r.community_id = e.community_id AND r.generation = e.generation
                           AND r.request_id = e.request_id AND r.activate_id IS NOT NULL
                           AND e.event_id IN(r.intent_id, r.generation_challenge_id, r.confirm_id, r.offer_id, r.activate_id)))
                           ORDER BY e.seq LIMIT 100"#,
                    )
                        .bind(t).bind(&g[..]).bind(&owner).bind(&host).bind(after).fetch_all(tx.conn()).await?;
                    result.seq = Some(rows.last().map_or(after, |r| r.get("seq")));
                    result.data.insert(
                        "messages".into(),
                        to_raw_value(&rows.iter().map(wrapper).collect::<Result<Vec<_>>>()?)?,
                    );
                } else if let Read::Requests(_, after) = read {
                    let rows = sqlx::query(
                        r#"SELECT request_id, intent_id, commit_id, terminal_id
                           FROM paired_host_requests WHERE community_id = $1
                           AND generation = $2 AND owner_key = $3 AND host_key = $4
                           AND ($5::bytea IS NULL OR request_id>$5) ORDER BY request_id
                           LIMIT 101"#,
                    )
                    .bind(t)
                    .bind(&g[..])
                    .bind(&owner)
                    .bind(&host)
                    .bind(after.as_ref().map(|id| &id[..]))
                    .fetch_all(tx.conn())
                    .await?;
                    if rows.len() > 100 {
                        result.generation = Some(rows[99].get("request_id"));
                    }
                    result.data.insert(
                        "requests".into(),
                        to_raw_value(&rows.iter().take(100).map(summary).collect::<Vec<_>>())?,
                    );
                } else if let Read::Request(_, r) = read {
                    let row = sqlx::query(
                        r#"SELECT * FROM paired_host_requests WHERE community_id = $1
                           AND generation = $2 AND owner_key = $3 AND host_key = $4
                           AND request_id = $5"#,
                    )
                    .bind(t)
                    .bind(&g[..])
                    .bind(&owner)
                    .bind(&host)
                    .bind(&r[..])
                    .fetch_optional(tx.conn())
                    .await?
                    .ok_or_else(denied)?;
                    result.data = serde_json::from_value(summary(&row))?;
                    let ids: Vec<Vec<u8>> = [
                        "intent_id",
                        "generation_challenge_id",
                        "confirm_id",
                        "offer_id",
                        "activate_id",
                        "commit_id",
                        "credentials_id",
                        "terminal_id",
                    ]
                    .into_iter()
                    .filter_map(|col| row.get::<Option<Vec<u8>>, _>(col))
                    .collect();
                    let messages=sqlx::query(
                        r#"WITH RECURSIVE evidence AS ( SELECT e.event_id, e.prev_id, e.seq, e.event_json, e.route_json
                           FROM paired_host_events e WHERE e.community_id = $1
                           AND e.generation = $2 AND e.owner_key = $3 AND e.host_key = $4
                           AND e.request_id = $5 AND e.event_id = ANY($6)
                           UNION SELECT p.event_id, p.prev_id, p.seq, p.event_json, p.route_json
                           FROM evidence child JOIN paired_host_events p ON p.event_id = child.prev_id
                           WHERE p.community_id = $1 AND p.generation = $2
                           AND p.owner_key = $3 AND p.host_key = $4 ) SELECT *
                           FROM evidence ORDER BY seq"#,
                    )
                        .bind(t).bind(&g[..]).bind(&owner).bind(&host).bind(&r[..]).bind(&ids).fetch_all(tx.conn()).await?;
                    let present: std::collections::HashSet<Vec<u8>> =
                        messages.iter().map(|m| m.get("event_id")).collect();
                    let missing = ids.iter().any(|id| !present.contains(id))
                        || messages
                            .iter()
                            .filter_map(|m| m.get::<Option<Vec<u8>>, _>("prev_id"))
                            .any(|id| !present.contains(&id));
                    result
                        .data
                        .insert("unresolved".into(), to_raw_value(&missing)?);
                    result.data.insert(
                        "messages".into(),
                        to_raw_value(&messages.iter().map(wrapper).collect::<Result<Vec<_>>>()?)?,
                    );
                } else {
                    let (data, seq) = binding(&mut tx, &g, signer).await?;
                    result.seq = Some(seq);
                    result.data = data;
                }
            }
        }
        tx.commit().await?;
        Ok(result)
    }
}
fn summary(row: &sqlx::postgres::PgRow) -> Value {
    let id = |name| row.get::<Option<Vec<u8>>, _>(name).map(hex::encode);
    json!({"request_id":id("request_id"),"intent_id":id("intent_id"),"commit_id":id("commit_id"),"terminal_id":id("terminal_id")})
}
#[derive(serde::Serialize)]
struct Delivery {
    event: Box<RawValue>,
    route: Value,
}
fn wrapper(row: &sqlx::postgres::PgRow) -> Result<Delivery> {
    Ok(Delivery {
        event: RawValue::from_string(row.try_get("event_json")?)?,
        route: row.try_get("route_json")?,
    })
}
async fn binding(tx: &mut AdmittedTx, g: &[u8], signer: &[u8]) -> Result<(Fields, i64)> {
    let row=sqlx::query(
        r#"SELECT b.state, b.owner_key, b.host_key, b.claim_id, b.ack_id, b.tombstone_id, (SELECT max(e.seq)
           FROM paired_host_events e WHERE e.community_id = b.community_id
           AND e.generation = b.generation AND e.owner_key = b.owner_key
           AND e.host_key = b.host_key) AS high_water FROM paired_host_bindings b
           WHERE community_id = $1 AND generation = $2 AND (owner_key = $3
           OR host_key = $3)"#,
    )
        .bind(tx.community().as_uuid()).bind(g).bind(signer).fetch_optional(tx.conn()).await?.ok_or_else(denied)?;
    let owner: Vec<u8> = row.get("owner_key");
    let host: Vec<u8> = row.get("host_key");
    let claim: Vec<u8> = row.get("claim_id");
    let ack: Vec<u8> = row.get("ack_id");
    let tombstone: Option<Vec<u8>> = row.get("tombstone_id");
    let mut data: Fields = serde_json::from_value(
        json!({"generation":hex::encode(g),"owner":hex::encode(&owner),"host":hex::encode(&host),"state":row.get::<String,_>("state"),"tombstone":null,"inventory":null,"unpair":null}),
    )?;
    let events = sqlx::query(
        r#"SELECT DISTINCT ON (operation) operation, event_json, route_json
           FROM paired_host_events WHERE community_id = $1
           AND generation = $2 AND owner_key = $3 AND host_key = $4
           AND (event_id = $5 OR event_id = $6 OR event_id = $7
           OR operation IN ('inventory', 'unpair')) ORDER BY operation, seq DESC"#,
    )
    .bind(tx.community().as_uuid())
    .bind(g)
    .bind(&owner)
    .bind(&host)
    .bind(claim)
    .bind(ack)
    .bind(&tombstone)
    .fetch_all(tx.conn())
    .await?;
    for event in events {
        let key = match event.get::<&str, _>("operation") {
            "pair-claim" => "claim",
            "pair-ack" => "ack",
            "unpaired" => "tombstone",
            "unpair" => "unpair",
            "inventory" => "inventory",
            _ => return Err(denied()),
        };
        data.insert(key.into(), to_raw_value(&wrapper(&event)?)?);
    }
    require(
        data.get("claim").is_some()
            && data.get("ack").is_some()
            && (tombstone.is_none() || data["tombstone"].get() != "null"),
    )?;
    Ok((data, row.try_get("high_water")?))
}
