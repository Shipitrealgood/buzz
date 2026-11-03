//! Conversion and verification for relay-discovered agents.

use std::collections::{BTreeSet, HashMap};

use nostr::Event;

use crate::managed_agents::{agent_events::managed_agent_content_from_event, RelayAgentInfo};

use super::{agents_from_events, first_tag_value, profile_valid_oa_owner_pubkey, tags_named};

/// Collect valid agent pubkeys from kind:30177 `d` tags for follow-up relay
/// queries. Malformed tags are ignored so one hostile event cannot invalidate
/// the whole directory request.
pub fn managed_agent_pubkeys_from_events(events: &[Event]) -> std::collections::HashSet<String> {
    events
        .iter()
        .filter(|event| event.kind == nostr::Kind::Custom(30177) && event.verify().is_ok())
        .filter_map(|event| first_tag_value(event, "d"))
        .filter_map(|pubkey| nostr::PublicKey::from_hex(pubkey).ok())
        .map(|pubkey| pubkey.to_hex())
        .collect()
}

fn event_is_newer(candidate: &Event, previous: &Event) -> bool {
    candidate.created_at > previous.created_at
        || (candidate.created_at == previous.created_at && candidate.id < previous.id)
}

fn latest_runtime_events(events: &[Event]) -> HashMap<String, &Event> {
    let mut latest: HashMap<String, &Event> = HashMap::new();
    for event in events {
        let pubkey = event.pubkey.to_hex();
        if latest
            .get(&pubkey)
            .is_none_or(|previous| event_is_newer(event, previous))
        {
            latest.insert(pubkey, event);
        }
    }

    latest
}

fn relay_agent_from_runtime(
    event: &Event,
    verified_owner: Option<&String>,
) -> Option<RelayAgentInfo> {
    if event.kind != nostr::Kind::Custom(10100) || event.verify().is_err() {
        return None;
    }
    let value = agents_from_events(std::slice::from_ref(event));
    let mut agent: RelayAgentInfo =
        serde_json::from_value(value.get("agents")?.as_array()?.first()?.clone()).ok()?;
    let content: serde_json::Value = serde_json::from_str(&event.content).ok()?;
    // The generic converter supplies compatibility defaults. Discovery exposes
    // only evidence from the signed event and the separately verified profile.
    agent.status = content["status"]
        .as_str()
        .filter(|s| matches!(*s, "online" | "away" | "offline"))
        .unwrap_or("unknown")
        .to_owned();
    agent.host_pubkey = content["host"]
        .as_str()
        .and_then(|key| nostr::PublicKey::from_hex(key).ok())
        .filter(|key| key.xonly().is_ok())
        .map(|key| key.to_hex());
    agent.channel_add_policy = content["channel_add_policy"]
        .as_str()
        .filter(|value| matches!(*value, "owner_only" | "anyone" | "nobody"))
        .map(str::to_owned);
    agent.owner_pubkey = verified_owner.cloned();
    agent.channel_ids.clear();
    Some(agent)
}

/// Merge self-authored kind:10100 runtime profiles with verified Desktop-managed
/// policy records. A verified managed coordinate reserves the agent identity even
/// when its current policy is malformed, so stale legacy permissions cannot win.
pub fn relay_agents_from_directory_events(
    directory_events: &[Event],
    managed_agent_events: &[Event],
    profile_events: &[Event],
) -> Vec<RelayAgentInfo> {
    let owners = verified_agent_owners_from_profiles(profile_events);
    let verified_policies = latest_verified_managed_policies(managed_agent_events, &owners);
    let runtimes = latest_runtime_events(directory_events);
    let mut agents: HashMap<String, RelayAgentInfo> = runtimes
        .iter()
        .filter_map(|(key, event)| {
            relay_agent_from_runtime(event, owners.get(key)).map(|agent| (key.clone(), agent))
        })
        .collect();
    for (agent_pubkey, event) in verified_policies {
        // Invalid latest policy reserves the coordinate. An agent-signed
        // runtime can describe its host, availability and separate add policy,
        // but cannot restore permissions or claim channel membership.
        let runtime = agents.remove(&agent_pubkey);
        if let Some(mut agent) = relay_agent_from_managed_policy(&agent_pubkey, event) {
            if let Some(runtime) = runtime {
                agent.status = runtime.status;
                agent.host_pubkey = runtime.host_pubkey;
                agent.channel_add_policy = runtime.channel_add_policy;
            } else if !runtimes.contains_key(&agent_pubkey) {
                // Preserve owner invitations for a policy-only identity.
                // A returned but invalid runtime must never use this fallback.
                agent.channel_add_policy = Some("owner_only".to_owned());
            }
            agents.insert(agent_pubkey, agent);
        }
    }

    let mut agents: Vec<_> = agents.into_values().collect();
    agents.sort_by(|left, right| left.name.cmp(&right.name));
    agents
}

/// Resolve each agent's owner from its latest signed NIP-OA profile.
pub fn verified_agent_owners_from_profiles(events: &[Event]) -> HashMap<String, String> {
    let mut latest_profiles: HashMap<String, &Event> = HashMap::new();
    for profile in events {
        let agent_pubkey = profile.pubkey.to_hex();
        if latest_profiles
            .get(&agent_pubkey)
            .is_none_or(|previous| event_is_newer(profile, previous))
        {
            latest_profiles.insert(agent_pubkey, profile);
        }
    }
    latest_profiles
        .into_iter()
        .filter_map(|(agent_pubkey, profile)| {
            if profile.kind != nostr::Kind::Metadata || profile.verify().is_err() {
                return None;
            }
            profile_valid_oa_owner_pubkey(profile).map(|owner| (agent_pubkey, owner))
        })
        .collect()
}

fn latest_verified_managed_policies<'a>(
    managed_agent_events: &'a [Event],
    verified_owners: &HashMap<String, String>,
) -> HashMap<String, &'a Event> {
    let mut latest: HashMap<String, &'a Event> = HashMap::new();
    for event in managed_agent_events {
        let Some(agent_pubkey) = first_tag_value(event, "d") else {
            continue;
        };
        if verified_owners.get(agent_pubkey) != Some(&event.pubkey.to_hex()) {
            continue;
        }
        if latest
            .get(agent_pubkey)
            .is_none_or(|previous| event_is_newer(event, previous))
        {
            latest.insert(agent_pubkey.to_string(), event);
        }
    }
    latest
}

fn relay_agent_from_managed_policy(agent_pubkey: &str, event: &Event) -> Option<RelayAgentInfo> {
    // Check the envelope as well as the declared author. Keep invalid latest
    // coordinates reserved above so they cannot revive older legacy permissions.
    if event.kind != nostr::Kind::Custom(30177) || event.verify().is_err() {
        return None;
    }
    let content = managed_agent_content_from_event(event).ok()?;
    Some(RelayAgentInfo {
        pubkey: agent_pubkey.to_string(),
        host_pubkey: None,
        channel_add_policy: None,
        has_managed_policy: true,
        owner_pubkey: Some(event.pubkey.to_hex()),
        name: content.name,
        agent_type: "agent".to_string(),
        channels: Vec::new(),
        channel_ids: Vec::new(),
        capabilities: Vec::new(),
        // Ownership/policy proves discovery, not conversational liveness.
        status: "unknown".to_string(),
        respond_to: Some(content.respond_to),
        respond_to_allowlist: content.respond_to_allowlist,
    })
}

/// Build the relay agent directory from owner-authenticated managed-agent
/// records. A kind:30177 event is accepted only when its author matches the
/// owner cryptographically declared by the agent's latest kind:0 NIP-OA tag.
pub fn relay_agents_from_managed_agent_events(
    managed_agent_events: &[Event],
    profile_events: &[Event],
) -> Vec<RelayAgentInfo> {
    let owners = verified_agent_owners_from_profiles(profile_events);
    let mut agents: Vec<_> = latest_verified_managed_policies(managed_agent_events, &owners)
        .into_iter()
        .filter_map(|(agent_pubkey, event)| relay_agent_from_managed_policy(&agent_pubkey, event))
        .collect();
    agents.sort_by(|left, right| left.name.cmp(&right.name));
    agents
}

/// Build a pubkey-to-channel-id candidate map from relay-signed membership
/// events. Known agent identities need not have the cosmetic `bot` role;
/// otherwise only explicit bot tags seed discovery.
pub fn member_agent_channel_ids_from_events(
    events: &[Event],
    relay_pubkey: &str,
    viewer_pubkey: &str,
    known_agent_pubkeys: &std::collections::HashSet<String>,
) -> HashMap<String, Vec<String>> {
    let mut latest: HashMap<String, &Event> = HashMap::new();
    for event in events {
        if event.kind != nostr::Kind::Custom(39002)
            || !event.pubkey.to_hex().eq_ignore_ascii_case(relay_pubkey)
            || event.verify().is_err()
        {
            continue;
        }
        let Some(channel_id) = first_tag_value(event, "d") else {
            continue;
        };
        if latest
            .get(channel_id)
            .is_none_or(|previous| event_is_newer(event, previous))
        {
            latest.insert(channel_id.to_string(), event);
        }
    }
    latest.retain(|_, event| {
        tags_named(event, "p").any(|tag| {
            tag.get(1)
                .is_some_and(|key| key.eq_ignore_ascii_case(viewer_pubkey))
        })
    });
    // Once a bot role seeds an agent, its ordinary memberships in the other
    // signed, visible rosters are equally authoritative.
    let mut agents = known_agent_pubkeys.clone();
    for event in latest.values() {
        agents.extend(
            tags_named(event, "p")
                .filter(|tag| tag.get(3).map(String::as_str) == Some("bot"))
                .filter_map(|tag| tag.get(1))
                .map(|key| key.to_ascii_lowercase()),
        );
    }
    let mut channel_ids: HashMap<String, BTreeSet<String>> = HashMap::new();
    for (channel_id, event) in latest {
        for tag in tags_named(event, "p") {
            let Some(pubkey) = tag
                .get(1)
                .and_then(|key| nostr::PublicKey::from_hex(key).ok())
            else {
                continue;
            };
            let pubkey = pubkey.to_hex();
            if !agents.contains(&pubkey) {
                continue;
            }
            channel_ids
                .entry(pubkey)
                .or_default()
                .insert(channel_id.to_string());
        }
    }

    channel_ids
        .into_iter()
        .map(|(pubkey, ids)| (pubkey, ids.into_iter().collect()))
        .collect()
}
