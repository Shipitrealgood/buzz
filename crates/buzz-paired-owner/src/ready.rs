//! Read back the agent's own identity, host/policy and all requested memberships.
//! The durable Ready receipt is historical. Presence remains a separate observation.
use super::*;
use anyhow::Context;
use buzz_core::verify_event;
use nostr::PublicKey;
use serde_json::{json, Value};

impl Endpoint {
    pub(crate) async fn verify_ready(
        &self,
        device: &mut Device,
        signer: &impl Fn() -> Result<Keys>,
        s: &Scope,
        r: &str,
    ) -> Result<()> {
        let (draft, agent, registration) = device.ready_manifest(s, r)?;
        let agent_key = PublicKey::from_hex(&agent)?;
        for kind in [0, 10100] {
            let events = self
                .query(
                    &signer()?,
                    json!({"kinds":[kind],"authors":[agent],"limit":1}),
                )
                .await?;
            verify_agent_record(&events, kind, &agent_key, s, &draft)?;
        }
        let head = self
            .channel_head(&signer()?, &s.owner.to_hex(), 30177, &agent)
            .await?;
        ensure!(head == registration, "registration superseded");
        if !draft.channels.is_empty() {
            let meta = self.metadata().await?;
            let relay =
                PublicKey::from_hex(meta["self"].as_str().context("relay signer missing")?)?
                    .to_hex();
            for channel in &draft.channels {
                let roster = self
                    .channel_head(&signer()?, &relay, 39002, channel)
                    .await?;
                ensure!(roster.tags.iter().any(|t|matches!(t.as_slice(),[kind,who,_,role] if kind=="p" && who==&agent && role=="bot")),"membership no longer authorized for {channel}");
            }
        }
        device.record_verified_ready(&s.community, &signer()?, s, r)
    }
}

fn verify_agent_record(
    events: &[Event],
    kind: u16,
    agent_key: &PublicKey,
    s: &Scope,
    draft: &Draft,
) -> Result<()> {
    let event = events.first().context("agent record not visible")?;
    ensure!(
        events.len() == 1 && event.kind.as_u16() == kind && event.pubkey == *agent_key,
        "wrong agent head"
    );
    verify_event(event)?;
    let mut proofs = event
        .tags
        .iter()
        .map(|tag| tag.as_slice())
        .filter(|a| a.first().is_some_and(|k| k == "auth"));
    let tag = proofs.next().context("agent owner missing")?;
    ensure!(
        tag.len() == 4
            && tag[2].is_empty()
            && proofs.next().is_none()
            && buzz_sdk::nip_oa::verify_auth_tag(&serde_json::to_string(tag)?, agent_key)?
                == s.owner,
        "agent owner changed"
    );
    let body: Value = serde_json::from_str(&event.content)?;
    ensure!(body["name"] == draft.name, "agent name conflict");
    if kind == 10100 {
        let allow: Vec<String> = serde_json::from_value(body["respond_to_allowlist"].clone())?;
        ensure!(
            body["host"] == s.host.to_hex()
                && body["respond_to"] == draft.policy.respond_to
                && allow == draft.policy.respond_to_allowlist
                && body["channel_add_policy"] == draft.policy.channel_add_policy,
            "agent host/policy conflict"
        );
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use nostr::{EventBuilder, Kind};
    #[test]
    fn read_back_requires_agent_signature_correct_owner_and_requested_policy() {
        let owner = Keys::generate();
        let agent = Keys::generate();
        let host = Keys::generate();
        let scope = Scope {
            community: "https://a.example".into(),
            owner: owner.public_key(),
            host: host.public_key(),
            generation: "12".repeat(32),
        };
        let draft = Draft {
            name: "Remote".into(),
            runtime: "test/echo".into(),
            channels: vec![],
            policy: Policy {
                respond_to: "owner-only".into(),
                respond_to_allowlist: vec![],
                channel_add_policy: "owner_only".into(),
            },
        };
        let record = |signer: &Keys, claimed_owner: &Keys, host: PublicKey, mode: &str| {
            let oa = buzz_sdk::nip_oa::compute_auth_tag(claimed_owner, &signer.public_key(), "")
                .unwrap();
            let tag = buzz_sdk::nip_oa::parse_auth_tag(&oa).unwrap();
            EventBuilder::new(Kind::Custom(10100),json!({"name":"Remote","host":host.to_hex(),"respond_to":mode,"respond_to_allowlist":[],"channel_add_policy":"owner_only"}).to_string()).tags([tag]).sign_with_keys(signer).unwrap()
        };
        let check =
            |e: Event| verify_agent_record(&[e], 10100, &agent.public_key(), &scope, &draft);
        assert!(check(record(&agent, &owner, scope.host, "owner-only")).is_ok());
        assert!(check(record(&agent, &owner, scope.host, "anyone")).is_err());
        assert!(check(record(&agent, &owner, scope.owner, "owner-only")).is_err());
        assert!(check(record(&agent, &host, scope.host, "owner-only")).is_err());
        assert!(check(record(&host, &owner, scope.host, "owner-only")).is_err());
    }
}
