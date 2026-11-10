//! Runs inside the existing production Management socket fixture.
use super::*;

pub(super) async fn exercise(
    management: &mut Management,
    responses: &std::sync::Mutex<Value>,
    owner: &Keys,
    host: &Keys,
    first: &Intent,
    command: &d::Command,
    status: &d::Status,
) {
    let scope = &first.protected.authority.scope;
    let op = &first.protected.authority.operation_id;
    let signer = || Ok(owner.clone());
    let missing = serde_json::json!({"state":"missing-or-superseded"});
    for state in ["pending", "needs-login", "applied", "failed"] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cleanup.sqlite");
        let mut device = Device::at(&path).unwrap();
        let snapshot =
            serde_json::json!({"record":{"pubkey":first.protected.authority.agent.to_hex()}});
        let deploy = device
            .prepare_deployment(
                &scope.community,
                owner,
                first.protected.authority.clone(),
                command,
                status,
                snapshot.clone(),
            )
            .unwrap();
        *responses.lock().unwrap() = missing.clone();
        assert!(management
            .plan_cleanup(&mut device, &signer, op, true)
            .await
            .is_err());
        assert_eq!(
            device
                .deployment_intents(&scope.community, owner)
                .unwrap()
                .len(),
            1
        );
        let admitted = serde_json::json!({"event":deploy.event,"state":state,"revision":1,"unit_id":"ef".repeat(32)});
        *responses.lock().unwrap() = admitted.clone();
        let plan = management
            .plan_cleanup(&mut device, &signer, op, true)
            .await
            .unwrap();
        let competing = management
            .plan_cleanup(&mut device, &signer, op, true)
            .await
            .unwrap();
        let second_owner = owner.clone();
        let second_path = path.clone();
        let community = scope.community.clone();
        let thread = std::thread::spawn(move || {
            Device::at(&second_path)
                .unwrap()
                .retain_cleanup(&community, &second_owner, competing)
                .unwrap()
        });
        let cleanup = device
            .retain_cleanup(&scope.community, owner, plan)
            .unwrap();
        assert_eq!(thread.join().unwrap().event, cleanup.event);
        let id = &cleanup.protected.authority.operation_id;
        assert!(id.starts_with(wire::CLEANUP_OPERATION_PREFIX));
        assert_eq!(cleanup.phase, 2); // No record/profile effects required.
        assert_eq!(
            device
                .cleanup_predecessor(&scope.community, owner, &cleanup)
                .unwrap()
                .event,
            deploy.event
        );
        assert!(device
            .prepare_deployment(
                &scope.community,
                owner,
                first.protected.authority.clone(),
                command,
                status,
                snapshot.clone()
            )
            .is_err());
        assert!(device
            .deployment_advance(&scope.community, owner, op, 1)
            .is_err());
        assert!(management.dispatch(&mut device, &signer, op).await.is_err());
        assert!(management
            .continue_deployment(
                &mut device,
                &signer,
                op,
                status,
                d::ModelSelection::PendingLogin {},
                snapshot
            )
            .await
            .is_err());
        *responses.lock().unwrap() = serde_json::json!({"by_operation":{op:admitted, id:missing}});
        let duplicate = management
            .plan_cleanup(&mut device, &signer, op, true)
            .await
            .unwrap();
        assert_eq!(
            device
                .retain_cleanup(&scope.community, owner, duplicate)
                .unwrap()
                .event,
            cleanup.event
        );
        management.dispatch(&mut device, &signer, id).await.unwrap();
        drop(device);
        let mut device = Device::at(&path).unwrap();
        let pending = serde_json::json!({"event":cleanup.event,"state":"cleanup-pending","revision":2,"unit_id":"ef".repeat(32)});
        *responses.lock().unwrap() = pending.clone();
        assert!(
            !management
                .cleanup_state(&mut device, &signer, id)
                .await
                .unwrap()
                .1
        );
        let result = d::DeploymentResult {
            request: cleanup.protected.authority.clone(),
            outcome: d::Outcome::Cleaned,
            applied_revision: Some(2),
        };
        let receipt = wire::result(host, &result, now().unwrap()).unwrap();
        let cleaned = serde_json::json!({"event":cleanup.event,"state":"cleaned","result":serde_json::to_string(&receipt).unwrap(),"revision":2,"unit_id":"ef".repeat(32)});
        for field in ["revision", "unit_id", "event", "result", "state"] {
            let mut bad = cleaned.clone();
            bad[field] = match field {
                "revision" => 3.into(),
                "unit_id" => "aa".repeat(32).into(),
                "event" => deploy.event.clone().into(),
                "result" => "{}".into(),
                _ => "pending".into(),
            };
            *responses.lock().unwrap() = bad;
            assert!(management
                .cleanup_state(&mut device, &signer, id)
                .await
                .is_err());
        }
        *responses.lock().unwrap() = cleaned;
        assert!(
            management
                .cleanup_state(&mut device, &signer, id)
                .await
                .unwrap()
                .1
        );
        device
            .deployment_advance(&scope.community, owner, id, 4)
            .unwrap();
        assert!(device
            .deployment_open(&scope.community, owner, &deploy.protected.authority)
            .is_err());
        *responses.lock().unwrap() = serde_json::json!({"denied":true});
        assert!(management
            .cleanup_state(&mut device, &signer, id)
            .await
            .is_err());
        assert_eq!(
            device
                .deployment_intent(&scope.community, owner, id)
                .unwrap()
                .unwrap()
                .phase,
            4
        );
    }
    // Explicit renewal of an expired never-admitted Cleanup proves the original
    // admitted predecessor again, and keeps the original permanent fence.
    let dir = tempfile::tempdir().unwrap();
    let mut device = Device::at(&dir.path().join("renew.sqlite")).unwrap();
    let deploy = device
        .prepare_deployment(
            &scope.community,
            owner,
            first.protected.authority.clone(),
            command,
            status,
            serde_json::json!({"record":{}}),
        )
        .unwrap();
    let mut authority = first.protected.authority.clone();
    authority.action = d::Action::Cleanup;
    authority.expected_revision = 1;
    authority.expires_at = now().unwrap() - 1;
    let old = seed_intent(
        &mut device,
        owner,
        authority,
        &d::Command::Cleanup {},
        serde_json::json!({"cleanup_predecessor":op,"cleanup_unit":"ef".repeat(32)}),
    );
    let old_id = &old.protected.authority.operation_id;
    let admitted = serde_json::json!({"event":deploy.event,"state":"failed","revision":1,"unit_id":"ef".repeat(32)});
    *responses.lock().unwrap() = serde_json::json!({"by_operation":{op:admitted,old_id:missing}});
    assert!(management
        .dispatch(&mut device, &signer, old_id)
        .await
        .is_err());
    let plan = management
        .plan_cleanup(&mut device, &signer, old_id, true)
        .await
        .unwrap();
    let renewed = device
        .retain_cleanup(&scope.community, owner, plan)
        .unwrap();
    assert_ne!(renewed.event, old.event);
    assert_eq!(renewed.protected.authority.expected_revision, 1);
    let new_id = &renewed.protected.authority.operation_id;
    let failed = serde_json::json!({"event":renewed.event,"state":"failed","revision":2,"unit_id":"ef".repeat(32)});
    let historical = serde_json::json!({"event":deploy.event,"state":"failed","revision":2,"unit_id":"ef".repeat(32)});
    *responses.lock().unwrap() =
        serde_json::json!({"by_operation":{op:historical,old_id:missing,new_id:failed}});
    let plan = management
        .plan_cleanup(&mut device, &signer, new_id, true)
        .await
        .unwrap();
    let after_failed = device
        .retain_cleanup(&scope.community, owner, plan)
        .unwrap();
    assert_eq!(after_failed.protected.authority.expected_revision, 2);
    assert_eq!(
        device
            .cleanup_predecessor(&scope.community, owner, &after_failed)
            .unwrap()
            .event,
        deploy.event
    );
    // H-1: A is still current Failed while its replacement B is retained but
    // unadmitted. Reopen must preserve and reuse B for both Check and Retry.
    let after_failed_id = &after_failed.protected.authority.operation_id;
    responses.lock().unwrap()["by_operation"][after_failed_id] = missing.clone();
    drop(device);
    let mut device = Device::at(&dir.path().join("renew.sqlite")).unwrap();
    for renew in [false, true] {
        let plan = management
            .plan_cleanup(&mut device, &signer, after_failed_id, renew)
            .await
            .expect("retained failed-Cleanup renewal must resume");
        let reused = device
            .retain_cleanup(&scope.community, owner, plan)
            .unwrap();
        assert_eq!(reused.event, after_failed.event);
        assert_eq!(reused.protected.authority.operation_id, *after_failed_id);
    }
    management
        .dispatch(&mut device, &signer, after_failed_id)
        .await
        .unwrap();
    {
        let mut replies = responses.lock().unwrap();
        let rows = &mut replies["by_operation"];
        rows[op]["revision"] = 3.into();
        rows[new_id]["revision"] = 3.into();
        rows[after_failed_id] = serde_json::json!({"event":after_failed.event,
            "state":"cleanup-pending","revision":3,"unit_id":"ef".repeat(32)});
    }
    let plan = management
        .plan_cleanup(&mut device, &signer, after_failed_id, false)
        .await
        .unwrap();
    assert_eq!(
        device
            .retain_cleanup(&scope.community, owner, plan)
            .unwrap()
            .event,
        after_failed.event
    );
    let receipt = wire::result(
        host,
        &d::DeploymentResult {
            request: after_failed.protected.authority.clone(),
            outcome: d::Outcome::Cleaned,
            applied_revision: Some(3),
        },
        now().unwrap(),
    )
    .unwrap();
    responses.lock().unwrap()["by_operation"][after_failed_id]["state"] = "cleaned".into();
    responses.lock().unwrap()["by_operation"][after_failed_id]["result"] =
        serde_json::to_string(&receipt).unwrap().into();
    assert!(
        management
            .cleanup_state(&mut device, &signer, after_failed_id)
            .await
            .unwrap()
            .1
    );
    device
        .deployment_advance(&scope.community, owner, after_failed_id, 4)
        .unwrap();
    assert!(device
        .deployment_open(&scope.community, owner, &deploy.protected.authority)
        .is_err());

    // The same interrupted renewal after B expires still requires A's fresh
    // current proof. Check retains exact B; explicit Retry can seal C at rev 2.
    let expired_dir = tempfile::tempdir().unwrap();
    let mut expired = Device::at(&expired_dir.path().join("expired-renew.sqlite")).unwrap();
    expired.deployment_schema().unwrap();
    for intent in [&deploy, &renewed] {
        expired
            .db
            .execute(
                "INSERT INTO deployment_intents VALUES(?,?,?,?)",
                params![
                    scope.community,
                    owner.public_key().to_hex(),
                    intent.protected.authority.operation_id,
                    serde_json::to_string(intent).unwrap(),
                ],
            )
            .unwrap();
    }
    let mut expires = after_failed.protected.authority.clone();
    expires.expires_at = now().unwrap() - 1;
    let expired_b = seed_intent(
        &mut expired,
        owner,
        expires,
        &d::Command::Cleanup {},
        serde_json::json!({"cleanup_predecessor":op,"cleanup_unit":"ef".repeat(32)}),
    );
    let expired_b_id = &expired_b.protected.authority.operation_id;
    *responses.lock().unwrap() = serde_json::json!({"by_operation":{
        op:{"event":deploy.event,"state":"failed","revision":2,"unit_id":"ef".repeat(32)},
        new_id:{"event":renewed.event,"state":"failed","revision":2,"unit_id":"ef".repeat(32)},
        expired_b_id:missing,
    }});
    let check = management
        .plan_cleanup(&mut expired, &signer, expired_b_id, false)
        .await
        .unwrap();
    assert_eq!(
        expired
            .retain_cleanup(&scope.community, owner, check)
            .unwrap()
            .event,
        expired_b.event
    );
    let retry = management
        .plan_cleanup(&mut expired, &signer, expired_b_id, true)
        .await
        .unwrap();
    let c = expired
        .retain_cleanup(&scope.community, owner, retry)
        .unwrap();
    assert_ne!(c.event, expired_b.event);
    assert_eq!(c.protected.authority.expected_revision, 2);
    assert!(expired
        .deployment_open(&scope.community, owner, &deploy.protected.authority)
        .is_err());
    *responses.lock().unwrap() = serde_json::json!({"denied":true});
    assert!(management
        .plan_cleanup(
            &mut expired,
            &signer,
            &c.protected.authority.operation_id,
            true
        )
        .await
        .is_err());

    // Even an expired command can complete from its fresh current signed receipt.
    let result = d::DeploymentResult {
        request: old.protected.authority.clone(),
        outcome: d::Outcome::Cleaned,
        applied_revision: Some(2),
    };
    let receipt = wire::result(host, &result, now().unwrap()).unwrap();
    *responses.lock().unwrap() = serde_json::json!({"event":old.event,"state":"cleaned","result":serde_json::to_string(&receipt).unwrap(),"revision":2,"unit_id":"ef".repeat(32)});
    management
        .dispatch(&mut device, &signer, old_id)
        .await
        .unwrap();
    assert!(
        management
            .cleanup_state(&mut device, &signer, old_id)
            .await
            .unwrap()
            .1
    );
}

// Construct a historical on-disk row with real signatures/encryption, without
// changing production time or weakening command expiry validation.
pub(super) fn seed_intent(
    device: &mut Device,
    owner: &Keys,
    authority: d::Authority,
    command: &d::Command,
    snapshot: Value,
) -> Intent {
    let (authority, event) =
        wire::seal_command(owner, authority, command, now().unwrap() - 120).unwrap();
    let plain = serde_json::to_vec(&Snapshot {
        authority: authority.clone(),
        data: snapshot,
    })
    .unwrap();
    let intent = Intent {
        protected: d::ProtectedCreateIntent {
            authority: authority.clone(),
            ciphertext: event.content.as_bytes().to_vec(),
        },
        event: serde_json::to_string(&event).unwrap(),
        materialization: nip44::encrypt(
            owner.secret_key(),
            &owner.public_key(),
            plain,
            Version::V2,
        )
        .unwrap(),
        phase: 2,
        result: None,
    };
    device.deployment_schema().unwrap();
    device
        .db
        .execute(
            "INSERT INTO deployment_intents VALUES(?,?,?,?)",
            params![
                authority.scope.community,
                owner.public_key().to_hex(),
                authority.operation_id,
                serde_json::to_string(&intent).unwrap()
            ],
        )
        .unwrap();
    intent
}
