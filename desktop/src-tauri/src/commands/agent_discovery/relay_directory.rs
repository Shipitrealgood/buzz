//! Relay-backed shared-agent directory discovery.

mod people;
pub use people::check_member_add_people;

use tauri::State;

use crate::{
    app_state::AppState,
    commands::identity_archive,
    managed_agents::RelayAgentInfo,
    nostr_convert,
    relay::{assert_expected_relay_scope, assert_expected_signer, query_relay_at_with_keys},
};

const RELAY_DIRECTORY_PAGE_SIZE: usize = 500;
const RELAY_FILTER_BATCH_SIZE: usize = 10;
/// Per-rebuild ceiling on directory-rebuild `/query` requests in flight at once.
/// The rebuild fans dozens of exact-author batches across the relay; issuing
/// them serially dominated agent-mention send latency (~6 s for ~100
/// candidates). A bounded window collapses that to a few round trips while
/// keeping the request rate well under the relay's admission gate, which
/// back-pressures any 429 anyway. Each rebuild builds one semaphore and shares
/// it across every phase, so a single rebuild's runtime-directory and
/// owner-profile phases — which run concurrently under one `try_join!` — never
/// exceed it together. (Overlapping rebuilds each hold their own budget.)
const RELAY_DIRECTORY_MAX_CONCURRENCY: usize = 8;

struct DirectoryRead<'a> {
    state: &'a AppState,
    target: identity_archive::RelayTarget,
    keys: nostr::Keys,
    generation: u64,
}

impl<'a> DirectoryRead<'a> {
    async fn capture(
        state: &'a AppState,
        expected_relay: Option<&str>,
        expected_signer: Option<&str>,
    ) -> Result<Self, String> {
        // The workspace transaction writes relay and keys under separate
        // mutexes. Hold its outer lock only while capturing the pair.
        let _apply = state.workspace_apply_lock.lock().await;
        let target = identity_archive::capture_relay_target(state);
        let generation = state
            .workspace_apply_generation
            .load(std::sync::atomic::Ordering::Acquire);
        let keys = state.signing_keys()?;
        assert_expected_relay_scope(expected_relay, &target.api_base_url)?;
        assert_expected_signer(expected_signer, &keys.public_key().to_hex())?;
        Ok(Self {
            state,
            target,
            keys,
            generation,
        })
    }

    async fn ensure_current(&self) -> Result<(), String> {
        let _apply = self.state.workspace_apply_lock.lock().await;
        if self.generation
            != self
                .state
                .workspace_apply_generation
                .load(std::sync::atomic::Ordering::Acquire)
        {
            return Err("active community changed during directory lookup".into());
        }
        let active = identity_archive::capture_relay_target(self.state);
        if active.ws_url != self.target.ws_url || active.api_base_url != self.target.api_base_url {
            return Err("active community changed during directory lookup".into());
        }
        assert_expected_signer(
            Some(&self.keys.public_key().to_hex()),
            &self.state.signing_keys()?.public_key().to_hex(),
        )
    }

    async fn query(&self, filters: &[serde_json::Value]) -> Result<Vec<nostr::Event>, String> {
        query_relay_at_with_keys(
            self.state,
            &self.target.api_base_url,
            filters,
            &self.keys,
            None,
        )
        .await
    }
}

/// Run one `query_relay` request per `RELAY_FILTER_BATCH_SIZE` chunk of
/// `filters`, each acquiring a permit from `semaphore` so the total in-flight
/// request count stays within the shared ceiling even when several batch sets
/// run concurrently. Returned events are concatenated; order is unspecified —
/// every caller keys the events by pubkey downstream, so ordering is irrelevant.
async fn query_filter_batches(
    read: &DirectoryRead<'_>,
    semaphore: &tokio::sync::Semaphore,
    filters: &[serde_json::Value],
    error_label: &str,
) -> Result<Vec<nostr::Event>, String> {
    let pages = futures_util::future::try_join_all(filters.chunks(RELAY_FILTER_BATCH_SIZE).map(
        |batch| async move {
            let _permit = semaphore.acquire().await.map_err(|error| {
                format!("{error_label}: directory concurrency semaphore closed: {error}")
            })?;
            read.query(batch)
                .await
                .map_err(|error| format!("{error_label}: {error}"))
        },
    ))
    .await?;
    Ok(pages.into_iter().flatten().collect())
}

fn exact_author_filters(pubkeys: &[String], kind: u16) -> Vec<serde_json::Value> {
    pubkeys
        .iter()
        .map(|pubkey| {
            serde_json::json!({
                "authors": [pubkey],
                "kinds": [kind],
                "limit": 1,
            })
        })
        .collect()
}

fn managed_policy_filters(
    candidate_pubkeys: &[String],
    verified_owners: &std::collections::HashMap<String, String>,
) -> Vec<serde_json::Value> {
    candidate_pubkeys
        .iter()
        .filter_map(|agent_pubkey| {
            verified_owners.get(agent_pubkey).map(|owner_pubkey| {
                serde_json::json!({
                    "authors": [owner_pubkey],
                    "kinds": [30177],
                    "#d": [agent_pubkey],
                    "limit": 1,
                })
            })
        })
        .collect()
}

pub(super) fn advance_relay_cursor(filter: &mut serde_json::Value, page: &[nostr::Event]) {
    let last = page
        .last()
        .expect("a full relay page always has a last event");
    filter["until"] = serde_json::json!(last.created_at.as_secs());
    filter["before_id"] = serde_json::json!(last.id.to_hex());
}

async fn query_all_relay_pages(
    read: &DirectoryRead<'_>,
    mut filter: serde_json::Value,
) -> Result<Vec<nostr::Event>, String> {
    filter["limit"] = serde_json::json!(RELAY_DIRECTORY_PAGE_SIZE);
    let mut events = Vec::new();
    loop {
        let page = read.query(std::slice::from_ref(&filter)).await?;
        let done = page.len() < RELAY_DIRECTORY_PAGE_SIZE;
        if !done {
            advance_relay_cursor(&mut filter, &page);
        }
        events.extend(page);
        if done {
            return Ok(events);
        }
    }
}

fn retain_agents_allowed_by_build(agents: &mut Vec<RelayAgentInfo>, require_verified_policy: bool) {
    if require_verified_policy {
        agents.retain(|agent| agent.has_managed_policy);
    }
}

#[cfg(test)]
pub(crate) async fn list_relay_agents_for_state(
    state: &AppState,
) -> Result<Vec<RelayAgentInfo>, String> {
    list_relay_agents_for_selection(state, None, None).await
}

#[cfg(test)]
async fn list_relay_agents_for_selection(
    state: &AppState,
    requested_pubkeys: Option<&std::collections::HashSet<String>>,
    channel_id: Option<&str>,
) -> Result<Vec<RelayAgentInfo>, String> {
    let read = DirectoryRead::capture(state, None, None).await?;
    list_in_scope(&read, requested_pubkeys, channel_id).await
}

async fn list_in_scope(
    read: &DirectoryRead<'_>,
    requested_pubkeys: Option<&std::collections::HashSet<String>>,
    channel_id: Option<&str>,
) -> Result<Vec<RelayAgentInfo>, String> {
    let viewer_pubkey = read.keys.public_key().to_hex();
    let relay_pubkey = identity_archive::fetch_relay_self_at(read.state, &read.target.ws_url)
        .await?
        .ok_or_else(|| "relay agent membership authority is unavailable".to_string())?;

    // Owned identities are relay state, even when this Desktop has never run
    // them or they have not joined a channel yet. Owner-authored coordinates
    // seed discovery only; the agent's signed NIP-OA profile still has to
    // authenticate ownership below. Scope selection queries to the exact keys.
    let mut owned_filter = serde_json::json!({
        "kinds": [30177],
        "authors": [&viewer_pubkey],
    });
    if let Some(requested_pubkeys) = requested_pubkeys {
        owned_filter["#d"] = serde_json::json!(requested_pubkeys);
    }
    let owned_query = async {
        query_all_relay_pages(read, owned_filter)
            .await
            .map_err(|error| format!("relay owned-agent query failed: {error}"))
    };

    // Membership remains the authoritative and bounded authorization scope,
    // visible only to this viewer. Known owned identities can have any
    // membership role; other candidates must still have explicit bot-role
    // evidence.
    let mut membership_filter = serde_json::json!({
        "kinds": [39002],
        "authors": [&relay_pubkey],
        "#p": [&viewer_pubkey],
    });
    if let Some(channel_id) = channel_id {
        membership_filter["#d"] = serde_json::json!([channel_id]);
    }
    let membership_query = async {
        query_all_relay_pages(read, membership_filter)
            .await
            .map_err(|error| format!("relay agent channel-membership query failed: {error}"))
    };
    // One semaphore per rebuild caps batched `/query` requests across this
    // rebuild's phases, so its runtime-directory and owner-profile phases stay
    // within the ceiling even though `try_join!` runs them concurrently. The
    // owned-agent and membership pagers are single sequential request streams
    // and run outside the semaphore, so the targeted path's ceiling is the
    // batches plus two.
    let semaphore = tokio::sync::Semaphore::new(RELAY_DIRECTORY_MAX_CONCURRENCY);
    let (member_agent_channel_ids, candidate_pubkeys, directory_events, profile_events) =
        if let Some(requested_pubkeys) = requested_pubkeys {
            // Targeted path: the caller already names the candidates, so
            // neither the owned-agent read nor the membership read gates the
            // directory/profile fan-out — they all join it, one round-trip
            // stage instead of three. The owned read is `#d`-scoped to the
            // requested keys, so it can only ever name candidates already in
            // this set. Directory, profile, and (below) policy reads may now
            // issue for requested pubkeys membership excludes — bounded by the
            // user-typed mention set — but the membership/owner retain on the
            // final result still drops them, so what is returned is identical.
            let candidate_pubkeys: Vec<String> = requested_pubkeys.iter().cloned().collect();
            let directory_filters = exact_author_filters(&candidate_pubkeys, 10100);
            let profile_filters = exact_author_filters(&candidate_pubkeys, 0);
            let (owned_events, membership_events, directory_events, profile_events) = tokio::try_join!(
                owned_query,
                membership_query,
                query_filter_batches(
                    read,
                    &semaphore,
                    &directory_filters,
                    "relay agent runtime-directory query failed",
                ),
                query_filter_batches(
                    read,
                    &semaphore,
                    &profile_filters,
                    "relay agent owner-profile query failed",
                ),
            )?;
            let mut known_agents = nostr_convert::managed_agent_pubkeys_from_events(&owned_events);
            // OA identifies the selected agent even when its viewer is not
            // its owner. The signed destination roster establishes membership;
            // the resolved response policy separately establishes who may send.
            known_agents.extend(
                nostr_convert::verified_agent_owners_from_profiles(&profile_events).into_keys(),
            );
            let mut member_agent_channel_ids = nostr_convert::member_agent_channel_ids_from_events(
                &membership_events,
                &relay_pubkey,
                &viewer_pubkey,
                &known_agents,
            );
            member_agent_channel_ids.retain(|pubkey, _| requested_pubkeys.contains(pubkey));
            (
                member_agent_channel_ids,
                candidate_pubkeys,
                directory_events,
                profile_events,
            )
        } else {
            // Full rebuild: the owned-agent and membership reads *discover* the
            // candidates, so both must resolve before the batch filters can be
            // built. Sequential shape retained — this is the autocomplete path,
            // not the send path.
            let owned_events = owned_query.await?;
            let membership_events = membership_query.await?;
            let owned_candidates = nostr_convert::managed_agent_pubkeys_from_events(&owned_events);
            let member_agent_channel_ids = nostr_convert::member_agent_channel_ids_from_events(
                &membership_events,
                &relay_pubkey,
                &viewer_pubkey,
                &owned_candidates,
            );
            let candidate_pubkeys: Vec<String> = member_agent_channel_ids
                .keys()
                .cloned()
                .chain(owned_candidates)
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();
            if candidate_pubkeys.is_empty() {
                read.ensure_current().await?;
                return Ok(Vec::new());
            }
            let directory_filters = exact_author_filters(&candidate_pubkeys, 10100);
            let profile_filters = exact_author_filters(&candidate_pubkeys, 0);
            let (directory_events, profile_events) = tokio::try_join!(
                query_filter_batches(
                    read,
                    &semaphore,
                    &directory_filters,
                    "relay agent runtime-directory query failed",
                ),
                query_filter_batches(
                    read,
                    &semaphore,
                    &profile_filters,
                    "relay agent owner-profile query failed",
                ),
            )?;
            (
                member_agent_channel_ids,
                candidate_pubkeys,
                directory_events,
                profile_events,
            )
        };

    // Only the agent's signed NIP-OA profile can name the owner coordinate to
    // query. Each exact `(owner, d=agent)` filter returns at most one current
    // replaceable event, so forged 30177 coordinates cannot amplify or crowd
    // the authentic policy out of a bounded result page.
    let verified_owners = nostr_convert::verified_agent_owners_from_profiles(&profile_events);
    let managed_filters = managed_policy_filters(&candidate_pubkeys, &verified_owners);
    let managed_agent_events = query_filter_batches(
        read,
        &semaphore,
        &managed_filters,
        "relay agent managed-policy query failed",
    )
    .await?;

    let mut agents = nostr_convert::relay_agents_from_directory_events(
        &directory_events,
        &managed_agent_events,
        &profile_events,
    );
    // Marked builds require verified managed policy even for OA-backed legacy
    // records, but do not require that owner to equal the viewer. The
    // verified owner's signed respond_to policy remains the authorization
    // boundary for independently operated relay agents.
    retain_agents_allowed_by_build(
        &mut agents,
        crate::managed_agents::owner_only_access_build(),
    );
    agents.retain(|agent| {
        if requested_pubkeys.is_some_and(|keys| !keys.contains(&agent.pubkey)) {
            return false;
        }
        // An exact OA-backed owner selection can prepare an invitation,
        // including into a new DM. Only the roster below grants membership.
        member_agent_channel_ids.contains_key(&agent.pubkey)
            || (agent.owner_pubkey.as_deref() == Some(viewer_pubkey.as_str())
                && (agent.has_managed_policy || requested_pubkeys.is_some()))
    });
    for agent in &mut agents {
        agent.channel_ids = member_agent_channel_ids
            .get(&agent.pubkey)
            .cloned()
            .unwrap_or_default();
    }
    read.ensure_current().await?;
    Ok(agents)
}

#[tauri::command]
pub async fn list_relay_agents(
    expected_relay_url: Option<String>,
    expected_signer_pubkey: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<RelayAgentInfo>, String> {
    let read = DirectoryRead::capture(
        &state,
        expected_relay_url.as_deref(),
        expected_signer_pubkey.as_deref(),
    )
    .await?;
    list_in_scope(&read, None, None).await
}

/// Revalidate only the selected relay agents in the target channel.
///
/// This preserves the full directory command for autocomplete while keeping
/// send-time authorization bounded by the actual mention set and destination.
#[tauri::command]
pub async fn revalidate_relay_agents(
    pubkeys: Vec<String>,
    channel_id: Option<String>,
    expected_relay_url: String,
    expected_signer_pubkey: String,
    state: State<'_, AppState>,
) -> Result<Vec<RelayAgentInfo>, String> {
    let requested_pubkeys = pubkeys
        .into_iter()
        .filter_map(|pubkey| nostr::PublicKey::from_hex(&pubkey).ok())
        .map(|pubkey| pubkey.to_hex())
        .collect::<std::collections::HashSet<_>>();
    if requested_pubkeys.is_empty() {
        return Ok(Vec::new());
    }
    if expected_relay_url.trim().is_empty() || expected_signer_pubkey.trim().is_empty() {
        return Err("agent revalidation requires the selected community and identity".into());
    }
    let read = DirectoryRead::capture(
        &state,
        Some(&expected_relay_url),
        Some(&expected_signer_pubkey),
    )
    .await?;
    list_in_scope(&read, Some(&requested_pubkeys), channel_id.as_deref()).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marked_build_requires_verified_policy_without_requiring_viewer_ownership() {
        let cross_owner = "b".repeat(64);
        let mut agents = vec![
            RelayAgentInfo {
                pubkey: "a".repeat(64),
                owner_pubkey: Some(cross_owner.clone()),
                has_managed_policy: true,
                host_pubkey: None,
                channel_add_policy: None,
                name: "Verified cross-owner".to_string(),
                agent_type: "agent".to_string(),
                channels: Vec::new(),
                channel_ids: Vec::new(),
                capabilities: Vec::new(),
                status: "offline".to_string(),
                respond_to: None,
                respond_to_allowlist: Vec::new(),
            },
            RelayAgentInfo {
                pubkey: "c".repeat(64),
                owner_pubkey: Some(cross_owner.clone()),
                has_managed_policy: false,
                host_pubkey: None,
                channel_add_policy: None,
                name: "OA-backed legacy".to_string(),
                agent_type: "agent".to_string(),
                channels: Vec::new(),
                channel_ids: Vec::new(),
                capabilities: Vec::new(),
                status: "online".to_string(),
                respond_to: None,
                respond_to_allowlist: Vec::new(),
            },
        ];

        retain_agents_allowed_by_build(&mut agents, true);

        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].name, "Verified cross-owner");
        assert_eq!(
            agents[0].owner_pubkey.as_deref(),
            Some(cross_owner.as_str())
        );
    }

    #[test]
    fn oss_build_preserves_ownerless_legacy_agents() {
        let mut agents = vec![RelayAgentInfo {
            pubkey: "a".repeat(64),
            owner_pubkey: None,
            has_managed_policy: false,
            host_pubkey: None,
            channel_add_policy: None,
            name: "Ownerless legacy".to_string(),
            agent_type: "agent".to_string(),
            channels: Vec::new(),
            channel_ids: Vec::new(),
            capabilities: Vec::new(),
            status: "online".to_string(),
            respond_to: None,
            respond_to_allowlist: Vec::new(),
        }];

        retain_agents_allowed_by_build(&mut agents, false);

        assert_eq!(agents.len(), 1);
        assert!(agents[0].owner_pubkey.is_none());
    }

    #[test]
    fn exact_author_queries_prevent_noisy_agent_crowd_out() {
        let pubkeys = vec!["a".repeat(64), "b".repeat(64)];

        let filters = exact_author_filters(&pubkeys, 10100);

        assert_eq!(filters.len(), 2);
        for (filter, pubkey) in filters.iter().zip(pubkeys) {
            assert_eq!(filter["authors"], serde_json::json!([pubkey]));
            assert_eq!(filter["kinds"], serde_json::json!([10100]));
            assert_eq!(filter["limit"], 1);
        }
    }

    #[test]
    fn managed_policy_queries_are_exact_coordinates() {
        let candidates = vec!["a".repeat(64), "b".repeat(64)];
        let owners = std::collections::HashMap::from([
            (candidates[0].clone(), "c".repeat(64)),
            (candidates[1].clone(), "d".repeat(64)),
        ]);

        let filters = managed_policy_filters(&candidates, &owners);

        assert_eq!(filters.len(), 2);
        for (filter, candidate) in filters.iter().zip(candidates) {
            assert_eq!(filter["authors"].as_array().map(Vec::len), Some(1));
            assert_eq!(filter["kinds"], serde_json::json!([30177]));
            assert_eq!(filter["#d"], serde_json::json!([candidate]));
            assert_eq!(filter["limit"], 1);
        }
    }

    #[test]
    fn relay_filter_batches_do_not_exceed_protocol_limit() {
        let pubkeys: Vec<_> = (0..25).map(|index| format!("{index:064x}")).collect();
        let filters = exact_author_filters(&pubkeys, 0);

        let batch_sizes: Vec<_> = filters
            .chunks(RELAY_FILTER_BATCH_SIZE)
            .map(<[_]>::len)
            .collect();

        assert_eq!(batch_sizes, vec![10, 10, 5]);
    }
}

#[cfg(all(test, not(target_os = "windows")))]
mod real_relay_tests {
    use super::*;
    use crate::{app_state::build_app_state, events, managed_agents, relay};
    use buzz_core_pkg::kind::KIND_MANAGED_AGENT;
    use nostr::{EventBuilder, Keys, Kind, Tag};
    use uuid::Uuid;

    fn relay_ws_url() -> String {
        std::env::var("RELAY_URL").unwrap_or_else(|_| "ws://localhost:3037".to_string())
    }

    fn state_for(keys: Keys) -> AppState {
        let state = build_app_state();
        *state.keys.lock().unwrap() = keys;
        *state.relay_url_override.lock().unwrap() = Some(relay_ws_url());
        state
    }

    async fn publish(builder: EventBuilder, signer: &Keys, state: &AppState) {
        relay::submit_event_with_keys(builder, state, signer, None)
            .await
            .expect("publish real-relay fixture");
    }

    #[tokio::test]
    #[ignore]
    async fn newly_retained_managed_policy_replaces_open_access_immediately_on_real_relay() {
        let owner = Keys::generate();
        let agent = Keys::generate();
        let state = state_for(owner.clone());
        let db_dir = tempfile::tempdir().unwrap();
        let db_path = db_dir.path().join("retention.sqlite3");
        let initial_content = serde_json::json!({
            "name": "Immediate Policy Probe",
            "parallelism": 1,
            "respond_to": "anyone"
        })
        .to_string();
        let initial_event =
            EventBuilder::new(Kind::Custom(KIND_MANAGED_AGENT as u16), initial_content)
                .tags([Tag::parse(["d", &agent.public_key().to_hex()]).unwrap()])
                .custom_created_at(nostr::Timestamp::from(
                    nostr::Timestamp::now().as_secs().saturating_sub(1),
                ));
        publish(initial_event, &owner, &state).await;

        let updated_content = serde_json::json!({
            "name": "Immediate Policy Probe",
            "parallelism": 1,
            "respond_to": "owner-only"
        })
        .to_string();
        let event = EventBuilder::new(Kind::Custom(KIND_MANAGED_AGENT as u16), updated_content)
            .tags([Tag::parse(["d", &agent.public_key().to_hex()]).unwrap()])
            .sign_with_keys(&owner)
            .unwrap();

        {
            use managed_agents::retention::{open_retention_db, retain_event, RetainedEvent};
            use nostr::JsonUtil;

            let conn = open_retention_db(&db_path).unwrap();
            retain_event(
                &conn,
                &RetainedEvent {
                    kind: KIND_MANAGED_AGENT,
                    pubkey: owner.public_key().to_hex(),
                    d_tag: agent.public_key().to_hex(),
                    content: event.content.clone(),
                    created_at: event.created_at.as_secs() as i64,
                    raw_event: event.as_json(),
                    pending_sync: true,
                },
            )
            .unwrap();
        }

        let flushed = managed_agents::persona_events::flush_pending_events_at(
            &db_path,
            &state,
            &relay_ws_url(),
            &owner,
        )
        .await
        .expect("create-path immediate policy flush");
        assert_eq!(flushed, 1);

        let queried = crate::relay::query_relay(
            &state,
            &[serde_json::json!({
                "kinds": [KIND_MANAGED_AGENT],
                "authors": [owner.public_key().to_hex()],
                "#d": [agent.public_key().to_hex()],
                "limit": 1
            })],
        )
        .await
        .expect("query immediately flushed policy");
        assert_eq!(queried.len(), 1);
        assert_eq!(queried[0].id, event.id);
        assert!(queried[0].content.contains("\"respond_to\":\"owner-only\""));
    }

    #[tokio::test]
    #[ignore]
    async fn cross_identity_managed_agent_is_discovered_and_emits_exact_p_tag_from_real_relay() {
        let owner = Keys::generate();
        let viewer = Keys::generate();
        let agent = Keys::generate();
        let owner_state = state_for(owner.clone());
        let viewer_state = state_for(viewer.clone());
        let channel_id = Uuid::new_v4();

        publish(
            events::build_create_channel(
                channel_id,
                &format!("agent-discovery-e2e-{channel_id}"),
                "private",
                "stream",
                None,
                None,
            )
            .unwrap(),
            &owner,
            &owner_state,
        )
        .await;
        publish(
            events::build_add_member(channel_id, &viewer.public_key().to_hex(), None).unwrap(),
            &owner,
            &owner_state,
        )
        .await;
        publish(
            events::build_add_member(channel_id, &agent.public_key().to_hex(), Some("bot"))
                .unwrap(),
            &owner,
            &owner_state,
        )
        .await;

        let compat_owner = nostr::Keys::parse(&owner.secret_key().to_secret_hex()).unwrap();
        let compat_agent = nostr::PublicKey::from_hex(&agent.public_key().to_hex()).unwrap();
        let auth_tag =
            buzz_sdk_pkg::nip_oa::compute_auth_tag(&compat_owner, &compat_agent, "").unwrap();
        relay::sync_managed_agent_profile(
            &owner_state,
            &relay_ws_url(),
            &agent,
            "Agent Probe",
            None,
            None,
            Some(&auth_tag),
        )
        .await
        .expect("publish agent kind:0 profile");

        let managed_content = serde_json::json!({
            "name": "Agent Probe",
            "parallelism": 1,
            "respond_to": "anyone"
        })
        .to_string();
        publish(
            EventBuilder::new(Kind::Custom(30177), managed_content).tags([Tag::parse([
                "d",
                &agent.public_key().to_hex(),
            ])
            .unwrap()]),
            &owner,
            &owner_state,
        )
        .await;

        let agents = list_relay_agents_for_state(&viewer_state)
            .await
            .expect("query production relay directory");
        assert_eq!(agents.len(), 1, "real relay directory returned {agents:?}");
        assert_eq!(agents[0].pubkey, agent.public_key().to_hex());
        assert_eq!(agents[0].name, "Agent Probe");
        assert_eq!(agents[0].channel_ids, vec![channel_id.to_string()]);

        // Exercise the final protocol boundary, not merely the directory DTO:
        // selecting this candidate must become the agent's exact lowercase
        // `p` tag in the signed stream event.
        let mention_pubkey = agents[0].pubkey.as_str();
        let signed_message = events::build_message(
            channel_id,
            "Ask @Agent Probe to reply",
            None,
            &[mention_pubkey],
            &[],
            &[],
            &[],
            &[],
            None,
            &relay_ws_url(),
        )
        .unwrap()
        .sign_with_keys(&viewer)
        .unwrap();
        let emitted_mentions: Vec<_> = signed_message
            .tags
            .iter()
            .filter_map(|tag| {
                let tag = tag.as_slice();
                (tag.first().map(String::as_str) == Some("p"))
                    .then(|| tag.get(1).cloned())
                    .flatten()
            })
            .collect();
        assert_eq!(emitted_mentions, vec![agent.public_key().to_hex()]);
    }
}

#[cfg(test)]
mod owned_tests;
