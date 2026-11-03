//! Exercise the production query plan against a loopback relay with signed fixtures.
use super::*;
use axum::{
    routing::{get, post},
    Json, Router,
};
use nostr::{EventBuilder, Keys, Kind, Tag};
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn remote_owned_discovery_and_membership_do_not_require_local_records() {
    let _serial = crate::relay_admission::TEST_SERIAL.lock().await;
    crate::relay_admission::reset_rate_limit_gate();
    let relay = Keys::generate();
    let owner = Keys::generate();
    let agent = Keys::generate();
    let stranger = Keys::generate();
    let agent_key = agent.public_key().to_hex();
    let owner_key = owner.public_key().to_hex();
    let relay_key = relay.public_key().to_hex();
    let auth = buzz_sdk_pkg::nip_oa::compute_auth_tag(&owner, &agent.public_key(), "").unwrap();
    let auth: Vec<String> = serde_json::from_str(&auth).unwrap();
    let profile = EventBuilder::new(Kind::Metadata, r#"{"display_name":"Remote Scout"}"#)
        .tags([Tag::parse(auth).unwrap()])
        .sign_with_keys(&agent)
        .unwrap();
    let policy = |key: &str| {
        EventBuilder::new(
            Kind::Custom(30177),
            r#"{"name":"Remote Scout","parallelism":1,"respond_to":"owner-only"}"#,
        )
        .tags([Tag::parse(["d", key]).unwrap()])
        .sign_with_keys(&owner)
        .unwrap()
    };
    // An owner-authored coordinate is a discovery hint, not ownership proof.
    let forged = policy(&stranger.public_key().to_hex());
    let stranger_profile = EventBuilder::new(Kind::Metadata, "{}")
        .sign_with_keys(&stranger)
        .unwrap();
    let events = Arc::new(Mutex::new(vec![
        profile.clone(),
        policy(&agent_key),
        forged,
        stranger_profile,
    ]));
    let queries = Arc::new(Mutex::new(Vec::<serde_json::Value>::new()));
    let query_events = events.clone();
    let query_log = queries.clone();
    let router = Router::new()
        .route(
            "/",
            get(move || {
                let key = relay_key.clone();
                async move { Json(serde_json::json!({"self": key})) }
            }),
        )
        .route(
            "/query",
            post(move |Json(filters): Json<Vec<serde_json::Value>>| {
                let events = query_events.clone();
                let queries = query_log.clone();
                async move {
                    queries.lock().unwrap().extend(filters.clone());
                    let events = events.lock().unwrap();
                    let result: Vec<_> = events
                        .iter()
                        .filter(|event| {
                            filters.iter().any(|filter| {
                                filter["kinds"]
                                    .as_array()
                                    .unwrap()
                                    .contains(&serde_json::json!(event.kind.as_u16()))
                                    && filter.get("authors").is_none_or(|authors| {
                                        authors
                                            .as_array()
                                            .unwrap()
                                            .contains(&serde_json::json!(event.pubkey.to_hex()))
                                    })
                                    && ["d", "p"].iter().all(|tag| {
                                        filter.get(format!("#{tag}")).is_none_or(|values| {
                                            event.tags.iter().any(|t| {
                                                t.as_slice().first().map(String::as_str)
                                                    == Some(*tag)
                                                    && t.as_slice().get(1).is_some_and(|value| {
                                                        values
                                                            .as_array()
                                                            .unwrap()
                                                            .contains(&serde_json::json!(value))
                                                    })
                                            })
                                        })
                                    })
                            })
                        })
                        .cloned()
                        .collect();
                    Json(result)
                }
            }),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let state = crate::app_state::build_app_state();
    *state.keys.lock().unwrap() = owner.clone();
    *state.relay_url_override.lock().unwrap() = Some(format!("ws://{address}"));

    let discovered = list_relay_agents_for_state(&state).await.unwrap();
    assert_eq!(discovered.len(), 1, "forged ownership must not be admitted");
    assert_eq!(discovered[0].pubkey, agent_key);
    assert_eq!(
        discovered[0].owner_pubkey.as_deref(),
        Some(owner_key.as_str())
    );
    assert!(
        discovered[0].channel_ids.is_empty(),
        "discovery is not membership"
    );

    let membership = EventBuilder::new(Kind::Custom(39002), "")
        .tags([
            Tag::parse(["d", "general"]).unwrap(),
            Tag::parse(["p", &owner_key, "", "member"]).unwrap(),
            Tag::parse(["p", &agent_key, "", "member"]).unwrap(),
        ])
        .sign_with_keys(&relay)
        .unwrap();
    events.lock().unwrap().push(membership);
    let requested = std::collections::HashSet::from([agent_key.clone()]);
    let admitted = list_relay_agents_for_selection(&state, Some(&requested), Some("general"))
        .await
        .unwrap();
    assert_eq!(admitted.len(), 1);
    assert_eq!(admitted[0].channel_ids, vec!["general".to_string()]);
    let outside = list_relay_agents_for_selection(&state, Some(&requested), Some("private-other"))
        .await
        .unwrap();
    assert_eq!(outside.len(), 1);
    assert!(
        outside[0].channel_ids.is_empty(),
        "ownership cannot fabricate destination membership"
    );
    // A newer signed snapshot revokes membership, even if an old snapshot
    // is also returned. The owned identity remains discoverable, not admitted.
    let removed = EventBuilder::new(Kind::Custom(39002), "")
        .tags([
            Tag::parse(["d", "general"]).unwrap(),
            Tag::parse(["p", &owner_key, "", "member"]).unwrap(),
        ])
        .custom_created_at(nostr::Timestamp::from(
            nostr::Timestamp::now().as_secs() + 1,
        ))
        .sign_with_keys(&relay)
        .unwrap();
    events.lock().unwrap().push(removed);
    let revoked = list_relay_agents_for_selection(&state, Some(&requested), Some("general"))
        .await
        .unwrap();
    assert!(revoked[0].channel_ids.is_empty());

    let deny = EventBuilder::new(
        Kind::Custom(30177),
        r#"{"name":"Remote Scout","parallelism":1,"respond_to":"nobody"}"#,
    )
    .tags([Tag::parse(["d", &agent_key]).unwrap()])
    .custom_created_at(nostr::Timestamp::from(
        nostr::Timestamp::now().as_secs() + 2,
    ))
    .sign_with_keys(&owner)
    .unwrap();
    events.lock().unwrap().push(deny);
    let denied = list_relay_agents_for_selection(&state, Some(&requested), Some("general"))
        .await
        .unwrap();
    assert!(
        denied.is_empty(),
        "latest unsupported policy cannot fall back to an older allow"
    );

    assert!(queries
        .lock()
        .unwrap()
        .iter()
        .any(|filter| filter["kinds"] == serde_json::json!([30177])
            && filter["authors"] == serde_json::json!([owner_key])
            && filter.get("#d").is_none()));
    // A-mode agents omit the 30177 owner coordinate. A shared bot
    // membership seeds them on a fresh device, while an unindexed runtime
    // alone cannot. Only the signed roster supplies actual channel IDs.
    let host = Keys::generate().public_key().to_hex();
    let runtime = EventBuilder::new(
        Kind::Custom(10100),
        serde_json::json!({
            "name": "Scout", "host": host, "respond_to": "owner-only",
            "channel_add_policy": "owner_only", "channel_ids": ["fabricated"],
        })
        .to_string(),
    )
    .sign_with_keys(&agent)
    .unwrap();
    *events.lock().unwrap() = vec![profile, runtime];
    assert!(list_relay_agents_for_state(&state)
        .await
        .unwrap()
        .is_empty());
    let bot_membership = EventBuilder::new(Kind::Custom(39002), "")
        .tags([
            Tag::parse(["d", "general"]).unwrap(),
            Tag::parse(["p", &owner_key, "", "member"]).unwrap(),
            Tag::parse(["p", &agent_key, "", "bot"]).unwrap(),
        ])
        .sign_with_keys(&relay)
        .unwrap();
    events.lock().unwrap().push(bot_membership);
    let legacy = list_relay_agents_for_state(&state).await.unwrap();
    assert_eq!(legacy.len(), 1);
    assert_eq!(legacy[0].owner_pubkey.as_deref(), Some(owner_key.as_str()));
    assert!(!legacy[0].has_managed_policy);
    assert_eq!(legacy[0].host_pubkey.as_deref(), Some(host.as_str()));
    assert_eq!(legacy[0].status, "unknown");
    assert_eq!(legacy[0].channel_ids, ["general"]);
    let dm = EventBuilder::new(Kind::Custom(39002), "")
        .tags([
            Tag::parse(["d", "dm"]).unwrap(),
            Tag::parse(["p", &owner_key, "", "member"]).unwrap(),
            Tag::parse(["p", &agent_key, "", "member"]).unwrap(),
        ])
        .sign_with_keys(&relay)
        .unwrap();
    events.lock().unwrap().push(dm);
    let in_dm = list_relay_agents_for_selection(&state, Some(&requested), Some("dm"))
        .await
        .unwrap();
    assert_eq!(in_dm[0].channel_ids, ["dm"]);
    let outside = list_relay_agents_for_selection(&state, Some(&requested), Some("elsewhere"))
        .await
        .unwrap();
    assert_eq!(outside.len(), 1, "owner can prepare an invite");
    assert!(
        outside[0].channel_ids.is_empty(),
        "publication still needs membership"
    );
    // A different viewer shares a bot-role stream and an ordinary-role DM
    // with the same attested agent. Exact reads must preserve both membership
    // and the declared actor policy. Missing either party removes the result.
    *state.keys.lock().unwrap() = stranger.clone();
    let viewer = stranger.public_key().to_hex();
    for (id, role, has_viewer, has_agent) in [
        ("stream-a", "bot", true, true),
        ("stream-b", "member", true, true),
        ("other-actor", "member", false, true),
        ("other-agent", "member", true, false),
    ] {
        let mut tags = vec![Tag::parse(["d", id]).unwrap()];
        if has_viewer {
            tags.push(Tag::parse(["p", &viewer, "", "member"]).unwrap());
        }
        if has_agent {
            tags.push(Tag::parse(["p", &agent_key, "", role]).unwrap());
        }
        events.lock().unwrap().push(
            EventBuilder::new(Kind::Custom(39002), "")
                .tags(tags)
                .sign_with_keys(&relay)
                .unwrap(),
        );
    }
    for (index, (name, mode)) in [
        ("Claude", "anyone"),
        ("Grok", "anyone"),
        ("Scout", "owner-only"),
    ]
    .iter()
    .enumerate()
    {
        events.lock().unwrap().push(
            EventBuilder::new(
                Kind::Custom(10100),
                serde_json::json!({
                    "name":name, "host":host, "respond_to":mode, "channel_add_policy":"owner_only",
                })
                .to_string(),
            )
            .custom_created_at(nostr::Timestamp::from(
                nostr::Timestamp::now().as_secs() + index as u64 + 1,
            ))
            .sign_with_keys(&agent)
            .unwrap(),
        );
        let all = list_relay_agents_for_state(&state).await.unwrap();
        assert_eq!(all[0].channel_ids, ["stream-a", "stream-b"]);
        let selected = list_relay_agents_for_selection(&state, Some(&requested), Some("stream-b"))
            .await
            .unwrap();
        assert_eq!(selected[0].channel_ids, ["stream-b"]);
        assert_eq!(
            serde_json::to_value(&selected[0]).unwrap()["respond_to"],
            *mode
        );
        assert_ne!(selected[0].owner_pubkey.as_deref(), Some(viewer.as_str()));
    }
    for missing in ["other-actor", "other-agent"] {
        assert!(
            list_relay_agents_for_selection(&state, Some(&requested), Some(missing))
                .await
                .unwrap()
                .is_empty()
        );
    }
    server.abort();
    crate::relay_admission::reset_rate_limit_gate();
}
