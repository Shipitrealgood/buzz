//! Default-off native entry point. UI creation remains blocked in agents.rs.
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
}
impl Create<'_> {
    pub(super) fn check(&self) -> Result<(), String> {
        self.session.keys().map(|_| ()).map_err(|e| e.to_string())
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
        let id = self
            .runtime
            .strip_prefix("acp/")
            .ok_or("Unsupported host runtime namespace")?;
        let command =
            crate::managed_agents::command_for_runtime_id(id).ok_or("Unknown main runtime")?;
        if payload["launch"]["command"] != command
            || record.acp_command != DEFAULT_ACP_COMMAND
            || record.agent_args != normalize_agent_args(&command, Vec::new())
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
                json!({"record":snapshot,"about":about}),
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}
/// Native-only callable path for the next UI slice and isolated native testing.
/// No command registration or WebView key transport is added here.
#[allow(dead_code)]
pub(crate) async fn create<R: tauri::Runtime>(
    mut input: CreateManagedAgentRequest,
    app: AppHandle<R>,
    state: &AppState,
    scope: buzz_core_pkg::paired_message::Scope,
    operation: String,
    runtime: String,
) -> Result<CreateManagedAgentResponse, String> {
    if std::env::var("BUZZ_NATIVE_HOST_DEPLOYMENT_V1").as_deref() != Ok("1") {
        return Err("Native host deployment disabled".into());
    }
    if !buzz_core_pkg::host_registry::identifier(&operation)
        || operation.starts_with(d::wire::CLEANUP_OPERATION_PREFIX)
    {
        return Err("Invalid Deploy operation ID: cleanup namespace is reserved".into());
    }
    let session = Session::capture(state, &scope.community, &scope.owner.to_hex())
        .await
        .map_err(|e| e.to_string())?;
    let keys = session.keys().map_err(|e| e.to_string())?;
    let device = crate::commands::paired_hosts::device(&app).map_err(|e| e.to_string())?;
    let mut management = Management::connect_guarded(&scope.community, true, &|| session.keys())
        .await
        .map_err(|e| e.to_string())?;
    let q = wire::Query {
        v: 1,
        scope: scope.clone(),
        view: "status".into(),
        nonce: Keys::generate().secret_key().to_secret_hex(),
        operation_id: None,
    };
    let reply = management
        .query(&|| session.keys(), &q)
        .await
        .map_err(|e| e.to_string())?;
    let status: nostr::Event = serde_json::from_str(
        reply.data["event"]
            .as_str()
            .ok_or("Host status unavailable")?,
    )
    .map_err(|e| e.to_string())?;
    let status = wire::open_status(&keys, &status, &scope, nostr::Timestamp::now().as_secs())
        .map_err(|e| e.to_string())?;
    let mut create = Create {
        session,
        device,
        scope,
        operation,
        runtime,
        status,
    };
    let intent = create
        .device
        .deployment_intent(&create.scope.community, &keys, &create.operation)
        .map_err(|e| e.to_string())?;
    let response = if let Some(intent) = intent {
        // Reopening after expiry still recovers this exact key/record. Dispatch
        // then reports pending admission rather than regenerating an identity.
        let snapshot = create
            .device
            .restore_materialization(&create.scope.community, &keys, &intent)
            .map_err(|e| e.to_string())?;
        let command = wire::restore_command(
            &keys,
            &intent.protected.authority,
            &intent.protected.ciphertext,
        )
        .map_err(|e| e.to_string())?;
        let d::Command::Deploy { identity, .. } = command else {
            return Err("Invalid create intent".into());
        };
        let mut value = snapshot["record"].clone();
        value["private_key_nsec"] = identity.private_key_nsec.clone().into();
        let record: ManagedAgentRecord =
            serde_json::from_value(value).map_err(|e| e.to_string())?;
        if record.backend
            != (BackendKind::RegisteredHost {
                scope: create.scope.clone(),
            })
            || record.pubkey != intent.protected.authority.agent.to_hex()
            || record.auth_tag != identity.auth_tag
            || record.relay_url != create.scope.community.replacen("https://", "wss://", 1)
        {
            return Err("Recovered destination changed".into());
        }
        let agent = {
            let _guard = state
                .managed_agents_store_lock
                .lock()
                .map_err(|e| e.to_string())?;
            create.session.keys().map_err(|e| e.to_string())?;
            let mut records = load_managed_agents(&app)?;
            persist_created_record(&app, &mut records, record.clone())?;
            retain_managed_agent_pending(&app, state, &record);
            let runtimes = state
                .managed_agent_processes
                .lock()
                .map_err(|e| e.to_string())?;
            summarize_from_disk(&app, &record, &runtimes)?
        };
        create.advance(1)?;
        let agent_keys = Keys::parse(&identity.private_key_nsec).map_err(|e| e.to_string())?;
        create.check()?;
        let error = profile::publish_agent_profile_with_about(
            state,
            &record.relay_url,
            &agent_keys,
            &record.name,
            record.avatar_url.as_deref(),
            snapshot["about"].as_str(),
            record.auth_tag.as_deref(),
        )
        .await;
        let error =
            super::super::agent_models::flush_managed_agent_policy(&app, state, error).await;
        CreateManagedAgentResponse {
            agent,
            private_key_nsec: identity.private_key_nsec,
            profile_sync_error: error,
            spawn_error: None,
        }
    } else {
        input.destination = None;
        input.backend = BackendKind::RegisteredHost {
            scope: create.scope.clone(),
        };
        input.relay_url = Some(create.scope.community.replacen("https://", "wss://", 1));
        input.spawn_after_create = false;
        create_managed_agent_phases(input, app, state, Some(&mut create)).await?
    };
    if response.profile_sync_error.is_none() {
        create.advance(2)?;
        management
            .dispatch(
                &mut create.device,
                &|| create.session.keys(),
                &create.operation,
            )
            .await
            .map_err(|e| format!("Hosted create retained; deployment pending: {e}"))?;
    }
    Ok(response)
}
