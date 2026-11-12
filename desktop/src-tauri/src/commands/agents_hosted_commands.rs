//! Narrow, secret-free WebView projections of the native hosted setup owner.
use super::*;
use buzz_core_pkg::{host_deployment::observation::Observation, paired_message::Scope};
use serde::Serialize;

/// Secret-free retained setup state exposed to the WebView.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostedSetup {
    pub(super) scope: Scope,
    pub(super) operation: String,
    pub(super) runtime: String,
    pub(super) pubkey: String,
    pub(super) persona_id: Option<String>,
    pub(super) name: String,
    pub(super) phase: u8,
    pub(super) model: Option<String>,
    pub(super) state: String,
    pub(super) unit: Option<String>,
    pub(super) revision: Option<u64>,
    pub(super) observation: Option<Observation>,
    pub(super) error: Option<String>,
    pub(super) action: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
/// Approved installed ID with canonical Rust harness metadata.
pub struct HostedRuntime {
    installed_id: String,
    metadata: Option<crate::managed_agents::AcpRuntimeCatalogEntry>,
    facts: Option<buzz_core_pkg::harness::RuntimeFacts>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
/// Current native host capability and its supported metadata projection.
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
        action: if intent.protected.authority.action == d::Action::Cleanup {
            "cleanup"
        } else {
            "deploy"
        }
        .into(),
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
            response.unit = if read.current { read.unit } else { None };
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
                        if create.check_runtime().is_ok()
                            && create
                                .status
                                .permits(
                                    &create.scope,
                                    &config.catalog_revision,
                                    &config.runtime_id,
                                    nostr::Timestamp::now().as_secs(),
                                )
                                .is_ok()
                            && observation
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
        Err(error) => {
            let category = buzz_paired_owner::deployment::operation_error_category(&error);
            response.error.get_or_insert_with(|| {
                format!("Current deployment evidence unavailable or read denied ({category})")
            });
        }
    }
    create.check()?;
    Ok(response)
}

/// Mint one opaque operation token after the native feature gate.
#[tauri::command]
pub fn hosted_setup_token() -> Result<String, String> {
    enabled()?;
    Ok(Keys::generate().secret_key().to_secret_hex())
}
/// Read current scoped capability without local harness discovery.
#[tauri::command]
pub async fn hosted_catalog(
    app: AppHandle,
    state: State<'_, AppState>,
    scope: Scope,
) -> Result<HostedCatalog, String> {
    let (create, _) = open(&app, &state, scope, String::new(), String::new()).await?;
    catalog(&create)
}

/// Read independent host models before any identity, profile or placement write.
#[tauri::command]
pub async fn hosted_models(
    app: AppHandle,
    state: State<'_, AppState>,
    scope: Scope,
    runtime: String,
    catalog_revision: String,
) -> Result<buzz_core_pkg::host_registry::HostModels, String> {
    let (create, _) = open(&app, &state, scope.clone(), String::new(), runtime.clone()).await?;
    create
        .status
        .permits(
            &scope,
            &catalog_revision,
            &runtime,
            nostr::Timestamp::now().as_secs(),
        )
        .map_err(|_| "Host runtime changed or unavailable".to_string())?;
    let signer = || create.session.keys();
    let mut registry =
        buzz_paired_owner::registry::Registry::connect_guarded(&scope.community, &signer)
            .await
            .map_err(|_| "Host discovery unavailable".to_string())?;
    let result = registry
        .models(&signer, &scope, &runtime, &catalog_revision)
        .await;
    let closed = registry.disconnect().await;
    create.session.keys().map_err(|e| e.to_string())?;
    let result = result.map_err(|_| "Host models unavailable or timed out".to_string())?;
    closed.map_err(|_| "Host discovery connection failed".to_string())?;
    Ok(result)
}

pub(super) fn catalog(create: &Create<'_>) -> Result<HostedCatalog, String> {
    let runtimes = match &create.status.capability {
        d::Capability::Supported {
            runtimes,
            harnesses,
            ..
        } => runtimes
            .iter()
            .map(|id| HostedRuntime {
                installed_id: id.clone(),
                facts: create
                    .status
                    .inventory
                    .as_ref()
                    .and_then(|m| m.get(id))
                    .cloned(),
                metadata: harnesses
                    .get(id)
                    .and_then(|key| crate::managed_agents::hosted_runtime_metadata(key))
                    .and_then(|mut metadata| {
                        let facts = create.status.inventory.as_ref()?.get(id)?;
                        if !facts.config_supported {
                            return None;
                        }
                        metadata.availability = facts.availability.clone();
                        metadata.auth_status = crate::managed_agents::AuthStatus::Unknown;
                        if let Some(version) = &facts.version {
                            metadata.label = format!("{} · {version}", metadata.label);
                        }
                        Some(metadata)
                    }),
            })
            .collect(),
        _ => Vec::new(),
    };
    // Capability reads have no deployment operation or retained intent.
    create.session.keys().map_err(|e| e.to_string())?;
    Ok(HostedCatalog {
        status: create.status.clone(),
        runtimes,
    })
}
/// Prepare through the existing main create phases and return a sanitized setup.
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
/// List protected local setups without making per-agent network requests.
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
    let selected = crate::relay::relay_api_base_url_with_override(&state);
    let intents = device
        .deployment_intents(&selected, &keys)
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
    for intent in &intents {
        let a = &intent.protected.authority;
        let key = (
            a.scope.host.to_hex(),
            a.scope.generation.clone(),
            a.agent.to_hex(),
        );
        if let Some(last) = intents
            .iter()
            .filter(|other| {
                other.protected.authority.scope == a.scope
                    && other.protected.authority.agent == a.agent
                    && other.protected.authority.action == d::Action::Cleanup
            })
            .max_by_key(|other| {
                (
                    other.protected.authority.expected_revision,
                    other.protected.authority.expires_at,
                    &other.protected.authority.operation_id,
                )
            })
        {
            if last.protected.authority.operation_id != a.operation_id {
                continue;
            }
        }
        if newest.get(&key) != Some(&a.expected_revision) {
            continue;
        }
        if a.action == d::Action::Cleanup {
            if intent.phase < 4 {
                setups.push(cleanup::retained(&device, &keys, &intent)?);
            }
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
/// Explicit continuation actions; the native owner supplies revisions and keys.
pub enum HostedAction {
    Check,
    Cleanup,
    Retry,
    Refresh,
    Launch { model: String },
}
/// Reconcile and continue the latest exact scoped setup under current permission.
#[tauri::command]
pub async fn hosted_setup_action(
    app: AppHandle,
    state: State<'_, AppState>,
    scope: Scope,
    operation: String,
    runtime: String,
    action: HostedAction,
) -> Result<HostedSetup, String> {
    enabled()?;
    let session = Session::capture(&state, &scope.community, &scope.owner.to_hex())
        .await
        .map_err(|e| e.to_string())?;
    let device = crate::commands::paired_hosts::device(&app).map_err(|e| e.to_string())?;
    let keys = session.keys().map_err(|e| e.to_string())?;
    let root = device
        .deployment_intent(&scope.community, &keys, &operation)
        .map_err(|e| e.to_string())?
        .ok_or("Hosted setup not found")?;
    if root.protected.authority.scope != scope {
        return Err("Hosted scope changed".into());
    }
    if matches!(action, HostedAction::Cleanup) || cleanup::has_cleanup(&device, &keys, &root)? {
        if matches!(action, HostedAction::Refresh | HostedAction::Launch { .. }) {
            return Err("Cleanup retained; deployment cannot continue".into());
        }
        return cleanup::run(
            &app,
            &state,
            scope,
            operation,
            !matches!(action, HostedAction::Check),
        )
        .await;
    }
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
