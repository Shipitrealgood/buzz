use super::*;
use buzz_core::{
    paired_host::{build_ack, verify_claim, Offer},
    paired_message::{open, seal, sign_possession, Body, Possession, Route},
};
use nostr::PublicKey;
const C: &str = "https://owner.example";
struct Setup {
    dir: tempfile::TempDir,
    d: Device,
    o: Keys,
    h: Keys,
    s: Scope,
    b: Binding,
}
impl Setup {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let mut d = Device::at(&dir.path().join("state")).unwrap();
        let o = Keys::generate();
        let h = Keys::generate();
        let t = now().unwrap();
        let (offer, proof) = Offer::issue(C.into(), h.public_key(), t).unwrap();
        let json=serde_json::json!({"v":1,"community":C,"generation":offer.generation,"host":offer.host.to_hex(),"expires_at":offer.expires_at,"proof":*proof.expose_hex()}).to_string();
        assert!(d.pair("https://different.example", &o, &json).is_err());
        let (s, claim) = d.pair(C, &o, &json).unwrap();
        assert_eq!(
            id(&claim).unwrap(),
            id(&d.pair(C, &o, &json).unwrap().1).unwrap()
        );
        let c = verify_claim(
            claim.event.get().as_bytes(),
            &h,
            &offer,
            Some(o.public_key()),
            &proof.digest(),
            t,
        )
        .unwrap();
        let ack = Packet {
            event: serde_json::value::to_raw_value(&build_ack(&h, &c, t).unwrap()).unwrap(),
            route: Route {
                r: None,
                prev: Some(id(&claim).unwrap()),
                expires_at: offer.expires_at,
            },
        };
        let b = Binding {
            generation: s.generation.clone(),
            host: s.host.to_hex(),
            owner: s.owner.to_hex(),
            state: "active".into(),
            claim,
            ack,
            tombstone: None,
            inventory: None,
            high_water: None,
        };
        d.restore(C, &o, &b).unwrap();
        Self { dir, d, o, h, s, b }
    }
    fn sign(&self, body: Body, ttl: u64) -> Packet {
        let t = now().unwrap();
        seal(&self.h, &self.s, body, t, t + ttl).unwrap()
    }
    fn inventory(&mut self) -> Packet {
        let q = self.d.inventory_query(C, &self.o, &self.s).unwrap();
        let Body::InventoryQuery { nonce, .. } =
            open(&self.h, &self.s, &q, now().unwrap()).unwrap().body
        else {
            panic!()
        };
        let p = self.sign(
            Body::Inventory {
                query_id: id(&q).unwrap(),
                nonce,
                sequence: 1,
                label: "Twin".into(),
                runtimes: vec!["test/echo".into()],
                capacity: 2,
                catalog_revision: "ab".repeat(32),
            },
            60,
        );
        self.d.accept_inventory(C, &self.o, &self.s, &p).unwrap();
        p
    }
    fn draft(&self) -> Draft {
        Draft {
            runtime: "test/echo".into(),
            name: "Remote".into(),
            policy: Policy {
                respond_to: "owner-only".into(),
                respond_to_allowlist: vec![],
                channel_add_policy: "owner_only".into(),
            },
            channels: vec!["2c84a2bb-c004-4431-93bb-d7e1af763d5c".into()],
        }
    }
    fn outgoing(&self, r: &str) -> Packet {
        self.d.outgoing(C, &self.o, &self.s, r).unwrap().unwrap()
    }
}
#[test]
fn enroll_inventory_revocation_and_restart() {
    let mut f = Setup::new();
    assert!(f
        .d
        .create(C, &f.o, &f.s, &"01".repeat(32), f.draft())
        .is_err());
    let inv = f.inventory();
    assert!(f.d.view(C, &f.o, &f.s).unwrap().fresh);
    assert!(f.d.accept_inventory(C, &f.o, &f.s, &inv).is_err());
    let mut fresh = Device::at(&f.dir.path().join("second-device")).unwrap();
    assert!(fresh.restore("https://other.example", &f.o, &f.b).is_err());
    assert!(fresh.restore(C, &Keys::generate(), &f.b).is_err());
    fresh.restore(C, &f.o, &f.b).unwrap();
    assert!(!fresh.view(C, &f.o, &f.s).unwrap().fresh);
    f.d = Device::at(&f.dir.path().join("state")).unwrap();
    assert!(!f.d.view(C, &f.o, &f.s).unwrap().fresh);
    let p = f.d.unpair(C, &f.o, &f.s).unwrap();
    assert_eq!(
        id(&p).unwrap(),
        id(&f.d.unpair(C, &f.o, &f.s).unwrap()).unwrap()
    );
    assert_eq!(f.d.view(C, &f.o, &f.s).unwrap().state, "unpair-pending");
    f.b.tombstone = Some(f.sign(
        Body::Unpaired {
            pair_ack_id: id(&f.b.ack).unwrap(),
            tombstone_nonce: "cc".repeat(32),
        },
        86400,
    ));
    f.d.restore(C, &f.o, &f.b).unwrap();
    f.b.tombstone = None;
    f.d.restore(C, &f.o, &f.b).unwrap();
    assert_eq!(f.d.view(C, &f.o, &f.s).unwrap().state, "unpaired");
    assert!(f.d.inventory_query(C, &f.o, &f.s).is_err());
}
#[test]
fn only_initiating_tap_can_automate_credentials_after_exact_commit() {
    let mut f = Setup::new();
    let inv = f.inventory();
    let draft = f.draft();
    let (r, i) =
        f.d.create(C, &f.o, &f.s, &id(&inv).unwrap(), draft)
            .unwrap();
    let intent_packet = i;
    let i = id(&intent_packet).unwrap();
    let challenge = f.sign(
        Body::Challenge {
            request_id: r.clone(),
            intent_id: i.clone(),
            nonce: "ab".repeat(32),
            phase: "generate".into(),
        },
        60,
    );
    f.d.receive(C, &f.o, &f.s, &r, &challenge, None).unwrap();
    let confirm_packet = f.outgoing(&r);
    let confirm = id(&confirm_packet).unwrap();
    f.d.receive(C, &f.o, &f.s, &r, &challenge, None).unwrap();
    assert_eq!(id(&f.outgoing(&r)).unwrap(), confirm);
    let candidate_offer = |a: &Keys| {
        let agent = a.public_key().to_hex();
        let t = now().unwrap();
        let nonce = "e1".repeat(32);
        let pop = sign_possession(
            &f.s,
            &Possession {
                request_id: &r,
                intent_id: &i,
                confirm_id: &confirm,
                nonce: &nonce,
                agent: a.public_key(),
                expires_at: t + 60,
            },
            a,
        )
        .unwrap();
        let offer = seal(
            &f.h,
            &f.s,
            Body::KeyOffer {
                request_id: r.clone(),
                intent_id: i.clone(),
                confirm_id: confirm.clone(),
                nonce,
                agent: agent.clone(),
                agent_sig: pop,
            },
            t,
            t + 60,
        )
        .unwrap();
        (agent, offer)
    };
    let hidden_host = Keys::generate();
    let (other, p) = Offer::issue(C.into(), hidden_host.public_key(), now().unwrap()).unwrap();
    let (_, foreign_offer) = candidate_offer(&hidden_host);
    f.d.pair(C, &f.o, &invitation(&other, &p.expose_hex()))
        .unwrap();
    assert!(f
        .d
        .receive(
            C,
            &f.o,
            &f.s,
            &r,
            &foreign_offer,
            Some(&hidden_host.public_key())
        )
        .is_err());
    let desktop_agent = Keys::generate();
    let (_, desktop_offer) = candidate_offer(&desktop_agent);
    let pk = desktop_agent.public_key();
    f.d = f.d.with_local_identities(move || Ok(vec![pk]));
    assert!(f
        .d
        .receive(C, &f.o, &f.s, &r, &desktop_offer, Some(&pk))
        .is_err());
    let a = Keys::generate();
    let (agent, offer) = candidate_offer(&a);
    assert!(f.d.receive(C, &f.o, &f.s, &r, &offer, None).is_err());
    f.d.receive(C, &f.o, &f.s, &r, &offer, Some(&a.public_key()))
        .unwrap();
    let activate = f.outgoing(&r);
    let make_commit = |act: String| {
        f.sign(
            Body::Committed {
                request_id: r.clone(),
                intent_id: i.clone(),
                activate_id: act,
                offer_id: id(&offer).unwrap(),
                agent: agent.clone(),
            },
            86400,
        )
    };
    let bad = make_commit("ab".repeat(32));
    let good = make_commit(id(&activate).unwrap());
    let copy = f.dir.path().join("newly-discovered");
    std::fs::copy(f.dir.path().join("state"), &copy).unwrap();
    let pk = a.public_key();
    let mut interrupted = Device::at(&copy)
        .unwrap()
        .with_local_identities(move || Ok(vec![pk]));
    assert!(interrupted.receive(C, &f.o, &f.s, &r, &good, None).is_err());
    assert!(interrupted
        .public_next(C, &f.o, &f.s, &r)
        .unwrap()
        .is_none());

    assert!(f.d.receive(C, &f.o, &f.s, &r, &bad, None).is_err());
    assert!(f.d.public_next(C, &f.o, &f.s, &r).unwrap().is_none());
    assert!(f
        .d
        .receive("https://other.example", &f.o, &f.s, &r, &good, None)
        .is_err());
    assert!(f
        .d
        .receive(C, &Keys::generate(), &f.s, &r, &good, None)
        .is_err());
    let mut second = Device::at(&f.dir.path().join("elsewhere")).unwrap();
    second.restore(C, &f.o, &f.b).unwrap();
    assert!(second.receive(C, &f.o, &f.s, &r, &good, None).is_err());
    f.d = Device::at(&f.dir.path().join("state")).unwrap();
    f.d.unpair(C, &f.o, &f.s).unwrap();
    assert!(f.d.receive(C, &f.o, &f.s, &r, &challenge, None).is_err());
    f.d.receive(C, &f.o, &f.s, &r, &good, None).unwrap();
    let credential = f.outgoing(&r);
    f.d.receive(C, &f.o, &f.s, &r, &good, None).unwrap();
    assert_eq!(credential.event.get(), f.outgoing(&r).event.get());
    let Body::Credentials {
        commit_id,
        registration,
        oa,
        ..
    } = open(&f.h, &f.s, &credential, now().unwrap()).unwrap().body
    else {
        panic!()
    };
    assert_eq!(commit_id, id(&good).unwrap());
    assert_eq!(registration.pubkey, f.o.public_key());
    assert_eq!(oa[1], f.o.public_key().to_hex());
    assert!(oa[2].is_empty());
    let auth = serde_json::to_string(&oa).unwrap();
    assert!(
        buzz_sdk::nip_oa::verify_auth_tag(&auth, &PublicKey::from_hex(&agent).unwrap()).is_ok()
    );
    assert_eq!(
        f.d.public_next(C, &f.o, &f.s, &r).unwrap().unwrap(),
        registration
    );
    f.d.public_confirm(C, &f.o, &f.s, &r, &registration)
        .unwrap();
    let add = f.d.public_next(C, &f.o, &f.s, &r).unwrap().unwrap();
    assert_eq!(add.kind.as_u16(), 9000);
    let incomplete = Body::Result {
        request_id: r.clone(),
        intent_id: i.clone(),
        agent: Some(agent.clone()),
        commit_id: Some(commit_id.clone()),
        state: "ready".into(),
        steps: vec!["verified".into()],
    };
    let p = f.sign(incomplete, 86400);
    assert!(f.d.receive(C, &f.o, &f.s, &r, &p, None).is_err());
    let mut steps: Vec<String> = [
        "profile-verified",
        "directory-verified",
        "registration-verified",
        "unit-active",
        "presence-confirmed",
        "verified",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    steps.push(format!("membership-verified:{}", f.draft().channels[0]));
    let p = f.sign(
        Body::Result {
            request_id: r.clone(),
            intent_id: i,
            agent: Some(agent),
            commit_id: Some(commit_id),
            state: "ready".into(),
            steps,
        },
        86400,
    );
    f.d.receive(C, &f.o, &f.s, &r, &p, None).unwrap();
    assert_eq!(
        f.d.request(C, &f.o, &f.s, &r).unwrap().state,
        "pending-publication"
    );
    f.d.public_confirm(C, &f.o, &f.s, &r, &add).unwrap();
    assert_eq!(f.d.request(C, &f.o, &f.s, &r).unwrap().state, "verifying");
    f.d.record_verified_ready(C, &f.o, &f.s, &r).unwrap();
    assert_eq!(f.d.request(C, &f.o, &f.s, &r).unwrap().state, "ready");
    let summary=serde_json::from_value(serde_json::json!({"request_id":r,"intent_id":id(&intent_packet).unwrap(),"commit_id":id(&good).unwrap(),"terminal_id":id(&p).unwrap()})).unwrap();
    let mut historical = vec![
        p,
        good,
        offer,
        challenge,
        confirm_packet,
        intent_packet,
        activate,
    ];
    assert_eq!(
        crate::history::inspect(&f.o, &f.s, &summary, &historical, false)
            .unwrap()
            .host_state,
        "ready"
    );
    let mut misleading = historical.clone();
    let Body::Result {
        request_id,
        intent_id,
        commit_id,
        agent,
        ..
    } = open(&f.o, &f.s, &misleading[0], now().unwrap())
        .unwrap()
        .body
    else {
        panic!()
    };
    misleading[0] = f.sign(
        Body::Result {
            request_id,
            intent_id,
            commit_id,
            agent,
            state: "ready".into(),
            steps: vec!["unit-failed".into()],
        },
        86400,
    );
    let misleading_summary=serde_json::from_value(serde_json::json!({"request_id":r,"intent_id":id(&historical[5]).unwrap(),"commit_id":id(&historical[1]).unwrap(),"terminal_id":id(&misleading[0]).unwrap()})).unwrap();
    assert!(crate::history::inspect(&f.o, &f.s, &misleading_summary, &misleading, false).is_err());
    historical.pop();
    assert!(crate::history::inspect(&f.o, &f.s, &summary, &historical, false).is_err());
    assert!(second.requests(C, &f.o, &f.s).unwrap().is_empty());
}

#[test]
fn status_replies_never_create_signing_authority() {
    let mut f = Setup::new();
    let inv = f.inventory();
    let (r, i) =
        f.d.create(C, &f.o, &f.s, &id(&inv).unwrap(), f.draft())
            .unwrap();
    let query =
        f.d.status_query(C, &f.o, &f.s, &r, "scoped-cursor")
            .unwrap()
            .unwrap();
    assert!(f
        .d
        .status_query(C, &f.o, &f.s, &r, "different")
        .unwrap()
        .is_none());
    let Body::StatusQuery {
        intent_id, nonce, ..
    } = open(&f.h, &f.s, &query, now().unwrap()).unwrap().body
    else {
        panic!()
    };
    let make = |nonce: String| {
        f.sign(
            Body::Status {
                request_id: r.clone(),
                intent_id: intent_id.clone(),
                query_id: id(&query).unwrap(),
                nonce,
                state: "ready".into(),
                commit_id: Some("14".repeat(32)),
                terminal_id: Some("13".repeat(32)),
            },
            60,
        )
    };
    let bad = make("dd".repeat(32));
    let status = make(nonce);
    assert!(f.d.status_response(C, &f.o, &f.s, &r, &bad).is_err());
    f.d.status_response(C, &f.o, &f.s, &r, &status).unwrap();
    assert_eq!(f.d.request(C, &f.o, &f.s, &r).unwrap().state, "pending");
    assert!(f.d.public_next(C, &f.o, &f.s, &r).unwrap().is_none());
    assert_eq!(id(&f.outgoing(&r)).unwrap(), id(&i).unwrap());
    assert_eq!(f.d.requests(C, &f.o, &f.s).unwrap().len(), 1);
    assert!(f
        .d
        .hosts("https://different.example", &f.o)
        .unwrap()
        .is_empty());
}

fn invitation(o: &Offer, proof: &str) -> String {
    serde_json::json!({"v":1,"community":o.community,"generation":o.generation,"host":o.host.to_hex(),"expires_at":o.expires_at,"proof":proof}).to_string()
}

#[test]
fn corrected_invitation_can_enroll_same_generation_and_exact_retry_survives_restart() {
    let mut f = Setup::new();
    let host = Keys::generate();
    let wall = now().unwrap();
    let (o, p) = Offer::issue(C.into(), host.public_key(), wall).unwrap();
    let wrong = invitation(&o, &"01".repeat(32));
    let right = invitation(&o, &p.expose_hex());
    let (_, bad) = f.d.pair(C, &f.o, &wrong).unwrap();
    assert!(verify_claim(
        bad.event.get().as_bytes(),
        &host,
        &o,
        None,
        &p.digest(),
        wall
    )
    .is_err());
    f.d = Device::at(&f.dir.path().join("state")).unwrap();
    assert_eq!(
        id(&bad).unwrap(),
        id(&f.d.pair(C, &f.o, &wrong).unwrap().1).unwrap()
    );
    let (s, good) = f.d.pair(C, &f.o, &right).unwrap();
    assert_ne!(id(&good).unwrap(), id(&bad).unwrap());
    let verified = verify_claim(
        good.event.get().as_bytes(),
        &host,
        &o,
        None,
        &p.digest(),
        wall,
    )
    .unwrap();
    let ack = Packet {
        event: serde_json::value::to_raw_value(&build_ack(&host, &verified, wall).unwrap())
            .unwrap(),
        route: Route {
            r: None,
            prev: Some(id(&good).unwrap()),
            expires_at: o.expires_at,
        },
    };
    // A delayed acknowledgement of a previously submitted correct claim is still ours.
    f.d.pair(C, &f.o, &wrong).unwrap();
    f.d.restore(
        C,
        &f.o,
        &Binding {
            generation: s.generation.clone(),
            owner: s.owner.to_hex(),
            host: s.host.to_hex(),
            state: "active".into(),
            claim: good,
            ack,
            tombstone: None,
            inventory: None,
            high_water: None,
        },
    )
    .unwrap();
    assert_eq!(f.d.view(C, &f.o, &s).unwrap().state, "paired");
}

#[test]
fn retry_rotation_delivers_ninth_pending_revocation_across_restart() {
    let mut f = Setup::new();
    let mut all = std::collections::BTreeSet::new();
    for _ in 0..9 {
        let h = Keys::generate();
        let wall = now().unwrap();
        let (o, p) = Offer::issue(C.into(), h.public_key(), wall).unwrap();
        let (s, claim) = f.d.pair(C, &f.o, &invitation(&o, &p.expose_hex())).unwrap();
        let c = verify_claim(
            claim.event.get().as_bytes(),
            &h,
            &o,
            None,
            &p.digest(),
            wall,
        )
        .unwrap();
        let ack = Packet {
            event: serde_json::value::to_raw_value(&build_ack(&h, &c, wall).unwrap()).unwrap(),
            route: Route {
                r: None,
                prev: Some(id(&claim).unwrap()),
                expires_at: o.expires_at,
            },
        };
        f.d.restore(
            C,
            &f.o,
            &Binding {
                generation: s.generation.clone(),
                owner: s.owner.to_hex(),
                host: s.host.to_hex(),
                state: "active".into(),
                claim,
                ack,
                tombstone: None,
                inventory: None,
                high_water: None,
            },
        )
        .unwrap();
        f.d.unpair(C, &f.o, &s).unwrap();
        all.insert(s.generation);
    }
    f.b.state = "fenced".into();
    f.d.restore(C, &f.o, &f.b).unwrap();
    let first = f.d.next_deliveries(C, &f.o).unwrap();
    assert_eq!(first.len(), 8);
    f.d = Device::at(&f.dir.path().join("state")).unwrap();
    let next = f.d.next_deliveries(C, &f.o).unwrap();
    let seen = first
        .iter()
        .chain(&next)
        .map(|(s, _, claim)| {
            assert!(!claim);
            s.generation.clone()
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(seen, all);
}

#[test]
fn short_signed_inventory_cannot_grant_a_minute_of_display_freshness() {
    let fraction = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .subsec_millis();
    if fraction > 400 {
        std::thread::sleep(std::time::Duration::from_millis((1001 - fraction).into()));
    }
    let mut f = Setup::new();
    let q = f.d.inventory_query(C, &f.o, &f.s).unwrap();
    let Body::InventoryQuery { nonce, .. } = open(&f.h, &f.s, &q, now().unwrap()).unwrap().body
    else {
        panic!()
    };
    let p = f.sign(
        Body::Inventory {
            nonce,
            query_id: id(&q).unwrap(),
            sequence: 1,
            label: "Short".into(),
            runtimes: vec!["test/echo".into()],
            capacity: 1,
            catalog_revision: "11".repeat(32),
        },
        1,
    );
    f.d.accept_inventory(C, &f.o, &f.s, &p).unwrap();
    let v = f.d.view(C, &f.o, &f.s).unwrap();
    assert_eq!(v.inventory_id.unwrap(), id(&p).unwrap());
    assert!(v.fresh_for_ms <= 1000);
}

#[test]
fn repeated_tap_is_idempotent_after_ipc_loss_and_process_restart() {
    let mut f = Setup::new();
    let inventory = id(&f.inventory()).unwrap();
    let r = random().unwrap();
    let initial =
        f.d.create_once(C, &f.o, &f.s, &inventory, &r, f.draft())
            .unwrap()
            .1;
    let draft = f.draft();
    f.d = Device::at(&f.dir.path().join("state")).unwrap();
    let replay =
        f.d.create_once(C, &f.o, &f.s, &inventory, &r, draft.clone())
            .unwrap()
            .1;
    assert_eq!(initial.event.get(), replay.event.get());
    let mut changed = draft.clone();
    changed.name = "Different".into();
    assert!(f
        .d
        .create_once(C, &f.o, &f.s, &inventory, &r, changed)
        .is_err());
    assert!(f
        .d
        .create_once(C, &f.o, &f.s, &"00".repeat(32), &r, draft.clone())
        .is_err());
    assert!(f
        .d
        .create_once(C, &Keys::generate(), &f.s, &inventory, &r, draft)
        .is_err());
    assert_eq!(f.d.requests(C, &f.o, &f.s).unwrap().len(), 1);
}
