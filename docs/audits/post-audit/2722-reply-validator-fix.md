# 2722 — reply validator fix post-audit

Date: 2026-11-12. **Verdict: merge-safe for the dispatched source correction.** No supported findings: C=0/H=0/M=0/L=0; no merge blockers or prerequisite gap. Native Windows and live validation remain pending; this is not a release-readiness or live-fix claim. Initial review, rework rounds: 0.

## Scope and provenance

Reviewed `/home/bryce/services/buzz-wt-3720-host-registration`, branch `proof/3720-host-registration-events`, range `b910a0e03150afa968d14ea93875ad6bc97e7cb1..af9994e8560e5229705472a1218c6d9b87e75040`. Initial worktree was clean at that head. Exclusive verification/report ownership followed executor release. Only this report was written; no implementation, live services, test1/Jeff, provider, installation, CI, push, board, or deployment actions.

Inputs: [review dispatch brief](../../briefs/2722-reply-validator-review.md), [execution receipt](../../executions/2722-reply-validator-fix.md), and approved audit `/home/bryce/services/buzz-agents/docs/audits/pre-audit/2722-prepare-refresh-read.md`. Read portable conductor/post-audit guidance, project AGENTS, relevant vision, architecture and testing guidance. Current bounded dispatch controls acceptance; broad CI/native/live checks are not this source gate. No architecture, clock-source unification, status-admission sibling, stale-display, eager-read, login, installer, or new-app work was reopened.

Requested reviewer configuration: `gpt-6-astra`, high. Actual served model/reasoning identity is not exposed; runtime identity unknown. Evidence below distinguishes **read**, **measured**, and **accepted executor receipt**.

## Acceptance review

| Obligation | Evidence and conclusion |
|---|---|
| Authenticated issuance bounds lifetime | **Read:** [wire.rs:487](../../../crates/buzz-core/src/host_deployment/wire.rs#L487) calls `parse` before using creation time; [parse at line 109](../../../crates/buzz-core/src/host_deployment/wire.rs#L109) verifies the event. Checked signed creation +10 now bounds expiry. `now >= expires_at` still rejects; the inclusive +30 future-event bound remains, with overflow rejected. No deadline is reset or extended. |
| Real signed boundary coverage | **Read + measured:** [wire_tests.rs:87](../../../crates/buzz-core/src/host_deployment/wire_tests.rs#L87) signs/encrypts and calls actual `open_reply` for status, operation and permit. It covers exact issuance, reader one second behind, exclusive expiry, overlong lifetime, late signing, DB ahead of signer, exact/far future, both addition overflows, wrong nonce/recipient/signer and altered signature. Positive-time rows make the binding negatives meaningful. No duplicate predicate is tested. |
| Existing effect authority remains | **Read:** the sole deployment transport call to `open_reply` is [WS line 36](../../../crates/buzz-ws-client/src/host_deployment.rs#L36), used by owner query and host query. Host [deployment_permit at line 147](../../../crates/buzz-hostd/src/deployment.rs#L147) still checks exact authority/unit/revision, active grant, cleanup fence and absolute expiry at line 177, then records nonce in its transaction. Prepare/start callers remain at lines 484–494. DB expiry issuance and relay signing are unchanged. DB-ahead-signer rejection is intentionally retained; this does not solve distributed clock disagreement. |
| Fixed categories contain no payload | **Read + measured:** [WS lines 16–48](../../../crates/buzz-ws-client/src/host_deployment.rs#L16) distinguish invalid, duplicate, missing and CLOSED using literals. CLOSED reason is ignored. [Owner helper at line 17](../../../crates/buzz-paired-owner/src/deployment.rs#L17) downcasts `anyhow` and returns only static allowlisted categories, typed timeout/closed categories, or a generic stage. Arbitrary text cannot be returned. Existing wire formats, transport API and error variant remain unchanged; the helper is an additive documented API. |
| Actual failure boundary and precedence | **Read + measured:** [owner journey line 486](../../../crates/buzz-paired-owner/src/deployment_tests.rs#L486) drives actual `Management::operation` → query → WS through the four failure modes and checks CLOSED sentinel exclusion. Local/transport/arbitrary-message errors with sentinel context exercise the actual projector at line 504. **Static only:** [Desktop setup lines 131–136](../../../desktop/src-tauri/src/commands/agents_hosted_commands.rs#L131) formats only the helper category and uses `get_or_insert_with`, preserving prior error precedence. This is not a Tauri/native UI run or a blanket sanitization claim about unchanged error paths. |
| Minimal scope and reuse | **Measured diff:** four production files, +37/−11 (net +26), two existing test files +115/−37, two executor documents +84/−0. The 17-line helper exposes the exact testable projection used by Desktop without a parallel error hierarchy or framework. Main creation, discovery/runtime, authorization/idempotency and API result shapes are untouched. Existing owner journey continues through refresh, repeat reads, exact retries, expired historical observations and cleanup. |

Minimalism/elegance/correctness: 9/10 each for this bounded correction. The production-seam signed matrix and existing socket journey are appropriate foundations; no additional abstraction or verification campaign is warranted by the inspected change.

## Independent verification

All Cargo runs used the existing cache, `HERMIT_SELF_UPDATE=false`, `. ./bin/activate-hermit`, `CARGO_BUILD_JOBS=2`, `CARGO_NET_OFFLINE=true`, `--offline --locked`. No dependencies installed.

| Command/check | Reviewer-measured outcome |
|---|---|
| `cargo test --offline --locked -p buzz-core host_deployment --lib` | Exit 0; 13 passed, 0 failed, 311 filtered. Includes signed reply matrix and historical observation regressions. |
| `cargo test --offline --locked -p buzz-paired-owner -p buzz-ws-client --lib` | Initial sandbox: 13 owner passed, 3 failed solely at localhost bind with EPERM; WS not reached. Narrow approved loopback rerun: exit 0; 16 owner +5 WS passed, none failed/ignored. Deployment journey, including new categories, completed. |
| Scoped Cargo fmt plus `rustfmt --check --edition 2021 desktop/src-tauri/src/commands/agents_hosted_commands.rs` | Passed, no formatting changes. |
| `git diff --check` on the pinned range; final source continuity | Passed. All eight implementation-commit files compared byte-for-byte equal to pinned HEAD after verification; worktree clean before report creation. |

**Accepted executor mutation evidence, not independently rerun:** read `/tmp/2722-reply-mutation.log`: restoring the old reader-relative upper comparison fails the real signed test at `status: signed=110, expiry=120, reader=109` (expected true, actual false). Read full-core restoration log: 324 passed. This receipt is supported by the unchanged final source/test and the independently green 13-test unmutated baseline above. It is not counted as a fresh reviewer mutation kill. No new mutants were required by dispatch.

Continuity SHA-256 values, measured after independent tests:

| Artifact | SHA-256 |
|---|---|
| `crates/buzz-core/src/host_deployment/wire.rs` | `5ab2f6ec16e4016e98e27d38a82b86f4ba4d007d8df01e388dc8bbce151dbaa9` |
| `crates/buzz-core/src/host_deployment/wire_tests.rs` | `59b40fd51a7243c75f3367447f2e0548d84e76dccdf07c97434d54b9fcf87472` |
| Executor mutation log | `d70a9a5f2c99f82d944bf7f15a349cff36d0c89e90c2eb01187ab6acc622328b` |
| Executor full-core restoration log | `306b11125bed7cf9bc7a3b03a4168cfe0a61fe1917ddea0008b30340b3194e01` |

**Not run:** full repository CI, additional full-core repetition, database/hostd/service suites, native Desktop/Windows, live validation, strict lint repetition. The [executor receipt](../../executions/2722-reply-validator-fix.md#verification) reports strict clippy failure from three pre-existing core warnings, seven total advisory warnings, and unavailable Linux GLib/WebKit prerequisites. Those checks are not represented as green; dispatch explicitly excludes unrelated remediation. Timeout/connection-closed category mapping uses typed errors, not an independently timed deployment-read failure. The five WS tests are existing transport regressions; the owner journey supplies the new deployment-branch coverage.

## Routing and handoff

Recommend conductor accept this source unit without rework and retain the separately authorized native Windows/live gate. No claim is made about the exact live failure's cause or current client/DB/signer clocks. No new task or prerequisite is proposed. All reviewer subprocesses are terminal; no source mutations or background work remain. The report is the only commit payload, with DCO signoff required; measured pre-commit and commit-msg hooks are absent, so no hook pass is claimed.

quality: model=unknown effort=unknown stage=post-audit verdict=merge-safe verify=ran commits=b910a0e03150afa968d14ea93875ad6bc97e7cb1..af9994e8560e5229705472a1218c6d9b87e75040 findings=C=0/H=0/M=0/L=0 rework_rounds=0
