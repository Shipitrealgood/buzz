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
    std::env::set_var("NOTIFY_SOCKET", &notify_path);
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
    std::env::remove_var("NOTIFY_SOCKET");
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
