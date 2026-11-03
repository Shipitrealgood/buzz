//! Read-only, signed request evidence for a second authorized owner device.
//! This module has no access to the local tap journal or its signer automation.
use super::*;
use anyhow::{bail, Context};
use buzz_core::paired_message::{open, restore_sent, Body, Message};
use serde::{Deserialize, Serialize};

/// A host-reported outcome from signed historical evidence. Only the original
/// device's business journal can resume a request or verify final public heads.
#[derive(Serialize)]
pub struct HistoryView {
    /// Original tap identifier.
    pub request_id: String,
    /// Name from the original owner's signed create intent.
    pub name: String,
    /// Last verifiable state; incomplete history is explicitly unresolved.
    pub host_state: String,
    /// Agent key for which a signed activation and commit exist.
    pub agent: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct Summary {
    request_id: String,
    intent_id: String,
    commit_id: Option<String>,
    terminal_id: Option<String>,
}
impl Endpoint {
    /// Discover the owner's immutable request coordinates, one bounded relay page
    /// at a time. Call `history` to verify a selected item before displaying its outcome.
    pub async fn history_ids(
        &self,
        keys: &Keys,
        s: &Scope,
        after: Option<&str>,
    ) -> Result<(Vec<String>, Option<String>)> {
        session(s, &self.origin, keys)?;
        let g = crate::http::bounded_id(&s.generation)?;
        let mut url = reqwest::Url::parse(&format!(
            "{}/api/paired-hosts/v1/bindings/{g}/requests",
            self.origin
        ))?;
        if let Some(cursor) = after {
            ensure!(cursor.len() <= 1024, "invalid cursor");
            url.query_pairs_mut().append_pair("after", cursor);
        }
        #[derive(Deserialize)]
        struct List {
            requests: Vec<Summary>,
            after: Option<String>,
        }
        let page: List = self
            .send(keys, &url.as_str()[self.origin.len()..], None)
            .await?;
        ensure!(
            page.requests.len() <= 100 && page.after.as_ref().is_none_or(|c| c.len() <= 1024),
            "invalid history page"
        );
        let mut ids = vec![];
        for row in page.requests {
            crate::http::bounded_id(&row.request_id)?;
            ids.push(row.request_id);
        }
        Ok((ids, page.after))
    }

    /// Verify a remote journal entry without creating, importing or resuming a local tap.
    pub async fn history(&self, keys: &Keys, s: &Scope, r: &str) -> Result<HistoryView> {
        session(s, &self.origin, keys)?;
        let g = crate::http::bounded_id(&s.generation)?;
        crate::http::bounded_id(r)?;
        #[derive(Deserialize)]
        struct Record {
            #[serde(flatten)]
            summary: Summary,
            messages: Vec<Packet>,
            unresolved: bool,
        }
        let record: Record = self
            .send(
                keys,
                &format!("/api/paired-hosts/v1/bindings/{g}/requests/{r}"),
                None,
            )
            .await?;
        ensure!(
            record.summary.request_id == r && record.messages.len() <= 1024,
            "invalid request record"
        );
        inspect(
            keys,
            s,
            &record.summary,
            &record.messages,
            record.unresolved,
        )
    }
}

pub(super) fn inspect(
    keys: &Keys,
    s: &Scope,
    summary: &Summary,
    packets: &[Packet],
    unresolved: bool,
) -> Result<HistoryView> {
    let now = now()?;
    let lookup = |expected: &str| -> Result<&Packet> {
        packets
            .iter()
            .find(|p| id(p).is_ok_and(|x| x == expected))
            .context("missing historical packet")
    };
    let from_host = |expected: &str| -> Result<Message> {
        let p = lookup(expected)?;
        let event: Event = serde_json::from_str(p.event.get())?;
        ensure!(
            event.created_at.as_secs() <= now.saturating_add(30),
            "future historical packet"
        );
        open(keys, s, p, event.created_at.as_secs()).map_err(Into::into)
    };
    let i = &summary.intent_id;
    let r = &summary.request_id;
    let intent = restore_sent(keys, s, lookup(i)?, i)?;
    let Body::Create {
        request_id, name, ..
    } = intent.body
    else {
        bail!("missing signed original intent")
    };
    ensure!(&request_id == r, "wrong original intent");
    let mut view = HistoryView {
        request_id,
        name,
        host_state: "pending".into(),
        agent: None,
    };
    if unresolved {
        view.host_state = "unresolved".into();
        return Ok(view);
    }
    if let Some(commit) = &summary.commit_id {
        let Body::Committed {
            request_id,
            intent_id,
            activate_id,
            offer_id,
            agent,
        } = from_host(commit)?.body
        else {
            bail!("invalid historical commit")
        };
        ensure!(
            &request_id == r && &intent_id == i,
            "wrong historical commit"
        );
        let activated = restore_sent(keys, s, lookup(&activate_id)?, &activate_id)?;
        let Body::Activate {
            request_id,
            intent_id,
            offer_id: authorized,
            agent: authenticated,
            nonce,
        } = activated.body
        else {
            bail!("missing owner activation")
        };
        ensure!(
            &request_id == r && &intent_id == i && authorized == offer_id && authenticated == agent,
            "wrong historical activation"
        );
        let offer = from_host(&offer_id)?;
        let Body::KeyOffer {
            request_id,
            intent_id,
            agent: offered,
            nonce: challenge,
            confirm_id,
            ..
        } = offer.body
        else {
            bail!("missing candidate proof")
        };
        ensure!(
            &request_id == r
                && &intent_id == i
                && offered == agent
                && nonce == challenge
                && activated.expires_at <= offer.expires_at
                && activated.expires_at <= intent.expires_at,
            "invalid historical proof"
        );
        let confirmed = restore_sent(keys, s, lookup(&confirm_id)?, &confirm_id)?;
        let Body::Confirm {
            request_id,
            intent_id,
            challenge_id,
            phase,
            nonce,
        } = confirmed.body
        else {
            bail!("missing owner confirmation")
        };
        ensure!(
            &request_id == r
                && &intent_id == i
                && matches!(from_host(&challenge_id)?.body,Body::Challenge{request_id,intent_id,phase:original,nonce:issued} if &request_id==r && &intent_id==i && original==phase && issued==nonce),
            "wrong challenge/confirmation"
        );
        view.host_state = "committed".into();
        view.agent = Some(agent);
    }
    if let Some(terminal) = &summary.terminal_id {
        let Body::Result {
            request_id,
            intent_id,
            commit_id,
            agent,
            state,
            ..
        } = from_host(terminal)?.body
        else {
            bail!("invalid historical terminal receipt")
        };
        ensure!(
            &request_id == r
                && &intent_id == i
                && commit_id == summary.commit_id
                && (commit_id.is_none() || agent == view.agent)
                && (state != "ready" || commit_id.is_some()),
            "conflicting historical output"
        );
        view.host_state = state;
    }
    Ok(view)
}
