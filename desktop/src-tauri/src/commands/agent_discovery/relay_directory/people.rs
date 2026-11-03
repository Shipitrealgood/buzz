//! Resolve ambiguous people-search hits while the shared agent directory is unavailable.

use super::{
    exact_author_filters, query_filter_batches, DirectoryRead, RELAY_DIRECTORY_MAX_CONCURRENCY,
};
use crate::app_state::AppState;
use nostr::{Event, Kind};
use std::collections::HashSet;
use tauri::State;

fn people_without_agent_records(requested: &[String], events: &[Event]) -> Vec<String> {
    let mut profiles = HashSet::new();
    let mut ambiguous = HashSet::new();
    for event in events {
        let key = event.pubkey.to_hex();
        // A runtime record (even an unreadable one), an ownership tag, or an
        // invalid profile prevents a people-search hit from bypassing policy.
        if event.kind == Kind::Metadata
            && event.verify().is_ok()
            && !event
                .tags
                .iter()
                .any(|tag| tag.as_slice().first().is_some_and(|name| name == "auth"))
        {
            profiles.insert(key);
        } else {
            ambiguous.insert(key);
        }
    }
    requested
        .iter()
        .filter(|key| profiles.contains(*key) && !ambiguous.contains(*key))
        .cloned()
        .collect()
}

/// Return ordinary people eligible for the normal channel invitation path
/// from this exact page of search hits. Relay admission remains authoritative.
#[tauri::command]
pub async fn check_member_add_people(
    pubkeys: Vec<String>,
    expected_relay_url: String,
    expected_signer_pubkey: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    if pubkeys.len() > 50
        || expected_relay_url.trim().is_empty()
        || expected_signer_pubkey.trim().is_empty()
    {
        return Err("member lookup requires a scoped page of at most 50 keys".into());
    }
    let keys = pubkeys
        .iter()
        .map(|key| {
            nostr::PublicKey::from_hex(key)
                .map(|k| k.to_hex())
                .map_err(|e| e.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let read = DirectoryRead::capture(
        &state,
        Some(&expected_relay_url),
        Some(&expected_signer_pubkey),
    )
    .await?;
    let mut filters = exact_author_filters(&keys, 0);
    filters.extend(exact_author_filters(&keys, 10100));
    let semaphore = tokio::sync::Semaphore::new(RELAY_DIRECTORY_MAX_CONCURRENCY);
    let events = query_filter_batches(
        &read,
        &semaphore,
        &filters,
        "member add identity check failed",
    )
    .await?;
    read.ensure_current().await?;
    Ok(people_without_agent_records(&keys, &events))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nostr::{EventBuilder, Keys, Tag};
    #[test]
    fn plain_people_require_a_verified_profile_and_absence_of_agent_evidence() {
        let human = Keys::generate();
        let bot = Keys::generate();
        let owned = Keys::generate();
        let missing = Keys::generate();
        let profile = |keys: &Keys| {
            EventBuilder::new(Kind::Metadata, "{}")
                .sign_with_keys(keys)
                .unwrap()
        };
        let runtime = EventBuilder::new(Kind::Custom(10100), r#"{"channel_add_policy":"nobody"}"#)
            .sign_with_keys(&bot)
            .unwrap();
        let oa = EventBuilder::new(Kind::Metadata, "{}")
            .tags([Tag::parse(["auth", "invalid"]).unwrap()])
            .sign_with_keys(&owned)
            .unwrap();
        let keys = [&human, &bot, &owned, &missing].map(|k| k.public_key().to_hex());
        assert_eq!(
            people_without_agent_records(&keys, &[profile(&human), profile(&bot), runtime, oa]),
            [human.public_key().to_hex()]
        );
    }
}
