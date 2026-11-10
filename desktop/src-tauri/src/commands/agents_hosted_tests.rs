//! Windows-only native phases, confined like the existing main-create tests.
use super::*;
use tauri::Manager;

#[test]
fn hosted_native_create_recovery_and_session_fence() {
    fixture_process("hosted_native_create_recovery_and_session_fence", false);
}
#[test]
fn hosted_native_cleanup_confirmation_retry_and_session_fence() {
    fixture_process(
        "hosted_native_cleanup_confirmation_retry_and_session_fence",
        true,
    );
}
fn fixture_process(name: &str, with_cleanup: bool) {
    const CHILD: &str = "BUZZ_HOSTED_PHASE_TEST_HOME";
    if let Some(home) = std::env::var_os(CHILD) {
        return native_phases(std::path::Path::new(&home), with_cleanup);
    }
    let home = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            &format!("commands::agents::hosted::tests::{name}"),
            "--nocapture",
        ])
        .env(CHILD, home.path())
        .env("HOME", home.path())
        .env("XDG_DATA_HOME", home.path())
        .env("APPDATA", home.path())
        .env("LOCALAPPDATA", home.path())
        .env_remove("BUZZ_NATIVE_HOST_DEPLOYMENT_V1")
        .env_remove("BUZZ_PRIVATE_KEY")
        .env_remove("BUZZ_AUTH_TAG")
        .env_remove("BUZZ_RELAY_URL")
        .env_remove("BUZZ_NETWORK_TRACE")
        .output()
        .unwrap();
    assert!(
        output.status.success() && String::from_utf8_lossy(&output.stdout).contains("1 passed"),
        "child failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
fn native_phases(home: &std::path::Path, with_cleanup: bool) {
    crate::managed_agents::storage::NO_KEYCHAIN_FOR_TEST
        .store(true, std::sync::atomic::Ordering::SeqCst);
    crate::managed_agents::pin_nest_dir_for_test(home.join("nest"));
    let mut context = tauri::test::mock_context(tauri::test::noop_assets());
    let app_data = home.join("app-data");
    context.config_mut().identifier = app_data.to_str().unwrap().to_owned();
    let mut state = crate::app_state::build_app_state();
    // Observe attempted publications only for the rejected drafts, then close
    // the fixture endpoint before the existing valid-create/recovery assertions.
    let publication_probe = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
    publication_probe.set_nonblocking(true).unwrap();
    let publication_address = publication_probe.local_addr().unwrap();
    let community = format!("https://localhost:{}", publication_address.port());
    let relay = format!("wss://localhost:{}", publication_address.port());
    // Deployment scopes require a canonical HTTPS domain, not an IP literal.
    // Keep that valid fixture domain pinned to loopback, without proxy/redirects.
    state.http_client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .resolve("localhost", publication_address)
        .timeout(std::time::Duration::from_secs(1))
        .build()
        .unwrap();
    *state.relay_url_override.lock().unwrap() = Some(relay.clone());
    let app = tauri::test::mock_builder()
        .manage(state)
        .build(context)
        .unwrap();
    assert_eq!(app.path().app_data_dir().unwrap(), app_data);
    tauri::async_runtime::block_on(async {
        let state = app.state::<AppState>();
        let owner = state.signing_keys().unwrap();
        let host = Keys::generate();
        let scope = buzz_core_pkg::paired_message::Scope {
            community: community.clone(),
            owner: owner.public_key(),
            host: host.public_key(),
            generation: "ab".repeat(32),
        };
        d::validate_scope(&scope).unwrap();
        let input = || {
            serde_json::from_value::<CreateManagedAgentRequest>(json!({
            "name":"Isolated hosted agent", "personaId":"one-definition", "systemPrompt":"A safe fixture prompt", "agentCommand":"grok", "agentArgs":["agent","--always-approve","stdio"],
            "backend":{"type":"registered_host","scope":scope}, "relayUrl":relay,
            "spawnAfterCreate":false, "startOnAppLaunch":false
        })).unwrap()
        };
        assert!(hosted_setup_token().is_err());
        assert!(create(
            input(),
            app.handle().clone(),
            &state,
            scope.clone(),
            "bc".repeat(32),
            "native/grok".into()
        )
        .await
        .unwrap_err()
        .contains("disabled"));
        assert!(load_managed_agents(app.handle()).unwrap().is_empty());
        let status = d::Status {
            inventory: Some(
                [(
                    "native/grok".into(),
                    buzz_core_pkg::harness::RuntimeFacts {
                        availability: buzz_core_pkg::harness::AcpAvailabilityStatus::Available,
                        version: None,
                        config_supported: true,
                    },
                )]
                .into(),
            ),
            scope: scope.clone(),
            availability: d::Availability::Current,
            capability: d::Capability::Supported {
                catalog_revision: "cd".repeat(32),
                runtimes: vec!["native/grok".into()],
                harnesses: [("native/grok".into(), "grok".into())].into(),
            },
            expires_at: nostr::Timestamp::now().as_secs() + 120,
        };
        let definition: crate::managed_agents::AgentDefinition = serde_json::from_value(json!({
            "id":"one-definition", "display_name":"Isolated hosted agent", "system_prompt":"A safe fixture prompt",
            "runtime":"grok", "created_at":"fixture", "updated_at":"fixture"
        })).unwrap();
        crate::managed_agents::save_personas(app.handle(), &[definition]).unwrap();
        let path = home.join("isolated-owner.sqlite");
        let mut context = Create {
            session: Session::capture(&state, &scope.community, &scope.owner.to_hex())
                .await
                .unwrap(),
            device: Device::at(&path).unwrap(),
            scope: scope.clone(),
            operation: "bc".repeat(32),
            runtime: "native/grok".into(),
            status,
            draft: serde_json::to_value(input()).unwrap(),
        };
        let valid_status = context.status.clone();
        let retention_scope =
            crate::managed_agents::retention::active_retention_scope(app.handle(), &state).unwrap();
        let retention =
            crate::managed_agents::retention::open_retention_db(&retention_scope.db_path).unwrap();
        assert!(
            crate::managed_agents::retention::get_pending_sync(&retention)
                .unwrap()
                .is_empty()
        );
        for scenario in [
            "missing-inventory",
            "missing-runtime",
            "unavailable",
            "unsupported",
        ] {
            context.status = valid_status.clone();
            match scenario {
                "missing-inventory" => context.status.inventory = None,
                "missing-runtime" => context.status.inventory.as_mut().unwrap().clear(),
                "unavailable" => {
                    context
                        .status
                        .inventory
                        .as_mut()
                        .unwrap()
                        .get_mut("native/grok")
                        .unwrap()
                        .availability = buzz_core_pkg::harness::AcpAvailabilityStatus::NotInstalled
                }
                "unsupported" => {
                    context
                        .status
                        .inventory
                        .as_mut()
                        .unwrap()
                        .get_mut("native/grok")
                        .unwrap()
                        .config_supported = false
                }
                _ => unreachable!(),
            }
            let error = create_managed_agent_phases(
                input(),
                app.handle().clone(),
                &state,
                Some(&mut context),
            )
            .await
            .err()
            .unwrap();
            assert!(
                error.contains("Current supported host runtime inventory unavailable"),
                "{scenario}: {error}"
            );
            assert!(
                context
                    .device
                    .deployment_intents(&scope.community, &owner)
                    .unwrap()
                    .is_empty(),
                "{scenario}: retained intent"
            );
            assert!(
                load_managed_agents(app.handle()).unwrap().is_empty(),
                "{scenario}: saved agent"
            );
            assert!(
                crate::managed_agents::retention::get_pending_sync(&retention)
                    .unwrap()
                    .is_empty(),
                "{scenario}: queued publication"
            );
            assert_eq!(
                publication_probe.accept().err().unwrap().kind(),
                std::io::ErrorKind::WouldBlock,
                "{scenario}: publication attempted"
            );
            assert!(state.managed_agent_processes.lock().unwrap().is_empty());
        }
        context.status = valid_status;
        drop(publication_probe);
        // The valid case keeps the existing closed-loopback publication failure
        // boundary. No host/provider process or live endpoint is reached.
        let first =
            create_managed_agent_phases(input(), app.handle().clone(), &state, Some(&mut context))
                .await
                .unwrap();
        assert!(first.profile_sync_error.is_some());
        assert!(first.spawn_error.is_none());
        let saved = context.record().unwrap();
        assert_eq!(first.agent.pubkey, saved.pubkey);
        assert!(saved.auth_tag.is_some());
        assert_eq!(context.intent().unwrap().phase, 1);
        assert!(state.managed_agent_processes.lock().unwrap().is_empty());
        // A concurrent losing key candidate cannot get past journal insertion.
        assert!(create_managed_agent_phases(
            input(),
            app.handle().clone(),
            &state,
            Some(&mut context)
        )
        .await
        .is_err());
        assert_eq!(load_managed_agents(app.handle()).unwrap().len(), 1);
        assert_eq!(
            context.record().unwrap().private_key_nsec,
            saved.private_key_nsec
        );
        let public = commands::retained_setup(
            saved.clone(),
            &context.intent().unwrap(),
            context.runtime.clone(),
        );
        let public = serde_json::to_string(&public).unwrap();
        for secret in [
            saved.private_key_nsec.as_str(),
            saved.auth_tag.as_deref().unwrap(),
        ] {
            assert!(!public.contains(secret));
        }
        assert!(!public.contains("ciphertext"));
        // Remove only this fixture's main record to simulate interrupted save;
        // reopen the real journal and recover its original key/profile inputs.
        save_managed_agents(app.handle(), &[]).unwrap();
        context.device = Device::at(&path).unwrap();
        assert!(context
            .recover(app.handle(), &state)
            .await
            .unwrap()
            .is_some());
        assert_eq!(load_managed_agents(app.handle()).unwrap().len(), 1);
        assert_eq!(
            context.record().unwrap().private_key_nsec,
            saved.private_key_nsec
        );
        // An interrupted follow-on has journaled the new model but has not
        // replaced the existing main record. Reopen and recover that intent,
        // preserving the original private identity and rejecting the old root.
        let root_operation = context.operation.clone();
        let prior = context.intent().unwrap();
        let mut snapshot = context.snapshot().unwrap();
        snapshot["record"]["model"] = "opaque/model:fixture".into();
        let d::Command::Deploy {
            mut config,
            identity,
        } = wire::restore_command(
            &owner,
            &prior.protected.authority,
            &prior.protected.ciphertext,
        )
        .unwrap()
        else {
            panic!("fixture deployment")
        };
        config.model = d::ModelSelection::Selected {
            id: "opaque/model:fixture".into(),
        };
        let mut authority = prior.protected.authority.clone();
        authority.operation_id = "ef".repeat(32);
        authority.expected_revision = 1;
        authority.content_sha256.clear();
        let (authority, event) = wire::seal_command(
            &owner,
            authority,
            &d::Command::Deploy { config, identity },
            nostr::Timestamp::now().as_secs(),
        )
        .unwrap();
        let plain = serde_json::to_vec(&json!({"authority":authority,"data":snapshot})).unwrap();
        let materialization = nostr::nips::nip44::encrypt(
            owner.secret_key(),
            &owner.public_key(),
            plain,
            nostr::nips::nip44::Version::V2,
        )
        .unwrap();
        let retained = json!({"protected":{"authority":authority,"ciphertext":event.content.as_bytes()},"event":serde_json::to_string(&event).unwrap(),"materialization":materialization,"phase":0,"result":null});
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute(
                "INSERT INTO deployment_intents VALUES(?,?,?,?)",
                rusqlite::params![
                    scope.community,
                    scope.owner.to_hex(),
                    authority.operation_id,
                    retained.to_string()
                ],
            )
            .unwrap();
        context.device = Device::at(&path).unwrap();
        assert!(context
            .recover(app.handle(), &state)
            .await
            .unwrap_err()
            .contains("newer retained"));
        context.operation = authority.operation_id;
        assert!(context
            .recover(app.handle(), &state)
            .await
            .unwrap()
            .is_some());
        let recovered = load_managed_agents(app.handle()).unwrap();
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].model.as_deref(), Some("opaque/model:fixture"));
        assert_eq!(recovered[0].private_key_nsec, saved.private_key_nsec);
        assert_eq!(recovered[0].auth_tag, saved.auth_tag);
        assert_eq!(context.intent().unwrap().phase, 1);
        context.operation = root_operation;
        assert!(context.recover(app.handle(), &state).await.is_err());
        context.operation = "ef".repeat(32);
        if with_cleanup {
            cleanup_tests::exercise(
                app.handle(),
                &state,
                &mut context,
                &host,
                &path,
                &retention_scope.db_path,
            )
            .await;
            return;
        }
        state
            .workspace_apply_generation
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        assert!(context.recover(app.handle(), &state).await.is_err());
        assert_eq!(load_managed_agents(app.handle()).unwrap().len(), 1);
    });
}

#[path = "agents_hosted_cleanup_tests.rs"]
mod cleanup_tests;
