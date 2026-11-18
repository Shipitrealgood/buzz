use super::*;
use std::os::unix::fs::DirBuilderExt;
struct TestDir(std::path::PathBuf);
impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn fresh() -> (TestDir, Store, Clock) {
    let path = std::env::temp_dir().join(format!("buzz-hostd-{}", random().unwrap()));
    fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
    let store = Store::load(&path, true).unwrap();
    (
        TestDir(path),
        store,
        Clock {
            source: None,
            wall: 1000,
            boot: "boot-A".into(),
            millis: 4000,
        },
    )
}
fn offer(store: &mut Store, clock: &Clock) -> (Offer, paired_host::Proof) {
    let json = store.offer("https://team.example".into(), clock).unwrap();
    let inv: serde_json::Value = serde_json::from_str(&json).unwrap();
    (
        Offer {
            community: inv["community"].as_str().unwrap().into(),
            generation: inv["generation"].as_str().unwrap().into(),
            host: store.keys().unwrap().public_key(),
            expires_at: inv["expires_at"].as_u64().unwrap(),
        },
        paired_host::Proof::from_hex(inv["proof"].as_str().unwrap()).unwrap(),
    )
}
fn claim(o: &Offer, p: &paired_host::Proof, owner: &Keys, now: u64) -> Packet {
    let (evt, _) = paired_host::build_claim(owner, o, p, now).unwrap();
    Packet {
        event: serde_json::value::to_raw_value(&evt).unwrap(),
        route: Route {
            r: None,
            prev: None,
            expires_at: o.expires_at,
        },
    }
}
fn test_runtime() -> buzz_hostd::Runtime {
    let exe = buzz_hostd::Executable {
        path: "/usr/bin/true".into(),
        sha256: hex::encode(sha2::Sha256::digest(fs::read("/usr/bin/true").unwrap())),
    };
    buzz_hostd::Runtime {
        harness_id: None,
        adapter: exe.clone(),
        harness: exe,
        args: vec![],
        credential_profile: "none".into(),
    }
}
use sha2::Digest;
#[test]
fn restart_restores_exact_ack_and_local_tombstone() {
    let (dir, mut s, time) = fresh();
    let (o, p) = offer(&mut s, &time);
    let owner = Keys::generate();
    let packet = claim(&o, &p, &owner, time.wall);
    s.claim(&o.generation, &packet, &time).unwrap();
    let ack: String =
        s.0.query_row("SELECT ack FROM grants", [], |r| r.get(0))
            .unwrap();
    drop(s);
    let mut s = Store::load(&dir.0, false).unwrap();
    s.claim(&o.generation, &packet, &time).unwrap();
    let recovered: String =
        s.0.query_row("SELECT ack FROM grants", [], |r| r.get(0))
            .unwrap();
    assert_eq!(ack, recovered);
    assert!(s.offer(o.community.clone(), &time).is_err());
    let ack: Packet = serde_json::from_str(&ack).unwrap();
    let scope = Scope {
        community: o.community.clone(),
        generation: o.generation.clone(),
        owner: owner.public_key(),
        host: o.host,
    };
    let cat = Catalog {
        local: None,
        community: Some(o.community.clone()),
        source: None,
        label: "Shared".into(),
        capacity: 2,
        relay_key: owner.public_key().to_hex(),
        runtimes: [("local/goose".into(), test_runtime())].into(),
    };
    let query = paired_message::seal(
        &owner,
        &scope,
        Body::InventoryQuery {
            pair_ack_id: packet_id(&ack).unwrap(),
            nonce: random().unwrap(),
        },
        time.wall,
        time.wall + 60,
    )
    .unwrap();
    for _ in 0..2 {
        s.receive(&o.generation, &query, &cat, &"34".repeat(32), &time)
            .unwrap();
    }
    let inv: String =
        s.0.query_row("SELECT packet FROM sent WHERE op='inventory'", [], |r| {
            r.get(0)
        })
        .unwrap();
    let m = paired_message::open(
        &owner,
        &scope,
        &serde_json::from_str(&inv).unwrap(),
        time.wall,
    )
    .unwrap();
    assert!(matches!(m.body, Body::Inventory { sequence: 1, .. }));
    for _ in 0..2 {
        s.revoke(&o.generation, &time).unwrap();
    }
    assert!(s
        .receive(&o.generation, &query, &cat, &"34".repeat(32), &time)
        .is_err());
    assert!(s.claim(&o.generation, &packet, &time).is_err());
    let count: u32 =
        s.0.query_row("SELECT COUNT(*) FROM sent WHERE op='unpaired'", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(count, 1);
    assert!(s.offer(o.community, &time).is_ok());
}
#[test]
fn wrong_proof_replay_spends_one_attempt_reboot_invalidates_offer() {
    let (_dir, mut s, mut time) = fresh();
    let (o, correct) = offer(&mut s, &time);
    let owner = Keys::generate();
    let wrong = paired_host::Proof::from_hex(&random().unwrap()).unwrap();
    for _ in 0..5 {
        let c = claim(&o, &wrong, &owner, time.wall);
        for _ in 0..5 {
            assert!(s.claim(&o.generation, &c, &time).is_err());
        }
    }
    let n: u8 =
        s.0.query_row("SELECT COUNT(*) FROM attempts", [], |r| r.get(0))
            .unwrap();
    assert_eq!(n, 5);
    assert!(s
        .claim(
            &o.generation,
            &claim(&o, &correct, &owner, time.wall),
            &time
        )
        .is_err());
    let (o, p) = offer(&mut s, &time);
    time.boot = "boot-B".into();
    assert!(s
        .claim(&o.generation, &claim(&o, &p, &owner, time.wall), &time)
        .is_err());
}
#[path = "request_tests.rs"]
mod request_tests;

#[tokio::test]
async fn old_resync_cycle_cannot_starve_current_and_clock_rollback_retains_cursor() {
    use serde_json::json;
    let (_dir, mut s, _) = fresh();
    let time = Clock::read().unwrap();
    let owner = Keys::generate();
    let (old, proof) = offer(&mut s, &time);
    s.claim(
        &old.generation,
        &claim(&old, &proof, &owner, time.wall),
        &time,
    )
    .unwrap();
    s.revoke(&old.generation, &time).unwrap();
    let inv = s.offer("https://new.example".into(), &time).unwrap();
    let value: serde_json::Value = serde_json::from_str(&inv).unwrap();
    let new = Offer {
        community: "https://new.example".into(),
        generation: value["generation"].as_str().unwrap().into(),
        host: old.host,
        expires_at: value["expires_at"].as_u64().unwrap(),
    };
    let proof = paired_host::Proof::from_hex(value["proof"].as_str().unwrap()).unwrap();
    s.claim(
        &new.generation,
        &claim(&new, &proof, &owner, time.wall),
        &time,
    )
    .unwrap();
    let ack: String =
        s.0.query_row(
            "SELECT ack_id FROM grants WHERE g=?",
            [&new.generation],
            |r| r.get(0),
        )
        .unwrap();
    let scope = Scope {
        community: new.community.clone(),
        generation: new.generation.clone(),
        owner: owner.public_key(),
        host: new.host,
    };
    let query = paired_message::seal(
        &owner,
        &scope,
        Body::InventoryQuery {
            pair_ack_id: ack,
            nonce: random().unwrap(),
        },
        time.wall,
        time.wall + 60,
    )
    .unwrap();
    let old_g = old.generation.clone();
    let host = old.host.to_hex();
    let owner = owner.public_key().to_hex();
    let net = crate::transport::Http::fixture(Box::new(move |url, body| {
        if body.is_some() {
            return Ok(json!({}));
        }
        if url.starts_with("https://new.example/") {
            return Ok(json!({"messages":[query],"after":"current"}));
        }
        if url.contains("/messages") {
            return Err(crate::transport::HttpStatus(410).into());
        }
        if url.contains("/requests") {
            let next = if url.ends_with("after=a") { "b" } else { "a" };
            return Ok(json!({"requests":[],"after":next}));
        }
        Ok(
            json!({"generation":old_g,"owner":owner,"host":host,"state":"dead","high_water":"snapshot","unpair":null,"tombstone":null}),
        )
    }));
    let cat = Catalog {
        local: None,
        source: None,
        community: Some(new.community),
        label: "Twin".into(),
        capacity: 1,
        relay_key: new.host.to_hex(),
        runtimes: [("local/goose".into(), test_runtime())].into(),
    };
    let rev = "aa".repeat(32);
    s.0.execute("UPDATE identity SET last_wall=?", [time.wall + 1000])
        .unwrap();
    assert!(crate::transport::poll(&mut s, &net, &cat, &rev)
        .await
        .is_err());
    let current = |s: &Store| {
        s.0.query_row(
            "SELECT cursor FROM grants WHERE g=?",
            [&new.generation],
            |r| r.get::<_, Option<String>>(0),
        )
        .unwrap()
    };
    assert!(current(&s).is_none());
    s.0.execute("UPDATE identity SET last_wall=?", [time.wall])
        .unwrap();
    for _ in 0..4 {
        let _ = crate::transport::poll(&mut s, &net, &cat, &rev).await;
    }
    assert_eq!(current(&s).as_deref(), Some("current"));
    assert_eq!(
        s.0.query_row("SELECT COUNT(*) FROM sent WHERE op='inventory'", [], |r| {
            r.get::<_, u32>(0)
        })
        .unwrap(),
        1
    );
    assert!(s
        .0
        .query_row(
            "SELECT cursor FROM grants WHERE g=?",
            [&old.generation],
            |r| r.get::<_, Option<String>>(0)
        )
        .unwrap()
        .is_none());
}
