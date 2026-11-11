//! Extend the existing synthetic service fixture; never invoke the host's manager/providers.
use super::*;
use crate::runtime_context::{Context, NetworkPolicy};
use std::os::unix::fs::{symlink, PermissionsExt};

fn fixture() -> (tempfile::TempDir, UserService, Prepared, Catalog) {
    let (dir, _layout, mut prepared, mut catalog, _context) =
        crate::service_actions::tests::fixture();
    let state = dir.path().join("staging");
    let manager = dir.path().join("manager");
    fs::write(&manager, r#"#!/usr/bin/python3
import pathlib,sys
root=pathlib.Path(__file__).parent
assert sys.argv[1:3]==['--user','--no-pager']
verb=sys.argv[3]
unit=sys.argv[-1]
with open(root/'calls','a') as f: f.write(' '.join(sys.argv[1:])+'\n')
if verb=='show':
    print('LoadState=loaded\nFragmentPath='+str(root/'units/buzz-hosted-agent@.service'))
    print('DropInPaths='+str(root/'units'/ (unit+'.d')/'50-buzz.conf') if (root/'units'/(unit+'.d/50-buzz.conf')).exists() else 'DropInPaths=')
    active=(root/'active').exists()
    print('ActiveState='+('active' if active else 'inactive'))
    print('MainPID='+('42' if active else '0')+'\nControlPID=0\nTasksCurrent='+('1' if active else '0'))
elif verb=='stop':
    if (root/'fail-stop').exists(): sys.exit(1)
    (root/'active').unlink(missing_ok=True)
elif verb=='start': (root/'active').touch()
elif verb!='daemon-reload': raise AssertionError(verb)
"#).unwrap();
    fs::set_permissions(&manager, fs::Permissions::from_mode(0o700)).unwrap();
    let home = dir.path().join("normal-home");
    let workspace = dir.path().join("repo");
    fs::create_dir(&home).unwrap();
    fs::create_dir(&workspace).unwrap();
    fs::write(home.join("provider-login"), "fixture-only").unwrap();
    fs::write(workspace.join("user-work"), "keep").unwrap();
    let tools = dir.path().join("tools");
    fs::create_dir(&tools).unwrap();
    fs::write(tools.join("grok"), "#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(tools.join("grok"), fs::Permissions::from_mode(0o700)).unwrap();
    catalog.runtimes.get_mut("native/grok").unwrap().args =
        vec!["agent".into(), "--always-approve".into(), "stdio".into()];
    catalog.local = Some(LocalContext {
        buzz_acp: None,
        home,
        workspace,
        tool_path: format!("{}:/usr/bin:/bin", tools.display()),
        environment: BTreeMap::new(),
        unit_dir: dir.path().join("units"),
        launcher: "/usr/bin/true".into(),
        systemctl: manager,
    });
    write_json(&state, "deployment.json", &catalog).unwrap();
    let service = UserService::installed(&state).unwrap();
    let (catalog, rev) = service.catalog().unwrap();
    prepared.instance.revision = rev.clone();
    prepared
        .instance
        .deployment
        .as_mut()
        .unwrap()
        .config
        .catalog_revision = rev;
    (dir, service, prepared, catalog)
}
fn calls(dir: &Path) -> String {
    fs::read_to_string(dir.join("calls")).unwrap_or_default()
}

#[tokio::test]
async fn credential_assignments_preserve_paths_and_repair_only_exact_legacy_override() {
    for component in ["plain", "space %n back\\slash \"quote\" $dollar :colon"] {
        let (dir, mut service, prepared, cat) = fixture();
        let state = dir.path().join(component);
        fs::rename(&service.staging, &state).unwrap();
        service = UserService::installed(&state).unwrap();
        let id = &prepared.instance.unit_id;
        let local = cat.local.as_ref().unwrap();
        service.stage(&prepared.instance).unwrap();
        service.action("prepare", id).await.unwrap();
        let dropin = local
            .unit_dir
            .join(format!("buzz-hosted-agent@{id}.service.d"));
        let file = dropin.join("50-buzz.conf");
        let correct = fs::read_to_string(&file).unwrap();
        let expected: Vec<_> = ["instance", "execution"]
            .map(|name| {
                format!(
                    "LoadCredential={name}:{}/hosted/{id}/{name}",
                    state.display().to_string().replace('%', "%%")
                )
            })
            .into();
        assert_eq!(
            correct
                .lines()
                .filter(|l| l.starts_with("LoadCredential="))
                .collect::<Vec<_>>(),
            expected
        );
        let identity = fs::read(service.dir(id).join("instance")).unwrap();
        // Reproduce the exact former generator, including ExecStart's distinct quoting.
        let legacy = format!("[Service]\nExecStart=\nExecStart={} --user-service {} {}\nLoadCredential={}\nLoadCredential={}\n",
            quote(&local.launcher).unwrap().replace('$', "$$"), quote(&state).unwrap().replace('$', "$$"), id,
            quote(Path::new(&format!("instance:{}/hosted/{id}/instance", state.display()))).unwrap(),
            quote(Path::new(&format!("execution:{}/hosted/{id}/execution", state.display()))).unwrap());
        write_bytes(&dropin, "50-buzz.conf", legacy.as_bytes()).unwrap();
        let reopened = UserService::installed(&state).unwrap();
        reopened.action("prepare", id).await.unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), correct);
        assert_eq!(
            fs::read(service.dir(id).join("instance")).unwrap(),
            identity
        );
        reopened.action("prepare", id).await.unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), correct);
        for foreign in [
            format!("{legacy}# operator edit\n"),
            legacy.replace("instance:", "other:"),
            legacy.replace("/instance", "/foreign"),
            legacy.replace("/usr/bin/true", "/usr/bin/false"),
            format!("{correct}Environment=FOREIGN=1\n"),
        ] {
            write_bytes(&dropin, "50-buzz.conf", foreign.as_bytes()).unwrap();
            let before = calls(dir.path()).matches("daemon-reload").count();
            assert!(reopened
                .action("prepare", id)
                .await
                .unwrap_err()
                .to_string()
                .contains("foreign user override"));
            assert_eq!(fs::read_to_string(&file).unwrap(), foreign);
            assert_eq!(calls(dir.path()).matches("daemon-reload").count(), before);
        }
        assert!(!calls(dir.path()).contains(" start "));
    }
}

#[test]
fn credential_paths_keep_existing_control_and_utf8_rejection() {
    use std::os::unix::ffi::OsStrExt;
    let (_dir, service, prepared, cat) = fixture();
    for path in [
        Path::new("/tmp/new\nline"),
        Path::new("/tmp/tab\tpath"),
        Path::new(std::ffi::OsStr::from_bytes(b"/tmp/\xff")),
    ] {
        let invalid = UserService {
            staging: path.into(),
        };
        assert!(invalid
            .override_text(
                cat.local.as_ref().unwrap(),
                &prepared.instance.unit_id,
                false
            )
            .is_err());
    }
    assert!(service
        .override_text(
            cat.local.as_ref().unwrap(),
            &prepared.instance.unit_id,
            false
        )
        .is_ok());
}

#[tokio::test]
#[ignore = "requires offline native systemd --test and disposable socket permission; syntax only"]
async fn generated_credentials_parse_offline_without_starting_a_service() {
    let (dir, service, prepared, cat) = fixture();
    let id = &prepared.instance.unit_id;
    service.stage(&prepared.instance).unwrap();
    service.action("prepare", id).await.unwrap();
    for name in ["basic.target", "shutdown.target"] {
        fs::write(
            cat.local.as_ref().unwrap().unit_dir.join(name),
            "[Unit]\nDefaultDependencies=no\n",
        )
        .unwrap();
    }
    let runtime = dir.path().join("private-runtime");
    fs::DirBuilder::new().mode(0o700).create(&runtime).unwrap();
    let output = tokio::time::timeout(
        Duration::from_secs(15),
        Command::new("/usr/lib/systemd/systemd")
            .args([
                "--user",
                "--test",
                &format!("--unit=buzz-hosted-agent@{id}.service"),
            ])
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", dir.path())
            .env("XDG_CONFIG_HOME", dir.path())
            .env("XDG_RUNTIME_DIR", runtime)
            .env("SYSTEMD_UNIT_PATH", &cat.local.as_ref().unwrap().unit_dir)
            .env("SYSTEMD_GENERATOR_PATH", "")
            .env("SYSTEMD_ENVIRONMENT_GENERATOR_PATH", "")
            .env("SYSTEMD_LOG_TARGET", "console")
            .kill_on_drop(true)
            .output(),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("Load State: loaded"));
    // --test computes and prints a transaction; it never executes it. It does not
    // expose parsed credential values, so this is deliberately only a syntax check.
    assert!(!calls(dir.path()).contains(" start "));
}

#[tokio::test]
async fn prepare_replay_restart_and_identity_fences_precede_effects() {
    let (dir, service, mut prepared, catalog) = fixture();
    let id = prepared.instance.unit_id.clone();
    service.stage(&prepared.instance).unwrap();
    assert_eq!(service.action("prepare", &id).await.unwrap(), "prepared\n");
    let before = private_file(&service.dir(&id).join("instance"), 131072).unwrap();
    let reopened = UserService::installed(&service.staging).unwrap();
    reopened.stage(&prepared.instance).unwrap();
    reopened.action("prepare", &id).await.unwrap();
    assert_eq!(
        private_file(&service.dir(&id).join("instance"), 131072).unwrap(),
        before
    );
    assert_eq!(
        fs::metadata(service.dir(&id)).unwrap().mode() & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(service.dir(&id).join("instance"))
            .unwrap()
            .mode()
            & 0o777,
        0o600
    );
    assert!(!calls(dir.path()).contains(&prepared.instance.secret));
    let effects = calls(dir.path());
    prepared.instance.scope.community = "https://wrong.test".into();
    assert!(reopened.stage(&prepared.instance).is_err());
    prepared.instance.scope.community = catalog.community.clone().unwrap();
    prepared.instance.revision = "ff".repeat(32);
    assert!(reopened.stage(&prepared.instance).is_err());
    prepared.instance.revision = service.catalog().unwrap().1;
    prepared.instance.unit_id = "ab".repeat(32);
    assert!(reopened.stage(&prepared.instance).is_err());
    assert_eq!(calls(dir.path()), effects);
    assert!(reopened
        .action("start", &id)
        .await
        .unwrap_err()
        .to_string()
        .contains("model selection pending"));
    assert!(!calls(dir.path()).contains(" start "));
    assert!(reopened.action("login", &id).await.is_err());
    assert!(reopened.action("prepare", "../escape").await.is_err());
    let owner = nostr::Keys::generate();
    let agent = nostr::Keys::generate();
    let first_key = prepared.instance.secret.clone();
    prepared.instance.scope.owner = owner.public_key();
    prepared.instance.secret = agent.secret_key().to_secret_hex();
    prepared.instance.agent = agent.public_key().to_hex();
    prepared.instance.oa =
        buzz_sdk::nip_oa::compute_auth_tag(&owner, &agent.public_key(), "").unwrap();
    prepared.instance.request_id = "dd".repeat(32);
    prepared.instance.unit_id =
        crate::unit_id(&prepared.instance.scope, &prepared.instance.request_id);
    reopened.stage(&prepared.instance).unwrap();
    reopened
        .action("prepare", &prepared.instance.unit_id)
        .await
        .unwrap();
    let second: Prepared = serde_json::from_slice(
        &private_file(
            &service.dir(&prepared.instance.unit_id).join("instance"),
            131072,
        )
        .unwrap(),
    )
    .unwrap();
    assert_ne!(first_key, second.instance.secret);
    assert!(!calls(dir.path()).contains(&second.instance.secret));
}

#[tokio::test]
async fn finite_actions_are_serialized_correlated_and_cancel_confirmed() {
    let (dir, service, prepared, _cat) = fixture();
    let id = &prepared.instance.unit_id;
    service.stage(&prepared.instance).unwrap();
    service.action("prepare", id).await.unwrap();
    let first: Execution =
        serde_json::from_str(&service.action("models", id).await.unwrap()).unwrap();
    assert!(service.action("models", id).await.is_err());
    assert!(service.action("start", id).await.is_err());
    let before = calls(dir.path());
    assert!(service
        .request("cancel", id, Some(&"ff".repeat(32)))
        .await
        .is_err());
    assert_eq!(
        calls(dir.path())
            .lines()
            .filter(|l| l.contains(" stop "))
            .count(),
        0
    );
    assert!(calls(dir.path()).len() >= before.len());
    fs::write(dir.path().join("fail-stop"), "").unwrap();
    assert!(service
        .request("cancel", id, Some(&first.nonce))
        .await
        .is_err());
    assert!(service.dir(id).join("execution").is_file());
    fs::remove_file(dir.path().join("fail-stop")).unwrap();
    assert_eq!(
        service
            .request("cancel", id, Some(&first.nonce))
            .await
            .unwrap(),
        "cancelled\n"
    );
    let reopened = UserService::installed(&service.staging).unwrap();
    let next: Execution =
        serde_json::from_str(&reopened.action("models", id).await.unwrap()).unwrap();
    assert_ne!(first.nonce, next.nonce);
    assert_eq!(first.unit, next.unit);
    assert!(reopened
        .request("observe", id, Some(&first.nonce))
        .await
        .is_err());
    reopened
        .request("cancel", id, Some(&next.nonce))
        .await
        .unwrap();
    // A held transaction refuses another actor before it reaches the manager.
    let _lock = service.lock(id).unwrap();
    let before = calls(dir.path());
    assert!(reopened.action("models", id).await.is_err());
    assert_eq!(calls(dir.path()), before);
}

#[tokio::test]
async fn cleanup_retries_preserve_shared_files_and_refuse_symlink_escapes() {
    let (dir, service, prepared, cat) = fixture();
    let id = &prepared.instance.unit_id;
    let local = cat.local.as_ref().unwrap();
    fs::write(local.unit_dir.join("unrelated.service"), "keep").unwrap();
    service.stage(&prepared.instance).unwrap();
    service.action("prepare", id).await.unwrap();
    service.action("models", id).await.unwrap();
    fs::write(dir.path().join("fail-stop"), "").unwrap();
    assert!(service.action("cleanup", id).await.is_err());
    assert!(service.dir(id).join("instance").is_file());
    assert!(service.dir(id).join("execution").is_file());
    assert!(service.stage(&prepared.instance).is_err());
    fs::remove_file(dir.path().join("fail-stop")).unwrap();
    let file = service.dir(id).join("instance");
    fs::remove_file(&file).unwrap();
    symlink(local.home.join("provider-login"), &file).unwrap();
    assert!(service.action("cleanup", id).await.is_err());
    assert_eq!(
        fs::read_to_string(local.home.join("provider-login")).unwrap(),
        "fixture-only"
    );
    fs::remove_file(&file).unwrap();
    write_json(&service.dir(id), "instance", &prepared).unwrap();
    let reopened = UserService::installed(&service.staging).unwrap();
    assert_eq!(reopened.action("cleanup", id).await.unwrap(), "cleaned\n");
    reopened.remove_staged(id).unwrap();
    assert_eq!(reopened.action("cleanup", id).await.unwrap(), "cleaned\n");
    assert!(!service.dir(id).exists());
    assert!(local.home.join("provider-login").is_file());
    assert!(local.workspace.join("user-work").is_file());
    assert!(local.unit_dir.join("unrelated.service").is_file());
    assert!(!calls(dir.path()).contains("unrelated.service"));
    // An escaped directory is refused, even for cleanup of a recorded ID.
    symlink(&local.home, service.dir(id)).unwrap();
    assert!(reopened.action("cleanup", id).await.is_err());
}

#[tokio::test]
async fn trusted_finite_runner_uses_account_context_and_bounds_process_tree() {
    let (dir, service, mut prepared, mut catalog) = fixture();
    let script = dir.path().join("fake-models");
    let local = catalog.local.as_ref().unwrap().clone();
    fs::write(
        &script,
        format!(
            r#"#!/usr/bin/python3
import os,sys,json
assert os.getuid()!=0
assert os.environ['HOME']=={home:?}
assert os.getcwd()=={workspace:?}
assert os.environ['PATH']=={tool_path:?}
assert 'GROK_HOME' not in os.environ
assert 'BUZZ_PRIVATE_KEY' not in os.environ
assert 'BUZZ_AUTH_TAG' not in os.environ
assert sys.argv[1:]==['models','--json']
assert os.environ['BUZZ_ACP_AGENT_ARGS']=='locally-configured'
print(json.dumps({{'unstable':{{'availableModels':[{{'modelId':'synthetic','name':'Synthetic'}}]}}}}))
"#,
            home = local.home.to_str().unwrap(),
            workspace = local.workspace.to_str().unwrap(),
            tool_path = local.tool_path
        ),
    )
    .unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
    let rt = catalog.runtimes.get_mut("native/grok").unwrap();
    rt.harness.path = script.clone();
    rt.args = vec!["locally-configured".into()];
    write_json(&service.staging, "deployment.json", &catalog).unwrap();
    let (catalog, rev) = service.catalog().unwrap();
    prepared.instance.revision = rev.clone();
    prepared
        .instance
        .deployment
        .as_mut()
        .unwrap()
        .config
        .catalog_revision = rev;
    let id = &prepared.instance.unit_id;
    service.stage(&prepared.instance).unwrap();
    service.action("prepare", id).await.unwrap();
    let notify = dir.path().join("user-notify");
    let _socket = std::os::unix::net::UnixDatagram::bind(&notify).unwrap();
    let ctx = Context {
        account: Some(local.clone()),
        home: service.dir(id),
        runtime: service.dir(id),
        notify: notify.into_os_string(),
    };
    let mut e = Execution::new(
        &prepared.instance,
        Mode::Models,
        nostr::Timestamp::now().as_secs(),
    )
    .unwrap();
    let result = crate::runtime_context::finite(
        &ctx,
        &prepared,
        &catalog,
        &e,
        NetworkPolicy::TrustedAccount,
    )
    .await
    .unwrap();
    assert!(matches!(
        result.models,
        buzz_core::host_deployment::observation::Models::Available { .. }
    ));
    assert_eq!(
        result.login,
        buzz_core::host_deployment::observation::LoginState::NotObserved
    );
    assert!(result.login_at.is_none());
    assert!(result.login_output.is_empty());
    assert!(!serde_json::to_string(&result)
        .unwrap()
        .contains(&prepared.instance.secret));
    // One per-agent output path, separate from the account's HOME and repository.
    assert!(ctx.home.join(e.filename()).is_file());
    assert!(!local.home.join(e.filename()).exists());
    e.mode = Mode::Login;
    assert!(crate::runtime_context::finite(
        &ctx,
        &prepared,
        &catalog,
        &e,
        NetworkPolicy::TrustedAccount
    )
    .await
    .is_err());
    e.mode = Mode::Models;
    fs::write(
        &script,
        "#!/usr/bin/python3\nimport sys\nsys.stdout.write('x'*70000)\n",
    )
    .unwrap();
    let result = crate::runtime_context::finite(
        &ctx,
        &prepared,
        &catalog,
        &e,
        NetworkPolicy::TrustedAccount,
    )
    .await
    .unwrap();
    assert!(matches!(
        result.models,
        buzz_core::host_deployment::observation::Models::Unavailable {
            reason: buzz_core::host_deployment::observation::UnavailableReason::Oversized
        }
    ));
    fs::write(&script,"#!/usr/bin/python3\nimport subprocess,time\np=subprocess.Popen(['/usr/bin/sleep','30'])\nopen('descendant','w').write(str(p.pid))\ntime.sleep(30)\n").unwrap();
    e.deadline = nostr::Timestamp::now().as_secs() + 1;
    let result = crate::runtime_context::finite(
        &ctx,
        &prepared,
        &catalog,
        &e,
        NetworkPolicy::TrustedAccount,
    )
    .await
    .unwrap();
    assert!(matches!(
        result.models,
        buzz_core::host_deployment::observation::Models::Unavailable {
            reason: buzz_core::host_deployment::observation::UnavailableReason::Timeout
        }
    ));
    let pid = fs::read_to_string(local.workspace.join("descendant")).unwrap();
    for _ in 0..100 {
        let state = fs::read_to_string(format!("/proc/{pid}/stat")).unwrap_or_default();
        if state.is_empty() || state.split_whitespace().nth(2) == Some("Z") {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("finite descendant still running");
}

#[test]
fn new_route_has_no_root_client_and_fixed_unit_uses_account_supervision() {
    for source in [
        include_str!("deployment.rs"),
        include_str!("registration_cli.rs"),
        include_str!("registration_refresh.rs"),
    ] {
        assert!(!source.contains("UnixStream"));
        assert!(!source.contains("OPERATOR_CATALOG"));
        assert!(!source.contains("/var/lib/buzz-hostd"));
    }
    let unit = include_str!("../deploy/buzz-hosted-agent@.service");
    assert!(unit.contains("KillMode=control-group"));
    assert!(unit.contains("Restart=no"));
    for forbidden in [
        "DynamicUser",
        "RootDirectory",
        "ProtectHome",
        "PrivateNetwork",
        "GROK_HOME",
    ] {
        assert!(!unit.contains(forbidden));
    }
}

#[tokio::test]
async fn cleanup_after_launch_paths_move_does_not_require_current_catalog() {
    for config in ["updated", "missing", "malformed"] {
        let (dir, service, mut prepared, mut catalog) = fixture();
        let launcher = dir.path().join("retired-launcher");
        fs::write(&launcher, "#!/bin/sh\nexit 0\n").unwrap();
        fs::set_permissions(&launcher, fs::Permissions::from_mode(0o700)).unwrap();
        catalog.local.as_mut().unwrap().launcher = launcher.clone();
        write_json(&service.staging, "deployment.json", &catalog).unwrap();
        let revision = service.catalog().unwrap().1;
        prepared.instance.revision = revision.clone();
        prepared
            .instance
            .deployment
            .as_mut()
            .unwrap()
            .config
            .catalog_revision = revision;
        let id = &prepared.instance.unit_id;
        service.stage(&prepared.instance).unwrap();
        service.action("prepare", id).await.unwrap();
        service.action("models", id).await.unwrap();
        let local = catalog.local.as_mut().unwrap();
        let moved_home = dir.path().join("moved-home");
        let moved_repo = dir.path().join("moved-repo");
        fs::rename(&local.home, &moved_home).unwrap();
        fs::rename(&local.workspace, &moved_repo).unwrap();
        fs::remove_file(&launcher).unwrap();
        // The old launch context still fails launch validation after reopening.
        assert!(service.catalog().is_err());
        local.home = moved_home.clone();
        local.workspace = moved_repo.clone();
        local.launcher = "/usr/bin/true".into();
        let config_path = service.staging.join("deployment.json");
        match config {
            "updated" => write_json(&service.staging, "deployment.json", &catalog).unwrap(),
            "missing" => fs::remove_file(&config_path).unwrap(),
            _ => fs::write(&config_path, "{broken").unwrap(),
        }
        let reopened = UserService::installed(&service.staging).unwrap();
        let before = calls(dir.path());
        assert!(reopened.stage(&prepared.instance).is_err());
        assert!(reopened.action("prepare", id).await.is_err());
        assert!(reopened.action("models", id).await.is_err());
        assert_eq!(
            before,
            calls(dir.path()),
            "{config}: launch must fail before effects"
        );
        fs::write(dir.path().join("fail-stop"), "").unwrap();
        assert!(reopened.action("cleanup", id).await.is_err());
        assert!(service.dir(id).join("context").is_file());
        assert!(service.dir(id).join("instance").is_file());
        assert!(dir.path().join("active").exists());
        fs::remove_file(dir.path().join("fail-stop")).unwrap();
        let restarted = UserService::installed(&service.staging).unwrap();
        assert_eq!(restarted.action("cleanup", id).await.unwrap(), "cleaned\n");
        restarted.remove_staged(id).unwrap();
        assert!(!dir.path().join("active").exists());
        assert!(!service.dir(id).exists());
        assert!(moved_home.join("provider-login").is_file());
        assert!(moved_repo.join("user-work").is_file());
    }
}

fn script(path: &Path, source: &str) {
    fs::write(path, source).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
}
fn save_catalog(service: &UserService, prepared: &mut Prepared, catalog: &Catalog) -> Catalog {
    write_json(&service.staging, "deployment.json", catalog).unwrap();
    let (cat, revision) = service.catalog().unwrap();
    prepared.instance.revision = revision.clone();
    prepared
        .instance
        .deployment
        .as_mut()
        .unwrap()
        .config
        .catalog_revision = revision;
    cat
}
fn account_context(service: &UserService, prepared: &Prepared, catalog: &Catalog) -> Context {
    Context {
        account: catalog.local.clone(),
        home: service.dir(&prepared.instance.unit_id),
        runtime: service.dir(&prepared.instance.unit_id),
        notify: Default::default(),
    }
}

#[tokio::test]
async fn automatic_main_candidates_resolve_each_account_without_manual_rows() {
    use buzz_core::harness::{AcpAvailabilityStatus as A, Readiness};
    let (dir, service, mut prepared, mut cat) = fixture();
    cat.runtimes.clear();
    cat.local.as_mut().unwrap().buzz_acp = Some("/usr/bin/true".into());
    let tools = dir.path().join("tools");
    script(&tools.join("claude-agent-acp"), "#!/bin/sh\nexit 0\n");
    script(
        &tools.join("claude"),
        "#!/bin/sh\ntest \"$1 $2\" = 'auth status' || exit 3\nexit 0\n",
    );
    script(
        &tools.join("codex-acp"),
        "#!/bin/sh\necho 'codex-acp 1.10.0'\n",
    );
    script(&tools.join("codex"), "#!/bin/sh\nexit 0\n");
    let cat = save_catalog(&service, &mut prepared, &cat);
    assert!(cat.runtimes.contains_key("main/claude"));
    assert!(cat.runtimes.contains_key("main/grok"));
    let ctx = account_context(&service, &prepared, &cat);
    let inventory = crate::discovery::inventory(&cat).await.unwrap();
    assert_eq!(inventory["main/claude"].availability, A::Available);
    assert_eq!(inventory["main/codex"].version.as_deref(), Some("1.10.0"));
    assert_eq!(
        crate::discovery::readiness(
            &ctx,
            &cat.runtimes["main/claude"],
            &inventory["main/claude"]
        )
        .await,
        Readiness::LoggedIn
    );
    assert_eq!(
        crate::discovery::readiness(&ctx, &cat.runtimes["main/grok"], &inventory["main/grok"])
            .await,
        Readiness::NotApplicable
    );
    fs::remove_file(tools.join("claude")).unwrap();
    assert_eq!(
        crate::discovery::inspect(&ctx, &cat.runtimes["main/claude"])
            .await
            .availability,
        A::CliMissing
    );
    script(&tools.join("claude"), "#!/bin/sh\nexit 0\n");
    fs::remove_file(tools.join("claude-agent-acp")).unwrap();
    assert_eq!(
        crate::discovery::inspect(&ctx, &cat.runtimes["main/claude"])
            .await
            .availability,
        A::AdapterMissing
    );
    script(
        &tools.join("codex-acp"),
        "#!/bin/sh\necho 'codex-acp 1.6.2'\n",
    );
    assert_eq!(
        crate::discovery::inspect(&ctx, &cat.runtimes["main/codex"])
            .await
            .availability,
        A::AdapterOutdated
    );
    let mut other = cat;
    let empty = dir.path().join("empty-tools");
    fs::create_dir(&empty).unwrap();
    other.local.as_mut().unwrap().tool_path = empty.display().to_string();
    let other = save_catalog(&service, &mut prepared, &other);
    let inventory = crate::discovery::inventory(&other).await.unwrap();
    assert_eq!(inventory["main/grok"].availability, A::NotInstalled);
    assert_eq!(inventory["main/claude"].availability, A::NotInstalled);
    assert!(!inventory["main/goose"].config_supported);
}

#[tokio::test]
async fn current_native_status_is_independent_of_history_models_and_entitlement() {
    use buzz_core::harness::{AcpAvailabilityStatus as A, Readiness};
    let (dir, service, mut prepared, mut cat) = fixture();
    let tools = dir.path().join("tools");
    script(&tools.join("claude-agent-acp"), "#!/bin/sh\nexit 0\n");
    let mut rt = cat.runtimes.remove("native/grok").unwrap();
    rt.harness_id = Some("claude".into());
    rt.args.clear();
    cat.runtimes.insert("native/claude".into(), rt);
    prepared.instance.runtime_id = "native/claude".into();
    let config = &mut prepared.instance.deployment.as_mut().unwrap().config;
    config.runtime_id = "native/claude".into();
    config.model = buzz_core::host_deployment::ModelSelection::Selected {
        id: "chosen".into(),
    };
    script(&tools.join("claude"), "#!/bin/sh\nexit 0\n");
    let cat = save_catalog(&service, &mut prepared, &cat);
    let ctx = account_context(&service, &prepared, &cat);
    let rt = &cat.runtimes["native/claude"];
    let facts = crate::discovery::inspect(&ctx, rt).await;
    let mut result = ActionResult {
        execution: Execution::new(
            &prepared.instance,
            Mode::Models,
            nostr::Timestamp::now().as_secs(),
        )
        .unwrap(),
        finished: true,
        observed_at: nostr::Timestamp::now().as_secs(),
        login: buzz_core::host_deployment::observation::LoginState::NotObserved,
        login_at: None,
        login_output: String::new(),
        availability: Some(A::Available),
        readiness: Some(crate::discovery::readiness(&ctx, rt, &facts).await),
        models: buzz_core::host_deployment::observation::Models::Available {
            catalog: buzz_core::agent_models::normalize_agent_models(
                &serde_json::json!({"unstable":{"availableModels":[{"modelId":"chosen"}]}}),
                None,
            ),
        },
    };
    assert_eq!(result.readiness, Some(Readiness::LoggedIn));
    crate::runtime_context::require_ready_selection(&prepared.instance, &result).unwrap();
    result.login = buzz_core::host_deployment::observation::LoginState::Succeeded;
    result.login_at = Some(result.observed_at);
    for (body, expected) in [
        ("#!/bin/sh\nexit 1\n", Readiness::LoggedOut),
        (
            "#!/bin/sh\necho 'Error loading configuration: unknown variant SECRET' >&2\nexit 1\n",
            Readiness::ConfigInvalid,
        ),
        ("#!/bin/sh\nsleep 10\n", Readiness::Unknown),
        ("#!/usr/bin/python3\nprint('x'*9000)\n", Readiness::Unknown),
    ] {
        script(&tools.join("claude"), body);
        result.readiness = Some(crate::discovery::readiness(&ctx, rt, &facts).await);
        assert_eq!(result.readiness, Some(expected));
        assert!(
            crate::runtime_context::require_ready_selection(&prepared.instance, &result).is_err()
        );
        assert!(!serde_json::to_string(&result).unwrap().contains("SECRET"));
    }
    result.readiness = Some(Readiness::LoggedIn);
    result.execution.catalog = "ff".repeat(32);
    assert!(crate::runtime_context::require_ready_selection(&prepared.instance, &result).is_err());
}

#[tokio::test]
#[ignore = "requires built BUZZ_TEST_RUNTIME and BUZZ_TEST_ACP; synthetic executables only"]
async fn actual_user_runtime_prepare_refresh_selected_conversation_and_cleanup() {
    use buzz_core::{
        harness::Readiness,
        host_deployment::{observation::Models, ModelSelection},
    };
    let (dir, service, mut prepared, mut cat) = fixture();
    let runtime = PathBuf::from(std::env::var_os("BUZZ_TEST_RUNTIME").unwrap());
    let acp = PathBuf::from(std::env::var_os("BUZZ_TEST_ACP").unwrap());
    let tools = dir.path().join("tools");
    let workspace = cat.local.as_ref().unwrap().workspace.clone();
    script(
        &tools.join("claude"),
        "#!/bin/sh\ntest \"$1 $2\" = 'auth status' || exit 3\ntest ! -f logged-out\n",
    );
    script(
        &tools.join("claude-agent-acp"),
        r#"#!/usr/bin/python3
import json,os,sys
assert 'BUZZ_PRIVATE_KEY' not in os.environ
for line in sys.stdin:
 m=json.loads(line)
 if m.get('method')=='initialize': r={'protocolVersion':1,'agentInfo':{'name':'synthetic','version':'1'},'agentCapabilities':{}}
 elif m.get('method')=='session/new':
  assert m['params'].get('mcpServers',[])==[]
  r={'sessionId':'s','configOptions':[{'category':'model','id':'model','currentValue':'chosen','type':'select','options':[{'value':'chosen','name':'Chosen'}]}]}
 else: raise AssertionError(str(m.get('method')))
 print(json.dumps({'jsonrpc':'2.0','id':m['id'],'result':r}),flush=True)
"#,
    );
    let harness = tools.join("fixture-buzz-acp");
    script(
        &harness,
        &format!(
            r#"#!/usr/bin/python3
import os,sys,json,socket
if sys.argv[1:]==['models','--json']:
 os.execv({acp:?},[{acp:?},'models','--json'])
assert sys.argv[1:]==[]
assert os.getcwd()=={workspace:?}
assert os.environ['HOME']=={home:?}
assert os.environ['PATH']=={path:?}
assert os.environ['BUZZ_PRIVATE_KEY']=={secret:?}
assert os.environ['BUZZ_AUTH_TAG']=={auth:?}
assert os.environ['BUZZ_ACP_AGENT_OWNER']=={owner:?}
assert os.environ['BUZZ_ACP_AGENT_COMMAND']=={adapter:?}
assert os.environ['BUZZ_ACP_AGENT_ARGS']==''
assert os.environ['BUZZ_ACP_RESPOND_TO']=='owner-only'
assert os.environ['BUZZ_ACP_MODEL']=='chosen'
assert os.environ['BUZZ_ACP_STRICT_MODEL']=='true'
assert os.environ['BUZZ_ACP_EXIT_AFTER_INACTIVITY']=='0'
assert os.environ['BUZZ_RELAY_URL']=='wss://fixture.test'
assert 'GROK_HOME' not in os.environ
open('conversation-invoked','w').write('checked')
s=socket.socket(socket.AF_UNIX,socket.SOCK_DGRAM);s.connect(os.environ['NOTIFY_SOCKET']);s.send(b'READY=1')
"#,
            acp = acp.display().to_string(),
            workspace = workspace.display().to_string(),
            home = cat.local.as_ref().unwrap().home.display().to_string(),
            path = cat.local.as_ref().unwrap().tool_path,
            secret = prepared.instance.secret,
            auth = prepared.instance.oa,
            owner = prepared.instance.scope.owner.to_hex(),
            adapter = tools.join("claude-agent-acp").display().to_string()
        ),
    );
    cat.runtimes.clear();
    cat.local.as_mut().unwrap().buzz_acp = Some(harness);
    prepared.instance.runtime_id = "main/claude".into();
    prepared
        .instance
        .deployment
        .as_mut()
        .unwrap()
        .config
        .runtime_id = "main/claude".into();
    let cat = save_catalog(&service, &mut prepared, &cat);
    let id = prepared.instance.unit_id.clone();
    let id = id.as_str();
    service.stage(&prepared.instance).unwrap();
    service.action("prepare", id).await.unwrap();
    let notify = dir.path().join("notify-user-runtime");
    let socket = std::os::unix::net::UnixDatagram::bind(&notify).unwrap();
    socket.set_nonblocking(true).unwrap();
    let run = || {
        let mut c = tokio::process::Command::new(&runtime);
        c.args([
            std::ffi::OsStr::new("--user-service"),
            service.staging.as_os_str(),
            std::ffi::OsStr::new(id),
        ])
        .env_clear()
        .env("CREDENTIALS_DIRECTORY", service.dir(id))
        .env("NOTIFY_SOCKET", &notify)
        .kill_on_drop(true);
        c
    };
    let action: Execution =
        serde_json::from_str(&service.action("models", id).await.unwrap()).unwrap();
    let status = run().status().await.unwrap();
    assert!(status.success());
    fs::remove_file(dir.path().join("active")).unwrap();
    let result: ActionResult = serde_json::from_str(
        &service
            .request("observe", id, Some(&action.nonce))
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(result.readiness, Some(Readiness::LoggedIn));
    assert!(
        matches!(result.models,Models::Available { ref catalog } if catalog.models.iter().any(|m| m.id=="chosen"))
    );
    assert_eq!(
        result.login,
        buzz_core::host_deployment::observation::LoginState::NotObserved
    );
    prepared.instance.deployment.as_mut().unwrap().revision += 1;
    prepared.instance.deployment.as_mut().unwrap().config.model = ModelSelection::Selected {
        id: "chosen".into(),
    };
    service.stage(&prepared.instance).unwrap();
    service.action("prepare", id).await.unwrap();
    assert_eq!(service.action("start", id).await.unwrap(), "active\n");
    let mut buf = [0; 32];
    while socket.recv(&mut buf).is_ok() {}
    assert!(run().status().await.unwrap().success());
    assert_eq!(
        fs::read_to_string(workspace.join("conversation-invoked")).unwrap(),
        "checked"
    );
    assert_eq!(socket.recv(&mut buf).unwrap(), 7);
    assert!(
        socket.recv(&mut buf).is_err(),
        "internal recheck must not notify READY before main launch"
    );
    // Exact retry retains identity and execution; never emits another start or probe.
    let before = calls(dir.path());
    service.stage(&prepared.instance).unwrap();
    service.action("prepare", id).await.unwrap();
    assert_eq!(service.action("start", id).await.unwrap(), "active\n");
    assert_eq!(
        before.matches(" start").count(),
        calls(dir.path()).matches(" start").count()
    );
    // Same runtime entry rechecks native login immediately before launch, ignoring earlier success.
    fs::remove_file(workspace.join("conversation-invoked")).unwrap();
    fs::write(workspace.join("logged-out"), "").unwrap();
    assert!(!run().status().await.unwrap().success());
    assert!(!workspace.join("conversation-invoked").exists());
    assert!(socket.recv(&mut buf).is_err());
    fs::remove_file(workspace.join("logged-out")).unwrap();
    // Selected model membership is checked by the same production entry.
    prepared.instance.deployment.as_mut().unwrap().config.model = ModelSelection::Selected {
        id: "absent".into(),
    };
    write_json(&service.dir(id), "instance", &prepared).unwrap();
    assert!(!run().status().await.unwrap().success());
    assert!(!workspace.join("conversation-invoked").exists());
    service.action("cleanup", id).await.unwrap();
    assert!(!service.dir(id).exists());
    assert!(cat.local.unwrap().home.join("provider-login").is_file());
    assert!(workspace.join("user-work").is_file());
}
