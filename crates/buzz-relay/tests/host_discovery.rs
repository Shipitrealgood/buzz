//! One real EVENT/REQ journey, independent of obsolete inline relay unit fixtures.
//! Use the isolated 3720 fixture and cached runtime/ACP; see the #2722 delivery receipt.
use anyhow::{ensure, Context, Result};
use buzz_core::{
    host_deployment::{self as d, observation::Models, wire},
    host_registry as registry_wire,
    paired_message::Scope,
};
use buzz_hostd::registration::{Host, RegistrationClock};
use buzz_paired_owner::{
    deployment::Management,
    registry::{self, Registry},
    Device,
};
use buzz_ws_client::NostrWsConnection;
use nostr::{Event, Keys, ToBech32};
use serde_json::json;
use std::{fs, sync::Arc, time::Duration};
#[path = "host_discovery/fixture.rs"]
mod fixture;
const ORIGIN: &str = "https://discovery-fixture.test";
const TEST: &str = "owner_relay_host_discovery_selected_deploy_and_revoke";
fn now() -> u64 {
    nostr::Timestamp::now().as_secs()
}
fn clock() -> Result<RegistrationClock> {
    RegistrationClock::read()
}
fn query(scope: &Scope, view: &str, operation: Option<&str>) -> wire::Query {
    wire::Query {
        v: 1,
        scope: scope.clone(),
        view: view.into(),
        nonce: Keys::generate().secret_key().to_secret_hex(),
        operation_id: operation.map(str::to_string),
    }
}

#[tokio::test]
#[ignore = "requires the network-none 3720 fixture and compiled runtime/ACP"]
async fn owner_relay_host_discovery_selected_deploy_and_revoke() -> Result<()> {
    ensure!(
        std::env::var("BUZZ_3720_ISOLATED_FIXTURE").as_deref() == Ok("1"),
        "isolated runner required"
    );
    let database = std::env::var("BUZZ_TEST_DATABASE_URL")?;
    let url = url::Url::parse(&database)?;
    ensure!(
        url.host_str() == Some("127.0.0.1")
            && url.username() == "buzz3720"
            && url.path().starts_with("/buzz_3720_fixture_"),
        "disposable DB required"
    );
    if std::env::var("BUZZ_DISCOVERY_CHILD").as_deref() != Ok("1") {
        let dir = tempfile::tempdir()?;
        // Config::from_env never observes the developer's HOME, endpoints or credentials.
        let mut child = tokio::process::Command::new(std::env::current_exe()?)
            .args(["--ignored", "--exact", TEST, "--nocapture"])
            .env_clear()
            .env("HOME", dir.path())
            .env("PATH", "/usr/bin:/bin")
            .env("BUZZ_3720_ISOLATED_FIXTURE", "1")
            .env("BUZZ_DISCOVERY_CHILD", "1")
            .env("BUZZ_TEST_DATABASE_URL", &database)
            .env("DATABASE_URL", &database)
            .env("REDIS_URL", "redis://127.0.0.1:6379/0")
            .env("BUZZ_GIT_REPO_PATH", dir.path().join("repos"))
            .env("BUZZ_GIT_PACK_CACHE_PATH", dir.path().join("pack-cache"))
            .current_dir(dir.path())
            .kill_on_drop(true)
            .spawn()?;
        let result = tokio::time::timeout(Duration::from_secs(120), child.wait()).await;
        if result.is_err() {
            child.kill().await?;
            child.wait().await?;
        }
        ensure!(
            result.context("bounded fixture deadline")??.success(),
            "fixture child failed"
        );
        return Ok(());
    }
    let mut f = fixture::Fixture::new(&database).await?;
    let result = journey(&f).await;
    f.stop().await;
    result
}

async fn journey(f: &fixture::Fixture) -> Result<()> {
    let owner = Keys::generate();
    let _ = f
        .state
        .db
        .add_relay_member(
            f.community,
            &owner.public_key().to_hex(),
            "member",
            None,
            &f.state.relay_keypair,
        )
        .await?;
    let relay = f.state.relay_keypair.public_key();
    let root = tempfile::tempdir()?;
    let state_dir = root.path().join("state");
    fs::create_dir(&state_dir)?;
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&state_dir, fs::Permissions::from_mode(0o700))?;
    let mut host = Host::at(&state_dir, true)?;
    let mut device = Device::at(&root.path().join("owner.sqlite"))?;
    let mut net = f.owner(&owner).await?;
    let invitation = host.offer(ORIGIN.into(), &clock()?)?;
    let (scope, claim) = device.pair_registry(ORIGIN, &owner, &invitation)?;
    net.publish(&owner, claim).await?;
    let mut host_ws = f.socket().await?;
    host.authenticate(&mut host_ws, &scope.generation).await?;
    let helper = fixture::installation(root.path(), &state_dir, relay)?;
    let (catalog, revision) = helper.catalog()?;
    host.reconcile_catalog(
        &mut host_ws,
        relay,
        &scope.generation,
        "Fixture",
        &clock()?,
        Some((&catalog, &revision)),
    )
    .await?;
    net.recover(&mut device, &owner).await?;
    ensure!(
        device.view(ORIGIN, &owner, &scope)?.state == "paired",
        "pair recovery"
    );
    host.reconcile_deployments(
        &mut host_ws,
        relay,
        &scope.generation,
        &catalog,
        &revision,
        &helper,
    )
    .await?;
    let mut ws = f.socket().await?;
    ws.authenticate(&owner, None).await?;
    let mut management =
        Management::from_authenticated(true, ORIGIN, owner.public_key(), relay, ws)?;
    let reply = management
        .query(&|| Ok(owner.clone()), &query(&scope, "status", None))
        .await?;
    let event: Event = serde_json::from_str(reply.data["event"].as_str().context("status")?)?;
    let status = wire::open_status(&owner, &event, &scope, now())?;
    ensure!(
        matches!(&status.capability, d::Capability::Supported { catalog_revision, runtimes, harnesses } if catalog_revision == &revision && runtimes.contains(&"main/claude".into()) && harnesses.get("main/claude").map(String::as_str) == Some("claude")),
        "installed host harness"
    );
    let before = f.effects().await?;
    ensure!(before.2 == 0 && before.3 == 0, "fresh placement DB");
    // The owner publishes via Registry::models; the host learns the exact request
    // through actual relay messages/evidence REQs, never an injected event.
    let serve = async {
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let count: i64 = sqlx::query_scalar(
                    "SELECT count(*) FROM paired_host_events WHERE operation='models-query'",
                )
                .fetch_one(f.state.db.pool())
                .await?;
                if count == 1 {
                    return Ok::<_, anyhow::Error>(());
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await??;
        host.reconcile_catalog(
            &mut host_ws,
            relay,
            &scope.generation,
            "Fixture",
            &clock()?,
            Some((&catalog, &revision)),
        )
        .await
    };
    let signer = || Ok(owner.clone());
    let (models, served) =
        tokio::join!(net.models(&signer, &scope, "main/claude", &revision), serve);
    served?;
    let models = models?;
    ensure!(
        models.request.runtime_id == "main/claude" && models.request.catalog_revision == revision,
        "correlated runtime/revision"
    );
    ensure!(
        models.readiness == buzz_core::harness::Readiness::LoggedIn,
        "synthetic readiness"
    );
    let Models::Available {
        catalog: model_catalog,
    } = models.models
    else {
        anyhow::bail!("model unavailable")
    };
    ensure!(model_catalog.models.len() == 1, "one selected model");
    let selected = model_catalog.models[0].id.clone();
    ensure!(
        selected == "opaque/model:fixture",
        "exact ACP-normalized model"
    );
    ensure!(
        f.effects().await? == before,
        "discovery created users/events/operations/placements"
    );
    ensure!(
        fs::read_dir(state_dir.join("hosted"))?.count() == 0
            && !root.path().join("calls").exists()
            && !root.path().join("conversation").exists(),
        "preCreate identity/service effects"
    );
    let local = rusqlite::Connection::open(&state_dir.join("host.sqlite"))?;
    ensure!(
        local.query_row("SELECT count(*) FROM deployment_units", [], |r| r
            .get::<_, i64>(0))?
            == 0,
        "preCreate host identity"
    );
    ensure!(
        fs::read_to_string(root.path().join("discovery-calls"))? == ".",
        "one discovery"
    );
    println!(
        "PASS actual owner -> relay EVENT/REQ -> host ACP -> exact model; zero preCreate effects"
    );

    let page = net
        .page(
            &owner,
            registry::query("messages", Some(&scope.generation), None)?,
        )
        .await?;
    let entries = page.data["events"].as_array().context("messages")?;
    let id = |op| {
        entries
            .iter()
            .find(|e| e["op"] == op)
            .and_then(|e| e["id"].as_str())
            .context("message ID")
    };
    let request = net
        .evidence(&owner, &scope.generation, id("models-query")?)
        .await?;
    let response_id = id("models")?.to_owned();
    // Restart the host journal and replay the exact signed request through the relay.
    drop(host);
    let mut host = Host::at(&state_dir, false)?;
    net.publish(&owner, request.clone()).await?;
    host.reconcile_catalog(
        &mut host_ws,
        relay,
        &scope.generation,
        "Fixture",
        &clock()?,
        Some((&catalog, &revision)),
    )
    .await?;
    ensure!(
        net.evidence(&owner, &scope.generation, &response_id)
            .await?
            .id
            .to_hex()
            == response_id,
        "exact response replay"
    );
    ensure!(
        fs::read_to_string(root.path().join("discovery-calls"))? == ".",
        "replay reexecuted discovery"
    );
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM paired_host_events WHERE operation IN ('models-query','models')",
    )
    .fetch_one(f.state.db.pool())
    .await?;
    ensure!(count == 2, "duplicate model rows");
    // Valid signed envelope, wrong enrollment parent: actual relay admission refuses.
    let wrong = registry_wire::seal(
        &owner,
        &scope,
        registry_wire::Body::ModelsQuery {
            request: registry_wire::ModelsQuery {
                ack_id: "ff".repeat(32),
                ..models.request
            },
        },
        "ee".repeat(32),
        Some("ff".repeat(32)),
        now(),
        now() + 60,
    )?;
    ensure!(
        net.publish(&owner, wrong).await.is_err(),
        "wrong parent admitted"
    );
    println!("PASS exact request/reply replay after host reopen; wrong parent refused");

    // Native main creation is a separately accepted gate. This fixture enters its
    // existing owner boundary with synthetic main materialization, after discovery.
    let agent = Keys::generate();
    let operation = "ab".repeat(32);
    let command = d::Command::Deploy {
        config: Box::new(d::Deploy {
            catalog_revision: revision.clone(),
            runtime_id: "main/claude".into(),
            name: "Disposable integration agent".into(),
            policy: buzz_core::paired_message::Policy {
                respond_to: "owner-only".into(),
                respond_to_allowlist: vec![],
                channel_add_policy: "owner_only".into(),
            },
            model: d::ModelSelection::Selected {
                id: selected.clone(),
            },
            provider: None,
            system_prompt: None,
            effort: None,
            parallelism: 1,
        }),
        identity: buzz_core::private_managed_agent::PrivateIdentity {
            private_key_nsec: agent.secret_key().to_bech32()?,
            auth_tag: Some(buzz_sdk::nip_oa::compute_auth_tag(
                &owner,
                &agent.public_key(),
                "",
            )?),
        },
    };
    let authority = d::Authority {
        v: 1,
        scope: scope.clone(),
        agent: agent.public_key(),
        action: d::Action::Deploy,
        operation_id: operation.clone(),
        expected_revision: 0,
        expires_at: now() + 500,
        content_sha256: String::new(),
    };
    let intent = device.prepare_deployment(
        ORIGIN,
        &owner,
        authority,
        &command,
        &status,
        json!({"record":{"pubkey":agent.public_key().to_hex()}}),
    )?;
    ensure!(
        management
            .dispatch(&mut device, &|| Ok(owner.clone()), &operation)
            .await
            .is_err(),
        "pre-materialization dispatch"
    );
    device.deployment_advance(ORIGIN, &owner, &operation, 1)?;
    device.deployment_advance(ORIGIN, &owner, &operation, 2)?;
    management
        .dispatch(&mut device, &|| Ok(owner.clone()), &operation)
        .await?;
    management
        .dispatch(&mut device, &|| Ok(owner.clone()), &operation)
        .await?;
    host.reconcile_deployments(
        &mut host_ws,
        relay,
        &scope.generation,
        &catalog,
        &revision,
        &helper,
    )
    .await?;
    let receipt = management
        .operation(&mut device, &|| Ok(owner.clone()), &operation)
        .await?;
    ensure!(
        receipt.state == "applied" && receipt.current,
        "selected Deploy not applied: {}",
        receipt.state
    );
    ensure!(
        receipt.body.context("deployment result")?.result.request == intent.protected.authority,
        "result authority"
    );
    ensure!(
        fs::read_to_string(root.path().join("conversation"))? == selected,
        "runtime selected model"
    );
    ensure!(
        fs::read_to_string(root.path().join("calls"))?
            .lines()
            .filter(|v| *v == "start")
            .count()
            == 1,
        "duplicate start"
    );
    let unit = buzz_hostd::unit_id(&scope, &operation);
    let prepared: buzz_hostd::Prepared = serde_json::from_slice(&fs::read(
        state_dir.join("hosted").join(&unit).join("instance"),
    )?)?;
    ensure!(
        prepared.instance.agent == agent.public_key().to_hex()
            && prepared
                .instance
                .deployment
                .context("instance deployment")?
                .config
                .model
                == d::ModelSelection::Selected { id: selected },
        "exact selected instance"
    );
    let counts = f.effects().await?;
    ensure!(counts.2 == 1 && counts.3 == 1, "one operation/placement");
    println!("PASS selected Deploy admitted once; actual runtime guard/ACP ran; fake conversation harness READY");

    let cleanup = d::Authority {
        v: 1,
        scope: scope.clone(),
        agent: agent.public_key(),
        action: d::Action::Cleanup,
        operation_id: "cd".repeat(32),
        expected_revision: 1,
        expires_at: now() + 500,
        content_sha256: String::new(),
    };
    let (cleanup, event) = wire::seal_command(&owner, cleanup, &d::Command::Cleanup {}, now())?;
    let mut writes = f.socket().await?;
    writes.authenticate(&owner, None).await?;
    ensure!(
        writes.send_event(event).await?.accepted,
        "cleanup admission"
    );
    host.reconcile_deployments(
        &mut host_ws,
        relay,
        &scope.generation,
        &catalog,
        &revision,
        &helper,
    )
    .await?;
    let reply = management
        .query(
            &|| Ok(owner.clone()),
            &query(&scope, "operation", Some(&cleanup.operation_id)),
        )
        .await?;
    let result: Event =
        serde_json::from_str(reply.data["result"].as_str().context("cleanup result")?)?;
    ensure!(
        wire::open_result_body(&owner, &result, &cleanup)?
            .result
            .outcome
            == d::Outcome::Cleaned,
        "cleanup outcome"
    );
    ensure!(
        !state_dir.join("hosted").join(&unit).exists() && !root.path().join("active").exists(),
        "service residue"
    );
    ensure!(
        fs::read_to_string(root.path().join("home/fixture-login"))? == "synthetic; preserve"
            && fs::read_to_string(root.path().join("workspace/user-work"))? == "preserve",
        "normal account state changed"
    );
    let revoke = device.unpair_registry(ORIGIN, &owner, &scope)?;
    net.publish(&owner, revoke).await?;
    ensure!(
        net.publish(&owner, request.clone()).await.is_err(),
        "revoked exact replay admitted"
    );
    ensure!(
        net.evidence(&owner, &scope.generation, &response_id)
            .await
            .is_err(),
        "revoked model read admitted"
    );
    host.reconcile_catalog(
        &mut host_ws,
        relay,
        &scope.generation,
        "Fixture",
        &clock()?,
        Some((&catalog, &revision)),
    )
    .await?;
    ensure!(
        host.state(&scope.generation)? == "dead",
        "host revocation not reconciled"
    );
    ensure!(
        host.receive_models(&scope.generation, &request, &catalog, &revision, &clock()?)
            .await
            .is_err(),
        "revoked local probe admitted"
    );
    println!(
        "PASS cleanup receipt; fixture home/work preserved; revoked query/read/execution refused"
    );
    writes.disconnect().await?;
    host_ws.disconnect().await?;
    net.disconnect().await?;
    Ok(())
}
