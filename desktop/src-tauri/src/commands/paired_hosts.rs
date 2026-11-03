//! Windows paired-host commands. WebView parameters can narrow the active
//! native workspace. They never supply a network destination or signing key.
use crate::{app_state::AppState, relay};
use anyhow::{anyhow, ensure, Result};
use buzz_paired_owner::{Device, Endpoint, HostView, Scope};
use nostr::{Keys, PublicKey};
use serde::Serialize;
use std::{path::PathBuf, sync::atomic::Ordering};
use tauri::{AppHandle, Manager, State};
use zeroize::Zeroizing;

fn journal_dir(app: &AppHandle) -> Result<PathBuf> {
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
fn device(app: &AppHandle) -> Result<Device> {
    let dir = journal_dir(app)?;
    std::fs::create_dir_all(&dir)?;
    Device::at(&dir.join("owner.sqlite"))
}
struct Session<'a> {
    state: &'a AppState,
    origin: String,
    owner: PublicKey,
    generation: u64,
}
impl<'a> Session<'a> {
    async fn capture(state: &'a AppState, expected: &str, owner: &str) -> Result<Self> {
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
    fn keys(&self) -> Result<Keys> {
        let _guard = self
            .state
            .workspace_apply_lock
            .try_lock()
            .map_err(|_| anyhow!("community switch in progress"))?;
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
        let local = device.hosts(&self.origin, &self.keys()?)?;
        if !self.origin.starts_with("https://") {
            return Ok(HostDirectory {
                supported: false,
                available: true,
                hosts: vec![],
            });
        }
        let endpoint = self.endpoint()?;
        let cached = || HostDirectory {
            supported: !local.is_empty(),
            available: false,
            hosts: local.clone(),
        };
        match endpoint.supported().await {
            Ok(false) => {
                return Ok(HostDirectory {
                    supported: false,
                    available: true,
                    hosts: vec![],
                })
            }
            Err(_) => {
                self.keys()?;
                return Ok(cached());
            }
            Ok(true) => (),
        }
        let bindings = match endpoint.hosts(&self.keys()?).await {
            Ok(bindings) => bindings,
            Err(_) => {
                self.keys()?;
                return Ok(cached());
            }
        };
        for binding in bindings {
            device.restore(&self.origin, &self.keys()?, &binding)?;
        }
        let hosts = device.hosts(&self.origin, &self.keys()?)?;
        let mut available = true;
        // Retry exact saved ciphertext after an exit or interrupted HTTP response.
        for h in hosts
            .iter()
            .filter(|h| matches!(h.state.as_str(), "pair-pending" | "unpair-pending"))
            .take(8)
        {
            let p = if h.state == "pair-pending" {
                device.pending_claim(&self.origin, &self.keys()?, &h.scope)?
            } else {
                device.pending_unpair(&self.origin, &self.keys()?, &h.scope)?
            };
            if let Some(p) = p {
                let result = if h.state == "pair-pending" {
                    endpoint.claim(&self.keys()?, &h.scope, &p).await
                } else {
                    endpoint.post(&self.keys()?, &h.scope, &p).await
                };
                if result.is_err() {
                    available = false;
                }
            }
        }
        self.keys()?;
        Ok(HostDirectory {
            supported: true,
            available,
            hosts,
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
        let endpoint = session.endpoint()?;
        ensure!(
            endpoint.supported().await?,
            "community does not support paired hosts"
        );
        let mut device = device(&app)?;
        let (scope, claim) = device.pair(&session.origin, &session.keys()?, &invitation)?;
        endpoint.claim(&session.keys()?, &scope, &claim).await?;
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
        session
            .endpoint()?
            .refresh(&mut device(&app)?, &|| session.keys(), &scope)
            .await
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
        let p = device.unpair(&session.origin, &session.keys()?, &scope)?;
        let endpoint = session.endpoint()?;
        if endpoint.post(&session.keys()?, &scope, &p).await.is_ok() {
            if let Ok(binding) = endpoint.binding(&session.keys()?, &scope).await {
                device.restore(&session.origin, &session.keys()?, &binding)?;
            }
        }
        device.view(&session.origin, &session.keys()?, &scope)
    }
    .await
    .map_err(|e| e.to_string())
}
