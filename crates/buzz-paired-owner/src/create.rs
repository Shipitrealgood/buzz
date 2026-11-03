use super::{
    device::{coordinate, grant},
    *,
};
use anyhow::{bail, Context};
use buzz_core::paired_message::{open, restore_sent, seal, Body};
use nostr::{EventBuilder, Kind, PublicKey, Tags, Timestamp};
use rusqlite::{params, Connection, TransactionBehavior};
use serde::{Deserialize, Serialize};

/// All inputs from the single initiating owner's tap.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Draft {
    /// Catalog runtime ID.
    pub runtime: String,
    /// Agent display name.
    pub name: String,
    /// Explicit response and add policies.
    pub policy: Policy,
    /// Exact requested canonical channel UUIDs.
    pub channels: Vec<String>,
}

/// Status suitable for UI display, scoped to the device's original request.
#[derive(Clone, Serialize, Deserialize)]
pub struct RequestView {
    /// Tap identifier.
    pub request_id: String,
    /// Immutable display name and selected catalog runtime.
    pub name: String,
    /// Runtime from the initiating tap.
    pub runtime: String,
    /// Response policy used by the native build gate on resume.
    pub policy: Policy,
    /// Authorized channels, exactly as selected.
    pub channels: Vec<String>,
    /// Authenticated phase, or unresolved after loss of delivery.
    pub state: String,
    /// The relay has not returned complete signed evidence for the local request.
    #[serde(default)]
    pub evidence_incomplete: bool,
    /// Last local reconciliation error, separate from authenticated host state.
    pub issue: Option<String>,
    /// Pinned candidate agent key, once verified.
    pub agent: Option<String>,
    /// Host-reported step codes, when a terminal receipt exists.
    pub steps: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct Request {
    draft: Draft,
    intent: Packet,
    confirm: Option<Packet>,
    #[serde(default)]
    answered: Vec<String>,
    offer: Option<Packet>,
    activate: Option<Packet>,
    commit: Option<Packet>,
    credentials: Option<Packet>,
    agent: Option<String>,
    result: Option<Packet>,
    #[serde(default)]
    terminal_state: Option<String>,
    public: Vec<Event>,
    published: usize,
    #[serde(default)]
    evidence_incomplete: bool,
    #[serde(default)]
    issue: Option<String>,
    #[serde(default)]
    verified_at: Option<u64>,
}
fn read(db: &Connection, s: &Scope, r: &str) -> Result<Request> {
    let raw: String = db.query_row(
        "SELECT data FROM requests WHERE scope=? AND r=?",
        params![coordinate(s)?, r],
        |row| row.get(0),
    )?;
    Ok(serde_json::from_str(&raw)?)
}
fn save(db: &Connection, s: &Scope, r: &str, v: &Request) -> Result<()> {
    db.execute(
        "UPDATE requests SET data=? WHERE scope=? AND r=?",
        params![serde_json::to_string(v)?, coordinate(s)?, r],
    )?;
    Ok(())
}
impl Device {
    /// Persist an immutable intent and original tap before dispatch. The UI should
    /// submit the exact catalog ID it displayed, even if the inventory was refreshed.
    pub fn create(
        &mut self,
        selected: &str,
        keys: &Keys,
        s: &Scope,
        catalog_id: &str,
        draft: Draft,
    ) -> Result<(String, Packet)> {
        self.create_once(selected, keys, s, catalog_id, &random()?, draft)
    }

    /// Return only the exact persisted tap on duplicate IPC delivery.
    pub(crate) fn saved_tap(
        db: &Connection,
        keys: &Keys,
        s: &Scope,
        r: &str,
        catalog_id: &str,
        draft: &Draft,
    ) -> Result<Option<Packet>> {
        use rusqlite::OptionalExtension;
        crate::http::bounded_id(r)?;
        let saved: Option<(String, String)> = db
            .query_row("SELECT scope,data FROM requests WHERE r=?", [r], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .optional()?;
        let Some((scope, data)) = saved else {
            return Ok(None);
        };
        ensure!(
            scope == coordinate(s)?,
            "tap belongs to another host or community"
        );
        let req: Request = serde_json::from_str(&data)?;
        let signed = restore_sent(keys, s, &req.intent, &id(&req.intent)?)?;
        ensure!(
            &req.draft == draft
                && matches!(signed.body,Body::Create{request_id,catalog_id:ref original,..} if request_id==r && original==catalog_id),
            "tap inputs changed"
        );
        Ok(Some(req.intent))
    }

    pub(crate) fn create_once(
        &mut self,
        selected: &str,
        keys: &Keys,
        s: &Scope,
        catalog_id: &str,
        r: &str,
        draft: Draft,
    ) -> Result<(String, Packet)> {
        session(s, selected, keys)?;
        let now = now()?;
        self.check_time(now)?;
        if let Some(packet) = Self::saved_tap(&self.db, keys, s, r, catalog_id, &draft)? {
            return Ok((r.into(), packet));
        }
        ensure!(catalog_id == self.catalog(s, now)?, "catalog changed");
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(packet) = Self::saved_tap(&tx, keys, s, r, catalog_id, &draft)? {
            return Ok((r.into(), packet));
        }
        let g = grant(&tx, s)?;
        ensure!(
            g.active()
                && g.inventory.as_ref().map(id).transpose()?.as_deref() == Some(catalog_id)
                && g.runtimes.contains(&draft.runtime),
            "runtime not offered"
        );
        let pending:i64=tx.query_row("SELECT count(*) FROM requests WHERE scope=? AND json_extract(data,'$.result') IS NULL
            AND (json_extract(data,'$.intent.route.expires_at')>? OR json_extract(data,'$.commit') IS NOT NULL)",params![coordinate(s)?,now],|row|row.get(0))?;
        ensure!(pending < 8, "eight requests already pending on this host");
        let r = r.to_owned();
        let intent = seal(
            keys,
            s,
            Body::Create {
                request_id: r.clone(),
                catalog_id: catalog_id.into(),
                runtime: draft.runtime.clone(),
                name: draft.name.clone(),
                policy: draft.policy.clone(),
                channels: draft.channels.clone(),
            },
            now,
            now + 900,
        )?;
        let req = Request {
            draft,
            intent: intent.clone(),
            confirm: None,
            answered: Vec::new(),
            offer: None,
            activate: None,
            commit: None,
            credentials: None,
            agent: None,
            result: None,
            terminal_state: None,
            public: vec![],
            published: 0,
            evidence_incomplete: false,
            issue: None,
            verified_at: None,
        };
        tx.execute(
            "INSERT INTO requests VALUES(?,?,?)",
            params![coordinate(s)?, r, serde_json::to_string(&req)?],
        )?;
        tx.commit()?;
        Ok((r, intent))
    }

    /// Handle authenticated peer messages for one original device-local tap.
    /// The caller must check the proposed key against known local/public identities
    /// before accepting its first offer. No owner artifacts are generated by a status.
    pub fn receive(
        &mut self,
        selected: &str,
        keys: &Keys,
        s: &Scope,
        r: &str,
        p: &Packet,
        candidate_unused: Option<&PublicKey>,
    ) -> Result<()> {
        session(s, selected, keys)?;
        let now = now()?;
        self.check_time(now)?;
        let m = open(keys, s, p, now)?;
        ensure!(p.route.r.as_deref() == Some(r), "wrong request");
        let discovered = if matches!(&m.body, Body::KeyOffer { .. } | Body::Committed { .. }) {
            self.local_keys
                .as_ref()
                .map(|reader| reader())
                .transpose()?
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        if !discovered.is_empty() {
            self.remember_identities(discovered)?;
        }
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let g = grant(&tx, s)?;
        let mut req = read(&tx, s, r)?;
        let intent = id(&req.intent)?;
        let live = g.active()
            && now < req.intent.route.expires_at
            && req.commit.is_none()
            && req.result.is_none();
        let deadline = m.expires_at.min(req.intent.route.expires_at);
        match m.body {
            Body::Challenge {
                intent_id,
                phase,
                nonce,
                ..
            } => {
                ensure!(
                    live && intent_id == intent && req.activate.is_none(),
                    "challenge outside live request"
                );
                if req.answered.contains(&m.id) {
                    return Ok(());
                }
                ensure!(req.answered.len() < 256, "challenge budget exhausted");
                req.answered.push(m.id.clone());
                req.confirm = Some(seal(
                    keys,
                    s,
                    Body::Confirm {
                        request_id: r.into(),
                        intent_id,
                        challenge_id: m.id,
                        phase,
                        nonce,
                    },
                    now,
                    deadline,
                )?);
            }
            Body::KeyOffer {
                intent_id,
                confirm_id,
                agent,
                nonce,
                ..
            } => {
                ensure!(
                    live && intent_id == intent
                        && req.confirm.as_ref().map(id).transpose()? == Some(confirm_id),
                    "offer outside confirmed request"
                );
                if let Some(a) = &req.activate {
                    ensure!(
                        a.route.prev.as_deref() == Some(&m.id),
                        "activation already issued"
                    );
                    return Ok(());
                }
                ensure!(
                    candidate_unused.map(|k| k.to_hex()).as_deref() == Some(agent.as_str()),
                    "candidate identity unavailable or pre-existing"
                );
                candidate_available(&tx, s, r, &agent)?;
                // open() checked the exact PoP transcript, owner/host exclusion and current expiry.
                req.agent = Some(agent.clone());
                req.offer = Some(p.clone());
                req.activate = Some(seal(
                    keys,
                    s,
                    Body::Activate {
                        request_id: r.into(),
                        intent_id,
                        offer_id: m.id,
                        agent,
                        nonce,
                    },
                    now,
                    deadline,
                )?);
            }
            Body::Committed {
                intent_id,
                activate_id,
                offer_id,
                agent,
                ..
            } => {
                if let Some(commit) = &req.commit {
                    ensure!(id(commit)? == m.id, "conflicting commits");
                    return Ok(());
                }
                ensure!(
                    req.result.is_none()
                        && intent_id == intent
                        && req.agent.as_deref() == Some(agent.as_str()),
                    "commit outside original tap"
                );
                let activate = req.activate.as_ref().context("missing local activation")?;
                let offer = req.offer.as_ref().context("missing candidate proof")?;
                ensure!(
                    id(activate)? == activate_id
                        && id(offer)? == offer_id
                        && activate.route.prev == Some(offer_id)
                        && activate.route.r.as_deref() == Some(r),
                    "commit does not match persisted activation"
                );
                let frozen = restore_sent(keys, s, activate, &activate_id)?;
                let at: Event = serde_json::from_str(offer.event.get())?;
                // The offer is historical evidence, never a fresh authorization.
                let proof = open(keys, s, offer, at.created_at.as_secs())?;
                let confirm_packet = req.confirm.as_ref().context("missing confirmation")?;
                let confirm = id(confirm_packet)?;
                let confirmed = restore_sent(keys, s, confirm_packet, &confirm)?;
                ensure!(
                    matches!(&confirmed.body,Body::Confirm{request_id,intent_id,..} if request_id==r && intent_id==&intent),
                    "confirmation changed"
                );
                let Body::KeyOffer {
                    request_id: offer_request,
                    intent_id: offer_intent,
                    confirm_id,
                    agent: offer_agent,
                    nonce,
                    ..
                } = proof.body
                else {
                    bail!("missing proof")
                };
                ensure!(
                    offer_request == r
                        && offer_intent == intent
                        && confirm_id == confirm
                        && offer_agent == agent,
                    "candidate evidence mismatch"
                );
                ensure!(
                    matches!(&frozen.body,Body::Activate{request_id,intent_id,offer_id,agent:activated,nonce:answer} if request_id==r && intent_id==&intent && offer_id==&proof.id && activated==&agent && answer==&nonce)
                        && frozen.created_at < proof.expires_at
                        && frozen.expires_at <= proof.expires_at
                        && frozen.expires_at <= req.intent.route.expires_at,
                    "activation evidence mismatch"
                );
                let original = restore_sent(keys, s, &req.intent, &intent)?;
                let Body::Create {
                    request_id,
                    runtime,
                    name,
                    policy,
                    channels,
                    ..
                } = original.body
                else {
                    bail!("missing original intent")
                };
                let signed_draft = Draft {
                    runtime,
                    name,
                    policy,
                    channels,
                };
                ensure!(
                    request_id == r && signed_draft == req.draft,
                    "original tap changed"
                );
                candidate_available(&tx, s, r, &agent)?;
                let (oa, registration, events) = credentials(keys, &agent, &signed_draft, now)?;
                req.credentials = Some(seal(
                    keys,
                    s,
                    Body::Credentials {
                        request_id: r.into(),
                        intent_id,
                        commit_id: m.id,
                        agent,
                        oa,
                        registration,
                    },
                    now,
                    now + 86400,
                )?);
                req.public = events;
                req.commit = Some(p.clone());
            }
            Body::Result {
                intent_id,
                commit_id,
                agent,
                state,
                steps,
                ..
            } => {
                ensure!(
                    intent_id == intent
                        && commit_id == req.commit.as_ref().map(id).transpose()?
                        && (req.agent.is_none() || req.agent == agent)
                        && (commit_id.is_none()
                            || (agent == req.agent
                                && matches!(state.as_str(), "ready" | "failed")))
                        && (state != "ready" || commit_id.is_some()),
                    "result disagrees with original tap"
                );
                validate_terminal_steps(&state, &steps, &req.draft.channels, commit_id.is_some())?;
                if let Some(result) = &req.result {
                    ensure!(id(result)? == m.id, "conflicting results");
                    return Ok(());
                }
                req.result = Some(p.clone());
                req.terminal_state = Some(state);
            }
            _ => bail!("unexpected response"),
        }
        save(&tx, s, r, &req)?;
        tx.commit()?;
        Ok(())
    }

    /// Exact request-level business packet for idempotent retry; a fence or deadline stops new authority.
    pub fn outgoing(
        &self,
        selected: &str,
        keys: &Keys,
        s: &Scope,
        r: &str,
    ) -> Result<Option<Packet>> {
        session(s, selected, keys)?;
        let req = read(&self.db, s, r)?;
        if req.result.is_some() {
            return Ok(None);
        }
        if let Some(p) = req.credentials {
            return Ok(Some(p));
        }
        let p = req.activate.or(req.confirm).unwrap_or(req.intent);
        Ok((grant(&self.db, s)?.active() && now()? < p.route.expires_at).then_some(p))
    }

    /// Next frozen public artifact. The caller submits and reads the exact head/membership
    /// before acknowledging. Registration and each requested channel are individually tracked.
    pub fn public_next(
        &self,
        selected: &str,
        keys: &Keys,
        s: &Scope,
        r: &str,
    ) -> Result<Option<Event>> {
        session(s, selected, keys)?;
        let req = read(&self.db, s, r)?;
        if let Some(p) = &req.result {
            let Body::Result { state, .. } = open(keys, s, p, now()?)?.body else {
                bail!("invalid terminal evidence")
            };
            if state != "ready" {
                return Ok(None);
            }
        }
        Ok(req.public.get(req.published).cloned())
    }
    /// Idempotently checkpoint a completed community read-back.
    pub fn public_confirm(
        &mut self,
        selected: &str,
        keys: &Keys,
        s: &Scope,
        r: &str,
        event: &Event,
    ) -> Result<()> {
        session(s, selected, keys)?;
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut req = read(&tx, s, r)?;
        ensure!(
            req.public.get(req.published) == Some(event),
            "unexpected publication receipt"
        );
        req.published += 1;
        save(&tx, s, r, &req)?;
        tx.commit()?;
        Ok(())
    }
    /// Inspect only evidence associated with this device's real tap.
    pub fn request(&self, selected: &str, keys: &Keys, s: &Scope, r: &str) -> Result<RequestView> {
        session(s, selected, keys)?;
        let req = read(&self.db, s, r)?;
        let (state, steps) = if let Some(result) = req.result {
            let Body::Result { state, steps, .. } = open(keys, s, &result, now()?)?.body else {
                bail!("invalid terminal receipt")
            };
            if state == "ready" && req.published != req.public.len() {
                ("pending-publication".into(), steps)
            } else if state == "ready" && req.verified_at.is_none() {
                ("verifying".into(), steps)
            } else {
                (state, steps)
            }
        } else {
            (
                if req.commit.is_some() {
                    "committed"
                } else if req.activate.is_some() {
                    "awaiting-commit"
                } else if now()? >= req.intent.route.expires_at {
                    "expired-awaiting-host"
                } else {
                    "pending"
                }
                .into(),
                vec![],
            )
        };
        Ok(RequestView {
            request_id: r.into(),
            evidence_incomplete: req.evidence_incomplete
                && !matches!(
                    state.as_str(),
                    "ready" | "failed" | "expired" | "revoked-before-commit"
                ),
            issue: req.issue,
            name: req.draft.name,
            runtime: req.draft.runtime,
            policy: req.draft.policy,
            channels: req.draft.channels,
            state,
            agent: req.agent,
            steps,
        })
    }
    /// Checkpoint the independent history diagnostic before any subsequent network wait.
    /// Only an exact copy of the device's signed chain can clear a previous gap.
    pub(crate) fn observe_history(
        &mut self,
        selected: &str,
        keys: &Keys,
        s: &Scope,
        r: &str,
        incomplete: bool,
        history: &[Packet],
    ) -> Result<()> {
        session(s, selected, keys)?;
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut req = read(&tx, s, r)?;
        if incomplete == req.evidence_incomplete {
            return Ok(());
        }
        if !incomplete {
            for saved in [
                Some(&req.intent),
                req.confirm.as_ref(),
                req.offer.as_ref(),
                req.activate.as_ref(),
                req.commit.as_ref(),
                req.credentials.as_ref(),
                req.result.as_ref(),
            ]
            .into_iter()
            .flatten()
            {
                let local: Event = serde_json::from_str(saved.event.get())?;
                let restored = history.iter().any(|p| {
                    p.route == saved.route
                        && buzz_core::paired_host::transport::verify_outer(
                            p.event.get().as_bytes(),
                            &s.community,
                            &s.generation,
                            local.pubkey,
                        )
                        .is_ok_and(|(event, ..)| event.id == local.id)
                });
                if !restored {
                    return Ok(());
                }
            }
        }
        req.evidence_incomplete = incomplete;
        save(&tx, s, r, &req)?;
        tx.commit()?;
        Ok(())
    }

    /// Keep a local transport/reconciliation diagnostic without changing host-reported state.
    pub fn note_issue(
        &mut self,
        selected: &str,
        keys: &Keys,
        s: &Scope,
        r: &str,
        issue: Option<String>,
    ) -> Result<()> {
        session(s, selected, keys)?;
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut req = read(&tx, s, r)?;
        if req.issue != issue {
            req.issue = issue;
            save(&tx, s, r, &req)?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Enumerate this profile's original requests for a specific owner/host binding.
    pub fn requests(&self, selected: &str, keys: &Keys, s: &Scope) -> Result<Vec<RequestView>> {
        session(s, selected, keys)?;
        let mut q = self
            .db
            .prepare("SELECT r FROM requests WHERE scope=? ORDER BY
            ((json_extract(data,'$.result') IS NULL AND (json_extract(data,'$.intent.route.expires_at')>unixepoch() OR json_extract(data,'$.commit') IS NOT NULL))
              OR (json_extract(data,'$.terminal_state')='ready' AND json_extract(data,'$.verified_at') IS NULL)) DESC, rowid DESC LIMIT 40")?;
        let rows = q.query_map([coordinate(s)?], |row| row.get::<_, String>(0))?;
        rows.map(|r| self.request(selected, keys, s, &r?)).collect()
    }

    /// Request fresh recovery evidence for a known original tap. At most once per
    /// minute per request in this session. A response cannot authorize any side effect.
    pub(crate) fn status_query(
        &mut self,
        selected: &str,
        keys: &Keys,
        s: &Scope,
        r: &str,
        high_water: &str,
    ) -> Result<Option<Packet>> {
        use std::time::{Duration, Instant};
        session(s, selected, keys)?;
        let req = read(&self.db, s, r)?;
        if req.result.is_some() {
            return Ok(None);
        }
        let now = now()?;
        self.check_time(now)?;
        self.status
            .retain(|_, q| q.since.elapsed() < Duration::from_secs(60));
        let key = (coordinate(s)?, r.to_string());
        if self.status.contains_key(&key) {
            return Ok(None);
        }
        let nonce = random()?;
        let packet = seal(
            keys,
            s,
            Body::StatusQuery {
                request_id: r.into(),
                intent_id: id(&req.intent)?,
                nonce: nonce.clone(),
            },
            now,
            now + 60,
        )?;
        self.status.insert(
            key,
            crate::device::StatusQuery {
                id: id(&packet)?,
                nonce,
                since: Instant::now(),
                after: high_water.to_owned(),
                consumed: false,
            },
        );
        Ok(Some(packet))
    }

    pub(crate) fn status_response(
        &mut self,
        selected: &str,
        keys: &Keys,
        s: &Scope,
        r: &str,
        p: &Packet,
    ) -> Result<()> {
        session(s, selected, keys)?;
        let req = read(&self.db, s, r)?;
        let Some(query) = self.status.get_mut(&(coordinate(s)?, r.into())) else {
            return Ok(());
        };
        let message = open(keys, s, p, now()?)?;
        let Body::Status {
            request_id,
            intent_id,
            query_id,
            nonce,
            commit_id,
            terminal_id,
            ..
        } = message.body
        else {
            bail!("wrong status message")
        };
        if query.consumed || query.since.elapsed().as_secs() >= 60 || query_id != query.id {
            return Ok(());
        }
        ensure!(
            request_id == r && intent_id == id(&req.intent)? && nonce == query.nonce,
            "incorrect status response"
        );
        if let Some(commit) = &req.commit {
            ensure!(
                commit_id.is_none() || commit_id.as_ref() == Some(&id(commit)?),
                "host lost commit evidence"
            );
        }
        if let Some(result) = &req.result {
            ensure!(
                terminal_id.is_none() || terminal_id.as_ref() == Some(&id(result)?),
                "host lost terminal evidence"
            );
        }
        query.consumed = true;
        Ok(())
    }

    /// Current catalog event ID, usable only for this process's verified response.
    pub fn catalog_id(&self, selected: &str, keys: &Keys, s: &Scope) -> Result<&str> {
        session(s, selected, keys)?;
        self.catalog(s, now()?)
    }
}

pub(super) fn validate_terminal_steps(
    state: &str,
    steps: &[String],
    channels: &[String],
    committed: bool,
) -> Result<()> {
    ensure!(
        !committed || matches!(state, "ready" | "failed"),
        "invalid post-commit state"
    );
    ensure!(state != "ready" || committed, "ready requires commit");
    if state == "ready" {
        let mut expected = vec![
            "profile-verified".to_owned(),
            "directory-verified".into(),
            "registration-verified".into(),
            "unit-active".into(),
            "presence-confirmed".into(),
            "verified".into(),
        ];
        expected.extend(channels.iter().map(|c| format!("membership-verified:{c}")));
        ensure!(
            steps.len() == expected.len() && expected.iter().all(|step| steps.contains(step)),
            "ready receipt lacks exact provisioning evidence"
        );
    }
    Ok(())
}

fn candidate_available(db: &Connection, s: &Scope, r: &str, agent: &str) -> Result<()> {
    let known:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM known_identities WHERE key=?1)
        OR EXISTS(SELECT 1 FROM grants WHERE json_extract(scope,'$.host')=?1 OR json_extract(scope,'$.owner')=?1)
        OR EXISTS(SELECT 1 FROM requests WHERE json_extract(data,'$.agent')=?1 AND NOT(scope=?2 AND r=?3))",params![agent,coordinate(s)?,r],|row|row.get(0))?;
    ensure!(!known, "candidate is an existing identity");
    Ok(())
}

fn credentials(
    keys: &Keys,
    agent: &str,
    draft: &Draft,
    now: u64,
) -> Result<([String; 4], Event, Vec<Event>)> {
    let key = PublicKey::from_hex(agent)?;
    let oa = serde_json::from_str(&buzz_sdk::nip_oa::compute_auth_tag(keys, &key, "")?)?;
    let content = serde_json::json!({"name":draft.name,"parallelism":1,"respond_to":draft.policy.respond_to,"respond_to_allowlist":draft.policy.respond_to_allowlist});
    let tags = Tags::parse([["d", agent], ["runtime", &draft.runtime]])?;
    let registration = EventBuilder::new(Kind::Custom(30177), content.to_string())
        .tags(tags)
        .custom_created_at(Timestamp::from(now))
        .sign_with_keys(keys)?;
    let mut events = vec![registration.clone()];
    for channel in &draft.channels {
        events.push(
            buzz_sdk::build_add_member(channel.parse()?, agent, Some(buzz_sdk::MemberRole::Bot))?
                .custom_created_at(Timestamp::from(now))
                .sign_with_keys(keys)?,
        );
    }
    Ok((oa, registration, events))
}

impl Device {
    /// Original manifest for independent public verification after a terminal receipt.
    pub(crate) fn ready_manifest(&self, s: &Scope, r: &str) -> Result<(Draft, String, Event)> {
        let req = read(&self.db, s, r)?;
        ensure!(
            req.result.is_some() && req.commit.is_some(),
            "ready receipt missing"
        );
        Ok((
            req.draft,
            req.agent.context("candidate missing")?,
            req.public.first().context("registration missing")?.clone(),
        ))
    }
    pub(crate) fn record_verified_ready(
        &mut self,
        selected: &str,
        keys: &Keys,
        s: &Scope,
        r: &str,
    ) -> Result<()> {
        session(s, selected, keys)?;
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut req = read(&tx, s, r)?;
        let result = req.result.as_ref().context("ready receipt missing")?;
        ensure!(
            matches!(open(keys,s,result,now()?)?.body,Body::Result{state,..} if state=="ready")
                && req.published == req.public.len(),
            "incomplete readiness evidence"
        );
        req.verified_at = Some(now()?);
        save(&tx, s, r, &req)?;
        tx.commit()?;
        Ok(())
    }
}
