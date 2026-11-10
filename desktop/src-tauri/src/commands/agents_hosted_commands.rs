//! Narrow, secret-free WebView projections of the native hosted setup owner.
use super::*;
use buzz_core_pkg::{host_deployment::observation::Observation, paired_message::Scope};
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostedSetup {
    scope: Scope,
    operation: String,
    runtime: String,
    pubkey: String,
    persona_id: Option<String>,
    name: String,
    phase: u8,
    model: Option<String>,
    state: String,
    unit: Option<String>,
    revision: Option<u64>,
    observation: Option<Observation>,
    error: Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostedRuntime {
    installed_id: String,
    metadata: Option<crate::managed_agents::AcpRuntimeCatalogEntry>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostedCatalog {
    status: d::Status,
    runtimes: Vec<HostedRuntime>,
}

pub(super) fn retained_setup(
    record: ManagedAgentRecord,
    intent: &buzz_paired_owner::deployment::Intent,
    runtime: String,
) -> HostedSetup {
    HostedSetup {
        scope: intent.protected.authority.scope.clone(),
        operation: intent.protected.authority.operation_id.clone(),
        runtime,
        pubkey: record.pubkey,
        persona_id: record.persona_id,
        name: record.name,
        phase: intent.phase,
        model: record.model,
        state: "unverified".into(),
        unit: None,
        revision: None,
        observation: None,
        error: None,
    }
}

pub(super) async fn setup(
    create: &mut Create<'_>,
    management: &mut Management,
    error: Option<String>,
) -> Result<HostedSetup, String> {
    let record = create.record()?;
    let intent = create.intent()?;
    let mut response = retained_setup(record, &intent, create.runtime.clone());
    response.error = error;
    match management
        .operation(
            &mut create.device,
            &|| create.session.keys(),
            &create.operation,
        )
        .await
    {
        Ok(read) => {
            response.state = if read.admitted && !read.current {
                "conflict".into()
            } else {
                read.state
            };
            response.unit = read.unit;
            response.revision = read.revision;
            if read.current {
                if let Some(body) = read.body {
                    let keys = create.session.keys().map_err(|e| e.to_string())?;
                    let d::Command::Deploy { config, .. } = wire::restore_command(
                        &keys,
                        &intent.protected.authority,
                        &intent.protected.ciphertext,
                    )
                    .map_err(|e| e.to_string())?
                    else {
                        return Err("Not a deployment".into());
                    };
                    if let Some(observation) = body.observation {
                        if observation
                            .current(&body.result, &config, nostr::Timestamp::now().as_secs())
                            .is_ok()
                        {
                            response.observation = Some(observation);
                        } else {
                            response.error.get_or_insert(
                                "Observation expired; refresh models explicitly".into(),
                            );
                        }
                    }
                }
            }
        }
        Err(_) => {
            response
                .error
                .get_or_insert("Current deployment evidence unavailable or read denied".into());
        }
    }
    create.check()?;
    Ok(response)
}

#[tauri::command]
pub fn hosted_setup_token() -> Result<String, String> {
    enabled()?;
    Ok(Keys::generate().secret_key().to_secret_hex())
}
#[tauri::command]
pub async fn hosted_catalog(
    app: AppHandle,
    state: State<'_, AppState>,
    scope: Scope,
) -> Result<HostedCatalog, String> {
    let (create, _) = open(&app, &state, scope, String::new(), String::new()).await?;
    let runtimes = match &create.status.capability {
        d::Capability::Supported {
            runtimes,
            harnesses,
            ..
        } => runtimes
            .iter()
            .map(|id| HostedRuntime {
                installed_id: id.clone(),
                metadata: harnesses
                    .get(id)
                    .and_then(|key| crate::managed_agents::hosted_runtime_metadata(key)),
            })
            .collect(),
        _ => Vec::new(),
    };
    create.check()?;
    Ok(HostedCatalog {
        status: create.status,
        runtimes,
    })
}
#[tauri::command]
pub async fn hosted_prepare(
    input: CreateManagedAgentRequest,
    app: AppHandle,
    state: State<'_, AppState>,
    scope: Scope,
    operation: String,
    runtime: String,
) -> Result<HostedSetup, String> {
    create(input, app, &state, scope, operation, runtime).await
}
#[tauri::command]
pub async fn hosted_setup_list(
    app: AppHandle,
    state: State<'_, AppState>,
    expected_relay: String,
    expected_owner: String,
) -> Result<Vec<HostedSetup>, String> {
    enabled()?;
    let session = Session::capture(&state, &expected_relay, &expected_owner)
        .await
        .map_err(|e| e.to_string())?;
    let device = crate::commands::paired_hosts::device(&app).map_err(|e| e.to_string())?;
    let keys = session.keys().map_err(|e| e.to_string())?;
    let intents = device
        .deployment_intents(&expected_relay, &keys)
        .map_err(|e| e.to_string())?;
    let mut newest = std::collections::BTreeMap::new();
    for intent in &intents {
        let a = &intent.protected.authority;
        let key = (
            a.scope.host.to_hex(),
            a.scope.generation.clone(),
            a.agent.to_hex(),
        );
        newest.insert(key, a.expected_revision);
    }
    let mut setups = Vec::new();
    // A local list grants no current authority. Reads/actions reconcile exact
    // candidates explicitly, so one unavailable host never hides other setups.
    for intent in intents {
        let a = &intent.protected.authority;
        let key = (
            a.scope.host.to_hex(),
            a.scope.generation.clone(),
            a.agent.to_hex(),
        );
        if newest.get(&key) != Some(&a.expected_revision) {
            continue;
        }
        let d::Command::Deploy { config, .. } =
            wire::restore_command(&keys, a, &intent.protected.ciphertext)
                .map_err(|e| e.to_string())?
        else {
            continue;
        };
        let record = restored_record(&device, &keys, &intent, &a.scope, &config.runtime_id)?;
        setups.push(retained_setup(record, &intent, config.runtime_id));
    }
    session.keys().map_err(|e| e.to_string())?;
    Ok(setups)
}
#[derive(serde::Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case", deny_unknown_fields)]
pub enum HostedAction {
    Check,
    Retry,
    Refresh,
    Launch { model: String },
}
#[tauri::command]
pub async fn hosted_setup_action(
    app: AppHandle,
    state: State<'_, AppState>,
    scope: Scope,
    operation: String,
    runtime: String,
    action: HostedAction,
) -> Result<HostedSetup, String> {
    let (mut create, mut management) = open(&app, &state, scope, operation, runtime).await?;
    create.latest(&mut management).await?;
    let mut error = None;
    if !matches!(action, HostedAction::Check) {
        create.check_runtime()?;
        if matches!(action, HostedAction::Refresh | HostedAction::Launch { .. }) {
            let mut snapshot = create.snapshot()?;
            let model = match action {
                HostedAction::Launch { model } => {
                    snapshot["record"]["model"] = model.clone().into();
                    d::ModelSelection::Selected { id: model }
                }
                _ => d::ModelSelection::PendingLogin {},
            };
            let next = management
                .continue_deployment(
                    &mut create.device,
                    &|| create.session.keys(),
                    &create.operation,
                    &create.status,
                    model,
                    snapshot,
                )
                .await
                .map_err(|e| e.to_string())?;
            create.operation = next.protected.authority.operation_id;
        }
        error = match create.recover(&app, &state).await {
            Ok(None) => management
                .dispatch(
                    &mut create.device,
                    &|| create.session.keys(),
                    &create.operation,
                )
                .await
                .err()
                .map(|_| "Deployment publication failed; retry exact retained intent".into()),
            Ok(Some(_)) => Some("Profile publication failed; retry retained setup".into()),
            Err(_) => Some("Main record recovery failed; retry retained setup".into()),
        };
    }
    setup(&mut create, &mut management, error).await
}
