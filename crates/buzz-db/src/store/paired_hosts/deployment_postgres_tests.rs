use super::*;
use buzz_core::{
    host_deployment::{wire, Authority},
    host_registry as registry,
    paired_message::Scope,
};
use nostr::{Event, Keys};
use sha2::{Digest, Sha256};

async fn enroll(
    db: &Db,
    c: CommunityId,
    owner: &Keys,
    host: &Keys,
    origin: &str,
) -> (Scope, Event) {
    let _ = db
        .add_relay_member(
            c,
            &owner.public_key().to_hex(),
            "member",
            None,
            &Keys::generate(),
        )
        .await
        .unwrap();
    let scope = Scope {
        community: origin.into(),
        generation: uuid::Uuid::new_v4().simple().to_string().repeat(2),
        owner: owner.public_key(),
        host: host.public_key(),
    };
    let n = nostr::Timestamp::now().as_secs();
    let claim = registry::seal(
        owner,
        &scope,
        registry::Body::PairClaim {
            proof: "ab".repeat(32),
        },
        "cc".repeat(32),
        None,
        n,
        n + 500,
    )
    .unwrap();
    db.post_registry_event(
        c,
        origin,
        owner.public_key(),
        &serde_json::to_vec(&claim).unwrap(),
        false,
        &[0; 32],
    )
    .await
    .unwrap();
    let ack = registry::seal(
        host,
        &scope,
        registry::Body::PairAck {
            claim_id: claim.id.to_hex(),
        },
        "dd".repeat(32),
        Some(claim.id.to_hex()),
        n,
        n + 500,
    )
    .unwrap();
    db.post_registry_event(
        c,
        origin,
        host.public_key(),
        &serde_json::to_vec(&ack).unwrap(),
        false,
        &[0; 32],
    )
    .await
    .unwrap();
    (scope, ack)
}
fn command(
    owner: &Keys,
    scope: &Scope,
    agent: PublicKey,
    op: &str,
    rev: u64,
    act: Action,
) -> (Authority, Event) {
    let n = nostr::Timestamp::now().as_secs();
    // The DB intentionally sees only a real NIP44 ciphertext; host validates plaintext.
    let content = nostr::nips::nip44::encrypt(
        owner.secret_key(),
        &scope.host,
        b"{\"fixture\":true}",
        nostr::nips::nip44::Version::V2,
    )
    .unwrap();
    let a = Authority {
        v: 1,
        scope: scope.clone(),
        agent,
        action: act,
        operation_id: op.into(),
        expected_revision: rev,
        expires_at: n + 500,
        content_sha256: hex::encode(Sha256::digest(content.as_bytes())),
    };
    let e = wire::sign_command(owner, &a, content.as_bytes(), n).unwrap();
    (a, e)
}
#[tokio::test]
#[ignore = "requires Postgres"]
async fn deployment_atomic_replay_placement_permissions_cleanup_and_revocation() {
    let db = Db::from_pool(
        sqlx::PgPool::connect(&crate::test_support::database_url())
            .await
            .unwrap(),
    );
    let name = format!("deployment-{}.test", uuid::Uuid::new_v4());
    let c = db.ensure_configured_community(&name).await.unwrap().id;
    let origin = format!("https://{name}");
    let owner = Keys::generate();
    let host = Keys::generate();
    let (scope, ack) = enroll(&db, c, &owner, &host, &origin).await;
    let agent = Keys::generate().public_key();
    let (a, e) = command(&owner, &scope, agent, &"11".repeat(32), 0, Action::Deploy);
    let raw = serde_json::to_vec(&e).unwrap();
    let mut expired = a.clone();
    expired.expires_at = nostr::Timestamp::now().as_secs() - 1;
    let expired = wire::sign_command(
        &owner,
        &expired,
        e.content.as_bytes(),
        expired.expires_at - 10,
    )
    .unwrap();
    assert!(db
        .post_host_deployment(
            c,
            &origin,
            owner.public_key(),
            &serde_json::to_vec(&expired).unwrap(),
            false
        )
        .await
        .is_err());
    assert!(db
        .post_host_deployment(c, &origin, host.public_key(), &raw, false)
        .await
        .is_err());
    let mut tamper: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    tamper["sig"] = "00".repeat(64).into();
    assert!(db
        .post_host_deployment(
            c,
            &origin,
            owner.public_key(),
            &serde_json::to_vec(&tamper).unwrap(),
            false
        )
        .await
        .is_err());
    db.post_host_deployment(c, &origin, owner.public_key(), &raw, false)
        .await
        .unwrap();
    db.post_host_deployment(c, &origin, owner.public_key(), &raw, false)
        .await
        .unwrap();
    let (_, conflict) = command(&owner, &scope, agent, &a.operation_id, 0, Action::Deploy);
    assert!(db
        .post_host_deployment(
            c,
            &origin,
            owner.public_key(),
            &serde_json::to_vec(&conflict).unwrap(),
            false
        )
        .await
        .is_err());
    assert!(db
        .post_host_deployment(c, "https://foreign.test", owner.public_key(), &raw, false)
        .await
        .is_err());
    let (_, stale) = command(&owner, &scope, agent, &"12".repeat(32), 0, Action::Deploy);
    assert!(db
        .post_host_deployment(
            c,
            &origin,
            owner.public_key(),
            &serde_json::to_vec(&stale).unwrap(),
            false
        )
        .await
        .is_err());
    let other_host = Keys::generate();
    let (other, _) = enroll(&db, c, &owner, &other_host, &origin).await;
    let (_, duplicate) = command(&owner, &other, agent, &"13".repeat(32), 0, Action::Deploy);
    assert!(db
        .post_host_deployment(
            c,
            &origin,
            owner.public_key(),
            &serde_json::to_vec(&duplicate).unwrap(),
            false
        )
        .await
        .is_err());
    let g = bytes(&scope.generation).unwrap();
    let mut q = wire::Query {
        v: 1,
        scope: scope.clone(),
        view: "permit".into(),
        nonce: "21".repeat(32),
        operation_id: Some(a.operation_id.clone()),
    };
    assert!(db
        .read_host_deployment(c, owner.public_key(), None, false, &q)
        .await
        .is_err());
    let admitted = db
        .read_host_deployment(c, host.public_key(), Some(&g), false, &q)
        .await
        .unwrap();
    assert_eq!(
        admitted.data["authority"],
        serde_json::to_value(&a).unwrap()
    );
    assert_eq!(admitted.data["unit_id"], a.operation_id);
    assert!(
        db.read_host_deployment(c, host.public_key(), Some(&g), false, &q)
            .await
            .is_err(),
        "challenge cannot authorize another attempt"
    );
    let _ = db
        .remove_relay_member(c, &owner.public_key().to_hex(), &Keys::generate())
        .await
        .unwrap();
    q.nonce = "24".repeat(32);
    assert!(
        db.read_host_deployment(c, host.public_key(), Some(&g), false, &q)
            .await
            .is_err(),
        "removed owner cannot authorize new attempts"
    );
    let _ = db
        .add_relay_member(
            c,
            &owner.public_key().to_hex(),
            "member",
            None,
            &Keys::generate(),
        )
        .await
        .unwrap();
    let (_, cleanup) = command(&owner, &scope, agent, &"14".repeat(32), 1, Action::Cleanup);
    db.post_host_deployment(
        c,
        &origin,
        owner.public_key(),
        &serde_json::to_vec(&cleanup).unwrap(),
        false,
    )
    .await
    .unwrap();
    q.nonce = "22".repeat(32);
    assert!(
        db.read_host_deployment(c, host.public_key(), Some(&g), false, &q)
            .await
            .is_err(),
        "queued deploy fenced by cleanup"
    );
    q.operation_id = Some("14".repeat(32));
    let cleanup_permit = db
        .read_host_deployment(c, host.public_key(), Some(&g), false, &q)
        .await
        .unwrap();
    assert_eq!(cleanup_permit.data["unit_id"], a.operation_id);
    let (_, resurrect) = command(&owner, &scope, agent, &"15".repeat(32), 2, Action::Deploy);
    assert!(db
        .post_host_deployment(
            c,
            &origin,
            owner.public_key(),
            &serde_json::to_vec(&resurrect).unwrap(),
            false
        )
        .await
        .is_err());
    let n = nostr::Timestamp::now().as_secs();
    let revoke = registry::seal(
        &owner,
        &scope,
        registry::Body::Unpair {
            ack_id: ack.id.to_hex(),
        },
        "31".repeat(32),
        Some(ack.id.to_hex()),
        n,
        n + 500,
    )
    .unwrap();
    let raw_revoke = serde_json::to_vec(&revoke).unwrap();
    q.nonce = "25".repeat(32);
    let (revoked, raced) = tokio::join!(
        db.post_registry_event(c, &origin, owner.public_key(), &raw_revoke, false, &[0; 32]),
        db.read_host_deployment(c, host.public_key(), Some(&g), false, &q)
    );
    revoked.unwrap();
    if let Ok(permit) = raced {
        assert_eq!(permit.data["unit_id"], a.operation_id);
    }

    q.nonce = "23".repeat(32);
    assert!(
        db.read_host_deployment(c, host.public_key(), Some(&g), false, &q)
            .await
            .is_err(),
        "revocation precedes subsequent permits"
    );
    q.view = "operation".into();
    assert!(db
        .read_host_deployment(c, owner.public_key(), None, false, &q)
        .await
        .is_err());
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM paired_agent_placements WHERE community_id=$1")
            .bind(c.as_uuid())
            .fetch_one(db.pool())
            .await
            .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
#[ignore = "requires Postgres"]
async fn migration_schema_deployment_82_preserves_registration_and_matches_desired() {
    let url = crate::test_support::database_url();
    let pool = sqlx::PgPool::connect(&url).await.unwrap();
    crate::migration::run_migrations_through(&pool, 81)
        .await
        .unwrap();
    let db = Db::from_pool(pool.clone());
    let name = format!("upgrade-{}.test", uuid::Uuid::new_v4());
    let c = db.ensure_configured_community(&name).await.unwrap().id;
    let origin = format!("https://{name}");
    let owner = Keys::generate();
    let host = Keys::generate();
    let (scope, ack) = enroll(&db, c, &owner, &host, &origin).await;
    // Explicit out-of-band expansion: serving registration does not run DDL.
    crate::migration::run_migrations(&pool).await.unwrap();
    let g = bytes(&scope.generation).unwrap();
    let session = db
        .registry_session(c, host.public_key().as_bytes(), &g, false)
        .await
        .unwrap();
    assert_eq!(session.owner, Some(owner.public_key().to_bytes()));
    db.post_registry_event(
        c,
        &origin,
        host.public_key(),
        &serde_json::to_vec(&ack).unwrap(),
        false,
        &[0; 32],
    )
    .await
    .unwrap();
    let (other, _) = enroll(&db, c, &owner, &Keys::generate(), &origin).await;
    assert_ne!(other.generation, scope.generation);
    let scratch = crate::test_support::ScratchDb::create(&url, "deployment_desired").await;
    let desired = scratch.connect().await;
    for ddl in [
        include_str!("../../../../../schema/tables/public/paired_agent_operations.sql"),
        include_str!("../../../../../schema/tables/public/paired_agent_placements.sql"),
        include_str!("../../../../../schema/tables/public/paired_host_deployment_status.sql"),
    ] {
        sqlx::raw_sql(ddl).execute(&desired).await.unwrap();
    }
    for table in [
        "paired_agent_operations",
        "paired_agent_placements",
        "paired_host_deployment_status",
    ] {
        async fn shape(pool: &sqlx::PgPool, table: &str) -> Vec<String> {
            sqlx::query_scalar("SELECT column_name||':'||data_type||':'||is_nullable||':'||coalesce(column_default,'') FROM information_schema.columns WHERE table_schema='public' AND table_name=$1 UNION ALL SELECT indexdef FROM pg_indexes WHERE schemaname='public' AND tablename=$1 ORDER BY 1")
                .bind(table).fetch_all(pool).await.unwrap()
        }
        assert_eq!(
            shape(&pool, table).await,
            shape(&desired, table).await,
            "{table}"
        );
    }
}

fn unchecked_command(owner: &Keys, a: &Authority, content: &str, created: u64) -> Event {
    nostr::EventBuilder::new(nostr::Kind::Custom(43104), content)
        .tags([
            nostr::Tag::parse([
                "deployment",
                &serde_json::to_string(&wire::Header::Command {
                    authority: a.clone(),
                })
                .unwrap(),
            ])
            .unwrap(),
            nostr::Tag::parse(["p", &a.scope.host.to_hex()]).unwrap(),
        ])
        .custom_created_at(nostr::Timestamp::from(created))
        .sign_with_keys(owner)
        .unwrap()
}
#[tokio::test]
#[ignore = "requires Postgres"]
async fn cleanup_reserved_capacity_and_bounded_replacements_preserve_anchor() {
    use buzz_core::host_deployment::{Command, DeploymentResult};
    let db = Db::from_pool(
        sqlx::PgPool::connect(&crate::test_support::database_url())
            .await
            .unwrap(),
    );
    for community_quota in [false, true] {
        let name = format!("cleanup-quota-{}.test", uuid::Uuid::new_v4());
        let c = db.ensure_configured_community(&name).await.unwrap().id;
        let origin = format!("https://{name}");
        let (owner, host) = (Keys::generate(), Keys::generate());
        let (scope, _) = enroll(&db, c, &owner, &host, &origin).await;
        let agent = Keys::generate().public_key();
        let (mut authority, first) =
            command(&owner, &scope, agent, &"01".repeat(32), 0, Action::Deploy);
        db.post_host_deployment(
            c,
            &origin,
            owner.public_key(),
            &serde_json::to_vec(&first).unwrap(),
            false,
        )
        .await
        .unwrap();
        let foreign_agent = Keys::generate().public_key();
        let (_, foreign) = command(
            &owner,
            &scope,
            foreign_agent,
            &"04".repeat(32),
            0,
            Action::Deploy,
        );
        db.post_host_deployment(
            c,
            &origin,
            owner.public_key(),
            &serde_json::to_vec(&foreign).unwrap(),
            false,
        )
        .await
        .unwrap();
        // Seed only quota history, not live authority. Multiple former owners fill
        // community history; the current owner remains below its own limit.
        let count = if community_quota { 4094 } else { 1022 };
        sqlx::query("INSERT INTO paired_agent_operations SELECT community_id,CASE WHEN $3 THEN decode(repeat(md5((n/1024)::text),2),'hex') ELSE owner_pubkey END,lpad(to_hex(n),64,'0'),agent_pubkey,host_pubkey,generation,action,expected_revision,expires_at,content_sha256,event_json,state,result_json,permit_nonce,permit_expires_at FROM paired_agent_operations CROSS JOIN generate_series(1,$2) n WHERE community_id=$1 AND operation_id=$4")
            .bind(c.as_uuid()).bind(count).bind(community_quota).bind(&authority.operation_id).execute(db.pool()).await.unwrap();
        let (_, blocked) = command(
            &owner,
            &scope,
            Keys::generate().public_key(),
            &"02".repeat(32),
            0,
            Action::Deploy,
        );
        assert!(db
            .post_host_deployment(
                c,
                &origin,
                owner.public_key(),
                &serde_json::to_vec(&blocked).unwrap(),
                false
            )
            .await
            .is_err());
        authority.action = Action::Cleanup;
        let mut anchor = String::new();
        let mut previous: Option<(Authority, Event)> = None;
        for revision in 1..=6 {
            authority.expected_revision = revision;
            let now = nostr::Timestamp::now().as_secs();
            let (a, e) =
                wire::seal_command(&owner, authority.clone(), &Command::Cleanup {}, now).unwrap();
            let raw = serde_json::to_string(&e).unwrap();
            db.post_host_deployment(c, &origin, owner.public_key(), raw.as_bytes(), false)
                .await
                .unwrap();
            db.post_host_deployment(c, &origin, owner.public_key(), raw.as_bytes(), false)
                .await
                .unwrap();
            assert!(
                db.post_host_deployment(
                    c,
                    &origin,
                    owner.public_key(),
                    format!(" {raw}").as_bytes(),
                    false
                )
                .await
                .is_err(),
                "retained event bytes are immutable"
            );
            let changed_time = unchecked_command(&owner, &a, &e.content, now - 1);
            assert!(db
                .post_host_deployment(
                    c,
                    &origin,
                    owner.public_key(),
                    &serde_json::to_vec(&changed_time).unwrap(),
                    false
                )
                .await
                .is_err());
            if revision == 1 {
                anchor = raw.clone();
            }
            if revision > 2 {
                let (old_a, old_e) = previous.as_ref().unwrap();
                assert!(
                    db.post_host_deployment(
                        c,
                        &origin,
                        owner.public_key(),
                        &serde_json::to_vec(old_e).unwrap(),
                        false
                    )
                    .await
                    .is_err(),
                    "retired replay is stale"
                );
                let mut changed = old_a.clone();
                changed.expected_revision = revision + 1;
                let changed = unchecked_command(&owner, &changed, &old_e.content, now);
                assert!(
                    db.post_host_deployment(
                        c,
                        &origin,
                        owner.public_key(),
                        &serde_json::to_vec(&changed).unwrap(),
                        false
                    )
                    .await
                    .is_err(),
                    "retired ID cannot receive fresh authority"
                );
                // Retired IDs cannot cross into opaque Deploy/first-Cleanup IDs,
                // including another fresh placement (which has no fence yet).
                for action in [Action::Deploy, Action::Cleanup] {
                    let mut reused = old_a.clone();
                    reused.agent = foreign_agent;
                    reused.action = action;
                    reused.expected_revision = 1;
                    let reused = unchecked_command(&owner, &reused, &old_e.content, now);
                    assert!(db
                        .post_host_deployment(
                            c,
                            &origin,
                            owner.public_key(),
                            &serde_json::to_vec(&reused).unwrap(),
                            false
                        )
                        .await
                        .is_err());
                }
                let q = wire::Query {
                    v: 1,
                    scope: scope.clone(),
                    view: "operation".into(),
                    nonce: "ab".repeat(32),
                    operation_id: Some(old_a.operation_id.clone()),
                };
                let missing = db
                    .read_host_deployment(c, owner.public_key(), None, false, &q)
                    .await
                    .unwrap();
                assert_eq!(missing.data["state"], "missing-or-superseded");
                assert!(missing.data["event"].is_null() && missing.data["result"].is_null());
            }
            let retained: Vec<String> = sqlx::query_scalar("SELECT event_json FROM paired_agent_operations WHERE community_id=$1 AND action='cleanup'").bind(c.as_uuid()).fetch_all(db.pool()).await.unwrap();
            assert_eq!(retained.len(), if revision == 1 { 1 } else { 2 });
            assert!(retained.contains(&anchor) && retained.contains(&raw));
            let placement = sqlx::query("SELECT unit_id,cleanup_operation_id FROM paired_agent_placements WHERE community_id=$1 AND agent_pubkey=$2").bind(c.as_uuid()).bind(agent.as_bytes().as_slice()).fetch_one(db.pool()).await.unwrap();
            assert_eq!(placement.get::<String, _>("unit_id"), "01".repeat(32));
            let anchor_event: Event = serde_json::from_str(&anchor).unwrap();
            let (
                _,
                wire::Header::Command {
                    authority: anchor_a,
                },
            ) = wire::verify(anchor.as_bytes(), &origin, owner.public_key()).unwrap()
            else {
                panic!()
            };
            assert_eq!(
                placement.get::<String, _>("cleanup_operation_id"),
                anchor_a.operation_id
            );
            assert_eq!(anchor_event.pubkey, owner.public_key());
            let (_, resurrect) = command(
                &owner,
                &scope,
                agent,
                &"03".repeat(32),
                revision + 1,
                Action::Deploy,
            );
            assert!(db
                .post_host_deployment(
                    c,
                    &origin,
                    owner.public_key(),
                    &serde_json::to_vec(&resurrect).unwrap(),
                    false
                )
                .await
                .is_err());
            let result = wire::result(
                &host,
                &DeploymentResult {
                    request: a.clone(),
                    outcome: Outcome::Cleaned,
                    applied_revision: Some(revision + 1),
                },
                now,
            )
            .unwrap();
            db.post_host_deployment(
                c,
                &origin,
                host.public_key(),
                &serde_json::to_vec(&result).unwrap(),
                false,
            )
            .await
            .unwrap();
            previous = Some((a, e));
        }
    }
}
