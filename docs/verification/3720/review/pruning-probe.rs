// Independent #3720 P11 probe against unmodified production Db APIs.
// Run only via scripts/3720-registration-proof.sh run with a fresh desired-state DB.
use buzz_core::{host_registry as wire, paired_message::Scope};
use buzz_db::{Db, DbConfig, paired_hosts::RegistryRead};
use nostr::{Event, Keys, JsonUtil};
fn main() { tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(probe()); }
async fn post(db: &Db, c: buzz_core::CommunityId, scope: &Scope, e: &Event) {
    db.post_registry_event(c, &scope.community, e.pubkey, e.as_json().as_bytes(), false, &[0;32]).await.unwrap();
}
async fn probe() {
    assert_eq!(std::env::var("BUZZ_3720_ISOLATED_FIXTURE").as_deref(), Ok("1"));
    let url=std::env::var("BUZZ_TEST_DATABASE_URL").unwrap();
    assert!(url.starts_with("postgres://buzz3720:synthetic3720@127.0.0.1:5432/buzz_3720_fixture_"));
    let db=Db::new(&DbConfig{database_url:url,max_connections:2,min_connections:0,..Default::default()}).await.unwrap();
    let c=db.ensure_configured_community("pruning-review.test").await.unwrap().id;
    let owner=Keys::generate(); let host=Keys::generate(); let relay=Keys::generate();
    let _ = db.add_relay_member(c,&owner.public_key().to_hex(),"member",None,&relay).await.unwrap();
    let s=Scope{community:"https://pruning-review.test".into(),generation:"37".repeat(32),owner:owner.public_key(),host:host.public_key()};
    let now=nostr::Timestamp::now().as_secs();
    let claim=wire::seal(&owner,&s,wire::Body::PairClaim{proof:"12".repeat(32)},"01".repeat(32),None,now,now+600).unwrap();
    post(&db,c,&s,&claim).await;
    let ack=wire::seal(&host,&s,wire::Body::PairAck{claim_id:claim.id.to_hex()},"02".repeat(32),Some(claim.id.to_hex()),now,now+600).unwrap();
    post(&db,c,&s,&ack).await;
    let g=wire::bytes(&s.generation).unwrap();
    let q=wire::Query{v:2,view:"messages".into(),nonce:"ab".repeat(32),cursor:None,generation:Some(s.generation.clone()),event_id:None};
    let page=db.read_registry(c,host.public_key().as_bytes(),false,RegistryRead{query:&q,after:None,snapshot:None,host_generation:Some(&g)}).await.unwrap();
    println!("BASELINE fresh messages: PASS, {} rows",page.data["events"].as_array().unwrap().len());
    let query=|op:&str| wire::seal(&owner,&s,wire::Body::StatusQuery{ack_id:ack.id.to_hex(),nonce:"cd".repeat(32)},op.repeat(32),Some(ack.id.to_hex()),now,now+60).unwrap();
    let first=query("03"); post(&db,c,&s,&first).await;
    // Deterministic retention setup only: age this synthetic row's received_at;
    // next real signed status query invokes the unchanged production pruning SQL.
    sqlx::query("UPDATE paired_host_events SET received_at=now()-interval '6 minutes' WHERE community_id=$1 AND event_id=$2").bind(c.as_uuid()).bind(first.id.as_bytes().as_slice()).execute(db.pool()).await.unwrap();
    post(&db,c,&s,&query("04")).await;
    let floor:i64=sqlx::query_scalar("SELECT retention_floor FROM paired_host_bindings WHERE community_id=$1 AND generation=$2").bind(c.as_uuid()).bind(g.as_slice()).fetch_one(db.pool()).await.unwrap();
    assert!(floor>0); println!("Production pruning advanced retention_floor={floor}");
    let result=db.read_registry(c,host.public_key().as_bytes(),false,RegistryRead{query:&q,after:None,snapshot:None,host_generation:Some(&g)}).await;
    match result { Err(buzz_db::DbError::PairedCursorPruned)=>println!("DEFECT CONFIRMED: null/fresh messages cursor returns PairedCursorPruned"), _=>panic!("expected reproduced fresh-cursor failure") }
    // Repeating from null is the native Host::reconcile reset behavior.
    let again=db.read_registry(c,host.public_key().as_bytes(),false,RegistryRead{query:&q,after:None,snapshot:None,host_generation:Some(&g)}).await;
    assert!(matches!(again,Err(buzz_db::DbError::PairedCursorPruned)));
    println!("Native reset shape repeats the permanent failure; no permitted starting cursor exists");
    db.pool().close().await;
}
