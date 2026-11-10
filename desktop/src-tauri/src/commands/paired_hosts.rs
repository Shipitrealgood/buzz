//! Windows paired-host commands. WebView parameters can narrow the active
//! native workspace. They never supply a network destination or signing key.
use crate::{app_state::AppState, relay};
use anyhow::{anyhow, ensure, Result};
use buzz_paired_owner::{
    registry::{Registry, Unsupported},
    Device, Endpoint, HostView, Scope,
};
use nostr::{Keys, PublicKey};
use serde::Serialize;
use std::{path::PathBuf, sync::atomic::Ordering};
use tauri::{AppHandle, Manager, State};
use zeroize::Zeroizing;
mod create;
pub use create::*;

fn journal_dir<R: tauri::Runtime>(app: &AppHandle<R>) -> Result<PathBuf> {
    Ok(app.path().app_local_data_dir()?.join("paired-hosts"))
}
/// The ordinary Windows app data directory can roam between devices. Delete
/// this separate, device-local journal on the same explicit profile reset.
pub(crate) fn reset_paired_host_journal(app: &AppHandle) -> Result<()> {
    match std::fs::remove_dir_all(journal_dir(app)?) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}
pub(crate) fn device<R: tauri::Runtime>(app: &AppHandle<R>) -> Result<Device> {
    let dir = journal_dir(app)?;
    buzz_paired_owner::protect_windows_journal(
        &dirs::data_local_dir().ok_or_else(|| anyhow!("local application data unavailable"))?,
        &dir,
    )?;
    let app = app.clone();
    Ok(
        Device::at(&dir.join("owner.sqlite"))?.with_local_identities(move || {
            let state = app.state::<AppState>();
            let _guard = state
                .managed_agents_store_lock
                .lock()
                .map_err(|_| anyhow!("agent inventory unavailable"))?;
            crate::managed_agents::managed_agent_identity_keys(&app).map_err(anyhow::Error::msg)
        }),
    )
}
pub(crate) struct Session<'a> {
    state: &'a AppState,
    origin: String,
    owner: PublicKey,
    generation: u64,
}
impl<'a> Session<'a> {
    pub(crate) async fn capture(state: &'a AppState, expected: &str, owner: &str) -> Result<Self> {
        ensure!(
            !expected.trim().is_empty() && !owner.is_empty(),
            "select a community and an unlocked identity"
        );
        let _guard = state.workspace_apply_lock.lock().await;
        let origin = relay::relay_api_base_url_with_override(state);
        relay::assert_expected_relay_scope(Some(expected), &origin).map_err(anyhow::Error::msg)?;
        let keys = state.signing_keys().map_err(anyhow::Error::msg)?;
        ensure!(
            keys.public_key().to_hex() == owner,
            "active identity changed"
        );
        ensure!(
            !state.reset_failed.load(Ordering::Acquire),
            "identity reset incomplete"
        );
        Ok(Self {
            state,
            origin,
            owner: keys.public_key(),
            generation: state.workspace_apply_generation.load(Ordering::Acquire),
        })
    }
    pub(crate) fn keys(&self) -> Result<Keys> {
        let _guard = self
            .state
            .workspace_apply_lock
            .try_lock()
            .map_err(|_| anyhow!("community switch in progress"))?;
        ensure!(
            !self.state.reset_failed.load(Ordering::Acquire),
            "identity reset incomplete"
        );
        ensure!(
            self.state
                .workspace_apply_generation
                .load(Ordering::Acquire)
                == self.generation
                && relay::relay_api_base_url_with_override(self.state) == self.origin,
            "active community changed"
        );
        let keys = self.state.signing_keys().map_err(anyhow::Error::msg)?;
        ensure!(keys.public_key() == self.owner, "active signer changed");
        Ok(keys)
    }
    fn endpoint(&self) -> Result<Endpoint> {
        Endpoint::new(&self.origin, self.owner)
    }
    fn scope(&self, host: &str, generation: &str) -> Result<Scope> {
        let key = PublicKey::from_hex(host).map_err(|_| anyhow!("invalid host key"))?;
        ensure!(
            key.to_hex() == host
                && generation.len() == 64
                && generation
                    .bytes()
                    .all(|v| v.is_ascii_digit() || (b'a'..=b'f').contains(&v)),
            "invalid host coordinate"
        );
        Ok(Scope {
            community: self.origin.clone(),
            generation: generation.to_owned(),
            owner: self.owner,
            host: key,
        })
    }
}
#[derive(Serialize)]
pub struct HostDirectory {
    supported: bool,
    available: bool,
    hosts: Vec<HostView>,
}

impl Session<'_> {
    async fn directory(&self, app: &AppHandle) -> Result<HostDirectory> {
        let mut device = device(app)?;
        let local = device.registry_hosts(&self.origin, &self.keys()?)?;
        if !self.origin.starts_with("https://") {
            return Ok(HostDirectory {
                supported: false,
                available: true,
                hosts: vec![],
            });
        }
        let mut registry = match Registry::connect_guarded(&self.origin, &|| self.keys()).await {
            Ok(registry) => registry,
            Err(error) => {
                self.keys()?;
                return Ok(HostDirectory {
                    supported: !error.is::<Unsupported>(),
                    available: error.is::<Unsupported>(),
                    hosts: local,
                });
            }
        };
        let result = registry
            .synchronize_guarded(&mut device, &|| self.keys())
            .await;
        self.keys()?;
        Ok(HostDirectory {
            supported: true,
            available: result.is_ok(),
            hosts: device.registry_hosts(&self.origin, &self.keys()?)?,
        })
    }
}

#[tauri::command]
pub async fn paired_host_list(
    app: AppHandle,
    state: State<'_, AppState>,
    expected_relay: String,
    expected_owner: String,
) -> Result<HostDirectory, String> {
    async {
        Session::capture(&state, &expected_relay, &expected_owner)
            .await?
            .directory(&app)
            .await
    }
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn paired_host_pair(
    app: AppHandle,
    state: State<'_, AppState>,
    expected_relay: String,
    expected_owner: String,
    invitation: String,
) -> Result<HostView, String> {
    let invitation = Zeroizing::new(invitation);
    async {
        let session = Session::capture(&state, &expected_relay, &expected_owner).await?;
        let mut registry = Registry::connect_guarded(&session.origin, &|| session.keys()).await?;
        let mut device = device(&app)?;
        let (scope, claim) =
            device.pair_registry(&session.origin, &session.keys()?, &invitation)?;
        registry.publish(&session.keys()?, claim).await?;
        device.view(&session.origin, &session.keys()?, &scope)
    }
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn paired_host_refresh(
    app: AppHandle,
    state: State<'_, AppState>,
    expected_relay: String,
    expected_owner: String,
    host: String,
    generation: String,
) -> Result<HostView, String> {
    async {
        let session = Session::capture(&state, &expected_relay, &expected_owner).await?;
        let scope = session.scope(&host, &generation)?;
        let mut registry = Registry::connect_guarded(&session.origin, &|| session.keys()).await?;
        let mut device = device(&app)?;
        registry
            .recover_guarded(&mut device, &|| session.keys())
            .await?;
        // A registration refresh does not certify connectivity or installed runtimes.
        device.view(&session.origin, &session.keys()?, &scope)
    }
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn paired_host_unpair(
    app: AppHandle,
    state: State<'_, AppState>,
    expected_relay: String,
    expected_owner: String,
    host: String,
    generation: String,
) -> Result<HostView, String> {
    async {
        let session = Session::capture(&state, &expected_relay, &expected_owner).await?;
        let scope = session.scope(&host, &generation)?;
        let mut device = device(&app)?;
        let event = device.unpair_registry(&session.origin, &session.keys()?, &scope)?;
        // Restrictive intent is already durable. A failed connection keeps the
        // visible pending state; listing retries these exact signed bytes.
        if let Ok(mut registry) =
            Registry::connect_guarded(&session.origin, &|| session.keys()).await
        {
            registry.publish(&session.keys()?, event).await.ok();
        }
        device.view(&session.origin, &session.keys()?, &scope)
    }
    .await
    .map_err(|e| e.to_string())
}
