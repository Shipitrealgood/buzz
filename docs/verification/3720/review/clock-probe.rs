    #[tokio::test]
    #[ignore = "requires Postgres, Redis and isolated Docker network"]
    async fn review_reconcile_must_refresh_clock_before_first_consumption() {
        fn expired_after_fetch() -> anyhow::Result<RegistrationClock> {
            let mut current = RegistrationClock::read()?;
            current.wall += 601;
            current.millis += 601_000;
            Ok(current)
        }
        let f = Fixture::new().await;
        let o = Keys::generate(); f.member(&o).await;
        let temp = tempfile::tempdir().unwrap();
        let mut d = Device::at(&temp.path().join("owner.sqlite")).unwrap();
        let (_hd, mut h) = host();
        let mut start = clock();
        let invitation = h.offer(f.origin(), &start).unwrap();
        let (scope, claim) = d.pair_registry(&f.origin(), &o, &invitation).unwrap();
        let mut net = f.owner(&o).await;
        net.publish(&o, claim).await.unwrap();
        let mut ws = f.host_socket(&h, &scope.generation).await;
        // Existing injectable source simulates local wall/monotonic expiry while
        // the async offers/evidence work runs. No system clock or relay change.
        start.source = Some(expired_after_fetch);
        assert!(start.refresh().unwrap().wall > start.wall + 600);
        let result = h.reconcile(&mut ws, f.state.relay_keypair.public_key(),
            &scope.generation, "clock probe", &start).await;
        let state = h.state(&scope.generation).unwrap();
        let count = f.binding_count(scope.host).await;
        println!("CLOCK_PROBE fresh_clock_after_expiry=true reconcile_ok={} local_state={state} live_bindings={count}",result.is_ok());
        ws.disconnect().await.unwrap(); net.disconnect().await.unwrap(); f.stop().await;
        assert_eq!(count, 0, "expired first consumption created a live relay binding using the pre-fetch clock");
        assert!(result.is_err());
    }
