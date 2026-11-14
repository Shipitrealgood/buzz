use super::*;
use std::{collections::BTreeMap, fs, os::unix::fs::PermissionsExt, path::Path};

pub struct Fixture {
    pub state: Arc<buzz_relay::AppState>,
    pub addr: std::net::SocketAddr,
    pub community: buzz_core::CommunityId,
    tasks: Vec<tokio::task::JoinHandle<()>>,
    audit: Option<buzz_relay::state::AuditShutdownHandle>,
}
impl Fixture {
    pub async fn new(database: &str) -> Result<Self> {
        // Caller reexecutes this test in a clean environment and disposable cwd.
        let mut config = buzz_relay::Config::from_env()?;
        config.database_url = database.into();
        config.require_auth_token = false;
        config.require_relay_membership = true;
        config.host_registry_enabled = true;
        config.host_deployment_enabled = true;
        config.relay_url = ORIGIN.replace("https://", "wss://");
        let pool = sqlx::PgPool::connect(database).await?;
        let db = buzz_db::Db::from_pool(pool.clone());
        let redis = deadpool_redis::Config::from_url(&config.redis_url)
            .create_pool(Some(deadpool_redis::Runtime::Tokio1))?;
        let pubsub =
            Arc::new(buzz_pubsub::PubSubManager::new(&config.redis_url, redis.clone()).await?);
        let auth = buzz_auth::AuthService::new(config.auth.clone());
        let search = buzz_search::SearchService::new(pool.clone());
        let workflow = Arc::new(buzz_workflow::WorkflowEngine::new(
            db.clone(),
            Default::default(),
        ));
        let media = buzz_media::MediaStorage::new(&config.media)?;
        let (state, audit) = buzz_relay::AppState::new(
            config,
            db,
            redis,
            buzz_audit::AuditService::new(pool),
            pubsub.clone(),
            auth,
            search,
            workflow,
            Keys::generate(),
            media,
        );
        let community = state
            .db
            .ensure_configured_community("discovery-fixture.test")
            .await?
            .id;
        let state = Arc::new(state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let router = buzz_relay::router::build_router(state.clone());
        let server = tokio::spawn(async move {
            axum::serve(
                listener,
                router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
            )
            .await
            .unwrap();
        });
        let subscriber = tokio::spawn(async move {
            let _ = pubsub.run_subscriber().await;
        });
        Ok(Self {
            state,
            addr,
            community,
            tasks: vec![server, subscriber],
            audit: Some(audit),
        })
    }
    pub async fn socket(&self) -> Result<NostrWsConnection> {
        // Existing registration fixture idiom: canonical Host over loopback WS.
        // TLS is not under test; signatures, NIP-42 and encrypted payloads are real.
        Ok(NostrWsConnection::connect_to_addr(
            "wss://discovery-fixture.test",
            &self.addr.to_string(),
        )
        .await?)
    }
    pub async fn owner(&self, owner: &Keys) -> Result<Registry> {
        let mut ws = self.socket().await?;
        ws.authenticate(owner, None).await?;
        Registry::from_authenticated(
            ORIGIN,
            owner.public_key(),
            self.state.relay_keypair.public_key(),
            ws,
        )
    }
    pub async fn effects(&self) -> Result<(i64, i64, i64, i64)> {
        Ok(sqlx::query_as("SELECT (SELECT count(*) FROM users), (SELECT count(*) FROM events), (SELECT count(*) FROM paired_agent_operations), (SELECT count(*) FROM paired_agent_placements)")
            .fetch_one(self.state.db.pool()).await?)
    }
    pub async fn stop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
        for task in self.tasks.drain(..) {
            let _ = task.await;
        }
        if let Some(audit) = self.audit.take() {
            audit.drain(Duration::from_secs(5)).await;
        }
        self.state.db.pool().close().await;
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
    }
}

fn script(path: &Path, contents: &str) -> Result<()> {
    fs::write(path, contents)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

pub fn installation(
    root: &Path,
    state: &Path,
    relay: nostr::PublicKey,
) -> Result<buzz_hostd::deployment::Helper> {
    for name in ["home", "workspace", "tools", "units"] {
        fs::create_dir(root.join(name))?;
    }
    fs::write(root.join("home/fixture-login"), "synthetic; preserve")?;
    fs::write(root.join("workspace/user-work"), "preserve")?;
    let binaries = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/debug")
        .canonicalize()?;
    ensure!(
        binaries.join("buzz-host-runtime").is_file() && binaries.join("buzz-acp").is_file(),
        "build cached runtime/ACP first"
    );
    script(
        &root.join("tools/claude"),
        "#!/bin/sh\ntest \"$1 $2\" = 'auth status'\n",
    )?;
    script(
        &root.join("tools/claude-agent-acp"),
        r#"#!/usr/bin/python3
import json,os,sys
assert 'BUZZ_PRIVATE_KEY' not in os.environ
for line in sys.stdin:
 m=json.loads(line)
 if m.get('method')=='initialize': r={'protocolVersion':1,'agentInfo':{'name':'synthetic','version':'1'},'agentCapabilities':{}}
 elif m.get('method')=='session/new':
  assert m['params'].get('mcpServers',[])==[]
  r={'sessionId':'s','configOptions':[{'category':'model','id':'model','currentValue':'opaque/model:fixture','type':'select','options':[{'value':'opaque/model:fixture','name':'Fixture'}]}]}
 else: raise AssertionError(str(m.get('method')))
 print(json.dumps({'jsonrpc':'2.0','id':m['id'],'result':r}),flush=True)
"#,
    )?;
    script(
        &root.join("tools/harness"),
        &format!(
            r#"#!/usr/bin/python3
import json,os,pathlib,socket,sys
root=pathlib.Path({root:?})
assert os.environ['HOME']==str(root/'home')
assert os.getcwd()==str(root/'workspace')
if sys.argv[1:]==['models','--json']:
 assert 'BUZZ_PRIVATE_KEY' not in os.environ
 with open(root/'discovery-calls','a') as f: f.write('.')
 os.execv({acp:?},[{acp:?},'models','--json'])
assert sys.argv[1:]==[]
assert os.environ['BUZZ_ACP_MODEL']=='opaque/model:fixture'
assert os.environ['BUZZ_ACP_STRICT_MODEL']=='true'
assert os.environ['BUZZ_ACP_RESPOND_TO']=='owner-only'
assert os.environ['BUZZ_RELAY_URL']=='wss://discovery-fixture.test'
assert os.environ['BUZZ_PRIVATE_KEY'] and os.environ['BUZZ_AUTH_TAG']
(root/'conversation').write_text(os.environ['BUZZ_ACP_MODEL'])
s=socket.socket(socket.AF_UNIX,socket.SOCK_DGRAM);s.connect(os.environ['NOTIFY_SOCKET']);s.send(b'READY=1')
"#,
            root = root.display().to_string(),
            acp = binaries.join("buzz-acp").display().to_string()
        ),
    )?;
    // Adapted existing UserService fake. Only the fixture launcher is run; no host systemctl.
    script(
        &root.join("manager"),
        &format!(
            r#"#!/usr/bin/python3
import os,pathlib,socket,subprocess,sys
root=pathlib.Path(__file__).parent
assert sys.argv[1:3]==['--user','--no-pager']
verb=sys.argv[3]; unit=sys.argv[-1]
with open(root/'calls','a') as f: f.write(verb+'\n')
if verb=='show':
 print('LoadState=loaded\nFragmentPath='+str(root/'units/buzz-hosted-agent@.service'))
 print('DropInPaths='+str(root/'units'/(unit+'.d/50-buzz.conf')) if (root/'units'/(unit+'.d/50-buzz.conf')).exists() else 'DropInPaths=')
 active=(root/'active').exists()
 print('ActiveState='+('active' if active else 'inactive'))
 print('MainPID='+('42' if active else '0')+'\nControlPID=0\nTasksCurrent='+('1' if active else '0'))
elif verb=='start':
 id=unit.removeprefix('buzz-hosted-agent@').removesuffix('.service')
 notify=root/'notify'; notify.unlink(missing_ok=True)
 s=socket.socket(socket.AF_UNIX,socket.SOCK_DGRAM);s.bind(str(notify));s.settimeout(3)
 result=subprocess.run([{runtime:?},'--user-service',{state:?},id],env={{'CREDENTIALS_DIRECTORY':str(pathlib.Path({state:?})/'hosted'/id),'NOTIFY_SOCKET':str(notify)}},timeout=30)
 assert result.returncode==0 and s.recv(32)==b'READY=1'
 s.close();notify.unlink();(root/'active').touch()
elif verb=='stop': (root/'active').unlink(missing_ok=True)
elif verb!='daemon-reload': raise AssertionError(verb)
"#,
            runtime = binaries.join("buzz-host-runtime").display().to_string(),
            state = state.display().to_string()
        ),
    )?;
    let cat = buzz_hostd::Catalog {
        local: Some(buzz_hostd::user_service::LocalContext {
            buzz_acp: Some(root.join("tools/harness")),
            home: root.join("home"),
            workspace: root.join("workspace"),
            tool_path: format!("{}:/usr/bin:/bin", root.join("tools").display()),
            environment: BTreeMap::new(),
            unit_dir: root.join("units"),
            launcher: binaries.join("buzz-host-runtime"),
            systemctl: root.join("manager"),
        }),
        community: Some(ORIGIN.into()),
        source: None,
        label: "Disposable fixture".into(),
        capacity: 2,
        relay_key: relay.to_hex(),
        runtimes: BTreeMap::new(),
    };
    let path = state.join("deployment.json");
    fs::write(&path, serde_json::to_vec(&cat)?)?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    buzz_hostd::deployment::Helper::installed(state)
}
