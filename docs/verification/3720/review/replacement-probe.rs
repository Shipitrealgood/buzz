    #[tokio::test]
    #[ignore = "requires Postgres, Redis and isolated Docker network"]
    async fn review_failed_old_request_must_not_retire_replacement() {
        let f = Fixture::new().await;
        let o = Keys::generate();
        f.member(&o).await;
        let ctx = buzz_auth::AuthContext {
            pubkey: o.public_key(), scopes: vec![], channel_ids: None,
            auth_method: buzz_auth::AuthMethod::Nip42, agent_owner_pubkey: None,
        };
        let mut tc = crate::connection::tests::test_conn_in(
            AuthState::Authenticated(ctx), None, f.community,
        );
        Arc::get_mut(&mut tc.conn).unwrap().tenant =
            buzz_core::TenantContext::resolved(f.community, f.host.clone());
        // Hold exactly the admission lock both protected reads must acquire.
        let mut tx = f.state.db.pool().begin().await.unwrap();
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('invite-admission:' || $1::text, 0))")
            .bind(f.community.as_uuid()).execute(&mut *tx).await.unwrap();
        let old = registry::query("binding", Some(&"ab".repeat(32)), None).unwrap();
        let frame = json!(["REQ","same-id",{"kinds":[43103],"#p":[o.public_key().to_hex()],"host_registry":old}]).to_string();
        assert!(dispatch(&frame, tc.conn.clone(), f.state.clone()).await);
        let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
        let first = loop {
            if let Some(owner) = tc.conn.subscriptions.lock().await.get("same-id").copied() { break owner; }
            assert!(tokio::time::Instant::now() < deadline);
            tokio::task::yield_now().await;
        };
        let new = registry::query("live", None, None).unwrap();
        let frame = json!(["REQ","same-id",{"kinds":[43103],"#p":[o.public_key().to_hex()],"host_registry":new}]).to_string();
        assert!(dispatch(&frame, tc.conn.clone(), f.state.clone()).await);
        let second = loop {
            if let Some(owner) = tc.conn.subscriptions.lock().await.get("same-id").copied() {
                if owner != first { break owner; }
            }
            assert!(tokio::time::Instant::now() < deadline);
            tokio::task::yield_now().await;
        };
        println!("REPLACEMENT_BARRIER old_owner={first} new_owner={second}");
        tx.commit().await.unwrap();
        let mut observed = Vec::new();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(1);
        while let Ok(Some(frame)) = tokio::time::timeout_at(deadline, tc.send_rx.recv()).await {
            if let axum::extract::ws::Message::Text(raw) = frame {
                let value: serde_json::Value = serde_json::from_str(&raw).unwrap();
                println!("REPLACEMENT_FRAME {value}");
                observed.push(value);
            }
        }
        let retained = tc.conn.subscriptions.lock().await.get("same-id").copied();
        tc.conn.cancel.cancel();
        crate::handlers::close::release_connection_subscriptions(&tc.conn, &f.state).await;
        f.stop().await;
        assert_eq!(retained, Some(second), "older failed read retired the newer live owner; frames={observed:?}");
    }
