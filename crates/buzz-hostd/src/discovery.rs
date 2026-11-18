//! Execute main discovery using only the configured account resolver and environment.
use crate::{runtime_context::Context, Catalog, Executable, Runtime};
use anyhow::{ensure, Result};
use buzz_core::harness::{
    self, AcpAvailabilityStatus as Availability, Candidate, Readiness, RuntimeFacts,
};
use std::{collections::BTreeMap, path::PathBuf, process::Stdio, time::Duration};
use tokio::{io::AsyncReadExt, process::Command};

fn resolve(path: &str, command: &str) -> Option<PathBuf> {
    std::env::split_paths(path)
        .filter(|p| p.is_absolute())
        .map(|p| p.join(command))
        .find(|p| {
            Executable {
                path: p.clone(),
                sha256: String::new(),
            }
            .available()
            .is_ok()
        })
}
fn candidate(rt: &Runtime) -> Option<Candidate> {
    harness::candidates()
        .into_iter()
        .find(|c| Some(c.id) == rt.harness_id.as_deref())
}
/// Resolve main's actual candidates. An optional local ACP path enables automatic inventory;
/// explicit local rows otherwise limit the eligible canonical harnesses.
pub fn resolve_catalog(cat: &mut Catalog) -> Result<()> {
    let Some(local) = &cat.local else {
        return Ok(());
    };
    if let Some(acp) = &local.buzz_acp {
        // No per-provider rows needed. Explicit rows retain their runtime IDs and local restrictions.
        for c in harness::candidates() {
            if cat
                .runtimes
                .values()
                .any(|r| r.harness_id.as_deref() == Some(c.id))
            {
                continue;
            }
            cat.runtimes.insert(
                format!("main/{}", c.id),
                Runtime {
                    harness_id: Some(c.id.into()),
                    harness: Executable {
                        path: acp.clone(),
                        sha256: String::new(),
                    },
                    adapter: Executable {
                        path: PathBuf::new(),
                        sha256: String::new(),
                    },
                    args: c.args.iter().map(|s| (*s).into()).collect(),
                    credential_profile: "none".into(),
                },
            );
        }
    }
    for rt in cat.runtimes.values_mut() {
        if let Some(c) = candidate(rt) {
            // Missing resolution is deliberately unusable; an old catalog path cannot mask removal.
            rt.adapter.path = c
                .commands
                .iter()
                .find_map(|name| resolve(&local.tool_path, name))
                .unwrap_or_default();
            rt.adapter.sha256.clear();
        }
    }
    ensure!(cat.runtimes.len() <= 64, "too many local candidates");
    Ok(())
}
/// Inspect one canonical runtime using main's classifiers. No provider login is attempted.
pub async fn inspect(context: &Context, rt: &Runtime) -> RuntimeFacts {
    let mut facts = RuntimeFacts {
        availability: Availability::NotInstalled,
        version: None,
        config_supported: false,
    };
    let (Some(local), Some(c)) = (&context.account, candidate(rt)) else {
        return facts;
    };
    facts.config_supported = !c.requires_config
        && rt.credential_profile == "none"
        && rt
            .args
            .iter()
            .map(String::as_str)
            .eq(c.args.iter().copied());
    let adapter = c
        .commands
        .iter()
        .find_map(|name| resolve(&local.tool_path, name).map(|p| (*name, p)));
    let cli = c
        .underlying_cli
        .and_then(|name| resolve(&local.tool_path, name));
    facts.availability =
        harness::classify_runtime(adapter.clone(), c.underlying_cli, cli.is_some()).0;
    if rt.harness.available().is_err() {
        facts.availability = Availability::NotInstalled;
    }
    if facts.availability == Availability::Available {
        // Runtime mapping must match the resolution included in the current configuration revision.
        if adapter.as_ref().map(|(_, p)| p) != Some(&rt.adapter.path) {
            facts.availability = Availability::NotInstalled;
        } else if let Some(minimum) = c.minimum_version {
            let version = capture(context.command(&rt.adapter.path).arg("--version"), 3)
                .await
                .filter(|(ok, _, _)| *ok)
                .and_then(|(_, out, _)| harness::parse_version(&out));
            facts.version = version.map(|(a, b, c)| format!("{a}.{b}.{c}"));
            facts.availability = harness::version_availability(version, minimum);
        }
    }
    facts
}
/// Current native status, interpreted by the same code as Desktop. Never reads credential files.
pub async fn readiness(context: &Context, rt: &Runtime, facts: &RuntimeFacts) -> Readiness {
    if facts.availability != Availability::Available {
        return Readiness::Unknown;
    }
    if !facts.config_supported {
        return Readiness::Unsupported;
    }
    let Some(c) = candidate(rt) else {
        return Readiness::Unsupported;
    };
    let Some(args) = c.auth_probe else {
        return Readiness::NotApplicable;
    };
    let Some(local) = &context.account else {
        return Readiness::Unknown;
    };
    let Some(binary) = resolve(&local.tool_path, args[0]) else {
        return Readiness::Unknown;
    };
    match capture(context.command(&binary).args(&args[1..]), 3).await {
        Some((success, _, stderr)) => harness::classify_probe_output(&stderr, success).into(),
        None => Readiness::Unknown,
    }
}
/// Bounded inventory uses the same inspection as launch; the whole pass has a finite budget.
pub async fn inventory(cat: &Catalog) -> Option<BTreeMap<String, RuntimeFacts>> {
    let local = cat.local.as_ref()?;
    let context = Context {
        account: Some(local.clone()),
        home: local.home.clone(),
        runtime: local.home.clone(),
        notify: Default::default(),
    };
    tokio::time::timeout(Duration::from_secs(10), async {
        let mut facts = BTreeMap::new();
        for (id, rt) in &cat.runtimes {
            facts.insert(id.clone(), inspect(&context, rt).await);
        }
        facts
    })
    .await
    .ok()
}
/// Short native status/version probes, with bounded output and process-group cleanup.
async fn capture(command: &mut Command, seconds: u64) -> Option<(bool, Vec<u8>, Vec<u8>)> {
    command
        .process_group(0)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().ok()?;
    let _group =
        crate::runtime_context::ProcessGroup(rustix::process::Pid::from_raw(child.id()? as i32)?);
    let mut stdout = child.stdout.take()?;
    let mut stderr = child.stderr.take()?;
    let outcome = tokio::time::timeout(Duration::from_secs(seconds), async {
        let (mut out,mut err)=(Vec::new(),Vec::new());
        let (mut a,mut b)=([0;4096],[0;4096]);
        let (mut end_a,mut end_b,mut status)=(false,false,None);
        loop {
            if end_a && end_b && status.is_some() { break; }
            tokio::select! {
                n=stdout.read(&mut a), if !end_a => { let n=n.ok()?; end_a=n==0;out.extend_from_slice(&a[..n]); },
                n=stderr.read(&mut b), if !end_b => { let n=n.ok()?; end_b=n==0;err.extend_from_slice(&b[..n]); },
                s=child.wait(), if status.is_none() => {status=Some(s.ok()?.success());},
            }
            if out.len()+err.len()>8192 { return None; }
        }
        Some((status?,out,err))
    }).await.ok().flatten();
    if child.try_wait().ok().flatten().is_none() {
        let _ = child.kill().await;
        let _ = child.wait().await;
    }
    outcome
}
