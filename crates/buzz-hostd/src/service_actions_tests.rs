use super::*;
use crate::runtime_context::{Context, NetworkPolicy};
use buzz_core::{
    host_deployment::{Deploy, ModelSelection},
    paired_message::{Policy, Scope},
};
use nostr::Keys;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::ExitStatusExt;

fn output(text: String) -> Output {
    Output {
        status: std::process::ExitStatus::from_raw(0),
        stdout: text.into_bytes(),
        stderr: vec![],
    }
}
fn fixture() -> (tempfile::TempDir, Layout, Prepared, crate::Catalog, Context) {
    let dir = tempfile::tempdir().unwrap();
    for name in ["staging", "private", "state", "runtime", "units"] {
        fs::create_dir(dir.path().join(name)).unwrap();
        fs::set_permissions(dir.path().join(name), fs::Permissions::from_mode(0o700)).unwrap();
    }
    let layout = Layout {
        staging: dir.path().join("staging"),
        private: dir.path().join("private"),
        state_root: dir.path().join("state"),
        runtime_root: dir.path().join("runtime"),
        template: dir.path().join("units/buzz-paired-agent@.service"),
        catalog: dir.path().join("catalog"),
        root_uid: rustix::process::geteuid().as_raw(),
    };
    fs::write(
        &layout.template,
        include_str!("../deploy/buzz-paired-agent@.service"),
    )
    .unwrap();
    let host = Keys::generate();
    let owner = Keys::generate();
    let agent = Keys::generate();
    let scope = Scope {
        community: "https://fixture.test".into(),
        generation: "aa".repeat(32),
        host: host.public_key(),
        owner: owner.public_key(),
    };
    let request = "bb".repeat(32);
    let unit = crate::unit_id(&scope, &request);
    let exe = crate::Executable {
        path: "/usr/bin/true".into(),
        sha256: "00".repeat(32),
    };
    let cat = crate::Catalog {
        community: Some(scope.community.clone()),
        source: None,
        label: "fixture".into(),
        capacity: 2,
        relay_key: host.public_key().to_hex(),
        runtimes: [(
            "native/grok".into(),
            crate::Runtime {
                harness_id: Some("grok".into()),
                harness: exe.clone(),
                adapter: exe,
                args: vec![],
                credential_profile: "none".into(),
            },
        )]
        .into(),
    };
    fs::write(&layout.catalog, serde_json::to_vec(&cat).unwrap()).unwrap();
    let (cat, revision) = crate::Catalog::read(&layout.catalog).unwrap();
    let policy = Policy {
        respond_to: "owner-only".into(),
        respond_to_allowlist: vec![],
        channel_add_policy: "owner_only".into(),
    };
    let config = Deploy {
        catalog_revision: revision.clone(),
        runtime_id: "native/grok".into(),
        name: "disposable".into(),
        policy: policy.clone(),
        model: ModelSelection::PendingLogin {},
        provider: None,
        system_prompt: None,
        effort: None,
        parallelism: 1,
    };
    let prepared = Prepared {
        provider: Default::default(),
        instance: crate::Instance {
            deployment: Some(crate::ManagedDeployment {
                operation_id: "cc".repeat(32),
                revision: 1,
                config,
            }),
            unit_id: unit.clone(),
            scope,
            request_id: request,
            runtime_id: "native/grok".into(),
            revision,
            channels: vec![],
            agent: agent.public_key().to_hex(),
            secret: agent.secret_key().to_secret_hex(),
            oa: buzz_sdk::nip_oa::compute_auth_tag(&owner, &agent.public_key(), "").unwrap(),
            policy,
        },
    };
    save(&layout, &unit, "cred", &prepared).unwrap();
    conversation(&layout, &prepared.instance).unwrap();
    let context = Context {
        home: layout.state_root.join(format!("buzz-agent-{unit}")),
        runtime: layout.runtime_root.join(format!("buzz-agent-{unit}")),
        notify: dir.path().join("notify").into_os_string(),
    };
    for p in [&context.home, &context.runtime] {
        fs::create_dir(p).unwrap();
        fs::set_permissions(p, fs::Permissions::from_mode(0o700)).unwrap();
    }
    (dir, layout, prepared, cat, context)
}
fn idle() -> String {
    "ActiveState=inactive\nUnitFileState=disabled\nMainPID=0\nControlPID=0\nTasksCurrent=0\n".into()
}

#[test]
fn owned_output_never_follows_agent_links_and_mode_shape_is_exact() {
    let (_dir, layout, prepared, _cat, ctx) = fixture();
    let id = &prepared.instance.unit_id;
    let e = Execution::new(
        &prepared.instance,
        Mode::Models,
        nostr::Timestamp::now().as_secs(),
    )
    .unwrap();
    let sentinel = layout.private.join("operator-secret");
    fs::write(&sentinel, b"private sentinel").unwrap();
    std::os::unix::fs::symlink(&sentinel, ctx.home.join(e.filename())).unwrap();
    assert!(read_output(&layout.state_root, id, &e).is_err());
    fs::remove_file(ctx.home.join(e.filename())).unwrap();
    fs::hard_link(&sentinel, ctx.home.join(e.filename())).unwrap();
    assert!(read_output(&layout.state_root, id, &e).is_err());
    save(&layout, id, "execution", &e).unwrap();
    set_mode(&layout, id, true, &mut |_, _| Ok(output(String::new()))).unwrap();
    let advertised = format!("DropInPaths={}\n", dropin(&layout, id).unwrap().display());
    assert_eq!(shape(&layout, id, &advertised).unwrap(), "DropInPaths=\n");
    fs::write(dropin(&layout, id).unwrap(), b"[Service]\nRestart=always\n").unwrap();
    assert!(shape(&layout, id, &advertised).is_err());
    assert!(!inactive(&idle().replace("MainPID=0", "MainPID=42")));
    assert!(!inactive(&idle().replace("inactive", "active")));
}

#[test]
fn finite_actions_are_correlated_local_login_only_and_refuse_busy() {
    let (_dir, layout, prepared, _cat, _ctx) = fixture();
    let id = &prepared.instance.unit_id;
    let mut calls = Vec::new();
    let mut system = |args: &[&str], _: &str| {
        calls.push(args.join(" "));
        Ok(output(idle()))
    };
    assert!(apply(&layout, id, "login", None, false, &idle(), &mut system).is_err());
    assert!(apply(
        &layout,
        id,
        "models",
        None,
        false,
        &idle().replace("inactive", "active"),
        &mut system
    )
    .is_err());
    assert!(calls.is_empty());
    let action = apply(
        &layout,
        id,
        "models",
        None,
        false,
        &idle(),
        &mut |args, _| {
            calls.push(args.join(" "));
            Ok(output(idle()))
        },
    )
    .unwrap();
    let e: Execution = serde_json::from_str(&action).unwrap();
    assert_eq!(calls, ["disable", "daemon-reload", "start --no-block"]);
    assert!(apply(
        &layout,
        id,
        "observe",
        Some(&"ff".repeat(32)),
        false,
        &idle(),
        &mut |_, _| Ok(output(idle()))
    )
    .is_err());
    assert_eq!(
        apply(
            &layout,
            id,
            "cancel",
            Some(&e.nonce),
            false,
            &idle(),
            &mut |_, _| Ok(output(idle()))
        )
        .unwrap(),
        "cancelled\n"
    );
    let login = apply(&layout, id, "login", None, true, &idle(), &mut |_, _| {
        Ok(output(idle()))
    })
    .unwrap();
    let login: Execution = serde_json::from_str(&login).unwrap();
    let state: serde_json::Value =
        serde_json::from_slice(&fs::read(path(&layout, id, "login")).unwrap()).unwrap();
    assert_eq!(state[0], "in-progress");
    apply(
        &layout,
        id,
        "cancel",
        Some(&login.nonce),
        true,
        &idle(),
        &mut |_, _| Ok(output(idle())),
    )
    .unwrap();
    let state: serde_json::Value =
        serde_json::from_slice(&fs::read(path(&layout, id, "login")).unwrap()).unwrap();
    assert_eq!(state[0], "cancelled");
    fs::write(path(&layout, id, "closed"), b"").unwrap();
    assert!(
        apply(&layout, id, "models", None, false, &idle(), &mut |_, _| Ok(
            output(idle())
        ))
        .is_err()
    );
}

#[tokio::test]
async fn production_finite_supervisor_preserves_context_and_bounds_output() {
    let (dir, layout, prepared, mut cat, ctx) = fixture();
    let id = &prepared.instance.unit_id;
    let notify_path = dir.path().join("notify");
    let socket = std::os::unix::net::UnixDatagram::bind(&notify_path).unwrap();
    socket
        .set_read_timeout(Some(std::time::Duration::from_secs(2)))
        .unwrap();

    std::env::set_var("ANTHROPIC_API_KEY", "inherited-provider-sentinel");
    let program = dir.path().join("grok");
    fs::write(
        &program,
        r##"#!/usr/bin/python3
import os,sys,json
assert 'ANTHROPIC_API_KEY' not in os.environ
assert 'BUZZ_PRIVATE_KEY' not in os.environ
assert 'BUZZ_AUTH_TAG' not in os.environ
assert os.getcwd()==os.environ['HOME']
assert os.environ['GROK_HOME']==os.environ['HOME']+'/.grok'
assert os.environ['XDG_CONFIG_HOME']==os.environ['HOME']+'/.config'
assert sys.argv[1:]==['login','--device-auth']
open('native-login-state','w').write('synthetic')
print('device-code-private-sentinel',flush=True)
"##,
    )
    .unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o700)).unwrap();
    cat.runtimes.get_mut("native/grok").unwrap().adapter.path = program.clone();
    let e = Execution::new(
        &prepared.instance,
        Mode::Login,
        nostr::Timestamp::now().as_secs(),
    )
    .unwrap();
    let r = crate::runtime_context::finite(&ctx, &prepared, &cat, &e, NetworkPolicy::Disabled)
        .await
        .unwrap();
    assert!(r.finished);
    assert_eq!(
        r.login,
        buzz_core::host_deployment::observation::LoginState::Succeeded
    );
    assert!(r.login_output.is_empty());
    assert!(ctx.home.join("native-login-state").is_file());
    assert!(!ctx.runtime.join(e.filename()).exists());
    assert_eq!(
        read_output(&layout.state_root, id, &e)
            .unwrap()
            .unwrap()
            .execution,
        e
    );
    let mut signal = [0u8; 16];
    let received = socket.recv(&mut signal).unwrap();
    assert_eq!(&signal[..received], b"READY=1");
    // A new process uses the same HOME and sees the persisted synthetic state.
    fs::write(&program,"#!/usr/bin/python3\nimport os,sys\nassert open('native-login-state').read()=='synthetic'\nsys.stdout.write('x'*70000)\n").unwrap();
    let e = Execution::new(
        &prepared.instance,
        Mode::Login,
        nostr::Timestamp::now().as_secs(),
    )
    .unwrap();
    let r = crate::runtime_context::finite(&ctx, &prepared, &cat, &e, NetworkPolicy::Disabled)
        .await
        .unwrap();
    assert_eq!(
        r.login,
        buzz_core::host_deployment::observation::LoginState::Failed
    );
    fs::write(
        &program,
        "#!/usr/bin/python3\nimport time\ntime.sleep(10)\n",
    )
    .unwrap();
    let mut e = Execution::new(
        &prepared.instance,
        Mode::Login,
        nostr::Timestamp::now().as_secs(),
    )
    .unwrap();
    e.deadline = nostr::Timestamp::now().as_secs() + 1;
    let r = crate::runtime_context::finite(&ctx, &prepared, &cat, &e, NetworkPolicy::Disabled)
        .await
        .unwrap();
    assert_eq!(
        r.login,
        buzz_core::host_deployment::observation::LoginState::Cancelled
    );

    std::env::remove_var("ANTHROPIC_API_KEY");
}

#[test]
fn installed_binding_preserves_one_journal_and_offline_same_unit() {
    let dropin = include_str!("../deploy/buzz-host-registration.service.d/50-deployment.conf");
    let helper = include_str!("../deploy/buzz-hostd-activate@.service");
    let base = include_str!("../deploy/buzz-host-registration.service");
    let unit = include_str!("../deploy/buzz-paired-agent@.service");
    assert!(base.contains("StateDirectory=buzz-host-registration\n"));
    assert!(dropin.contains("User=buzz-hostd\nGroup=buzz-hostd"));
    assert!(dropin.contains("BindPaths=/var/lib/buzz-host-registration:/var/lib/buzz-hostd"));
    assert!(
        helper.contains("BindReadOnlyPaths=/var/lib/buzz-host-registration:/var/lib/buzz-hostd")
    );
    assert!(dropin.contains("registration run --deployment"));
    assert!(unit.contains("PrivateNetwork=yes"));
    assert!(unit.contains("RootDirectory=/var/lib/buzz-agent-root"));
    assert!(unit.contains("KillMode=control-group"));
    assert!(unit.contains("StandardOutput=null"));
    assert!(unit.contains("LoadCredential=execution:/var/lib/buzz-paired-units/%i.execution"));
    assert!(serde_json::from_str::<NetworkPolicy>("\"online\"").is_err());
    assert_eq!(
        serde_json::from_str::<NetworkPolicy>(include_str!("../deploy/execution-policy.json"))
            .unwrap(),
        NetworkPolicy::Disabled
    );
}

#[test]
fn prepare_refresh_refuses_live_unit_and_wrong_staging_uid_before_stop() {
    let (_dir, layout, mut prepared, _cat, _ctx) = fixture();
    let id = prepared.instance.unit_id.clone();
    let original = fs::read(path(&layout, &id, "cred")).unwrap();
    let d = prepared.instance.deployment.as_mut().unwrap();
    d.revision += 1;
    d.operation_id = "dd".repeat(32);
    fs::write(
        layout.staging.join(format!("{id}.cred")),
        serde_json::to_vec(&prepared.instance).unwrap(),
    )
    .unwrap();
    fs::set_permissions(
        layout.staging.join(format!("{id}.cred")),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    let shape = format!(
        "{}FragmentPath={}\nDropInPaths=\n",
        idle().replace("inactive", "active"),
        layout.template.display()
    );
    for uid in [layout.root_uid, layout.root_uid.wrapping_add(1)] {
        let mut calls = Vec::new();
        assert!(crate::activation::apply(
            &layout,
            uid,
            &format!("prepare {id}"),
            &mut |args, _| {
                calls.push(args[0].to_string());
                Ok(output(shape.clone()))
            }
        )
        .is_err());
        assert_eq!(calls, ["show"]);
        assert_eq!(fs::read(path(&layout, &id, "cred")).unwrap(), original);
    }
}

#[tokio::test]
#[ignore = "requires a built buzz-acp binary supplied by BUZZ_TEST_ACP"]
async fn actual_acp_models_uses_fixed_private_context_without_prompt_or_entitlement() {
    let (dir, layout, mut prepared, mut cat, ctx) = fixture();
    let _notify = std::os::unix::net::UnixDatagram::bind(&ctx.notify).unwrap();
    let harness =
        PathBuf::from(std::env::var_os("BUZZ_TEST_ACP").expect("explicit built ACP binary"));
    assert!(harness.is_file());
    let adapter = dir.path().join("synthetic-acp");
    fs::write(&adapter,r##"#!/usr/bin/python3
import json,os,sys
assert 'BUZZ_PRIVATE_KEY' not in os.environ
assert 'BUZZ_AUTH_TAG' not in os.environ
assert os.getcwd()==os.environ['HOME']
assert os.environ['GROK_HOME']==os.getcwd()+'/.grok'
log=open('acp-methods','a',buffering=1)
mode=open('fixture-mode').read()
for line in sys.stdin:
 m=json.loads(line); method=m.get('method'); log.write(str(method)+'\n')
 if method=='initialize': r={'protocolVersion':1,'agentInfo':{'name':'synthetic','version':'1'},'agentCapabilities':{}}
 elif method=='session/new':
  assert m['params'].get('mcpServers',[])==[]
  if mode=='error': print(json.dumps({'jsonrpc':'2.0','id':m['id'],'error':{'code':-32602,'message':'private error'}}),flush=True); continue
  options=[] if mode=='empty' else [{'category':'model','id':'model','currentValue':'native-model','type':'select','options':[{'value':'native-model','name':'Native model'}]}]
  r={'sessionId':'s','configOptions':options}
 else: raise AssertionError('unexpected '+str(method))
 print(json.dumps({'jsonrpc':'2.0','id':m['id'],'result':r}),flush=True)
"##).unwrap();
    fs::set_permissions(&adapter, fs::Permissions::from_mode(0o700)).unwrap();
    let rt = cat.runtimes.get_mut("native/grok").unwrap();
    rt.harness.path = harness;
    rt.adapter.path = adapter;
    let runtime = cat.runtimes.remove("native/grok").unwrap();
    for harness_id in ["grok", "claude"] {
        let installed_id = format!("approved/{harness_id}");
        let mut runtime = runtime.clone();
        runtime.harness_id = Some(harness_id.into());
        cat.runtimes = [(installed_id.clone(), runtime)].into();
        cat.validate().unwrap();
        prepared.instance.runtime_id = installed_id.clone();
        prepared
            .instance
            .deployment
            .as_mut()
            .unwrap()
            .config
            .runtime_id = installed_id;
        if harness_id == "claude" {
            let login = Execution::new(
                &prepared.instance,
                Mode::Login,
                nostr::Timestamp::now().as_secs(),
            )
            .unwrap();
            let error = crate::runtime_context::finite(
                &ctx,
                &prepared,
                &cat,
                &login,
                NetworkPolicy::Disabled,
            )
            .await
            .unwrap_err();
            assert!(error.to_string().contains("native login unsupported"));
        }
        for mode in ["normal", "empty", "error"] {
            fs::write(ctx.home.join("fixture-mode"), mode).unwrap();
            let e = Execution::new(
                &prepared.instance,
                Mode::Models,
                nostr::Timestamp::now().as_secs(),
            )
            .unwrap();
            let result =
                crate::runtime_context::finite(&ctx, &prepared, &cat, &e, NetworkPolicy::Disabled)
                    .await
                    .unwrap();
            assert!(result.finished);
            assert_eq!(
                result.login,
                buzz_core::host_deployment::observation::LoginState::NotObserved
            );
            let reason = if mode == "error" {
                buzz_core::host_deployment::observation::UnavailableReason::Execution
            } else {
                buzz_core::host_deployment::observation::UnavailableReason::NetworkDisabled
            };
            assert_eq!(
                result.models,
                buzz_core::host_deployment::observation::Models::Unavailable { reason }
            );
            assert!(
                read_output(&layout.state_root, &prepared.instance.unit_id, &e)
                    .unwrap()
                    .is_some()
            );
        }
    }
    let methods = fs::read_to_string(ctx.home.join("acp-methods")).unwrap();
    assert_eq!(methods.lines().filter(|m| *m == "initialize").count(), 6);
    assert_eq!(methods.lines().filter(|m| *m == "session/new").count(), 6);
    assert!(!methods.contains("session/prompt"));
}

#[test]
fn stopped_login_handover_retains_outcome_and_refresh_retires_outputs() {
    let (_dir, layout, mut prepared, _cat, ctx) = fixture();
    let id = prepared.instance.unit_id.clone();
    let mut system = |_: &[&str], _: &str| Ok(output(idle()));
    for refresh in [false, true, true] {
        let e: Execution = serde_json::from_str(
            &apply(&layout, &id, "login", None, true, &idle(), &mut system).unwrap(),
        )
        .unwrap();
        let result = ActionResult {
            execution: e.clone(),
            finished: true,
            observed_at: nostr::Timestamp::now().as_secs(),
            login: LoginState::Succeeded,
            login_at: None,
            models: buzz_core::host_deployment::observation::Models::Unsupported,
            login_output: String::new(),
        };
        fs::write(
            ctx.home.join(e.filename()),
            serde_json::to_vec(&result).unwrap(),
        )
        .unwrap();
        fs::set_permissions(
            ctx.home.join(e.filename()),
            fs::Permissions::from_mode(0o600),
        )
        .unwrap();
        if refresh {
            let d = prepared.instance.deployment.as_mut().unwrap();
            d.revision += 1;
            d.operation_id = format!("{:064x}", d.revision);
            let stage = layout.staging.join(format!("{id}.cred"));
            fs::write(&stage, serde_json::to_vec(&prepared.instance).unwrap()).unwrap();
            fs::set_permissions(stage, fs::Permissions::from_mode(0o600)).unwrap();
            crate::activation::apply(
                &layout,
                layout.root_uid,
                &format!("prepare {id}"),
                &mut |_, _| {
                    let drop = dropin(&layout, &id).unwrap();
                    Ok(output(format!(
                        "{}FragmentPath={}\nDropInPaths={}\n",
                        idle(),
                        layout.template.display(),
                        if drop.exists() {
                            drop.display().to_string()
                        } else {
                            String::new()
                        }
                    )))
                },
            )
            .unwrap();
        } else {
            apply(&layout, &id, "models", None, false, &idle(), &mut system).unwrap();
        }
        let (state, at): (LoginState, u64) =
            serde_json::from_slice(&fs::read(path(&layout, &id, "login")).unwrap()).unwrap();
        assert_eq!(state, LoginState::Succeeded);
        assert_eq!(at, result.observed_at);
        assert_eq!(fs::read_dir(&ctx.home).unwrap().count(), 0);
        assert_eq!(fs::read_dir(&ctx.runtime).unwrap().count(), 0);
        assert!(apply(
            &layout,
            &id,
            "observe",
            Some(&e.nonce),
            true,
            &idle(),
            &mut system
        )
        .is_err());
    }
    // An interrupted unlink/reload retains the finite descriptor; stale manager
    // advertisement is recoverable only when stopped, and retry finishes restore.
    apply(&layout, &id, "models", None, false, &idle(), &mut system).unwrap();
    let advertised = format!(
        "{}DropInPaths={}\n",
        idle(),
        dropin(&layout, &id).unwrap().display()
    );
    assert!(
        restore(&layout, &prepared.instance, &mut |_, _| Err(anyhow!(
            "reload interrupted"
        )))
        .is_err()
    );
    assert!(shape(&layout, &id, &advertised).is_ok());
    assert!(shape(&layout, &id, &advertised.replace("inactive", "active")).is_err());
    restore(&layout, &prepared.instance, &mut system).unwrap();
    clear(&layout, &id, &mut system).unwrap();
    assert_eq!(fs::read_dir(&ctx.home).unwrap().count(), 0);
    assert!(!path(&layout, &id, "execution").exists());
}

#[test]
fn failed_login_start_and_missing_terminal_output_never_stay_in_progress() {
    let (_dir, layout, prepared, _cat, _ctx) = fixture();
    let id = &prepared.instance.unit_id;
    for transport_error in [false, true] {
        assert!(
            apply(&layout, id, "login", None, true, &idle(), &mut |args, _| {
                if args[0] == "start" {
                    if transport_error {
                        return Err(anyhow!("manager unavailable"));
                    }
                    let mut r = output(String::new());
                    r.status = std::process::ExitStatus::from_raw(256);
                    return Ok(r);
                }
                Ok(output(idle()))
            })
            .is_err()
        );
        let state: (LoginState, u64) =
            serde_json::from_slice(&fs::read(path(&layout, id, "login")).unwrap()).unwrap();
        assert_eq!(state.0, LoginState::Failed);
    }
    apply(&layout, id, "login", None, true, &idle(), &mut |_, _| {
        Ok(output(idle()))
    })
    .unwrap();
    apply(&layout, id, "models", None, false, &idle(), &mut |_, _| {
        Ok(output(idle()))
    })
    .unwrap();
    let state: (LoginState, u64) =
        serde_json::from_slice(&fs::read(path(&layout, id, "login")).unwrap()).unwrap();
    assert_eq!(state.0, LoginState::Failed);
}
