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
            connected(&mut host, &mut ws, &g, &f.settings, BACKSTOP, COALESCE)
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
        result=connected(&mut host,&mut ws,&g,&f.settings,BACKSTOP,Duration::from_millis(100)) => panic!("{result:?}"),
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
