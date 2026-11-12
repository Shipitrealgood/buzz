# Independent review: reply validator correction (#2722)

## Where the work lives

Workspace `/home/bryce/services/buzz-wt-3720-host-registration`, branch `proof/3720-host-registration-events` (not main). Review the one implementation commit from base `b910a0e03150afa968d14ea93875ad6bc97e7cb1`; the conductor supplies its final SHA. Execute `git diff b910a0e03150afa968d14ea93875ad6bc97e7cb1..HEAD` in that workspace. Ownership transfers after the executor commit; all test processes are terminal.

## What landed

Reply maximum lifetime now uses authenticated event creation time with checked arithmetic; absolute expiry remains exclusive. Existing WS failure branches have four fixed categories. A 17-line owner helper projects only static categories at the desktop setup Err boundary. Tests extend existing signed-reply and owner socket journeys. [Execution receipt](../executions/2722-reply-validator-fix.md) records exact gates, counts and limitations.

## Your job

Independently assess this narrow correction and payload-free error reporting. The accepted audit is `/home/bryce/services/buzz-agents/docs/audits/pre-audit/2722-prepare-refresh-read.md`; the task is not proof of the live failure's cause. Strict lint is not green; native Windows has not been validated.

## Specifically look for

1. Signed issuance versus current validity: `crates/buzz-core/src/host_deployment/wire.rs:487`.
   Probe: parse verifies signature before trusting created_at; exclusive expiry and 30-second future bound still reject; either checked addition overflow returns Error.
2. Signed test table: `crates/buzz-core/src/host_deployment/wire_tests.rs:87`.
   Probe: three views cover reader109/110/120, overlong121, late signer111, DB-ahead-signer109, exact future boundary and overflow. The old predicate was falsified at the one-second-ahead case.
3. WS category origin: `crates/buzz-ws-client/src/host_deployment.rs:16`.
   Probe: invalid, duplicate, missing-at-EOSE and CLOSED are distinguishable without forwarding CLOSED text. Existing API/error variant unchanged.
4. Safe projection: `crates/buzz-paired-owner/src/deployment.rs:17`; `desktop/src-tauri/src/commands/agents_hosted_commands.rs:131`.
   Probe: only a static allowlist survives anyhow downcast; arbitrary strings/context/transport detail and local failures produce generic stage. Prior response.error precedence remains unchanged. Desktop evidence is static only.
5. Real journey coverage: `crates/buzz-paired-owner/src/deployment_tests.rs:486` and CLOSED fixture at line396.
   Probe: actual Management/WS path produces each category; SECRET_SENTINEL is absent from CLOSED error and arbitrary error projection. Following normal repeated reads, historical expired observations and exact retry still pass.

## Deviations / decisions

No product/protocol deviation. Owner helper added solely to make the actual anyhow-to-static projection reusable and testable without Tauri; no new error hierarchy, dependencies or API return signatures. The existing future-event bound also uses checked addition so overflow cannot panic/wrap. Native checks are pending due missing GLib/WebKit; strict lint reports existing unrelated warnings, preserved.

## What not to chase

Keep DB/signer clock-source unification, status-admission clock sibling, stale-model display, eager-read removal, login, installer, new app port and shared runtime behavior outside this review. No expiry extension or weakened signatures/revocation/idempotency. No live calls, test1/Jeff, package installation, provider changes, push/release or board edits are authorized by this brief.

## Deliverable and pre-flight

Return findings and gate evidence to conductor; executor evidence is not an independent gate. Verify the supplied final SHA and clean worktree, read the accepted audit and execution receipt, and retain the existing workspace/cache. For tests activate `HERMIT_SELF_UPDATE=false; . ./bin/activate-hermit`, use `CARGO_BUILD_JOBS=2 CARGO_NET_OFFLINE=true` and Cargo `--offline --locked`. The owner/WS tests need loopback socket permission; the sandbox EPERM was resolved via approved escalation. No dependency changes are needed. Native Windows and later live validation remain pending, not implied by source-test success.
