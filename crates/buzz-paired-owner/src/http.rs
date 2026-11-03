//! Exact-origin NIP-98 transport for the native owner. No redirects or ambient auth.
use super::*;
use anyhow::{anyhow, Context};
use base64::{engine::general_purpose::STANDARD, Engine};
use nostr::{EventBuilder, Kind, PublicKey, Tags};
use reqwest::{Client, Method, Url};
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::time::Duration;

/// Untrusted wire binding. Only `Device::restore` accepts it as enrollment.
#[allow(missing_docs)]
#[derive(Deserialize)]
pub struct Binding {
    pub generation: String,
    pub owner: String,
    pub host: String,
    pub state: String,
    pub claim: Packet,
    pub ack: Packet,
    pub tombstone: Option<Packet>,
    pub inventory: Option<Packet>,
    pub high_water: Option<String>,
}
/// One participant's scoped mailbox page.
#[allow(missing_docs)]
#[derive(Deserialize)]
pub struct Page {
    pub messages: Vec<Packet>,
    pub after: Option<String>,
}

#[derive(Debug)]
struct CursorPruned;
impl std::fmt::Display for CursorPruned {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("paired mailbox history was pruned")
    }
}
impl std::error::Error for CursorPruned {}

/// Native transport pinned to one selected canonical community and signer.
pub struct Endpoint {
    client: Client,
    pub(crate) origin: String,
    owner: PublicKey,
}
pub(crate) fn bounded_id(v: &str) -> Result<&str> {
    ensure!(
        v.len() == 64
            && v.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "invalid coordinate"
    );
    Ok(v)
}
impl Endpoint {
    /// Ordinary production TLS roots and exact origin; no settings come from invitation data.
    pub fn new(origin: &str, owner: PublicKey) -> Result<Self> {
        Self::build(origin, owner, None)
    }
    /// An additional trusted CA for the loopback staging harness.
    pub fn staging(origin: &str, owner: PublicKey, ca: reqwest::Certificate) -> Result<Self> {
        ensure!(
            Url::parse(origin)?.host_str() == Some("localhost"),
            "staging CA is loopback only"
        );
        Self::build(origin, owner, Some(ca))
    }
    fn build(origin: &str, owner: PublicKey, ca: Option<reqwest::Certificate>) -> Result<Self> {
        let url = Url::parse(origin)?;
        ensure!(
            url.scheme() == "https"
                && url.origin().ascii_serialization() == origin
                && url.domain().is_some(),
            "invalid selected community"
        );
        let mut builder = Client::builder()
            .https_only(true)
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(10));
        if let Some(ca) = ca {
            builder = builder.add_root_certificate(ca);
        }
        Ok(Self {
            client: builder.build()?,
            origin: origin.into(),
            owner,
        })
    }
    async fn decode<T: DeserializeOwned>(mut res: reqwest::Response) -> Result<T> {
        let status = res.status();
        if status == reqwest::StatusCode::GONE {
            return Err(CursorPruned.into());
        }
        ensure!(
            status.is_success(),
            "community endpoint returned HTTP {}",
            status.as_u16()
        );
        let mut bytes = Vec::new();
        while let Some(chunk) = res
            .chunk()
            .await
            .map_err(|_| anyhow!("incomplete community response"))?
        {
            ensure!(
                bytes.len() + chunk.len() <= 4 * 1024 * 1024,
                "response too large"
            );
            bytes.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&bytes).map_err(|_| anyhow!("invalid community response"))
    }
    pub(crate) async fn metadata(&self) -> Result<Value> {
        Self::decode(
            self.client
                .get(&self.origin)
                .header("Accept", "application/nostr+json")
                .send()
                .await
                .map_err(|_| anyhow!("community unavailable"))?,
        )
        .await
    }
    /// Unsupported relays omit the capability. Availability errors propagate.
    pub async fn supported(&self) -> Result<bool> {
        Ok(self.metadata().await?["paired_hosts_v1"] == true)
    }
    pub(crate) async fn send<T: DeserializeOwned>(
        &self,
        keys: &Keys,
        path: &str,
        body: Option<String>,
    ) -> Result<T> {
        ensure!(keys.public_key() == self.owner, "active signer changed");
        let url = format!("{}{path}", self.origin);
        let method = if body.is_some() {
            Method::POST
        } else {
            Method::GET
        };
        let mut tags = vec![
            vec!["u".into(), url.clone()],
            vec!["method".into(), method.to_string()],
            vec!["nonce".into(), random()?],
        ];
        if let Some(bytes) = &body {
            tags.push(vec![
                "payload".into(),
                hex::encode(Sha256::digest(bytes.as_bytes())),
            ]);
        }
        let auth = EventBuilder::new(Kind::HttpAuth, "")
            .tags(Tags::parse(tags)?)
            .sign_with_keys(keys)?;
        let mut request = self.client.request(method, url).header(
            "Authorization",
            format!("Nostr {}", STANDARD.encode(serde_json::to_vec(&auth)?)),
        );
        if let Some(b) = body {
            request = request.header("Content-Type", "application/json").body(b);
        }
        Self::decode(
            request
                .send()
                .await
                .map_err(|_| anyhow!("community unavailable"))?,
        )
        .await
    }
    /// Store the original claim at its one bootstrap endpoint.
    pub async fn claim(&self, keys: &Keys, s: &Scope, p: &Packet) -> Result<()> {
        session(s, &self.origin, keys)?;
        let g = bounded_id(&s.generation)?;
        self.send::<Value>(
            keys,
            &format!("/api/paired-hosts/v1/offers/{g}"),
            Some(serde_json::to_string(p)?),
        )
        .await?;
        Ok(())
    }
    /// Fetch exact signed enrollment, inventory and tombstone evidence.
    pub async fn binding(&self, keys: &Keys, s: &Scope) -> Result<Binding> {
        session(s, &self.origin, keys)?;
        let g = bounded_id(&s.generation)?;
        let b: Binding = self
            .send(keys, &format!("/api/paired-hosts/v1/bindings/{g}"), None)
            .await?;
        ensure!(
            b.generation == *g && b.owner == s.owner.to_hex() && b.host == s.host.to_hex(),
            "wrong binding response"
        );
        Ok(b)
    }
    /// List scoped bindings including locally revoked hosts from another owner device.
    pub async fn hosts(&self, keys: &Keys) -> Result<Vec<Binding>> {
        #[derive(Deserialize)]
        struct Hosts {
            hosts: Vec<Binding>,
            after: Option<String>,
        }
        let mut path = "/api/paired-hosts/v1/hosts?include=history".to_owned();
        let mut all = vec![];
        for _ in 0..100 {
            let page: Hosts = self.send(keys, &path, None).await?;
            all.extend(page.hosts);
            let Some(cursor) = page.after else {
                return Ok(all);
            };
            ensure!(cursor.len() <= 2048, "invalid cursor");
            let mut q = reqwest::Url::parse(&format!("{}/api/paired-hosts/v1/hosts", self.origin))?;
            q.query_pairs_mut()
                .append_pair("include", "history")
                .append_pair("after", &cursor);
            path = q.as_str()[self.origin.len()..].into();
        }
        anyhow::bail!("too many host pages")
    }
    /// Store a sealed business packet, using a new one-shot request proof.
    pub async fn post(&self, keys: &Keys, s: &Scope, p: &Packet) -> Result<()> {
        session(s, &self.origin, keys)?;
        let g = bounded_id(&s.generation)?;
        self.send::<Value>(
            keys,
            &format!("/api/paired-hosts/v1/bindings/{g}/messages"),
            Some(serde_json::to_string(p)?),
        )
        .await?;
        Ok(())
    }
    async fn evidence(&self, keys: &Keys, s: &Scope, r: &str) -> Result<(Vec<Packet>, bool)> {
        session(s, &self.origin, keys)?;
        let g = bounded_id(&s.generation)?;
        bounded_id(r)?;
        #[derive(Deserialize)]
        struct Evidence {
            request_id: String,
            messages: Vec<Packet>,
            unresolved: bool,
        }
        let e: Evidence = self
            .send(
                keys,
                &format!("/api/paired-hosts/v1/bindings/{g}/requests/{r}"),
                None,
            )
            .await?;
        ensure!(
            e.request_id == r && e.messages.len() <= 1024,
            "invalid request evidence"
        );
        Ok((e.messages, e.unresolved))
    }
    pub(crate) async fn query(&self, keys: &Keys, mut filter: Value) -> Result<Vec<Event>> {
        filter["consistency"] = json!("strong");
        let events: Vec<Event> = self
            .send(keys, "/query", Some(json!([filter]).to_string()))
            .await?;
        for event in &events {
            event.verify()?;
        }
        Ok(events)
    }
    async fn unused(&self, keys: &Keys, pk: &str) -> Result<bool> {
        Ok(self
            .query(keys, json!({"kinds":[0,1,9,10100],"authors":[pk],"limit":1}))
            .await?
            .is_empty()
            && self
                .query(keys, json!({"kinds":[30177],"#d":[pk],"limit":1}))
                .await?
                .is_empty())
    }

    pub(crate) async fn channel_head(
        &self,
        keys: &Keys,
        relay: &str,
        kind: u16,
        channel: &str,
    ) -> Result<Event> {
        let mut events = self
            .query(
                keys,
                json!({"kinds":[kind],"authors":[relay],"#d":[channel],"limit":1}),
            )
            .await?;
        ensure!(
            events.len() == 1,
            "channel evidence unavailable for {channel}"
        );
        let head = events.remove(0);
        ensure!(
            head.kind.as_u16() == kind
                && head.pubkey.to_hex() == relay
                && head.tags.iter().any(|t| t.as_slice() == ["d", channel]),
            "invalid channel head"
        );
        Ok(head)
    }

    /// Validate requested memberships against the selected community's signed
    /// metadata and roster, then persist the exact tap. The relay rechecks on write.
    pub async fn start(
        &self,
        device: &mut Device,
        signer: &impl Fn() -> Result<Keys>,
        s: &Scope,
        catalog_id: &str,
        draft: crate::Draft,
    ) -> Result<String> {
        use buzz_core::channel::{decide_member_put, ChannelJoinRole, MemberPut, MemberRole};
        session(s, &self.origin, &signer()?)?;
        ensure!(draft.channels.len() <= 8, "too many initial channels");
        if !draft.channels.is_empty() {
            let meta = self.metadata().await?;
            let relay = PublicKey::from_hex(
                meta["self"]
                    .as_str()
                    .context("community signing identity missing")?,
            )?
            .to_hex();
            let owner = self.owner.to_hex();
            for channel in &draft.channels {
                ensure!(
                    uuid::Uuid::parse_str(channel)?.to_string() == *channel,
                    "invalid channel ID"
                );
                let head = self
                    .channel_head(&signer()?, &relay, 39000, channel)
                    .await?;
                let roster = self
                    .channel_head(&signer()?, &relay, 39002, channel)
                    .await?;
                ensure!(
                    !head
                        .tags
                        .iter()
                        .any(|t| t.as_slice() == ["archived", "true"]),
                    "archived channel {channel}"
                );
                let private = !head.tags.iter().any(|t| t.as_slice() == ["public"]);
                let join_role = head
                    .tags
                    .iter()
                    .find_map(|t| match t.as_slice() {
                        [k, v] if k == "join_role" => Some(ChannelJoinRole::parse_fail_closed(v)),
                        _ => None,
                    })
                    .unwrap_or(ChannelJoinRole::Guest);
                let actor_role = roster
                    .tags
                    .iter()
                    .find_map(|t| match t.as_slice() {
                        [p, k, _, role] if p == "p" && k == &owner => {
                            Some(role.parse::<MemberRole>())
                        }
                        _ => None,
                    })
                    .transpose()
                    .map_err(|_| anyhow!("unknown channel role"))?;
                ensure!(
                    decide_member_put(MemberPut {
                        private,
                        join_role,
                        actor_role,
                        target_is_actor: false,
                        target_role: None,
                        requested: Some(MemberRole::Bot)
                    })
                    .is_ok(),
                    "cannot add an agent to {channel}"
                );
            }
        }
        let latest = self.binding(&signer()?, s).await?;
        device.restore(&self.origin, &signer()?, &latest)?;
        Ok(device
            .create(&self.origin, &signer()?, s, catalog_id, draft)?
            .0)
    }

    /// Advance one pending tap using fresh signer checks at each signing boundary.
    /// Owner code receives no authority from a relay phase or an absent response.
    pub async fn reconcile(
        &self,
        device: &mut Device,
        signer: &impl Fn() -> Result<Keys>,
        s: &Scope,
        r: &str,
        known: &impl Fn(&PublicKey) -> bool,
    ) -> Result<RequestView> {
        let binding = self.binding(&signer()?, s).await?;
        device.restore(&self.origin, &signer()?, &binding)?;
        // Re-submit the same persisted artifact after a lost response. Look at
        // retained history even if submission failed: a fence or later phase may have won.
        let mut send_error = None;
        let prior = device.outgoing(&self.origin, &signer()?, s, r)?;
        if let Some(p) = &prior {
            if let Err(e) = self.post(&signer()?, s, p).await {
                send_error = Some(e);
            }
        }
        let high_water = binding
            .high_water
            .as_deref()
            .context("missing scoped high-water cursor")?;
        ensure!(high_water.len() <= 1024, "invalid cursor");
        let (mut history, unresolved) = self.evidence(&signer()?, s, r).await?;
        if let Some(q) = device.status_query(&self.origin, &signer()?, s, r, high_water)? {
            self.post(&signer()?, s, &q).await?;
        }
        // Status traffic is transient and is deliberately excluded from the
        // immutable request detail. Read forward from the snapshot captured
        // before asking, preserving updates that raced the history read.
        let key = (crate::device::coordinate(s)?, r.to_owned());
        let mut next_cursor = None;
        if let Some(query) = device.status.get(&key).filter(|q| !q.consumed) {
            let mut url = Url::parse(&format!(
                "{}/api/paired-hosts/v1/bindings/{}/messages",
                self.origin,
                bounded_id(&s.generation)?
            ))?;
            url.query_pairs_mut().append_pair("after", &query.after);
            match self
                .send::<Page>(&signer()?, &url.as_str()[self.origin.len()..], None)
                .await
            {
                Ok(page) => {
                    ensure!(page.messages.len() <= 100, "oversized mailbox page");
                    history.extend(
                        page.messages
                            .into_iter()
                            .filter(|p| p.route.r.as_deref() == Some(r)),
                    );
                    if let Some(next) = page.after {
                        ensure!(next.len() <= 1024, "invalid mailbox cursor");
                        next_cursor = Some(next);
                    }
                }
                Err(e) if e.is::<CursorPruned>() => {
                    device.status.remove(&key);
                }
                Err(e) => return Err(e),
            }
        }
        let now = now()?;
        for p in history {
            let Ok(msg) = buzz_core::paired_message::open(&signer()?, s, &p, now) else {
                continue;
            };
            if p.route.r.as_deref() != Some(r) {
                continue;
            }
            if matches!(&msg.body, buzz_core::paired_message::Body::Status { .. }) {
                device.status_response(&self.origin, &signer()?, s, r, &p)?;
                continue;
            }
            let before = device.request(&self.origin, &signer()?, s, r)?;
            if let (Some(pinned), buzz_core::paired_message::Body::KeyOffer { agent, .. }) =
                (&before.agent, &msg.body)
            {
                ensure!(pinned == agent, "host changed the pinned candidate");
            }
            let candidate = match &msg.body {
                buzz_core::paired_message::Body::KeyOffer { agent, .. }
                    if before.agent.is_none() =>
                {
                    let pk = PublicKey::from_hex(agent)?;
                    ensure!(
                        !known(&pk) && self.unused(&signer()?, agent).await?,
                        "candidate identity is already known"
                    );
                    Some(pk)
                }
                _ => None,
            };
            // Only reprocess the next eligible phase, or immutable receipts for conflict checking.
            use buzz_core::paired_message::Body;
            let relevant = match msg.body {
                Body::Challenge { .. } | Body::KeyOffer { .. } => before.state == "pending",
                Body::Committed { .. } | Body::Result { .. } => true,
                _ => false,
            };
            if relevant {
                device.receive(&self.origin, &signer()?, s, r, &p, candidate.as_ref())?;
            }
        }
        if let (Some(next), Some(query)) = (next_cursor, device.status.get_mut(&key)) {
            query.after = next;
        }
        if let Some(packet) = device.outgoing(&self.origin, &signer()?, s, r)? {
            if prior.as_ref().map(id).transpose()?.as_ref() != Some(&id(&packet)?) {
                self.post(&signer()?, s, &packet).await?;
            } else if let Some(e) = send_error {
                return Err(e);
            }
        }
        if let Some(event) = device.public_next(&self.origin, &signer()?, s, r)? {
            self.publish_verify(signer, &event).await?;
            device.public_confirm(&self.origin, &signer()?, s, r, &event)?;
        }
        let view = device.request(&self.origin, &signer()?, s, r)?;
        if view.state == "verifying" {
            self.verify_ready(device, signer, s, r).await?;
            return device.request(&self.origin, &signer()?, s, r);
        }
        if unresolved
            && !matches!(
                view.state.as_str(),
                "ready" | "failed" | "expired" | "revoked-before-commit"
            )
        {
            return Ok(RequestView {
                state: "unresolved".into(),
                ..view
            });
        }
        Ok(view)
    }

    async fn publish_verify(
        &self,
        signer: &impl Fn() -> Result<Keys>,
        event: &Event,
    ) -> Result<()> {
        ensure!(event.pubkey == self.owner, "wrong artifact signer");
        let tag = |k: &str| {
            event
                .tags
                .iter()
                .find_map(|t| match t.as_slice() {
                    [n, v] if n == k => Some(v.as_str()),
                    _ => None,
                })
                .context("invalid frozen public artifact")
        };
        let (kind, author, coord) = if event.kind == Kind::Custom(30177) {
            (30177, self.owner.to_hex(), tag("d")?)
        } else {
            ensure!(event.kind.as_u16() == 9000, "unexpected public artifact");
            let meta = self.metadata().await?;
            (
                39002,
                PublicKey::from_hex(
                    meta["self"]
                        .as_str()
                        .context("relay signing identity missing")?,
                )?
                .to_hex(),
                tag("h")?,
            )
        };
        let receipt: Value = self
            .send(&signer()?, "/events", Some(serde_json::to_string(event)?))
            .await?;
        ensure!(
            receipt["accepted"] == true
                && receipt["event_id"].as_str() == Some(event.id.to_hex().as_str()),
            "public event denied for {coord}"
        );
        let events = self
            .query(
                &signer()?,
                json!({"kinds":[kind],"authors":[author],"#d":[coord],"limit":1}),
            )
            .await?;
        let head = events.first().context("public read-back pending")?;
        ensure!(
            head.kind.as_u16() == kind
                && head.pubkey.to_hex() == author
                && head.tags.iter().any(|t| t.as_slice() == ["d", coord]),
            "read-back coordinate mismatch"
        );
        if kind == 30177 {
            ensure!(head == event, "registration superseded");
        } else {
            let target = tag("p")?;
            ensure!(
                head.tags
                    .iter()
                    .any(|t| t.as_slice().first().is_some_and(|v| v == "p")
                        && t.as_slice().get(1).is_some_and(|v| v == target)
                        && t.as_slice().get(3).is_some_and(|v| v == "bot")),
                "membership unconfirmed for {coord}"
            );
        }
        Ok(())
    }
}

impl Endpoint {
    /// Issue a new query; wait only for a matching authenticated response.
    /// A client switch or signer lock aborts before further signing.
    pub async fn refresh(
        &self,
        device: &mut Device,
        signer: &impl Fn() -> Result<Keys>,
        s: &Scope,
    ) -> Result<HostView> {
        let b = self.binding(&signer()?, s).await?;
        device.restore(&self.origin, &signer()?, &b)?;
        let query = device.inventory_query(&self.origin, &signer()?, s)?;
        self.post(&signer()?, s, &query).await?;
        for _ in 0..50 {
            let b = self.binding(&signer()?, s).await?;
            device.restore(&self.origin, &signer()?, &b)?;
            let view = device.view(&self.origin, &signer()?, s)?;
            ensure!(view.state == "paired", "host grant revoked");
            if let Some(p) = b.inventory {
                if p.route.prev.as_deref() == Some(id(&query)?.as_str()) {
                    device.accept_inventory(&self.origin, &signer()?, s, &p)?;
                    return device.view(&self.origin, &signer()?, s);
                }
            }
            tokio::time::sleep(Duration::from_millis(750)).await;
        }
        anyhow::bail!("host has not confirmed current inventory")
    }
}
