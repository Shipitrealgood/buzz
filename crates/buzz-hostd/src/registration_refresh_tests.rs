//! Synthetic signed WS fixture. No external relay, identity or service access.
use super::*;
use buzz_core::{host_registry as wire, paired_message::Scope};
use futures_util::{SinkExt, StreamExt};
use nostr::{Event, Keys};
use serde_json::{json, Value};
use std::{
    os::unix::fs::PermissionsExt,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
};
use tokio::sync::broadcast;
use tokio_tungstenite::tungstenite::Message;

#[derive(Default)]
struct Counts {
    auth: AtomicUsize,
    pages: AtomicUsize,
    live: AtomicUsize,
    pongs: AtomicUsize,
    open: AtomicUsize,
    writes: Mutex<Vec<String>>,
    order: Mutex<Vec<String>>,
    dead: AtomicBool,
    drop_write: AtomicBool,
    during_read: AtomicBool,
}
struct Fixture {
    settings: Settings,
    counts: Arc<Counts>,
    commands: broadcast::Sender<&'static str>,
    server: tokio::task::JoinHandle<()>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.server.abort();
    }
}
impl Fixture {
    async fn new() -> Self {
        let relay = Keys::generate();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let settings = Settings {
            deployment: false,
            community: "https://refresh.test".into(),
            url: format!("ws://{}", listener.local_addr().unwrap()),
            relay: relay.public_key(),
            label: "synthetic".into(),
            connector: None,
        };
        let counts = Arc::new(Counts::default());
        let (commands, _) = broadcast::channel(64);
        let (counter, sender) = (counts.clone(), commands.clone());
        let community = settings.community.clone();
        let server = tokio::spawn(async move {
            let mut sockets = JoinSet::new();
            loop {
                tokio::select! {
                    accepted = listener.accept() => {
                        let (stream, _) = accepted.unwrap();
                        let (counts, relay, community, mut commands) = (counter.clone(), relay.clone(), community.clone(), sender.subscribe());
                        sockets.spawn(async move {
                            let mut socket = tokio_tungstenite::accept_async(stream).await.unwrap();
                            counts.open.fetch_add(1, Ordering::SeqCst);
                            struct Open(Arc<Counts>);
                            impl Drop for Open { fn drop(&mut self) { self.0.open.fetch_sub(1, Ordering::SeqCst); } }
                            let _open = Open(counts.clone());
                            socket.send(Message::Text(json!(["AUTH","synthetic"]).to_string().into())).await.unwrap();
                            let mut who = None;
                            let mut live: Option<(String, wire::Query)> = None;
                            let mut high = 0u64;
                            loop {
                                tokio::select! {
                                    command = commands.recv() => {
                                        let Ok(command) = command else { break };
                                        if command == "close" { break; }
                                        if command == "ping" {
                                            if socket.send(Message::Ping(vec![1,2].into())).await.is_err() { break; }
                                        } else if let Some((sub,q)) = &live {
                                            high += 1;
                                            let mut reply = wire::Reply {
                                                v:2, community:community.clone(), recipient: who.unwrap_or(relay.public_key()).to_hex(),
                                                op:"registry-change".into(), query:q.clone(), high_water:format!("{high:064x}"),
                                                data:json!({"generation":q.generation}), complete:true, next_cursor:None,
                                            };
                                            let signer = if command == "untrusted" { Keys::generate() } else { relay.clone() };
                                            if command == "wrong-scope" { reply.community = "https://other.test".into(); }
                                            if command == "wrong-generation" { reply.data["generation"] = "ab".repeat(32).into(); }
                                            let mut event = wire::reply(&signer, who.unwrap(), reply, RegistrationClock::read().unwrap().wall).unwrap();
                                            if command == "malformed" { event.content = "not ciphertext".into(); }
                                            if socket.send(Message::Text(json!(["EVENT",sub,event]).to_string().into())).await.is_err() { break; }
                                        }
                                    }
                                    frame = socket.next() => {
                                        let Some(Ok(frame)) = frame else { break };
                                        if matches!(frame, Message::Pong(_)) { counts.pongs.fetch_add(1,Ordering::SeqCst); continue; }
                                        let Message::Text(text) = frame else { continue };
                                        let frame: Value = serde_json::from_str(&text).unwrap();
                                        match frame[0].as_str().unwrap() {
                                            "AUTH" => {
                                                let event: Event = serde_json::from_value(frame[1].clone()).unwrap();
                                                event.verify().unwrap(); who = Some(event.pubkey);
                                                counts.auth.fetch_add(1,Ordering::SeqCst);
                                                socket.send(Message::Text(json!(["OK",event.id,true,""]).to_string().into())).await.unwrap();
                                            }
                                            "EVENT" => {
                                                let event: Event = serde_json::from_value(frame[1].clone()).unwrap();
                                                event.verify().unwrap();
                                                counts.writes.lock().unwrap().push(event.id.to_hex());
                                                counts.order.lock().unwrap().push("write".into());
                                                if counts.drop_write.swap(false,Ordering::SeqCst) { break; }
                                                socket.send(Message::Text(json!(["OK",event.id,true,""]).to_string().into())).await.unwrap();
                                            }
                                            "REQ" => {
                                                let q: wire::Query = serde_json::from_value(frame[2]["host_registry"].clone()).unwrap();
                                                let sub = frame[1].as_str().unwrap().to_string();
                                                counts.order.lock().unwrap().push(q.view.clone());
                                                let data = match q.view.as_str() {
                                                    "live" => {counts.live.fetch_add(1,Ordering::SeqCst); live=Some((sub.clone(),q.clone())); json!({})},
                                                    "binding" => { counts.pages.fetch_add(1,Ordering::SeqCst); json!({"state":if counts.dead.load(Ordering::SeqCst){"dead"}else{"active"}}) },
                                                    "offers" | "messages" => {counts.pages.fetch_add(1,Ordering::SeqCst); json!({"events":[]})},
                                                    other => panic!("unexpected query {other}"),
                                                };
                                                if q.view == "binding" && counts.during_read.swap(false,Ordering::SeqCst) {
                                                    let (live_sub,live_q) = live.as_ref().unwrap();
                                                    high += 1;
                                                    let hint = wire::reply(&relay,who.unwrap(),wire::Reply {
                                                        v:2, community:community.clone(), recipient:who.unwrap().to_hex(),op:"registry-change".into(),
                                                        query:live_q.clone(),high_water:format!("{high:064x}"),data:json!({"generation":live_q.generation}),complete:true,next_cursor:None,
                                                    },RegistrationClock::read().unwrap().wall).unwrap();
                                                    socket.send(Message::Text(json!(["EVENT",live_sub,hint]).to_string().into())).await.unwrap();
                                                }
                                                let reply = wire::reply(&relay, who.unwrap(), wire::Reply {
                                                    v:2, community:community.clone(), recipient:who.unwrap().to_hex(), op:"registry-page".into(),
                                                    query:q, high_water:format!("{high:064x}"), data, complete:true, next_cursor:None,
                                                },RegistrationClock::read().unwrap().wall).unwrap();
                                                if socket.send(Message::Text(json!(["EVENT",sub,reply]).to_string().into())).await.is_err() { break; }
                                                if socket.send(Message::Text(json!(["EOSE",sub]).to_string().into())).await.is_err() { break; }
                                            }
                                            other => panic!("unexpected frame {other}"),
                                        }
                                    }
                                }
                            }
                        });
                    }
                    _ = sockets.join_next(), if !sockets.is_empty() => {}
                }
            }
        });
        Self {
            settings,
            counts,
            commands,
            server,
        }
    }
}
fn host(active: bool) -> (tempfile::TempDir, Host, String) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut host = Host::at(dir.path(), true).unwrap();
    let g = offer(&mut host, active);
    (dir, host, g)
}
fn offer(host: &mut Host, active: bool) -> String {
    let clock = RegistrationClock::read().unwrap();
    let invitation: Value =
        serde_json::from_str(&host.offer("https://refresh.test".into(), &clock).unwrap()).unwrap();
    let generation = invitation["generation"].as_str().unwrap().to_string();
    if active {
        let owner = Keys::generate();
        let scope = Scope {
            community: "https://refresh.test".into(),
            generation: generation.clone(),
            owner: owner.public_key(),
            host: host.public_key().unwrap(),
        };
        let claim = wire::seal(
            &owner,
            &scope,
            wire::Body::PairClaim {
                proof: invitation["proof"].as_str().unwrap().into(),
            },
            "12".repeat(32),
            None,
            clock.wall,
            invitation["expires_at"].as_u64().unwrap(),
        )
        .unwrap();
        host.claim(&generation, &claim, &clock).unwrap();
        for event in host.pending().unwrap() {
            host.delivered(&event.id.to_hex()).unwrap();
        }
    }
    generation
}
async fn wait_for(mut condition: impl FnMut() -> bool) {
    tokio::time::timeout(Duration::from_secs(10), async {
        while !condition() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn idle_retains_auth_and_snapshot_and_services_heartbeat() {
    let f = Fixture::new().await;
    let (dir, _host, _g) = host(true);
    let driver = async {
        wait_for(|| f.counts.pages.load(Ordering::SeqCst) == 2).await;
        for _ in 0..12 {
            f.commands.send("ping").unwrap();
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        assert_eq!(f.counts.auth.load(Ordering::SeqCst), 1);
        assert_eq!(f.counts.live.load(Ordering::SeqCst), 1);
        assert_eq!(f.counts.pages.load(Ordering::SeqCst), 2);
        assert_eq!(f.counts.open.load(Ordering::SeqCst), 1);
        assert_eq!(f.counts.pongs.load(Ordering::SeqCst), 12);
        println!("REFRESH_IDLE seconds=12 auth=1 live=1 finite_pages=2 pongs=12 open=1");
    };
    tokio::select! { result=run(dir.path(),f.settings.clone(),false) => panic!("controller exited: {result:?}"), _=driver=>{} }
}

#[tokio::test]
async fn local_new_generation_and_offline_revoke_progress_without_reauth() {
    let f = Fixture::new().await;
    let (dir, mut host, g) = host(true);
    let driver = async {
        wait_for(|| f.counts.pages.load(Ordering::SeqCst) == 2).await;
        host.revoke(&g, &RegistrationClock::read().unwrap())
            .unwrap();
        let other = offer(&mut host, false);
        wait_for(|| f.counts.auth.load(Ordering::SeqCst) == 2).await;
        wait_for(|| !host.has_pending(&g).unwrap()).await;
        wait_for(|| f.counts.open.load(Ordering::SeqCst) == 1).await;
        assert_eq!(host.state(&g).unwrap(), "dead");
        assert_eq!(host.state(&other).unwrap(), "offered");
        assert_eq!(f.counts.auth.load(Ordering::SeqCst), 2);
        println!("REFRESH_LOCAL new_generation=observed revoked_pending=delivered retired_socket=closed auth=2");
    };
    tokio::select! { result=run(dir.path(),f.settings.clone(),false) => panic!("controller exited: {result:?}"), _=driver=>{} }
}

#[tokio::test]
async fn disconnected_exact_restrictive_write_retries_before_live_read() {
    let f = Fixture::new().await;
    let (dir, mut host, g) = host(true);
    host.revoke(&g, &RegistrationClock::read().unwrap())
        .unwrap();
    let exact = host.pending().unwrap()[0].id.to_hex();
    f.counts.drop_write.store(true, Ordering::SeqCst);
    let driver = async {
        wait_for(|| f.counts.writes.lock().unwrap().len() >= 2).await;
        wait_for(|| !host.has_pending(&g).unwrap()).await;
        assert_eq!(*f.counts.writes.lock().unwrap(), vec![exact.clone(), exact]);
        let order = f.counts.order.lock().unwrap();
        assert_eq!(&order[..2], &["write", "write"]);
        println!("REFRESH_RETRY auth=2 lost_ok=1 exact_retry=true writes_before_reads=true");
    };
    tokio::select! { result=run(dir.path(),f.settings.clone(),false) => panic!("controller exited: {result:?}"), _=driver=>{} }
}

#[tokio::test]
async fn verified_bursts_coalesce_and_missed_notice_recovers_on_backstop() {
    let f = Fixture::new().await;
    let (_dir, mut host, g) = host(true);
    let mut ws = f.settings.connect().await.unwrap();
    host.authenticate(&mut ws, &g).await.unwrap();
    host.subscribe_live(&mut ws, f.settings.relay, &g)
        .await
        .unwrap();
    let driver = async {
        wait_for(|| f.counts.pages.load(Ordering::SeqCst) == 2).await;
        for _ in 0..20 {
            f.commands.send("change").unwrap();
        }
        wait_for(|| f.counts.pages.load(Ordering::SeqCst) == 4).await;
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert_eq!(f.counts.pages.load(Ordering::SeqCst), 4);
        f.counts.dead.store(true, Ordering::SeqCst); // deliberately NO hint
    };
    let session = connected(
        &mut host,
        &mut ws,
        &g,
        &f.settings,
        Duration::from_secs(2),
        Duration::from_millis(250),
        _dir.path(),
    );
    let (_, result) = tokio::join!(driver, session);
    result.unwrap();
    assert_eq!(host.state(&g).unwrap(), "dead");
    assert_eq!(f.counts.auth.load(Ordering::SeqCst), 1);
    println!("REFRESH_HINTS changes=20 coalesced_refresh=1 missed_revocation=backstop auth=1");
}

#[tokio::test]
async fn untrusted_and_wrong_scope_notifications_fail_closed() {
    for command in ["untrusted", "wrong-scope", "wrong-generation", "malformed"] {
        let f = Fixture::new().await;
        let (_dir, mut host, g) = host(true);
        let mut ws = f.settings.connect().await.unwrap();
        host.authenticate(&mut ws, &g).await.unwrap();
        host.subscribe_live(&mut ws, f.settings.relay, &g)
            .await
            .unwrap();
        let driver = async {
            wait_for(|| f.counts.pages.load(Ordering::SeqCst) == 2).await;
            f.counts.dead.store(true, Ordering::SeqCst);
            f.commands.send(command).unwrap();
        };
        let (_, result) = tokio::join!(
            driver,
            connected(
                &mut host,
                &mut ws,
                &g,
                &f.settings,
                BACKSTOP,
                COALESCE,
                _dir.path()
            )
        );
        assert!(result.is_err(), "{command}");
        assert_eq!(host.state(&g).unwrap(), "active");
        assert!(f.counts.writes.lock().unwrap().is_empty());
    }
}

#[test]
fn failure_retry_is_jittered_and_capped() {
    assert!(retry_delay(6).is_err());
    for failure in 1..6 {
        let cap = (2u64 << failure.min(5)).min(60);
        for _ in 0..100 {
            let delay = retry_delay(failure).unwrap();
            assert!(delay >= Duration::from_millis(cap * 500) && delay <= Duration::from_secs(cap));
        }
    }
}

#[tokio::test]
async fn hint_during_snapshot_is_not_discarded() {
    let f = Fixture::new().await;
    let (_dir, mut host, g) = host(true);
    let mut ws = f.settings.connect().await.unwrap();
    host.authenticate(&mut ws, &g).await.unwrap();
    host.subscribe_live(&mut ws, f.settings.relay, &g)
        .await
        .unwrap();
    f.counts.during_read.store(true, Ordering::SeqCst);
    tokio::select! {
        result=connected(&mut host,&mut ws,&g,&f.settings,BACKSTOP,Duration::from_millis(100),_dir.path()) => panic!("{result:?}"),
        _=wait_for(||f.counts.pages.load(Ordering::SeqCst)==4)=>{}
    }
    assert_eq!(f.counts.auth.load(Ordering::SeqCst), 1);
    assert_eq!(f.counts.live.load(Ordering::SeqCst), 1);
    println!("REFRESH_INTERLEAVE hint_during_binding_page=retained second_snapshot=true");
}

#[tokio::test]
async fn failed_old_generation_does_not_block_new_offer() {
    let f = Fixture::new().await;
    let (dir, mut host, g) = host(true);
    host.revoke(&g, &RegistrationClock::read().unwrap())
        .unwrap();
    let other = offer(&mut host, false);
    f.counts.drop_write.store(true, Ordering::SeqCst);
    let driver = async {
        wait_for(|| f.counts.live.load(Ordering::SeqCst) == 1).await;
        assert_eq!(host.state(&other).unwrap(), "offered");
        assert_eq!(f.counts.auth.load(Ordering::SeqCst), 2);
        assert!(host.has_pending(&g).unwrap());
        println!("REFRESH_FAIRNESS old_write_failed=true new_offer_subscribed=true");
    };
    tokio::select! {result=run(dir.path(),f.settings.clone(),false)=>panic!("{result:?}"),_=driver=>{}}
}

#[tokio::test]
async fn cleanup_failures_and_slow_generation_do_not_starve_discovery_or_rotation() {
    let f = Fixture::new().await;
    let (dir, mut h, first) = host(true);
    h.deployment_cleanup_generations(&f.settings.community)
        .unwrap();
    let db = rusqlite::Connection::open(dir.path().join("host.sqlite")).unwrap();
    // Existing journal entries for forty retired generations, each with an
    // explicit persisted cleanup. No network grant is manufactured by cleanup.
    let mut generations = vec![first];
    for i in 0..40 {
        if i > 0 {
            generations.push(offer(&mut h, true));
        }
        h.revoke(&generations[i], &RegistrationClock::read().unwrap())
            .unwrap();
        for event in h.pending().unwrap() {
            h.delivered(&event.id.to_hex()).unwrap();
        }
    }
    generations.sort();
    let mut slow_unit = String::new();
    for (i, g) in generations.iter().enumerate() {
        let id = format!("{i:064x}");
        let owner = Keys::generate();
        db.execute(
            "UPDATE grants SET owner=? WHERE g=?",
            rusqlite::params![owner.public_key().to_hex(), g],
        )
        .unwrap();
        let scope = Scope {
            community: f.settings.community.clone(),
            generation: g.clone(),
            owner: owner.public_key(),
            host: h.public_key().unwrap(),
        };
        let authority = buzz_core::host_deployment::Authority {
            v: 1,
            scope,
            agent: Keys::generate().public_key(),
            action: buzz_core::host_deployment::Action::Cleanup,
            operation_id: id.clone(),
            expected_revision: 0,
            expires_at: nostr::Timestamp::now().as_secs() + 500,
            content_sha256: String::new(),
        };
        let (authority, event) = buzz_core::host_deployment::wire::seal_command(
            &owner,
            authority,
            &buzz_core::host_deployment::Command::Cleanup {},
            nostr::Timestamp::now().as_secs(),
        )
        .unwrap();
        db.execute("INSERT INTO deployment_units(agent,generation,unit_id,operation,event,revision,cleanup) VALUES(?,?,?,?,?,1,1)", rusqlite::params![authority.agent.to_hex(),g,id,authority.operation_id,serde_json::to_string(&event).unwrap()]).unwrap();
        if i == 0 {
            let owner: String = db
                .query_row("SELECT owner FROM grants WHERE g=?", [g], |r| r.get(0))
                .unwrap();
            slow_unit = buzz_hostd::unit_id(
                &Scope {
                    community: f.settings.community.clone(),
                    owner: PublicKey::from_hex(&owner).unwrap(),
                    host: h.public_key().unwrap(),
                    generation: g.clone(),
                },
                &id,
            );
        }
    }
    let log = dir.path().join("manager.log");
    let release = dir.path().join("release");
    let manager = dir.path().join("fake-systemctl");
    std::fs::write(&manager, format!(r#"#!/usr/bin/python3
import sys,time,pathlib
root=pathlib.Path(__file__).parent
unit=sys.argv[-1]
if 'show' in sys.argv:
    with open(root/'manager.log','a') as f: f.write(unit+'\n')
    if unit=='buzz-hosted-agent@{slow_unit}.service':
        while not (root/'release').exists(): time.sleep(.02)
    print('LoadState=loaded\nFragmentPath='+str(root/'buzz-hosted-agent@.service')+'\nDropInPaths=\nActiveState=inactive\nMainPID=0\nControlPID=0\nTasksCurrent=0')
else: sys.exit(1)
"#)).unwrap();
    std::fs::set_permissions(&manager, std::fs::Permissions::from_mode(0o700)).unwrap();
    let local = json!({"home":dir.path(),"workspace":dir.path(),"tool_path":"/usr/bin:/bin","unit_dir":dir.path(),"launcher":"/usr/bin/true","systemctl":manager});
    let config = json!({"local":local,"community":f.settings.community,"label":"fixture","capacity":1,"relay_key":f.settings.relay.to_hex(),"runtimes":{"native/test":{"harness":{"path":"/usr/bin/true"},"adapter":{"path":"/usr/bin/true"},"args":[],"credential_profile":"none"}}});
    std::fs::write(
        dir.path().join("deployment.json"),
        serde_json::to_vec(&config).unwrap(),
    )
    .unwrap();
    std::fs::set_permissions(
        dir.path().join("deployment.json"),
        std::fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    buzz_hostd::deployment::Helper::installed(dir.path()).unwrap();
    // Persist the same per-unit local context used by real cleanup, without starting anything.
    let mut stmt = db
        .prepare("SELECT unit_id,generation FROM deployment_units")
        .unwrap();
    for row in stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .unwrap()
    {
        let (id, g) = row.unwrap();
        let owner: String = db
            .query_row("SELECT owner FROM grants WHERE g=?", [&g], |r| r.get(0))
            .unwrap();
        let unit = buzz_hostd::unit_id(
            &Scope {
                community: f.settings.community.clone(),
                host: h.public_key().unwrap(),
                owner: PublicKey::from_hex(&owner).unwrap(),
                generation: g,
            },
            &id,
        );
        let path = dir.path().join("hosted").join(unit);
        std::fs::create_dir(&path).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        std::fs::write(path.join("context"), serde_json::to_vec(&local).unwrap()).unwrap();
        std::fs::set_permissions(path.join("context"), std::fs::Permissions::from_mode(0o600))
            .unwrap();
    }
    let seen = || {
        std::fs::read_to_string(&log)
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect::<BTreeSet<_>>()
            .len()
    };
    let driver = async {
        wait_for(|| seen() >= 4).await;
        // Add a new invitation while one helper request is deliberately held.
        offer(&mut h, false);
        wait_for(|| f.counts.auth.load(Ordering::SeqCst) > 0).await;
        tokio::time::timeout(Duration::from_secs(20), async {
            while seen() < 40 {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM deployment_units WHERE cleanup=1 AND cleaned=0",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            40
        );
        // Arrival at the fake manager precedes its reply and the durable retry write.
        wait_for(|| {
            db.query_row(
                "SELECT count(*) FROM deployment_units WHERE issue IS NOT NULL",
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap()
                >= 39
        })
        .await;
        assert!(
            db.query_row(
                "SELECT count(*) FROM deployment_units WHERE issue IS NOT NULL",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap()
                >= 39
        );
        std::fs::write(&release, b"go").unwrap();
    };
    // Exercise the production controller constructor, not an injected executor.
    let mut settings = f.settings.clone();
    settings.deployment = true;
    std::fs::remove_file(dir.path().join("deployment.json")).unwrap();
    tokio::select! {
        result=run(dir.path(),settings.clone(),false)=>panic!("cleanup stopped controller: {result:?}"),
        _=driver=>{}
    }
    // Reopening with malformed launch config also reaches durable cleanup retries.
    std::fs::write(dir.path().join("deployment.json"), "{broken").unwrap();
    std::fs::set_permissions(
        dir.path().join("deployment.json"),
        std::fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    let before = std::fs::read_to_string(&log).unwrap().lines().count();
    // Registration/deployment reconciliation may fail; restrictive cleanup still runs.
    let _ = run(dir.path(), settings, true).await;
    assert!(std::fs::read_to_string(&log).unwrap().lines().count() > before);
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM deployment_units WHERE cleanup=1 AND cleaned=0",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        40
    );
    // Restart enumeration continues from its durable generation cursor.
    drop(h);
    let reopened = Host::at(dir.path(), false).unwrap();
    let next = reopened
        .deployment_cleanup_generations(&f.settings.community)
        .unwrap();
    assert_eq!(next.len(), 4);
    let cursor: String = db
        .query_row(
            "SELECT r FROM work_cursor WHERE g=?",
            [format!("deployment-generations:{}", f.settings.community)],
            |r| r.get(0),
        )
        .unwrap();
    generations.sort_by_key(|g| (g.as_str() <= cursor.as_str(), g.clone()));
    assert_eq!(next, generations[..4]);
}
