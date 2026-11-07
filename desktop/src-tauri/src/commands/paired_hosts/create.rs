//! The original device resumes an existing signed tap while this process is
//! alive. A new app session discovers and resumes its own journal on opening Agents.
use super::*;
use buzz_paired_owner::{Draft, HistoryView, Policy, RequestView};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, LazyLock, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::{Mutex as AsyncMutex, Semaphore};

type SharedDevice = Arc<AsyncMutex<Device>>;
type LiveKey = (PathBuf, u64, String);
static LIVE: LazyLock<Mutex<HashMap<LiveKey, (Instant, SharedDevice)>>> =
    LazyLock::new(Default::default);
type JobKey = (LiveKey, String);
static JOBS: LazyLock<Mutex<HashSet<JobKey>>> = LazyLock::new(Default::default);
static NETWORK: Semaphore = Semaphore::const_new(4);

fn key(app: &AppHandle, s: &Session<'_>, scope: &Scope) -> Result<LiveKey> {
    Ok((
        journal_dir(app)?,
        s.generation,
        serde_json::to_string(scope)?,
    ))
}
pub(super) fn live_device(s: &Session<'_>, app: &AppHandle, scope: &Scope) -> Result<SharedDevice> {
    s.keys()?;
    let k = key(app, s, scope)?;
    let mut live = LIVE
        .lock()
        .map_err(|_| anyhow!("host session unavailable"))?;
    live.retain(|_, (last, handle)| {
        last.elapsed() < Duration::from_secs(120) || Arc::strong_count(handle) > 1
    });
    if let Some((last, handle)) = live.get_mut(&k) {
        *last = Instant::now();
        return Ok(Arc::clone(handle));
    }
    ensure!(live.len() < 64, "too many current host sessions");
    let d = device(app)?;
    d.view(&s.origin, &s.keys()?, scope)?;
    let handle = Arc::new(AsyncMutex::new(d));
    live.insert(k, (Instant::now(), Arc::clone(&handle)));
    Ok(handle)
}

fn access(policy: &Policy) -> Result<()> {
    checked_access(policy, crate::managed_agents::owner_only_access_build())
}
fn checked_access(policy: &Policy, owner_only: bool) -> Result<()> {
    ensure!(
        !owner_only
            || (policy.respond_to == "owner-only" && policy.respond_to_allowlist.is_empty()),
        "This build requires owner-only agent access"
    );
    Ok(())
}
struct ActiveJob(JobKey);
impl Drop for ActiveJob {
    fn drop(&mut self) {
        if let Ok(mut jobs) = JOBS.lock() {
            jobs.remove(&self.0);
        }
    }
}

fn terminal(state: &str) -> bool {
    matches!(
        state,
        "ready" | "failed" | "expired" | "revoked-before-commit"
    )
}
fn note(error: &anyhow::Error) -> String {
    let reason = error.to_string();
    for prefix in [
        "membership unconfirmed for ",
        "membership no longer authorized for ",
        "public event denied for ",
    ] {
        if let Some(channel) = reason
            .strip_prefix(prefix)
            .filter(|id| uuid::Uuid::parse_str(id).is_ok())
        {
            return format!("Membership on {channel} has not been confirmed.");
        }
    }
    match reason.as_str() {
        "community unavailable" | "incomplete community response" => {
            "Community offline. Delivery is pending."
        }
        "community endpoint returned HTTP 401" | "community endpoint returned HTTP 403" => {
            "Community authorization was denied."
        }
        "registration superseded" | "agent host/policy conflict" | "agent name conflict" => {
            "Published agent information conflicts with the original request."
        }
        "candidate is an existing identity" | "candidate identity is already known" => {
            "The proposed agent key is already in use."
        }
        _ => "Waiting for verified host and community evidence.",
    }
    .into()
}

fn schedule(app: &AppHandle, s: &Session<'_>, scope: &Scope, r: &str) -> Result<()> {
    let job = (key(app, s, scope)?, r.to_owned());
    let mut jobs = JOBS
        .lock()
        .map_err(|_| anyhow!("request scheduler unavailable"))?;
    if jobs.contains(&job) {
        return Ok(());
    }
    ensure!(jobs.len() < 256, "too many pending requests");
    jobs.insert(job.clone());
    drop(jobs);
    let guard = ActiveJob(job);
    let app = app.clone();
    let scope = scope.clone();
    let generation = s.generation;
    let r = r.to_string();
    tauri::async_runtime::spawn(async move {
        let _guard = guard;
        let state = app.state::<AppState>();
        let s = Session {
            state: &state,
            origin: scope.community.clone(),
            owner: scope.owner,
            generation,
        };
        let Ok(mut d) = device(&app) else { return };
        let Ok(net) = s.endpoint() else { return };
        let started = Instant::now();
        let mut backoff = 1u64;
        while started.elapsed() < Duration::from_secs(900) {
            let Ok(permit) = NETWORK.acquire().await else {
                return;
            };
            let Ok(keys) = s.keys() else { return };
            let Ok(view) = d.request(&s.origin, &keys, &scope, &r) else {
                return;
            };
            if terminal(&view.state) {
                return;
            }
            if access(&view.policy).is_err() {
                let _ = d.note_issue(
                    &s.origin,
                    &keys,
                    &scope,
                    &r,
                    Some("This build requires owner-only agent access.".into()),
                );
                return;
            }
            drop(keys);
            let result = net.reconcile(&mut d, &|| s.keys(), &scope, &r).await;
            let Ok(keys) = s.keys() else { return };
            let (issue, finished) = match result {
                Ok(v) => {
                    backoff = 2;
                    (
                        None,
                        terminal(&v.state) || v.state == "expired-awaiting-host",
                    )
                }
                Err(e) => {
                    backoff = (2 * backoff).min(30);
                    (Some(note(&e)), view.state == "expired-awaiting-host")
                }
            };
            if d.note_issue(&s.origin, &keys, &scope, &r, issue).is_err() {
                return;
            }
            drop(permit);
            drop(keys);
            if finished {
                return;
            }
            tokio::time::sleep(Duration::from_secs(backoff)).await;
        }
    });
    Ok(())
}

/// Commit one original GUI tap locally and drive that same request to completion.
#[tauri::command]
pub async fn paired_host_create(
    app: AppHandle,
    state: State<'_, AppState>,
    expected_relay: String,
    expected_owner: String,
    host: String,
    generation: String,
    catalog_id: String,
    tap: String,
    draft: Draft,
) -> Result<RequestView, String> {
    async {
        let s = Session::capture(&state, &expected_relay, &expected_owner).await?;
        access(&draft.policy)?;
        let scope = s.scope(&host, &generation)?;
        let handle = live_device(&s, &app, &scope)?;
        let mut device = handle.lock().await;
        let request = s
            .endpoint()?
            .start_tap(&mut device, &|| s.keys(), &scope, &catalog_id, &tap, draft)
            .await?;
        let view = device.request(&s.origin, &s.keys()?, &scope, &request)?;
        schedule(&app, &s, &scope, &request)?;
        Ok(view)
    }
    .await
    .map_err(|e: anyhow::Error| e.to_string())
}

/// Resume only a locally persisted original tap. A remote history item is read-only.
#[tauri::command]
pub async fn paired_host_resume(
    app: AppHandle,
    state: State<'_, AppState>,
    expected_relay: String,
    expected_owner: String,
    host: String,
    generation: String,
    request_id: String,
) -> Result<RequestView, String> {
    async {
        let s = Session::capture(&state, &expected_relay, &expected_owner).await?;
        let scope = s.scope(&host, &generation)?;
        let v = device(&app)?.request(&s.origin, &s.keys()?, &scope, &request_id)?;
        if !terminal(&v.state) {
            access(&v.policy)?;
            schedule(&app, &s, &scope, &request_id)?;
        }
        Ok(v)
    }
    .await
    .map_err(|e: anyhow::Error| e.to_string())
}

#[tauri::command]
pub async fn paired_host_history_ids(
    app: AppHandle,
    state: State<'_, AppState>,
    expected_relay: String,
    expected_owner: String,
    host: String,
    generation: String,
    after: Option<String>,
) -> Result<(Vec<String>, Option<String>), String> {
    async {
        let s = Session::capture(&state, &expected_relay, &expected_owner).await?;
        let scope = s.scope(&host, &generation)?;
        device(&app)?.view(&s.origin, &s.keys()?, &scope)?;
        s.endpoint()?
            .history_ids(&|| s.keys(), &scope, after.as_deref())
            .await
    }
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn paired_host_history(
    app: AppHandle,
    state: State<'_, AppState>,
    expected_relay: String,
    expected_owner: String,
    host: String,
    generation: String,
    request_id: String,
) -> Result<HistoryView, String> {
    async {
        let s = Session::capture(&state, &expected_relay, &expected_owner).await?;
        let scope = s.scope(&host, &generation)?;
        device(&app)?.view(&s.origin, &s.keys()?, &scope)?;
        s.endpoint()?
            .history(&|| s.keys(), &scope, &request_id)
            .await
    }
    .await
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unchanged_wider_tap_is_rejected_when_resumed_in_an_owner_only_build() {
        let mut policy = Policy {
            respond_to: "anyone".into(),
            respond_to_allowlist: vec![],
            channel_add_policy: "owner_only".into(),
        };
        assert!(checked_access(&policy, false).is_ok());
        assert!(checked_access(&policy, true).is_err());
        policy.respond_to = "owner-only".into();
        assert!(checked_access(&policy, true).is_ok());
    }
    #[test]
    fn private_desktop_inventory_reads_the_real_instance_store_without_keyring_or_public_profile() {
        let tmp = tempfile::tempdir().unwrap();
        let mut context = tauri::test::mock_context(tauri::test::noop_assets());
        context.config_mut().identifier = tmp.path().to_str().unwrap().to_owned();
        let app = tauri::test::mock_builder().build(context).unwrap();
        assert_eq!(app.path().app_data_dir().unwrap(), tmp.path());
        let hidden = Keys::generate().public_key();
        let path = crate::managed_agents::managed_agents_store_path(app.handle()).unwrap();
        std::fs::write(path,serde_json::to_vec(&serde_json::json!([{"name":"Unpublished","pubkey":hidden.to_hex()},{"name":"Definition","pubkey":""}])).unwrap()).unwrap();
        assert_eq!(
            crate::managed_agents::managed_agent_identity_keys(app.handle()).unwrap(),
            [hidden]
        );
    }
    #[tokio::test]
    async fn captured_session_rejects_lock_and_workspace_generation_switch() {
        let state = crate::app_state::build_app_state();
        *state.relay_url_override.lock().unwrap() = Some("wss://paired.example".into());
        let owner = state.signing_keys().unwrap().public_key().to_hex();
        assert!(Session::capture(&state, "wss://wrong.example", &owner)
            .await
            .is_err());
        assert!(Session::capture(
            &state,
            "wss://paired.example",
            &Keys::generate().public_key().to_hex()
        )
        .await
        .is_err());
        let session = Session::capture(&state, "wss://paired.example", &owner)
            .await
            .unwrap();
        assert!(session.keys().is_ok());
        state.reset_failed.store(true, Ordering::Release);
        assert!(session.keys().is_err());
        state.reset_failed.store(false, Ordering::Release);
        let guard = state.workspace_apply_lock.lock().await;
        assert!(session.keys().is_err());
        drop(guard);
        *state.relay_url_override.lock().unwrap() = Some("wss://other.example".into());
        assert!(session.keys().is_err());
        *state.relay_url_override.lock().unwrap() = Some("wss://paired.example".into());
        let original = state.signing_keys().unwrap();
        *state.keys.lock().unwrap() = Keys::generate();
        assert!(session.keys().is_err());
        *state.keys.lock().unwrap() = original;
        state.keyring_locked.store(true, Ordering::Release);
        assert!(session.keys().is_err());
        state.keyring_locked.store(false, Ordering::Release);
        state
            .workspace_apply_generation
            .fetch_add(1, Ordering::Release);
        assert!(session.keys().is_err());
    }
}
