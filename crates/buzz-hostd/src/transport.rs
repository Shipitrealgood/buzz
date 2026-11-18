use super::{Catalog, Clock, Store};
use anyhow::{anyhow, ensure, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use buzz_core::paired_message::Packet;
use nostr::{EventBuilder, Keys, Kind, Tags};
use reqwest::{Client, Method};
use rusqlite::{params, OptionalExtension};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::Duration;

#[derive(Debug)]
pub(crate) struct HttpStatus(pub u16);
impl std::fmt::Display for HttpStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "paired endpoint returned {}", self.0)
    }
}
impl std::error::Error for HttpStatus {}
#[cfg(test)]
type Network = Box<dyn Fn(&str, Option<&str>) -> Result<serde_json::Value> + Send + Sync>;
pub struct Http(Client, #[cfg(test)] Option<Network>);
impl Http {
    pub fn new() -> Result<Self> {
        Ok(Self(
            Client::builder()
                .https_only(true)
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(Duration::from_secs(10))
                .build()?,
            #[cfg(test)]
            None,
        ))
    }
    #[cfg(test)]
    pub(crate) fn fixture(f: Network) -> Self {
        Self(Client::new(), Some(f))
    }
    async fn send<T: DeserializeOwned>(
        &self,
        keys: &Keys,
        url: &str,
        body: Option<String>,
    ) -> Result<T> {
        self.authed(keys, url, body, None).await
    }
    pub(crate) async fn authed<T: DeserializeOwned>(
        &self,
        keys: &Keys,
        url: &str,
        body: Option<String>,
        oa: Option<&str>,
    ) -> Result<T> {
        #[cfg(test)]
        if let Some(net) = &self.1 {
            return Ok(serde_json::from_slice(&serde_json::to_vec(&net(
                url,
                body.as_deref(),
            )?)?)?);
        }
        let method = if body.is_some() {
            Method::POST
        } else {
            Method::GET
        };
        let mut tags = vec![
            vec!["u".to_string(), url.to_string()],
            vec!["method".into(), method.to_string()],
            vec!["nonce".into(), super::store::random()?],
        ];
        if let Some(bytes) = &body {
            tags.push(vec![
                "payload".into(),
                hex::encode(Sha256::digest(bytes.as_bytes())),
            ]);
        }
        let proof = EventBuilder::new(Kind::HttpAuth, "")
            .tags(Tags::parse(tags)?)
            .sign_with_keys(keys)?;
        let mut req = self.0.request(method, url).header(
            "Authorization",
            format!("Nostr {}", STANDARD.encode(serde_json::to_vec(&proof)?)),
        );
        if let Some(oa) = oa {
            req = req.header("x-auth-tag", oa);
        }
        if let Some(body) = body {
            req = req.header("Content-Type", "application/json").body(body);
        }
        let mut res = req
            .send()
            .await
            .map_err(|_| anyhow!("paired endpoint unavailable"))?;
        if !res.status().is_success() {
            return Err(HttpStatus(res.status().as_u16()).into());
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = res.chunk().await? {
            ensure!(
                bytes.len() + chunk.len() <= 4 * 1024 * 1024,
                "oversized paired response"
            );
            bytes.extend_from_slice(&chunk);
        }
        Ok(serde_json::from_slice(&bytes)?)
    }
}

#[derive(Deserialize)]
struct Page {
    messages: Vec<Packet>,
    after: Option<String>,
}

pub async fn poll(store: &mut Store, net: &Http, cat: &Catalog, revision: &str) -> Result<()> {
    let grants = {
        let mut stmt = store.0.prepare(
            "SELECT c,g,state,cursor FROM grants WHERE state IN ('active','offered','dead') ORDER BY state='dead',rowid",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
            ))
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    let mut incomplete = false;
    let mut historical = Vec::new();
    for (c, g, state, cursor) in grants {
        if state == "dead" {
            historical.push((c, g, state, cursor));
            continue;
        }
        if grant(store, net, cat, revision, &c, &g, &state, cursor)
            .await
            .is_err()
        {
            incomplete = true;
        }
        if pending(store, net, cat, revision, &g).await.is_err() {
            incomplete = true;
        }
    }
    if !historical.is_empty() {
        let i = store.1 % historical.len();
        store.1 = store.1.wrapping_add(1);
        let (c, g, state, cursor) = historical.swap_remove(i);
        if !matches!(
            tokio::time::timeout(
                Duration::from_secs(5),
                grant(store, net, cat, revision, &c, &g, &state, cursor)
            )
            .await,
            Ok(Ok(()))
        ) {
            incomplete = true;
        }
        if !matches!(
            tokio::time::timeout(
                Duration::from_secs(5),
                pending(store, net, cat, revision, &g)
            )
            .await,
            Ok(Ok(()))
        ) {
            incomplete = true;
        }
    }
    ensure!(!incomplete, "some paired-host operations await delivery");
    Ok(())
}

async fn pending(
    store: &mut Store,
    net: &Http,
    cat: &Catalog,
    revision: &str,
    g: &str,
) -> Result<()> {
    let tx = store.tx()?;
    let r:Option<String>=tx.query_row("SELECT r FROM requests WHERE g=?1 AND (json_extract(record,'$.state')='committed' OR (json_extract(record,'$.state')='ready' AND COALESCE(json_extract(record,'$.sealed'),0)=0)) AND json_extract(record,'$.credentials') IS NOT NULL ORDER BY r<=COALESCE((SELECT r FROM work_cursor WHERE g=?1),''),r LIMIT 1",[g],|row|row.get(0)).optional()?;
    let Some(r) = r else {
        return Ok(());
    };
    tx.execute(
        "INSERT OR REPLACE INTO work_cursor(g,r) VALUES(?,?)",
        params![g, r],
    )?;
    tx.commit()?;
    tokio::time::timeout(
        Duration::from_secs(95),
        crate::onboard::reconcile(store, g, &r, net, cat, revision),
    )
    .await?
}

async fn grant(
    store: &mut Store,
    net: &Http,
    cat: &Catalog,
    revision: &str,
    c: &str,
    g: &str,
    state: &str,
    cursor: Option<String>,
) -> Result<()> {
    let keys = store.keys()?;
    let mut incomplete = false;
    let base = format!("{c}/api/paired-hosts/v1");
    let now = Clock::read()?;
    crate::request::tick(store, g, &now, revision, false)?;
    store.0.execute("UPDATE sent SET pending=0 WHERE g=? AND pending=1 AND op IN ('inventory','challenge','key-offer','status') AND json_extract(packet,'$.route.expires_at')<=?",params![g,now.wall])?;
    let out = {
        let mut stmt = store.0.prepare(
            "SELECT id,op,packet FROM sent WHERE pending=1 AND g=? ORDER BY rowid LIMIT 100",
        )?;
        let rows = stmt.query_map([&g], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    for (id, op, packet) in out {
        let url = if op == "pair-ack" {
            format!("{base}/offers/{g}/ack")
        } else {
            format!("{base}/bindings/{g}/messages")
        };
        let result = net
            .send::<serde_json::Value>(&keys, &url, Some(packet))
            .await;
        if result.is_ok() {
            store
                .0
                .execute("UPDATE sent SET pending=0 WHERE id=?", [id])?;
        } else {
            incomplete = true;
        }
    }
    if state == "offered" {
        let Ok(page) = net
            .send::<Page>(&keys, &format!("{base}/offers/{g}"), None)
            .await
        else {
            return Err(anyhow!("delivery incomplete"));
        };
        for packet in page.messages {
            let _ = store.claim(&g, &packet, &Clock::read()?);
        }
    } else {
        let mut url = reqwest::Url::parse(&format!("{base}/bindings/{g}/messages"))?;
        if let Some(cursor) = cursor {
            url.query_pairs_mut().append_pair("after", &cursor);
        }
        let page = match net.send::<Page>(&keys, url.as_str(), None).await {
            Ok(page) => page,
            Err(e) if e.downcast_ref::<HttpStatus>().is_some_and(|s| s.0 == 410) => {
                tokio::time::timeout(
                    Duration::from_secs(10),
                    resync(store, net, &keys, &base, g, cat, revision),
                )
                .await??;
                return Ok(());
            }
            Err(_) => {
                return Err(anyhow!("delivery incomplete"));
            }
        };
        let caught_up = page.messages.len() < 100;
        for packet in page.messages {
            // A hostile sender cannot pin the cursor with an invalid artifact.
            // Database/operational failures must keep this page replayable.
            if let Err(err) = store.receive(&g, &packet, cat, revision, &Clock::read()?) {
                if err.downcast_ref::<crate::store::InvalidPacket>().is_none() {
                    return Err(err);
                }
            }
        }
        if let Some(after) = page.after {
            store
                .0
                .execute("UPDATE grants SET cursor=? WHERE g=?", params![after, g])?;
        }
        if caught_up && state == "active" {
            crate::request::tick(store, g, &Clock::read()?, revision, true)?;
        }
    }
    ensure!(!incomplete, "outbox delivery incomplete");
    Ok(())
}

/// Capture high water before listing retained request chains. Repeat safely
/// across process failure and explicitly fence a generation on relay revocation.
async fn resync(
    store: &mut Store,
    net: &Http,
    keys: &Keys,
    base: &str,
    g: &str,
    cat: &Catalog,
    revision: &str,
) -> Result<()> {
    #[derive(Deserialize)]
    struct Binding {
        generation: String,
        host: String,
        owner: String,
        state: String,
        high_water: String,
        unpair: Option<Packet>,
        tombstone: Option<Packet>,
    }
    #[derive(Deserialize)]
    struct Summary {
        request_id: String,
    }
    #[derive(Deserialize)]
    struct Requests {
        requests: Vec<Summary>,
        after: Option<String>,
    }
    #[derive(Serialize, Deserialize)]
    struct Snapshot {
        high: String,
        cursor: Option<String>,
        remaining: std::collections::VecDeque<String>,
        last: bool,
        seen: std::collections::BTreeSet<String>,
    }
    let root = format!("{base}/bindings/{g}");
    let old: Option<String> = store
        .0
        .query_row("SELECT state FROM resync WHERE g=?", [g], |r| r.get(0))
        .optional()?;
    let mut snap: Snapshot = if let Some(json) = old {
        serde_json::from_str(&json)?
    } else {
        let b: Binding = net.send(keys, &root, None).await?;
        let owner: String =
            store
                .0
                .query_row("SELECT owner FROM grants WHERE g=?", [g], |row| row.get(0))?;
        ensure!(
            b.generation == g && b.host == keys.public_key().to_hex() && b.owner == owner,
            "resync scope mismatch"
        );
        if let Some(unpair) = b.unpair {
            store.receive(g, &unpair, cat, revision, &Clock::read()?)?;
        }
        if !b.state.eq_ignore_ascii_case("active") {
            // The relay may deny service but cannot produce a canonical tombstone for the host.
            let local: String =
                store
                    .0
                    .query_row("SELECT state FROM grants WHERE g=?", [g], |row| row.get(0))?;
            ensure!(
                local == "dead",
                "unresolved remote fence; require signed unpair"
            );
            if let Some(tomb) = b.tombstone {
                let id: String =
                    store
                        .0
                        .query_row("SELECT tomb FROM grants WHERE g=?", [g], |row| row.get(0))?;
                ensure!(
                    super::store::packet_id(&tomb)? == id,
                    "inconsistent tombstone"
                );
            }
        }
        Snapshot {
            high: b.high_water,
            cursor: None,
            remaining: Default::default(),
            last: false,
            seen: Default::default(),
        }
    };
    let save = |store: &Store, s: &Snapshot| -> Result<()> {
        store.0.execute(
            "INSERT OR REPLACE INTO resync(g,state) VALUES(?,?)",
            params![g, serde_json::to_string(s)?],
        )?;
        Ok(())
    };
    if snap.remaining.is_empty() && !snap.last {
        let mut url = reqwest::Url::parse(&format!("{root}/requests"))?;
        if let Some(after) = &snap.cursor {
            url.query_pairs_mut().append_pair("after", after);
        }
        let page: Requests = net.send(keys, url.as_str(), None).await?;
        ensure!(page.requests.len() <= 100, "oversized request page");
        if let Some(after) = &page.after {
            ensure!(
                after.len() <= 512 && snap.seen.len() < 4096 && snap.seen.insert(after.clone()),
                "repeated or unbounded cursor"
            );
        }
        snap.last = page.after.is_none();
        snap.cursor = page.after;
        for summary in page.requests {
            ensure!(buzz_hostd::id(&summary.request_id), "invalid request id");
            snap.remaining.push_back(summary.request_id);
        }
        save(store, &snap)?;
    }
    for _ in 0..3 {
        let Some(r) = snap.remaining.front() else {
            break;
        };
        let detail: Page = net
            .send(keys, &format!("{root}/requests/{r}"), None)
            .await?;
        for pkt in detail.messages {
            if pkt.route.r.as_ref() == Some(r) {
                if let Err(err) = store.receive(g, &pkt, cat, revision, &Clock::read()?) {
                    if err.downcast_ref::<crate::store::InvalidPacket>().is_none() {
                        return Err(err);
                    }
                }
            }
        }
        snap.remaining.pop_front();
        save(store, &snap)?;
    }
    if snap.last && snap.remaining.is_empty() {
        let tx = store.tx()?;
        tx.execute(
            "UPDATE grants SET cursor=? WHERE g=?",
            params![snap.high, g],
        )?;
        tx.execute("DELETE FROM resync WHERE g=?", [g])?;
        tx.commit()?;
    }
    Ok(())
}
