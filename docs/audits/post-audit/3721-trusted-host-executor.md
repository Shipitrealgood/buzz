# #3721 — trusted-host executor independent post-audit

Date: 2026-11-10. Reviewer: `/root/trusted_host_review`, `codex-chuck`.
Requested configuration: `gpt-6-astra` / `high`. Actual served model/reasoning: not independently exposed; unknown.
Rework round: 0 (initial review).

**Verdict: rework.** One Medium correctness finding, M-1, prevents S1 source acceptance until fixed under the conductor's ruling. C=0 / H=0 / M=1 / L=0. No additional architecture prerequisite or product decision is needed. Source tests pass; the focused cleanup falsification fails.

## Pin and contract

Worktree: `/home/bryce/services/buzz-wt-3720-host-registration`, branch `proof/3720-host-registration-events`.
Reviewed range: `b8a9ccd97e62465d804bbc85125ce858a873fe8d..e0cc1d529af62bac6689de60e52e21bd9e544e80`; implementation commit `b21fe0ff8`. Starting tree was clean. Review documentation is a separate commit after that pin.

Governing inputs: [ratified S1](../../specs/3722-main-discovery-native-login.md#s1--first-finite-code-change-local-user-service-executor), [executor review dispatch](../../briefs/3721-trusted-host-executor-review.md), [executor receipt](../../executions/3721-trusted-host-executor.md), project `AGENTS.md`, `VISION.md`, `VISION_AGENT.md`, `VISION_REMOTE_AGENTS.md`, `ARCHITECTURE.md` registration contract, `TESTING.md`, portable conductor contract and post-audit skill. The user-approved shared non-root account/HOME/native login/network model controls; same-account agents are trusted.

Scope: local user-service execution, ordinary deployment wiring, shared finite runner changes, and registration preservation. S2 registry/readiness and S3 UI remain excluded. Conversation deliberately remains unavailable. Legacy helper compatibility is a separate caller, with no new-route root fallback. Review used fake commands, disposable directories/sockets and the existing synthetic ACP fixture. No native manager, provider login, real credentials, live registration/relay, existing agent, deployment, board mutation, merge or push was accessed/performed.

## M-1 (Medium, high confidence) — cleanup depends on obsolete launch paths

**Read:** [cleanup calls `local.validate()` before stop](../../../crates/buzz-hostd/src/user_service.rs#L362). That validator [requires the saved HOME/workspace to exist and be canonical, and the launcher to remain executable](../../../crates/buzz-hostd/src/user_service.rs#L40). These are launch prerequisites, not prerequisites for identifying and stopping the recorded service. [Executor construction also validates the current launch catalog](../../../crates/buzz-hostd/src/user_service.rs#L89), including before the controller can [start durable cleanup retries](../../../crates/buzz-hostd/src/registration_refresh.rs#L38).

**Measured failure:** prepare one fixture unit; start a finite action; move its repository from `repo` to `moved-repo`; update the current local catalog to the new workspace; reopen the executor successfully; request cleanup. Cleanup returns `No such file or directory (os error 2)` before any manager call. The fake unit remains active, the instance/credential artifact remains, and the moved repository remains intact. The cleanup tombstone is written, but retries reload the same obsolete saved path and fail again. Updating the current valid configuration does not recover this recorded placement.

This is a normal account maintenance scenario, not a hostile same-account agent: users move/remove workspaces or replace old launcher paths. The real finite runtime still has its own deadline; the reproduction does not claim an indefinitely running native service. The concrete defect is refusal to attempt the authorized stop and inability to converge generated-artifact cleanup without recreating unrelated launch paths. It violates S1's durable cleanup/restart contract.

**Suggested bounded repair:** separate launch validation from restrictive cleanup validation. Retain canonical/private state checks, the recorded manager/unit location, exact unit ownership checks and confirmed-stop requirement. Cleanup must not require the former workspace/HOME/launcher or a usable current launch catalog. Keep the record until stop/deletion completes. Cover both a valid updated catalog with an obsolete saved workspace (the reproduced case) and cleanup after reopen with unavailable launch configuration. No alternate manager, root fallback, migration or new recovery protocol is warranted.

| Finding | Proposed owner / next action | Disposition |
| --- | --- | --- |
| M-1 | Existing #3721 executor; separate cleanup prerequisites, add the focused regression, return to this reviewer for the original obligation and introduced-defect re-gate | fix in lane — conductor accepted the concrete failure on 2026-11-10 and directed repair in existing #3721 before S1 acceptance |

## Obligation results and call-chain review

| S1 obligation | Independent result and evidence |
| --- | --- |
| Stable prepare/replay/reopen, identity and scope/revision fences | PASS within existing cases. Read owner signature/scope/placement checks and durable journal before execution at [deployment.rs:373](../../../crates/buzz-hostd/src/deployment.rs#L373), identity staging/permit/prepare at [458](../../../crates/buzz-hostd/src/deployment.rs#L458), and retained identity/revision checks at [user_service.rs:202](../../../crates/buzz-hostd/src/user_service.rs#L202). Measured `prepare_replay_restart_and_identity_fences_precede_effects`; distinct keys remain private. |
| Non-root account context, fixed local argv/workspace, no remote shell/env/path, no key to finite child | PASS in source/disposable execution. [Non-root controller](../../../crates/buzz-hostd/src/user_service.rs#L89), [closed local environment](../../../crates/buzz-hostd/src/user_service.rs#L54), [cleared child environment and workspace](../../../crates/buzz-hostd/src/runtime_context.rs#L168), [fixed model command](../../../crates/buzz-hostd/src/runtime_context.rs#L236), and [credential entry](../../../crates/buzz-hostd/src/runtime_context.rs#L472). Measured account-context fixture checks HOME/PATH/argv/cwd/UID and absence of Buzz credentials; closed remote config/identity contract tests pass. The credential-loading entry itself was read, not executed under a native manager. |
| Finite actions: exclusion, nonce/deadline, descendants, failed start/stop retry | PASS in bounded fixtures. Existing `finite_actions_are_serialized_correlated_and_cancel_confirmed` and `trusted_finite_runner_uses_account_context_and_bounds_process_tree` pass. Reviewer probe adds a failed manager start after activation plus failed stop, retries cancel using retained execution, then successfully starts the next nonce; terminal observation waits for inactive state and rejects wrong embedded nonce. [Source transaction](../../../crates/buzz-hostd/src/user_service.rs#L261), [finite supervisor](../../../crates/buzz-hostd/src/runtime_context.rs#L260). No native cgroup claim. |
| Cleanup fence, confirmed stop, partial failure, preservation and symlink safety | PARTIAL / M-1. Existing failed-stop retry and symlink/preservation test passes; [stop verification](../../../crates/buzz-hostd/src/user_service.rs#L526) precedes [bounded exact artifact removal](../../../crates/buzz-hostd/src/user_service.rs#L364). HOME/provider fixture, repository and unrelated unit survive. Moved-workspace cleanup fails before stop. |
| Ordinary route independent of root helper; registration preserved; truthful gate | PASS in source/existing fixtures. [Single alias](../../../crates/buzz-hostd/src/deployment.rs#L16), [conditional CLI preflight](../../../crates/buzz-hostd/src/registration_cli.rs#L88), [conditional refresh](../../../crates/buzz-hostd/src/registration_refresh.rs#L273). `deployment=false` never constructs this executor or loads its catalog. Existing registration identity/auth/owner/revoke/connection implementations are unchanged in the pin; all 13 controller tests pass, including idle heartbeat, offline revoke, reconnect/retry, wrong scope, and cleanup scheduling. Installed registration service/sandbox files are absent from the diff. [Start refusal](../../../crates/buzz-hostd/src/user_service.rs#L257), [finite login refusal](../../../crates/buzz-hostd/src/runtime_context.rs#L221), and [NotObserved/no login receipt checks](../../../crates/buzz-hostd/src/user_service.rs#L302) preserve the S1 limit. |

## Scope, reuse and quality judgment

**Read:** the approximately 594 net production-bearing lines fill the approved local lifecycle gap. The controller journal, instance validation, execution/result types, inactive predicate, model normalization and finite runner are reused. There is no second provider registry or generic service-manager framework. Retaining the legacy implementation for its explicit caller is justified by S1; the `Helper` alias selects exactly one implementation. The added `LocalContext` and fixed drop-in are proportionate to account-local configuration and credential custody. No structural prerequisite finding.

Minimalism: 9/10. Elegance: 9/10. Correctness: 8/10 because M-1 couples restrictive cleanup to obsolete launch prerequisites; the proposed separation addresses that concrete defect. Useful strengths are per-unit locking, retained stop-failure context, exact derived artifact paths, explicit unavailable conversation behavior, and reuse of main model normalization.

## Independent verification ledger

All Cargo commands used `export HERMIT_SELF_UPDATE=false`, `. ./bin/activate-hermit`, `--offline`, and serial test execution. The executor's receipts were read first. This reviewer independently ran the affected suites below, without a new test platform or mutation campaign.

| Command / check | Measured result |
| --- | --- |
| `cargo test --offline -p buzz-hostd --all-targets -- --test-threads=1` (sandbox) | 21 library pass, 2 socket `EPERM` failures, 1 ignored; stopped before other targets. Environment failure retained in this ledger. |
| Same command with narrowly approved disposable-socket escalation | Library 23 PASS / 1 ignored; activation binary 1 PASS; controller binary 13 PASS; runtime binary and examples each 0 tests. Exit 0. |
| `cargo test --offline -p buzz-core --lib host_deployment -- --test-threads=1` | 12 PASS, 302 filtered; exit 0. |
| `BUZZ_TEST_ACP="$PWD/target/debug/buzz-acp" cargo test --offline -p buzz-hostd --lib actual_acp_models -- --ignored --test-threads=1` | 1 PASS; six synthetic Grok/Claude ACP invocations via the previously built local harness. No production harness source changed in this range. Disposable-socket escalation only. |
| `cargo test --offline -p buzz-hostd --lib review_ -- --test-threads=1 --nocapture` after appending the two probes below to the existing fixture | 1 PASS (`review_start_failure_retry_and_terminal_observation`), 1 FAIL (`review_cleanup_after_workspace_move`); exit 101. No production mutation. Exact failure receipt below. |
| `cargo fmt -p buzz-hostd -- --check`; `git diff --check`; `git diff --exit-code -- crates` after probe restoration | PASS; temporary test additions fully restored to the reviewed pin. |

No redundant clippy/size campaign: the executor's normal clippy, differential frontend size checks and compile receipts remain applicable to unchanged production source; these were not independently rerun or represented as reviewer passes. Strict clippy remains reported blocked by pre-existing `buzz-core` warnings in the executor receipt. Full `just ci`, Windows/Desktop builds, real systemd credential/cgroup/user-manager persistence, provider/native login/readiness/quota/chat, live relay/database integration and human acceptance were NOT RUN. Under the dispatch, these do not independently block source-only S1 acceptance; M-1 does require repair.

### Reproducible focused falsification

Append the following to the existing `user_service_tests.rs` fixture at the reviewed pin, then run the `review_` command above. It invokes the actual executor methods. The fake manager's `active` file represents its service state; no actual unit is started. This exact probe source is preserved here so the evidence does not depend on temporary files.

```rust
#[tokio::test]
async fn review_cleanup_after_workspace_move() {
    let (dir, service, prepared, mut catalog) = fixture();
    let id = &prepared.instance.unit_id;
    service.stage(&prepared.instance).unwrap();
    service.action("prepare", id).await.unwrap();
    service.action("models", id).await.unwrap();
    let local = catalog.local.as_mut().unwrap();
    let moved = dir.path().join("moved-repo");
    fs::rename(&local.workspace, &moved).unwrap();
    local.workspace = moved.clone();
    write_json(&service.staging, "deployment.json", &catalog).unwrap();
    let reopened = UserService::installed(&service.staging).unwrap();
    let before = calls(dir.path());
    let result = reopened.action("cleanup", id).await;
    eprintln!("cleanup={result:?}; active={}; retained_instance={}; manager_calls_unchanged={}; repository_preserved={}",
        dir.path().join("active").exists(), service.dir(id).join("instance").exists(),
        calls(dir.path()) == before, moved.join("user-work").is_file());
    assert!(result.is_ok(), "cleanup must not require the former workspace");
}

#[tokio::test]
async fn review_start_failure_retry_and_terminal_observation() {
    let (dir, service, prepared, _catalog) = fixture();
    let id = &prepared.instance.unit_id;
    service.stage(&prepared.instance).unwrap();
    service.action("prepare", id).await.unwrap();
    let manager = dir.path().join("manager");
    let source = fs::read_to_string(&manager).unwrap();
    fs::write(&manager, source.replace("elif verb=='start': (root/'active').touch()", "elif verb=='start':\n    (root/'active').touch()\n    if (root/'fail-start').exists(): sys.exit(1)")).unwrap();
    fs::write(dir.path().join("fail-start"), "").unwrap();
    fs::write(dir.path().join("fail-stop"), "").unwrap();
    assert!(service.action("models", id).await.is_err());
    let failed: Execution = serde_json::from_slice(&private_file(&service.dir(id).join("execution"),4096).unwrap()).unwrap();
    assert!(dir.path().join("active").exists());
    fs::remove_file(dir.path().join("fail-stop")).unwrap();
    service.request("cancel", id, Some(&failed.nonce)).await.unwrap();
    assert!(!dir.path().join("active").exists());
    fs::remove_file(dir.path().join("fail-start")).unwrap();
    let next: Execution = serde_json::from_str(&service.action("models", id).await.unwrap()).unwrap();
    assert_ne!(next.nonce, failed.nonce);
    let mut result = ActionResult {
        execution: next.clone(), finished: true,
        observed_at: nostr::Timestamp::now().as_secs(),
        login: buzz_core::host_deployment::observation::LoginState::NotObserved,
        login_at: None, models: buzz_core::host_deployment::observation::Models::Empty,
        login_output: String::new(),
    };
    write_json(&service.dir(id), &next.filename(), &result).unwrap();
    assert_eq!(service.request("observe",id,Some(&next.nonce)).await.unwrap(),"pending\n");
    fs::remove_file(dir.path().join("active")).unwrap();
    let output: ActionResult = serde_json::from_str(&service.request("observe",id,Some(&next.nonce)).await.unwrap()).unwrap();
    assert_eq!(output.execution,next);
    result.execution.nonce = failed.nonce;
    write_json(&service.dir(id), &next.filename(), &result).unwrap();
    assert!(service.request("observe",id,Some(&next.nonce)).await.is_err());
    service.request("cancel",id,Some(&next.nonce)).await.unwrap();
}
```

Measured cleanup receipt:

```text
cleanup=Err(No such file or directory (os error 2)); active=true; retained_instance=true; manager_calls_unchanged=true; repository_preserved=true
review_cleanup_after_workspace_move ... FAILED
review_start_failure_retry_and_terminal_observation ... ok
1 passed; 1 failed; 24 filtered out
```

## Return and re-gate contract

The conductor ruled M-1 “fix in same #3721 lane” during this review and accepted the realistic cleanup failure. No board mutation was performed by this reviewer. The report itself carries the single-fix handoff: executor repairs M-1, preserves S1's safety boundaries, commits code/regression, and returns this same checkout for a re-gate limited to M-1 and defects introduced by its fix. S2/S3, native installation/login, existing-agent adoption and registration redesign stay excluded.

Review changed documentation only after restoring temporary tests. All reviewer test processes completed; targeted process inspection at return found no remaining review fixture processes. Exclusive write/test ownership is released with the report commit. Source-only acceptance and native rollout remain separate.

quality: model=unknown effort=unknown stage=post-audit verdict=rework verify=ran commits=b8a9ccd97e62465d804bbc85125ce858a873fe8d..e0cc1d529af62bac6689de60e52e21bd9e544e80 findings=C=0/H=0/M=1/L=0 rework_rounds=0
