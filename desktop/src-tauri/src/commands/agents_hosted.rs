//! Default-off native hosted setup; identity and recovery stay outside the WebView.
//! Main owns key generation, resolution, attestation, record and publications.
use super::*;
use crate::commands::paired_hosts::Session;
use buzz_core_pkg::host_deployment::{self as d, wire};
use buzz_paired_owner::{deployment::Management, Device};
use serde_json::json;

pub(super) struct Create<'a> {
    session: Session<'a>,
    device: Device,
    scope: buzz_core_pkg::paired_message::Scope,
    operation: String,
    runtime: String,
    status: d::Status,
    draft: serde_json::Value,
}
impl Create<'_> {
    pub(super) fn check(&self) -> Result<(), String> {
        let keys = self.session.keys().map_err(|e| e.to_string())?;
        if let Some(intent) = self
            .device
            .deployment_intent(&self.scope.community, &keys, &self.operation)
            .map_err(|e| e.to_string())?
        {
            self.device
                .deployment_open(&self.scope.community, &keys, &intent.protected.authority)
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub(super) fn advance(&mut self, phase: u8) -> Result<(), String> {
        let keys = self.session.keys().map_err(|e| e.to_string())?;
        let intent = self
            .device
            .deployment_intent(&self.scope.community, &keys, &self.operation)
            .map_err(|e| e.to_string())?
            .ok_or("Missing hosted create intent")?;
        if intent.phase < phase {
            self.device
                .deployment_advance(&self.scope.community, &keys, &self.operation, phase)
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    pub(super) fn prepare<R: tauri::Runtime>(
        &mut self,
        app: &AppHandle<R>,
        state: &AppState,
        record: &ManagedAgentRecord,
        about: Option<String>,
    ) -> Result<(), String> {
        let keys = self.session.keys().map_err(|e| e.to_string())?;
        // This is the same effective launch projection used by main's remote
        // deployment path. Only explicitly supported structured fields cross over.
        let payload = build_deploy_payload(app, state, record)?;
        let d::Capability::Supported { harnesses, .. } = &self.status.capability else {
            return Err("Host runtime metadata unavailable".into());
        };
        let metadata = harnesses
            .get(&self.runtime)
            .and_then(|id| crate::managed_agents::hosted_runtime_metadata(id))
            .ok_or("Unsupported host runtime mapping")?;
        let command = metadata.command.ok_or("Unsupported host harness")?;
        if payload["launch"]["command"] != command
            || record.acp_command != DEFAULT_ACP_COMMAND
            || record.agent_args != metadata.default_args
            || record.session_policy != crate::managed_agents::AcpSessionPolicy::default()
            || !record.env_vars.is_empty()
            || !payload["env_vars"]
                .as_object()
                .is_some_and(|e| e.is_empty())
            || !payload["provider"].is_null()
            || record.idle_timeout_seconds.is_some()
            || record.max_turn_duration_seconds.is_some()
        {
            return Err("Hosted deployment does not support these command, environment, provider or timeout overrides.".into());
        }
        let d::Capability::Supported {
            catalog_revision, ..
        } = &self.status.capability
        else {
            return Err("Host deployment unavailable".into());
        };
        let config = d::Deploy {
            catalog_revision: catalog_revision.clone(),
            runtime_id: self.runtime.clone(),
            name: record.name.clone(),
            policy: buzz_core_pkg::paired_message::Policy {
                respond_to: payload["respond_to"]
                    .as_str()
                    .ok_or("Missing response policy")?
                    .into(),
                respond_to_allowlist: serde_json::from_value(
                    payload["respond_to_allowlist"].clone(),
                )
                .map_err(|e| e.to_string())?,
                channel_add_policy: "owner_only".into(),
            },
            // New identity has no native authentication/model catalog yet.
            model: d::ModelSelection::PendingLogin {},
            provider: None,
            system_prompt: serde_json::from_value(payload["system_prompt"].clone())
                .map_err(|e| e.to_string())?,
            effort: record.effort_level.as_ref().map(|e| e.to_string()),
            parallelism: payload["parallelism"]
                .as_u64()
                .ok_or("Missing concurrency")? as u32,
        };
        let authority = d::Authority {
            v: 1,
            scope: self.scope.clone(),
            agent: nostr::PublicKey::from_hex(&record.pubkey).map_err(|e| e.to_string())?,
            action: d::Action::Deploy,
            operation_id: self.operation.clone(),
            expected_revision: 0,
            expires_at: nostr::Timestamp::now().as_secs() + 600,
            content_sha256: String::new(),
        };
        let command = d::Command::Deploy {
            config: Box::new(config),
            identity: buzz_core_pkg::private_managed_agent::PrivateIdentity {
                private_key_nsec: record.private_key_nsec.clone(),
                auth_tag: record.auth_tag.clone(),
            },
        };
        let mut snapshot = serde_json::to_value(record).map_err(|e| e.to_string())?;
        snapshot
            .as_object_mut()
            .ok_or("Invalid main record")?
            .remove("private_key_nsec");
        self.device
            .prepare_deployment(
                &self.scope.community,
                &keys,
                authority,
                &command,
                &self.status,
                json!({"record":snapshot,"about":about,"draft":self.draft}),
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[path = "agents_hosted_cleanup.rs"]
pub(super) mod cleanup;

#[path = "agents_hosted_commands.rs"]
mod commands;
use commands::setup;
pub use commands::*;

fn enabled() -> Result<(), String> {
    if std::env::var("BUZZ_NATIVE_HOST_DEPLOYMENT_V1").as_deref() != Ok("1") {
        return Err("Native host deployment disabled".into());
    }
    Ok(())
}

async fn open<'a, R: tauri::Runtime>(
    app: &AppHandle<R>,
    state: &'a AppState,
    scope: buzz_core_pkg::paired_message::Scope,
    operation: String,
    runtime: String,
) -> Result<(Create<'a>, Management), String> {
    enabled()?;
    d::validate_scope(&scope).map_err(|e| e.to_string())?;
    let session = Session::capture(state, &scope.community, &scope.owner.to_hex())
        .await
        .map_err(|e| e.to_string())?;
    let mut management = Management::connect_guarded(&scope.community, true, &|| session.keys())
        .await
        .map_err(|e| e.to_string())?;
    let reply = management
        .query(
            &|| session.keys(),
            &wire::Query {
                v: 1,
                scope: scope.clone(),
                view: "status".into(),
                nonce: Keys::generate().secret_key().to_secret_hex(),
                operation_id: None,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    let keys = session.keys().map_err(|e| e.to_string())?;
    let event: nostr::Event = serde_json::from_str(
        reply.data["event"]
            .as_str()
            .ok_or("Host status unavailable")?,
    )
    .map_err(|e| e.to_string())?;
    let status = wire::open_status(&keys, &event, &scope, nostr::Timestamp::now().as_secs())
        .map_err(|e| e.to_string())?;
    let device = crate::commands::paired_hosts::device(app).map_err(|e| e.to_string())?;
    Ok((
        Create {
            session,
            device,
            scope,
            operation,
            runtime,
            status,
            draft: serde_json::Value::Null,
        },
        management,
    ))
}

impl Create<'_> {
    fn intent(&self) -> Result<buzz_paired_owner::deployment::Intent, String> {
        self.device
            .deployment_intent(
                &self.scope.community,
                &self.session.keys().map_err(|e| e.to_string())?,
                &self.operation,
            )
            .map_err(|e| e.to_string())?
            .ok_or("Hosted setup not found".into())
    }
    fn snapshot(&self) -> Result<serde_json::Value, String> {
        self.device
            .restore_materialization(
                &self.scope.community,
                &self.session.keys().map_err(|e| e.to_string())?,
                &self.intent()?,
            )
            .map_err(|e| e.to_string())
    }
    /// Validate the selected host facts without requiring a generated identity or record.
    pub(super) fn check_inventory(
        &self,
    ) -> Result<crate::managed_agents::AcpRuntimeCatalogEntry, String> {
        self.check()?;
        let d::Capability::Supported {
            harnesses,
            catalog_revision,
            ..
        } = &self.status.capability
        else {
            return Err("Host catalog unsupported".into());
        };
        self.status
            .permits(
                &self.scope,
                catalog_revision,
                &self.runtime,
                nostr::Timestamp::now().as_secs(),
            )
            .map_err(|e| e.to_string())?;
        if !self
            .status
            .inventory
            .as_ref()
            .and_then(|facts| facts.get(&self.runtime))
            .is_some_and(|facts| {
                facts.config_supported
                    && facts.availability
                        == buzz_core_pkg::harness::AcpAvailabilityStatus::Available
            })
        {
            return Err("Current supported host runtime inventory unavailable".into());
        }
        let metadata = harnesses
            .get(&self.runtime)
            .and_then(|id| crate::managed_agents::hosted_runtime_metadata(id))
            .ok_or("Host harness mapping unavailable")?;
        Ok(metadata)
    }
    fn check_runtime(&self) -> Result<(), String> {
        let metadata = self.check_inventory()?;
        let record = self.record()?;
        if metadata.command.as_deref() != Some(&record.agent_command)
            || metadata.default_args != record.agent_args
        {
            return Err("Host harness mapping changed".into());
        }
        Ok(())
    }
    fn record(&self) -> Result<ManagedAgentRecord, String> {
        restored_record(
            &self.device,
            &self.session.keys().map_err(|e| e.to_string())?,
            &self.intent()?,
            &self.scope,
            &self.runtime,
        )
    }
    async fn latest(&mut self, management: &mut Management) -> Result<(), String> {
        let root = self.intent()?;
        let keys = self.session.keys().map_err(|e| e.to_string())?;
        let mut candidates: Vec<_> = self
            .device
            .deployment_intents(&self.scope.community, &keys)
            .map_err(|e| e.to_string())?
            .into_iter()
            .filter(|i| {
                i.protected.authority.scope == self.scope
                    && i.protected.authority.agent == root.protected.authority.agent
            })
            .collect();
        let revision = candidates
            .iter()
            .map(|i| i.protected.authority.expected_revision)
            .max()
            .ok_or("Missing setup")?;
        candidates.retain(|i| i.protected.authority.expected_revision == revision);
        if candidates.len() == 1 {
            self.operation = candidates.remove(0).protected.authority.operation_id;
            return Ok(());
        }
        let mut winner = None;
        let mut expired_absent: Option<(u64, String)> = None;
        for intent in candidates {
            let op = &intent.protected.authority.operation_id;
            let read = management
                .operation(&mut self.device, &|| self.session.keys(), op)
                .await
                .map_err(|e| e.to_string())?;
            if (read.admitted && !read.current)
                || (!read.admitted && (intent.phase >= 3 || intent.result.is_some()))
            {
                return Err("Placement conflict or admission unresolved".into());
            }
            if !read.admitted
                && intent.protected.authority.expires_at <= nostr::Timestamp::now().as_secs()
            {
                let candidate = (intent.protected.authority.expires_at, op.clone());
                if expired_absent.as_ref().is_none_or(|old| &candidate > old) {
                    expired_absent = Some(candidate);
                }
            }
            if read.current
                || (!read.admitted
                    && intent.protected.authority.expires_at > nostr::Timestamp::now().as_secs())
            {
                if winner.is_some() {
                    return Err("Conflicting retained setup operations".into());
                }
                winner = Some(op.clone());
            }
        }
        self.operation = winner
            .or_else(|| expired_absent.map(|(_, op)| op))
            .ok_or("Setup admission unresolved; no current intent")?;
        Ok(())
    }
    async fn recover<R: tauri::Runtime>(
        &mut self,
        app: &AppHandle<R>,
        state: &AppState,
    ) -> Result<Option<String>, String> {
        let record = self.record()?;
        let intent = self.intent()?;
        {
            let _guard = state
                .managed_agents_store_lock
                .lock()
                .map_err(|e| e.to_string())?;
            self.check()?;
            let current = &intent.protected.authority;
            let keys = self.session.keys().map_err(|e| e.to_string())?;
            if self
                .device
                .deployment_intents(&self.scope.community, &keys)
                .map_err(|e| e.to_string())?
                .iter()
                .any(|i| {
                    let a = &i.protected.authority;
                    a.scope == current.scope
                        && a.agent == current.agent
                        && a.operation_id != current.operation_id
                        && (a.expected_revision > current.expected_revision
                            || (a.expected_revision == current.expected_revision
                                && (a.expires_at > nostr::Timestamp::now().as_secs()
                                    || i.phase >= 3)))
                })
            {
                return Err("A newer retained setup must be recovered first".into());
            }
            let mut records = load_managed_agents(app)?;
            if let Some(existing) = records.iter_mut().find(|r| r.pubkey == record.pubkey) {
                let mut before = serde_json::to_value(&*existing).map_err(|e| e.to_string())?;
                let after = serde_json::to_value(&record).map_err(|e| e.to_string())?;
                // Only continuation's journaled model update may differ.
                if intent.protected.authority.expected_revision > 0 {
                    before["model"] = after["model"].clone();
                }
                if before != after {
                    return Err("Saved setup configuration changed".into());
                }
                *existing = record.clone();
                save_managed_agents(app, &records)?;
            } else {
                persist_created_record(app, &mut records, record.clone())?;
            }
            retain_managed_agent_pending(app, state, &record);
            self.advance(1)?;
        }
        if self.intent()?.phase >= 2 {
            return Ok(None);
        }
        self.check()?;
        let keys = Keys::parse(&record.private_key_nsec).map_err(|e| e.to_string())?;
        let about = self.snapshot()?["about"].as_str().map(str::to_string);
        let error = profile::publish_agent_profile_with_about(
            state,
            &record.relay_url,
            &keys,
            &record.name,
            record.avatar_url.as_deref(),
            about.as_deref(),
            record.auth_tag.as_deref(),
        )
        .await;
        self.check()?;
        let error = super::super::agent_models::flush_managed_agent_policy(app, state, error).await;
        self.check()?;
        if error.is_none() {
            self.advance(2)?;
        }
        Ok(error)
    }
}

async fn create<R: tauri::Runtime>(
    mut input: CreateManagedAgentRequest,
    app: AppHandle<R>,
    state: &AppState,
    scope: buzz_core_pkg::paired_message::Scope,
    operation: String,
    runtime: String,
) -> Result<HostedSetup, String> {
    enabled()?;
    if !buzz_core_pkg::host_registry::identifier(&operation)
        || operation.starts_with(wire::CLEANUP_OPERATION_PREFIX)
    {
        return Err("Invalid setup token".into());
    }
    let (mut create, mut management) = open(&app, state, scope, operation, runtime).await?;
    create.draft = serde_json::to_value(&input).map_err(|e| e.to_string())?;
    let keys = create.session.keys().map_err(|e| e.to_string())?;
    if create
        .device
        .deployment_intent(&create.scope.community, &keys, &create.operation)
        .map_err(|e| e.to_string())?
        .is_none()
    {
        input.destination = None;
        input.backend = BackendKind::RegisteredHost {
            scope: create.scope.clone(),
        };
        input.relay_url = Some(create.scope.community.replacen("https://", "wss://", 1));
        input.spawn_after_create = false;
        input.start_on_app_launch = false;
        // Main owns generation, attestation, validation, save and profile. A
        // concurrent token loser is never persisted: prepare refuses it first.
        let result =
            create_managed_agent_phases(input, app.clone(), state, Some(&mut create)).await;
        if let Ok(response) = &result {
            if response.profile_sync_error.is_none() {
                create.advance(2)?;
            }
        }
        if let Err(error) = result {
            if create
                .device
                .deployment_intent(&create.scope.community, &keys, &create.operation)
                .map_err(|e| e.to_string())?
                .is_none()
            {
                return Err(error);
            }
        }
    }
    let snapshot = create.snapshot()?;
    if snapshot["draft"] != create.draft {
        return Err("Prepared form inputs changed; continue retained setup".into());
    }
    create.latest(&mut management).await?;
    create.check_runtime()?;
    let error = match create.recover(&app, state).await {
        Ok(None) => management
            .dispatch(
                &mut create.device,
                &|| create.session.keys(),
                &create.operation,
            )
            .await
            .err()
            .map(|_| "Deployment publication failed; retained setup can retry".to_string()),
        Ok(Some(_)) => Some("Profile publication failed; retained setup can retry".into()),
        Err(_) => Some("Main record recovery failed; retained setup can retry".into()),
    };
    setup(&mut create, &mut management, error).await
}

fn restored_record(
    device: &Device,
    keys: &Keys,
    intent: &buzz_paired_owner::deployment::Intent,
    scope: &buzz_core_pkg::paired_message::Scope,
    runtime: &str,
) -> Result<ManagedAgentRecord, String> {
    if intent.protected.authority.scope != *scope {
        return Err("Hosted setup scope changed".into());
    }
    let d::Command::Deploy { identity, config } = wire::restore_command(
        keys,
        &intent.protected.authority,
        &intent.protected.ciphertext,
    )
    .map_err(|e| e.to_string())?
    else {
        return Err("Not a setup intent".into());
    };
    if config.runtime_id != runtime {
        return Err("Hosted runtime changed".into());
    }
    let mut value = device
        .restore_materialization(&scope.community, keys, intent)
        .map_err(|e| e.to_string())?["record"]
        .clone();
    value["private_key_nsec"] = identity.private_key_nsec.into();
    let record: ManagedAgentRecord = serde_json::from_value(value).map_err(|e| e.to_string())?;
    if record.backend
        != (BackendKind::RegisteredHost {
            scope: scope.clone(),
        })
        || record.pubkey != intent.protected.authority.agent.to_hex()
        || record.auth_tag != identity.auth_tag
        || record.relay_url != scope.community.replacen("https://", "wss://", 1)
    {
        return Err("Recovered destination changed".into());
    }
    Ok(record)
}

#[cfg(test)]
#[path = "agents_hosted_tests.rs"]
mod tests;
