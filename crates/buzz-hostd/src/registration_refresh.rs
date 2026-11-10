//! Registration scheduling only; every generation retains its own AUTH ceiling.
use anyhow::{ensure, Result};
use buzz_hostd::registration::{Host, RegistrationClock};
use buzz_ws_client::{NostrWsConnection, WsClientError};
use nostr::PublicKey;
use std::{collections::BTreeSet, path::Path, time::Duration};
use tokio::{
    task::{JoinSet, LocalSet},
    time::Instant,
};

const LOCAL_CHECK: Duration = Duration::from_secs(1);
const COALESCE: Duration = Duration::from_secs(5);
const BACKSTOP: Duration = Duration::from_secs(60);
const ATTEMPT: Duration = Duration::from_secs(45);
const MAX_SESSIONS: usize = 32;

#[derive(Clone)]
pub(super) struct Settings {
    pub community: String,
    pub url: String,
    pub relay: PublicKey,
    pub label: String,
    pub connector: Option<buzz_ws_client::Connector>,
    pub deployment: bool,
}
impl Settings {
    async fn connect(&self) -> Result<NostrWsConnection> {
        Ok(match &self.connector {
            Some(tls) => {
                NostrWsConnection::connect_with_tls_connector(&self.url, tls.clone()).await?
            }
            None => NostrWsConnection::connect(&self.url).await?,
        })
    }
}

pub(super) async fn run(dir: &Path, settings: Settings, once: bool) -> Result<()> {
    let helper = settings
        .deployment
        .then(|| buzz_hostd::deployment::Helper::installed(dir))
        .transpose()?;
    run_with_helper(dir, settings, once, helper).await
}

async fn run_with_helper(
    dir: &Path,
    settings: Settings,
    once: bool,
    helper: Option<buzz_hostd::deployment::Helper>,
) -> Result<()> {
    let mut host = Host::at(dir, false)?;
    if once {
        let mut failed = false;
        for g in host.work(&settings.community, &RegistrationClock::read()?)? {
            let result = tokio::time::timeout(ATTEMPT, async {
                let mut ws = settings.connect().await?;
                host.authenticate(&mut ws, &g).await?;
                host.reconcile(
                    &mut ws,
                    settings.relay,
                    &g,
                    &settings.label,
                    &RegistrationClock::read()?,
                )
                .await?;
                deployment(&mut host, &mut ws, &g, &settings, dir).await?;
                ws.disconnect().await?;
                Ok::<_, anyhow::Error>(())
            })
            .await;
            failed |= !matches!(result, Ok(Ok(())));
        }
        // One-shot registration discovery precedes bounded restrictive retries.
        if let Some(helper) = &helper {
            host.retry_deployment_cleanup(&settings.community, helper)
                .await?;
        }
        ensure!(
            !failed,
            "registration reconciliation incomplete; exact retry remains in journal"
        );
        return Ok(());
    }
    // SQLite handles are owned by individual local tasks. Network awaits in one
    // generation cannot starve another generation's heartbeat or local revoke.
    LocalSet::new().run_until(async {
        let mut tasks = JoinSet::new();
        let mut running = BTreeSet::new();
        let mut cleanups: JoinSet<(String, Result<()>)> = JoinSet::new();
        let mut cleaning = BTreeSet::new();
        let mut check = tokio::time::interval(LOCAL_CHECK);
        check.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                biased;
                result = tasks.join_next(), if !tasks.is_empty() => {
                    let (g, result) = result.ok_or_else(|| anyhow::anyhow!("missing registration task"))??;
                    running.remove(&g);
                    result?;
                }
                result = cleanups.join_next(), if !cleanups.is_empty() => {
                    let (g, result) = result.ok_or_else(|| anyhow::anyhow!("missing cleanup task"))??;
                    cleaning.remove(&g);
                    if result.is_err() { eprintln!("restrictive cleanup pending; journal retained"); }
                }
                _ = check.tick() => {
                    if let Some(helper) = &helper {
                        for g in host.deployment_cleanup_generations(&settings.community)? {
                            if cleaning.len() < 4 && cleaning.insert(g.clone()) {
                                let (dir, helper) = (dir.to_owned(), helper.clone());
                                cleanups.spawn_local(async move {
                                    let result = async { Host::at(&dir, false)?.cleanup_deployments(&g, &helper).await }.await;
                                    (g, result)
                                });
                            }
                        }
                    }
                    for g in host.work(&settings.community, &RegistrationClock::read()?)? {
                        // Retired invitations may still be in a bounded network
                        // await. Count them too, not just the latest work query.
                        if running.len() < MAX_SESSIONS && running.insert(g.clone()) {
                            let dir = dir.to_owned();
                            let settings = settings.clone();
                            tasks.spawn_local(async move {
                                let result = generation(&dir, &g, &settings).await;
                                (g, result)
                            });
                        }
                    }
                }
            }
        }
    }).await
}

fn retry_delay(failures: u32) -> Result<Duration> {
    ensure!(
        failures < 6,
        "registration unavailable after six bounded attempts; restart controller to retry"
    );
    let cap = (2u64 << failures.min(5)).min(60);
    Ok(Duration::from_millis(
        cap * 500 + rand::random::<u64>() % (cap * 500 + 1),
    ))
}

async fn generation(dir: &Path, g: &str, settings: &Settings) -> Result<()> {
    let mut host = Host::at(dir, false)?;
    let mut failures = 0;
    loop {
        if !host
            .work(&settings.community, &RegistrationClock::read()?)?
            .iter()
            .any(|work| work == g)
        {
            return Ok(());
        }
        let result = tokio::time::timeout(ATTEMPT, async {
            let mut ws = settings.connect().await?;
            host.authenticate(&mut ws, g).await?;
            // Restrictive and timely exact retries precede any potentially denied
            // read. All remaining authority still comes from protected snapshots.
            host.flush_pending(&mut ws, g).await?;
            if host.state(g)? == "dead" {
                return Ok(None);
            }
            host.subscribe_live(&mut ws, settings.relay, g).await?;
            Ok::<_, anyhow::Error>(Some(ws))
        })
        .await;
        if matches!(result, Ok(Ok(None))) {
            return Ok(());
        }
        if let Ok(Ok(Some(mut ws))) = result {
            let started = Instant::now();
            let backstop = if settings.deployment {
                COALESCE
            } else {
                BACKSTOP
            };
            let result = connected(&mut host, &mut ws, g, settings, backstop, COALESCE).await;
            if result.is_ok() {
                return Ok(());
            }
            // A successful handshake followed immediately by denial is still a
            // failed attempt. Reset only after a full backstop of healthy service.
            if started.elapsed() >= BACKSTOP {
                failures = 0;
            }
        }
        failures += 1;
        let delay = retry_delay(failures)?;
        eprintln!("registration unavailable; exact retry retained");
        tokio::time::sleep(delay).await;
    }
}

async fn connected(
    host: &mut Host,
    ws: &mut NostrWsConnection,
    g: &str,
    settings: &Settings,
    backstop: Duration,
    coalesce: Duration,
) -> Result<()> {
    let mut refresh = true;
    let mut next_refresh = Instant::now();
    let mut next_backstop = Instant::now() + backstop;
    let mut next_local = Instant::now();
    loop {
        let now = Instant::now();
        // Check timers before receiving another frame, so storms cannot starve
        // local revocation, expiry or the authoritative backstop.
        if now >= next_local {
            if !host
                .work(&settings.community, &RegistrationClock::read()?)?
                .iter()
                .any(|work| work == g)
            {
                return Ok(());
            }
            refresh |= host.has_pending(g)?;
            next_local = now + LOCAL_CHECK;
        }
        refresh |= host.take_live_change();
        if now >= next_backstop {
            refresh = true;
        }
        if refresh && now >= next_refresh {
            refresh = false;
            tokio::time::timeout(ATTEMPT, async {
                host.reconcile(
                    ws,
                    settings.relay,
                    g,
                    &settings.label,
                    &RegistrationClock::read()?,
                )
                .await
            })
            .await??;
            if settings.deployment {
                deployment(host, ws, g, settings, Path::new("/var/lib/buzz-hostd")).await?;
            }
            next_refresh = Instant::now() + coalesce;
            next_backstop = Instant::now() + backstop;
        }
        let mut deadline = next_local.min(next_backstop);
        if refresh {
            deadline = deadline.min(next_refresh);
        }
        // An outer deadline also bounds Ping/Pong floods in the shared client.
        match tokio::time::timeout_at(deadline, ws.next_event(Duration::from_secs(60))).await {
            Err(_) | Ok(Err(WsClientError::Timeout)) => {}
            Ok(message) => host.observe_live(&message?)?,
        }
    }
}

#[cfg(test)]
#[path = "registration_refresh_tests.rs"]
mod tests;

async fn deployment(
    host: &mut Host,
    ws: &mut NostrWsConnection,
    g: &str,
    settings: &Settings,
    dir: &Path,
) -> Result<()> {
    if !settings.deployment {
        return Ok(());
    }
    let helper = buzz_hostd::deployment::Helper::installed(dir)?;
    let (catalog, revision) = buzz_hostd::Catalog::read(Path::new(buzz_hostd::OPERATOR_CATALOG))?;
    host.reconcile_deployments(ws, settings.relay, g, &catalog, &revision, &helper)
        .await
}
