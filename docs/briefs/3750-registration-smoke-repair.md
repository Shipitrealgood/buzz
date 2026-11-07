# Review scope: registration smoke repair (#3750 M-1)

**Date:** 2026-11-07. **Requester:** conductor. **Next agent:** same independent Astra/high reviewer.

## 1. Where the work lives

Worktree: `/home/bryce/services/buzz-wt-3720-host-registration`.
Branch: `proof/3720-host-registration-events`, not trunk.
Clean repair base: `ccf89c9cabcac8370f94cc9c373e44a06bc95a3c`.
Test commit: `d97823a753b4f14a1634ba2a136ab9e274babd86`; one following docs-only receipt/brief commit completes handoff.

```sh
cd /home/bryce/services/buzz-wt-3720-host-registration
git diff ccf89c9cabcac8370f94cc9c373e44a06bc95a3c..HEAD -- desktop/tests/e2e/paired-hosts.spec.ts
```

## 2. What landed

The existing smoke spec and its local mock now exercise the approved registration-only UX, including honest unsupported/unavailable states and duplicate-label identity routing. Its six cases pass against the unchanged app build. The old discriminating case was measured failing at the removed `Check host` selector before the repair. No production or shared-helper changes were made.

Source: [M-1 addendum](../audits/post-audit/3750-registration-integration-smoke-addendum.md), accepted [milestone](../specs/windows-netcup-registration-milestone.md), [finite checklist](../plans/3750-acceptance-checklist.md). Previous [implementation brief](3750-registration-integration-execution.md) remains the broader context.

## 3. Your job

Re-gate M-1 against the approved UX and inspect whether the mock conceals incorrect routing or forbidden commands. Preserve the distinction between browser mock evidence and actual native/platform evidence.

**Special instruction:** these assertions intentionally replace obsolete expectations. Judge them against the registration contract rather than treating current markup alone as authority. The local fixture accepts only four paired-host IPC commands and checks owner/relay on every call, but it cannot prove native signer or relay behavior.

## 4. Specifically look for

1. **Exact identity routing** — `desktop/tests/e2e/paired-hosts.spec.ts:167`.
   **Probe:** both hosts are named Workshop; refresh and revoke must select host `22` / generation `32`, include active owner/relay, and leave the first row Registered. Cancel must emit no unpair IPC.
2. **Empty directory and secret entry** — same file, line 221.
   **Probe:** Add Host starts from a supported empty list; cancel clears without IPC, expired input clears and permits retry, success shows pending before fixture acknowledgement. Password input and recorded synthetic invitations are deliberate.
3. **Honest supported/available states** — same file, line 258.
   **Probe:** unsupported keeps the explanatory panel; failed/unavailable lists withhold Add Host until successful Retry. Neither path should imply an empty supported directory before recovery.
4. **No provisioning/capability claims** — same file, lines 56 and 136.
   **Probe:** seeded legacy freshness/runtime/capacity fields must stay invisible. The helper rejects forbidden visible controls, online claims and non-registration paired-host IPC; no success mock for provisioning/history remains.
5. **Platform boundary and unchanged registration** — same file, line 302, and `desktop/playwright.config.ts:25`.
   **Probe:** Linux hides the section and emits no paired-host IPC. The spec remains explicitly in the smoke project and has no skips. Platform simulation is browser-only evidence.
6. **Receipt scope and continuity** — [repair receipts](../verification/3750/smoke-repair/README.md).
   **Probe:** the 6,580-entry Git object comparison excludes only docs and this test. Supplemental test-inclusive TypeScript failure was reproduced on baseline with identical helper diagnostics; normal app tsc/build and the six runtime cases passed. No test-level diagnostic was suppressed.

## 5. Spec deviations

No product/spec deviations. All mock changes stay within this test file. A supplemental typecheck was added as a one-off ignored config because the normal app tsconfig excludes tests; it exposes pre-existing shared-helper errors and is explicitly not a green gate. Its temporary baseline test file is removed. No helper/dependency changes were absorbed.

## 6. What NOT to audit / chase

This continuation is the single M-1 test repair plus evidence. The accepted native/registry/hostd implementation and prior foundational decisions stay at their reviewed production content. Shared bridge type cleanup is adjacent, outside this repair. Existing jsdom tests and broader suites were not rerun because shared mocks and product bindings are unchanged.

No deployment, board mutation, workflow trigger/cancellation, public listener, real account secret or agent lifecycle action occurred. Existing [Windows journey](../verification/3750/windows-journey.md) and [WSL handoff](../verification/3750/wsl-fixture-handoff.md) remain the coordinated operator instructions; no new interactive fixture framework is needed.

## 7. Deliverables and pre-flight

Return the M-1 verdict and independently evaluated platform receipts to the conductor. Full #3750 acceptance still requires the native Windows human journey and remaining artifact checks; a browser mock pass does not satisfy that gate.

- Verify clean worktree and signed-off repair/docs commits after the supplied base.
- Read the M-1 addendum and this brief before expanding scope.
- Inspect [red](../verification/3750/smoke-repair/red.txt), [green](../verification/3750/smoke-repair/green.txt), [continuity](../verification/3750/smoke-repair/production-continuity.json) and [cleanup](../verification/3750/smoke-repair/cleanup.json) receipts.
- Platform status in the receipt index is attributed to the conductor, not this worker's certification.

Requested executor configuration: **gpt-6-astra/high**; independently observable runtime identity: **unknown**. Same-task repair only; no subagents.
