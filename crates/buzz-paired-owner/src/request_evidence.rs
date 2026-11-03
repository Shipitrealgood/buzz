//! Verify a retained request page as a graph. The trusted enrollment and
//! original tap are local. No decision about completeness creates a signature.
use super::*;
use crate::{
    device::{verify_binding, Grant},
    history::Summary,
};
use anyhow::{bail, Context};
use buzz_core::paired_message::{open, restore_sent, Body, Message};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
pub(crate) struct Evidence {
    #[serde(flatten)]
    pub summary: Summary,
    pub messages: Vec<Packet>,
    pub unresolved: bool,
}

pub(super) fn verify<'a>(
    page: &'a Evidence,
    keys: &Keys,
    s: &Scope,
    r: &str,
    grant: &Grant,
    req: &Request,
) -> Result<Option<(&'a Packet, String)>> {
    let wall = now()?;
    let intent_id = id(&req.intent)?;
    let summary = &page.summary;
    ensure!(
        !page.unresolved && summary.request_id == r && summary.intent_id == intent_id,
        "incomplete request detail"
    );
    let ack = grant.ack.as_ref().context("no accepted enrollment")?;
    let claim_id = id(&grant.claim)?;
    let ack_id = id(ack)?;
    let mut packets = HashMap::new();
    let mut messages: HashMap<String, Message> = HashMap::new();
    for packet in &page.messages {
        let event: Event = serde_json::from_str(packet.event.get())?;
        let eid = event.id.to_hex();
        ensure!(
            event.created_at.as_secs() <= wall.saturating_add(30),
            "future evidence"
        );
        ensure!(
            packets.insert(eid.clone(), (packet, event.sig)).is_none(),
            "duplicate evidence"
        );
        if eid == claim_id || eid == ack_id {
            continue;
        }
        let msg = if event.pubkey == s.owner {
            restore_sent(keys, s, packet, &eid)?
        } else {
            // Historical verification alone has no live challenge authority.
            open(keys, s, packet, event.created_at.as_secs())?
        };
        messages.insert(eid, msg);
    }
    // An expired, unaccepted confirmation can disappear when the host replaces
    // its generation nonce. This exception requires a fresh unanswered challenge,
    // no later local phase, and a page which itself contains no confirm or later
    // phase. Every signed parent of the replacement still has to verify below.
    let superseded=req.offer.is_none() && req.activate.is_none() && req.commit.is_none() && req.result.is_none()
        && req.confirm.as_ref().is_some_and(|p|p.route.expires_at<=wall)
        && messages.values().all(|m|matches!(m.body,Body::Create{..}|Body::Challenge{..}|Body::Inventory{..}|Body::InventoryQuery{..}))
        && messages.values().any(|m|matches!(&m.body,Body::Challenge{request_id,intent_id:i,..} if request_id==r && i==&intent_id)
            && wall<m.expires_at && !req.answered.contains(&m.id));
    let check_envelope = |local: &Packet| -> Result<()> {
        let event: Event = serde_json::from_str(local.event.get())?;
        let (remote, sig) = packets
            .get(&event.id.to_hex())
            .context("missing local event")?;
        ensure!(
            remote.route == local.route && sig == &event.sig,
            "different signed envelope"
        );
        Ok(())
    };
    for local in [&grant.claim, ack, &req.intent] {
        check_envelope(local)?;
    }
    verify_binding(keys, s, packets[&claim_id].0, packets[&ack_id].0, wall)?;
    let get = |id: &str| messages.get(id).context("missing signed predecessor");
    let body = |id: &str| Ok::<_, anyhow::Error>(&get(id)?.body);
    let original = get(&intent_id)?;
    let Body::Create { channels, .. } = &original.body else {
        bail!("invalid intent root")
    };
    for (event_id, msg) in &messages {
        let packet = packets[event_id].0;
        if !matches!(
            msg.body,
            Body::InventoryQuery { .. } | Body::Inventory { .. }
        ) {
            ensure!(packet.route.r.as_deref() == Some(r), "wrong request");
        }
        // open/restore_sent checked the exact sidecar against the signed,
        // decrypted body. Resolve every predecessor without relying on page order.
        let prev = packet
            .route
            .prev
            .as_deref()
            .context("missing predecessor ID")?;
        let valid = match &msg.body {
            Body::InventoryQuery { .. } => prev == ack_id,
            Body::Inventory { nonce, .. } => {
                matches!(body(prev)?,Body::InventoryQuery{nonce:n,..} if n==nonce)
                    && msg.expires_at <= get(prev)?.expires_at
            }
            Body::Create { runtime, .. } => {
                event_id == &intent_id
                    && matches!(body(prev)?,Body::Inventory{runtimes,..} if runtimes.contains(runtime))
            }
            Body::Challenge { .. } => prev == intent_id && msg.expires_at <= original.expires_at,
            Body::Confirm {
                intent_id: i,
                phase,
                nonce,
                ..
            } => {
                i == &intent_id
                    && matches!(body(prev)?,Body::Challenge{phase:p,nonce:n,..} if p==phase && n==nonce)
                    && msg.expires_at <= get(prev)?.expires_at
            }
            Body::KeyOffer { intent_id: i, .. } => {
                i == &intent_id
                    && matches!(body(prev)?, Body::Confirm { .. })
                    && msg.expires_at <= original.expires_at
            }
            Body::Activate {
                intent_id: i,
                agent,
                nonce,
                ..
            } => {
                i == &intent_id
                    && matches!(body(prev)?,Body::KeyOffer{agent:a,nonce:n,..} if a==agent && n==nonce)
                    && msg.expires_at <= get(prev)?.expires_at
            }
            Body::Committed {
                intent_id: i,
                agent,
                offer_id,
                ..
            } => {
                i == &intent_id
                    && Some(event_id) == summary.commit_id.as_ref()
                    && matches!(body(prev)?,Body::Activate{agent:a,offer_id:o,..} if a==agent && o==offer_id)
            }
            Body::Credentials {
                intent_id: i,
                agent,
                ..
            } => i == &intent_id && matches!(body(prev)?,Body::Committed{agent:a,..} if a==agent),
            Body::Result {
                intent_id: i,
                agent,
                commit_id,
                state,
                steps,
                ..
            } => {
                crate::create::validate_terminal_steps(
                    state,
                    steps,
                    channels,
                    commit_id.is_some(),
                )?;
                i == &intent_id
                    && Some(event_id) == summary.terminal_id.as_ref()
                    && commit_id == &summary.commit_id
                    && if commit_id.is_some() {
                        matches!(body(prev)?,Body::Committed{agent:a,..} if Some(a)==agent.as_ref())
                    } else {
                        prev == intent_id
                    }
            }
            _ => false,
        };
        ensure!(valid, "invalid request predecessor");
    }
    if let Some(c) = &summary.commit_id {
        ensure!(
            matches!(body(c)?, Body::Committed { .. }),
            "missing committed receipt"
        );
    }
    let mut closure = None;
    if let Some(t) = &summary.terminal_id {
        let Body::Result {
            state,
            agent,
            commit_id,
            ..
        } = body(t)?
        else {
            bail!("missing terminal receipt")
        };
        if commit_id.is_none() {
            ensure!(
                req.commit.is_none()
                    && req.credentials.is_none()
                    && compatible_candidate(req.agent.as_deref(), agent.as_deref()),
                "precommit result conflicts with local evidence"
            );
            closure = Some((packets[t].0, state.clone()));
        }
    }

    // Peer receipts and the exact local envelopes which they reference remain
    // mandatory. Signing the current outbox item does not prove its delivery.
    for packet in [&req.offer, &req.commit, &req.credentials, &req.result]
        .into_iter()
        .flatten()
    {
        check_envelope(packet)?;
    }
    for (packet, can_be_absent) in [
        (
            &req.confirm,
            superseded || (closure.is_some() && req.offer.is_none()),
        ),
        (&req.activate, closure.is_some()),
    ] {
        if let Some(packet) = packet {
            if !can_be_absent || packets.contains_key(&id(packet)?) {
                check_envelope(packet)?;
            }
        }
    }
    Ok(closure)
}
