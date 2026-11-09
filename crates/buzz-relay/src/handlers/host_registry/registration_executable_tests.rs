// Included in the existing isolated external-infrastructure test namespace.
#[tokio::test]
#[ignore = "requires isolated Postgres/Redis/TLS fixture and compiled buzz-hostd"]
async fn executable_registration_restart_recovery_revoke_and_native_scope_fence() {
    use rustls::pki_types::pem::PemObject;
    use std::sync::atomic::{AtomicUsize, Ordering};
    let _ = rustls::crypto::ring::default_provider().install_default();
    let mut f = Fixture::new().await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:32000")
        .await
        .unwrap();
    let router = crate::router::build_router(f.state.clone());
    f.tasks.push(tokio::spawn(async move {
        axum::serve(
            listener,
            router.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    }));
    let community = f
        .state
        .db
        .ensure_configured_community("registry-tls-a.test:34443")
        .await
        .unwrap()
        .id;
    let owner = Keys::generate();
    let _ = f
        .state
        .db
        .add_relay_member(
            community,
            &owner.public_key().to_hex(),
            "member",
            None,
            &f.state.relay_keypair,
        )
        .await
        .unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let binary = root
        .join("target/debug/buzz-hostd")
        .canonicalize()
        .expect("build actual hostd first");
    let ca = root.join("target/3720-tls/ca.pem");
    let pem = std::fs::read(&ca).unwrap();
    let mut roots = rustls::RootCertStore::empty();
    for cert in rustls::pki_types::CertificateDer::pem_slice_iter(&pem) {
        roots.add(cert.unwrap()).unwrap();
    }
    let config = Arc::new(
        rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth(),
    );
    let origin = "https://registry-tls-a.test:34443";
    let mut socket = NostrWsConnection::connect_with_tls_connector(
        "wss://registry-tls-a.test:34443",
        buzz_ws_client::Connector::Rustls(config),
    )
    .await
    .unwrap();
    socket.authenticate(&owner, None).await.unwrap();
    let mut registry = Registry::from_authenticated(
        origin,
        owner.public_key(),
        f.state.relay_keypair.public_key(),
        socket,
    )
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let base = || {
        let mut command = tokio::process::Command::new(&binary);
        command
            .args(["--state-dir"])
            .arg(dir.path())
            .arg("registration")
            .kill_on_drop(true);
        command
    };
    let init = base().arg("init").output().await.unwrap();
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );
    let public_key = String::from_utf8(init.stdout).unwrap();
    assert_eq!(public_key.trim().len(), 64);
    let refused = base()
        .args(["offer", "--community", origin])
        .output()
        .await
        .unwrap();
    assert!(!refused.status.success());
    assert!(
        refused.stdout.is_empty(),
        "private invitation must not enter redirected logs"
    );
    // The offer uses the same persistent store; all reconciliation below runs the binary.
    let mut host = Host::at(dir.path(), false).unwrap();
    let offer = host.offer(origin.into(), &clock()).unwrap();
    let device_dir = tempfile::tempdir().unwrap();
    let mut device = Device::at(&device_dir.path().join("owner.sqlite")).unwrap();
    let (scope, claim) = device.pair_registry(origin, &owner, &offer).unwrap();
    registry.publish(&owner, claim).await.unwrap();
    let pass = || {
        let mut command = base();
        command
            .args([
                "run",
                "--community",
                origin,
                "--relay-key",
                &f.state.relay_keypair.public_key().to_hex(),
                "--ca-cert",
            ])
            .arg(&ca)
            .args(["--once"]);
        command
    };
    let result = pass().output().await.unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    registry.recover(&mut device, &owner).await.unwrap();
    assert_eq!(device.view(origin, &owner, &scope).unwrap().state, "paired");
    assert!(!device.view(origin, &owner, &scope).unwrap().fresh);
    let retry = pass().output().await.unwrap();
    assert!(
        retry.status.success(),
        "{}",
        String::from_utf8_lossy(&retry.stderr)
    );
    assert!(
        String::from_utf8(base().arg("status").output().await.unwrap().stdout)
            .unwrap()
            .contains(public_key.trim())
    );
    let recovery_dir = tempfile::tempdir().unwrap();
    let mut recovered = Device::at(&recovery_dir.path().join("recovered.sqlite")).unwrap();
    let checks = AtomicUsize::new(0);
    let changed = || {
        let call = checks.fetch_add(1, Ordering::SeqCst);
        anyhow::ensure!(call < 4, "native workspace changed across fetch");
        Ok(owner.clone())
    };
    assert!(registry
        .recover_guarded(&mut recovered, &changed)
        .await
        .is_err());
    assert!(checks.load(Ordering::SeqCst) >= 5);
    assert!(recovered.registry_hosts(origin, &owner).unwrap().is_empty());
    registry.recover(&mut recovered, &owner).await.unwrap();
    assert_eq!(recovered.registry_hosts(origin, &owner).unwrap().len(), 1);
    // Actual long-running command holds the controller lock across network waits.
    let mut controller = base()
        .args([
            "run",
            "--community",
            origin,
            "--relay-key",
            &f.state.relay_keypair.public_key().to_hex(),
            "--ca-cert",
        ])
        .arg(&ca)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;
    let second = pass().output().await.unwrap();
    assert!(!second.status.success());
    assert!(String::from_utf8_lossy(&second.stderr).contains("another host controller"));
    let host_key = host.public_key().unwrap();
    let ids = || {
        f.state
            .conn_manager
            .connection_ids_for_pubkey_in_community(community, host_key.as_bytes())
    };
    // Old successful five-second polling leaves no idle authenticated socket.
    let retained = ids();
    assert_eq!(
        retained.len(),
        1,
        "idle registration must retain its authenticated connection"
    );
    tokio::time::sleep(Duration::from_secs(6)).await;
    assert_eq!(ids(), retained, "idle registration reauthenticated");
    let query = device
        .registry_status_query(origin, &owner, &scope)
        .unwrap();
    registry.publish(&owner, query).await.unwrap();
    let status_count = || async {
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM paired_host_events WHERE community_id=$1 AND generation=$2 AND operation='status'")
            .bind(community.as_uuid()).bind(hex::decode(&scope.generation).unwrap())
            .fetch_one(f.state.db.pool()).await.unwrap()
    };
    tokio::time::timeout(Duration::from_secs(10), async {
        while status_count().await != 1 {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("verified live notice must refresh before the 60-second backstop");
    tokio::time::sleep(Duration::from_secs(6)).await;
    assert_eq!(
        status_count().await,
        1,
        "self-notification amplified status writes"
    );
    assert_eq!(ids(), retained);
    let revoke = device.unpair_registry(origin, &owner, &scope).unwrap();
    registry.publish(&owner, revoke).await.unwrap();
    tokio::time::timeout(Duration::from_secs(10), async {
        while host.state(&scope.generation).unwrap() != "dead" {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("live revocation must refresh before the backstop");
    println!("REFRESH_EXECUTABLE retained_connection=true idle_seconds=6 status_reply=1 self_amplification=0 notification_revoke=true");
    controller.kill().await.unwrap();
    controller.wait().await.unwrap();
    let result = pass().output().await.unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    registry.recover(&mut device, &owner).await.unwrap();
    assert_eq!(
        device.view(origin, &owner, &scope).unwrap().state,
        "unpaired"
    );
    let status = base().arg("status").output().await.unwrap();
    assert!(status.status.success());
    assert!(String::from_utf8_lossy(&status.stdout).contains("dead"));
    assert!(base()
        .args(["unpair", "--generation", &scope.generation])
        .status()
        .await
        .unwrap()
        .success());
    assert!(pass().status().await.unwrap().success());
    // Host-local offline revocation precedes an unsent owner intent. The same
    // synchronization used by Windows listing must recover before it retries.
    let invitation = host.offer(origin.into(), &clock()).unwrap();
    let (next, claim) = device.pair_registry(origin, &owner, &invitation).unwrap();
    registry.publish(&owner, claim).await.unwrap();
    assert!(pass().status().await.unwrap().success());
    registry.recover(&mut device, &owner).await.unwrap();
    let _unsent = device.unpair_registry(origin, &owner, &next).unwrap();
    assert!(base()
        .args(["unpair", "--generation", &next.generation])
        .status()
        .await
        .unwrap()
        .success());
    assert!(pass().status().await.unwrap().success());
    registry
        .synchronize_guarded(&mut device, &|| Ok(owner.clone()))
        .await
        .unwrap();
    assert_eq!(
        device.view(origin, &owner, &next).unwrap().state,
        "unpaired"
    );
    assert!(device.next_deliveries(origin, &owner).unwrap().is_empty());
    f.stop().await;
}
