//! Both Delete entry points share runtime-independent confirmed Cleanup.
use super::*;
use buzz_paired_owner::deployment::Intent;

pub(super) fn retained(
    device: &Device,
    keys: &Keys,
    intent: &Intent,
) -> Result<HostedSetup, String> {
    let prior = device
        .cleanup_predecessor(&intent.protected.authority.scope.community, keys, intent)
        .map_err(|e| e.to_string())?;
    let d::Command::Deploy { config, .. } = wire::restore_command(
        keys,
        &prior.protected.authority,
        &prior.protected.ciphertext,
    )
    .map_err(|e| e.to_string())?
    else {
        return Err("Invalid Cleanup predecessor".into());
    };
    let record = restored_record(
        device,
        keys,
        &prior,
        &prior.protected.authority.scope,
        &config.runtime_id,
    )?;
    let mut setup = commands::retained_setup(record, intent, config.runtime_id);
    setup.state = if intent.phase == 4 {
        "deleted"
    } else {
        "cleanup-pending"
    }
    .into();
    Ok(setup)
}

pub(super) fn has_cleanup(device: &Device, keys: &Keys, root: &Intent) -> Result<bool, String> {
    Ok(device
        .deployment_intents(&root.protected.authority.scope.community, keys)
        .map_err(|e| e.to_string())?
        .iter()
        .any(|i| {
            i.protected.authority.scope == root.protected.authority.scope
                && i.protected.authority.agent == root.protected.authority.agent
                && i.protected.authority.action == d::Action::Cleanup
        }))
}

pub(super) async fn run<R: tauri::Runtime>(
    app: &AppHandle<R>,
    state: &AppState,
    scope: buzz_core_pkg::paired_message::Scope,
    operation: String,
    send: bool,
) -> Result<HostedSetup, String> {
    enabled()?;
    d::validate_scope(&scope).map_err(|e| e.to_string())?;
    let session = Session::capture(state, &scope.community, &scope.owner.to_hex())
        .await
        .map_err(|e| e.to_string())?;
    let mut device = crate::commands::paired_hosts::device(app).map_err(|e| e.to_string())?;
    let mut management = Management::connect_guarded(&scope.community, true, &|| session.keys())
        .await
        .map_err(|e| e.to_string())?;
    coordinate(
        app,
        state,
        &session,
        &mut device,
        &mut management,
        &scope,
        &operation,
        send,
    )
    .await
}

// The injectable existing Device/Management seam also exercises native tests.
pub(super) async fn coordinate<R: tauri::Runtime>(
    app: &AppHandle<R>,
    state: &AppState,
    session: &Session<'_>,
    device: &mut Device,
    management: &mut Management,
    scope: &buzz_core_pkg::paired_message::Scope,
    operation: &str,
    send: bool,
) -> Result<HostedSetup, String> {
    let keys = session.keys().map_err(|e| e.to_string())?;
    let root = device
        .deployment_intent(&scope.community, &keys, operation)
        .map_err(|e| e.to_string())?
        .ok_or("Missing hosted intent")?;
    if root.protected.authority.scope != *scope {
        return Err("Hosted scope changed".into());
    }
    let pubkey = root.protected.authority.agent.to_hex();
    let before = {
        let _guard = state
            .managed_agents_store_lock
            .lock()
            .map_err(|e| e.to_string())?;
        session.keys().map_err(|e| e.to_string())?;
        load_managed_agents(app)?
            .into_iter()
            .find(|r| r.pubkey == pubkey)
            .map(|r| serde_json::to_value(r).map_err(|e| e.to_string()))
            .transpose()?
    };
    let plan = management
        .plan_cleanup(device, &|| session.keys(), operation, send)
        .await
        .map_err(|e| e.to_string())?;
    let intent = {
        let _guard = state
            .managed_agents_store_lock
            .lock()
            .map_err(|e| e.to_string())?;
        let keys = session.keys().map_err(|e| e.to_string())?;
        device
            .retain_cleanup(&scope.community, &keys, plan)
            .map_err(|e| e.to_string())?
    };
    let id = &intent.protected.authority.operation_id;
    let mut response = retained(device, &session.keys().map_err(|e| e.to_string())?, &intent)?;
    if send {
        if let Err(error) = management.dispatch(device, &|| session.keys(), id).await {
            response.error = Some(format!("Cleanup request retained: {error}"));
        }
    }
    session.keys().map_err(|e| e.to_string())?;
    match management
        .cleanup_state(device, &|| session.keys(), id)
        .await
    {
        Ok((_, true)) => {
            response.state = "cleanup-local-pending".into();
            let result = finish(
                app,
                state,
                session,
                device,
                &intent,
                before,
                #[cfg(test)]
                || Ok(()),
            );
            match result {
                Ok(()) => {
                    response.state = "deleted".into();
                    response.phase = 4;
                    response.error = None;
                }
                Err(error) => {
                    response.error = Some(format!(
                        "Host cleanup confirmed; retry local removal: {error}"
                    ))
                }
            }
        }
        Ok((state, false)) => {
            response.state = if state == "failed" {
                "cleanup-failed"
            } else {
                "cleanup-pending"
            }
            .into();
        }
        Err(error) => {
            response.state = if error.to_string().contains("conflict") {
                "conflict"
            } else {
                "unavailable"
            }
            .into();
            response.error = Some(format!("Cleanup confirmation unavailable: {error}"));
        }
    }
    let keys = session.keys().map_err(|e| e.to_string())?;
    response.phase = device
        .deployment_intent(&scope.community, &keys, id)
        .map_err(|e| e.to_string())?
        .ok_or("Missing Cleanup intent")?
        .phase;
    Ok(response)
}

pub(super) fn finish<R: tauri::Runtime>(
    app: &AppHandle<R>,
    state: &AppState,
    session: &Session<'_>,
    device: &mut Device,
    intent: &Intent,
    before: Option<serde_json::Value>,
    #[cfg(test)] before_remove: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    session.with_local_commit(|keys| {
        let _guard = state
            .managed_agents_store_lock
            .lock()
            .map_err(|e| e.to_string())?;
        let a = &intent.protected.authority;
        let pubkey = a.agent.to_hex();
        let mut records = load_managed_agents(app)?;
        let current = records.iter().find(|r| r.pubkey == pubkey);
        if current
            .map(|r| serde_json::to_value(r).map_err(|e| e.to_string()))
            .transpose()?
            != before
        {
            return Err("Saved agent changed during Cleanup; retry local removal".into());
        }
        if current.is_some_and(|r| {
            r.backend
                != (BackendKind::RegisteredHost {
                    scope: a.scope.clone(),
                })
        }) {
            return Err("Hosted destination changed".into());
        }
        // Both writer guards remain held through record/key removal, enqueue and
        // phase 4. Derive the destination from the confirmed intent and captured
        // signer, never from a later read of the globally active workspace.
        let relay_url = a.scope.community.replacen("https://", "wss://", 1);
        let scope = crate::managed_agents::retention::RetentionScope {
            db_path: crate::managed_agents::retention::scoped_retention_db_path(
                &managed_agents_base_dir(app)?,
                &relay_url,
                &keys.public_key().to_hex(),
            ),
            relay_url,
            owner_keys: keys.clone(),
        };
        std::fs::create_dir_all(
            scope
                .db_path
                .parent()
                .ok_or("Missing retention directory")?,
        )
        .map_err(|e| e.to_string())?;
        finalize_managed_agent_deletion(app, state, &pubkey, &mut records, Some(&scope), |_| {
            #[cfg(test)]
            before_remove()?;
            Ok(())
        })?;
        device
            .deployment_advance(&a.scope.community, keys, &a.operation_id, 4)
            .map_err(|e| e.to_string())?;
        Ok(())
    })?;
    try_regenerate_nest(app);
    Ok(())
}

pub(in super::super) async fn delete_if_hosted<R: tauri::Runtime>(
    app: &AppHandle<R>,
    state: &AppState,
    pubkey: &str,
) -> Result<bool, String> {
    let record = {
        let _guard = state
            .managed_agents_store_lock
            .lock()
            .map_err(|e| e.to_string())?;
        load_managed_agents(app)?
            .into_iter()
            .find(|r| r.pubkey == pubkey)
    };
    if record
        .as_ref()
        .is_some_and(|r| !matches!(r.backend, BackendKind::RegisteredHost { .. }))
    {
        return Ok(false);
    }
    if record.is_none() && enabled().is_err() {
        return Ok(false);
    }
    let keys = state.signing_keys()?;
    let community = crate::relay::relay_api_base_url_with_override(state);
    let device = crate::commands::paired_hosts::device(app).map_err(|e| e.to_string())?;
    let intents = device
        .deployment_intents(&community, &keys)
        .map_err(|e| e.to_string())?;
    let root = intents.iter().rev().find(|i| {
        i.protected.authority.agent.to_hex() == pubkey
            && record.as_ref().is_none_or(|r| {
                r.backend
                    == (BackendKind::RegisteredHost {
                        scope: i.protected.authority.scope.clone(),
                    })
            })
    });
    let Some(root) = root else {
        return if record.is_some() {
            Err("No retained hosted setup; identity preserved".into())
        } else {
            Ok(false)
        };
    };
    let response = run(
        app,
        state,
        root.protected.authority.scope.clone(),
        root.protected.authority.operation_id.clone(),
        true,
    )
    .await?;
    if response.state != "deleted" {
        return Err(response.error.unwrap_or_else(|| {
            "Host cleanup pending; use Check status or Retry in agent setup".into()
        }));
    }
    Ok(true)
}
