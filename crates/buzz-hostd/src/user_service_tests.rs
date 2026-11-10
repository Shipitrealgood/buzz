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
    catalog.local = Some(LocalContext {
        home,
        workspace,
        tool_path: "/usr/bin:/bin".into(),
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
        .contains("readiness"));
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
assert os.environ['PATH']=='/usr/bin:/bin'
assert 'GROK_HOME' not in os.environ
assert 'BUZZ_PRIVATE_KEY' not in os.environ
assert 'BUZZ_AUTH_TAG' not in os.environ
assert sys.argv[1:]==['models','--json']
assert os.environ['BUZZ_ACP_AGENT_ARGS']=='locally-configured'
print(json.dumps({{'unstable':{{'availableModels':[{{'modelId':'synthetic','name':'Synthetic'}}]}}}}))
"#,
            home = local.home.to_str().unwrap(),
            workspace = local.workspace.to_str().unwrap()
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
