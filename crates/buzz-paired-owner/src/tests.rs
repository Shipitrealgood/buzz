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
    let a = Keys::generate();
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
        &a,
    )
    .unwrap();
    let offer = seal(
        &f.h,
        &f.s,
        Body::KeyOffer {
            request_id: r.clone(),
            intent_id: i.clone(),
            confirm_id: confirm,
            nonce,
            agent: agent.clone(),
            agent_sig: pop,
        },
        t,
        t + 60,
    )
    .unwrap();
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
