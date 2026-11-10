# #3725 confirmed hosted Cleanup — independent review

## Current handoff: Windows failure and exact-wire repair

Same reviewer: assess source delta
`8cec4d7cf5742b7d4fbcb3f093dd2242f6865d21..d093d6dcd556a6ef6017cf8a2a70c109c14627e8`,
then the terminal Windows receipts. H-1/H-2 source repairs were accepted in round 1;
this is a concrete production defect exposed by their required native gate.
**No third Windows run is authorized to proceed until your re-gate/release.**

- `8279e0dbf`: first native run failed on an incorrect admission-error expectation.
  Fixture now asserts the exact competing-admission refusal its unadmitted
  follow-on Deploy actually represents. Second native run passed that assertion.
- `d093d6dcd`: second native run failed later with `unavailable` instead of pending.
  Investigation proved production `send_event` converted typed Event into Value,
  reordering bytes that owner journals and relay reconciliation compare exactly.
  Existing sender now serializes the typed tuple directly. Fake peer preserves
  incoming RawValue, as the real relay does. Production delta is **+5/-1**, no API
  or protocol change; tests **+11/-7**. Guards and main finalizer remain unchanged.

**Probe the actual bytes, not only semantic JSON equality.**
`crates/buzz-ws-client/src/connection.rs:179` retains its existing event-ID and OK
wait. `crates/buzz-paired-owner/src/deployment_tests.rs:372` captures raw event
elements and `:466` compares three sends/retries with the original retained bytes.
This assertion was measured RED before repair, GREEN afterward. Cross-check typed
journal serialization at `deployment.rs:168` and `deployment/cleanup.rs:131` with
real relay `handlers/host_deployment.rs:96`/store `paired_hosts/deployment.rs:140`;
the relay preserves wire bytes. Do not canonicalize the response or weaken the
reconciliation guard to hide a transport mismatch.

Native fake peer `agents_hosted_cleanup_tests.rs:43` now preserves RawValue rather
than rebuilding Value. Finite same-assumption check found other response event
fields use retained strings and result fields use typed result serialization.
Later state assertions now print attached errors. No wider fixture redesign.

[Execution receipt](../executions/3725-confirmed-hosted-cleanup.md#second-native-failure-actual-sender-serialization-defect)
preserves both failures, diagnostic source/claims, artifacts and exact commands.
Affected local gates passed: ws-client **5**, owner **2**, format/whitespace/size.
Both native runs compiled application/tests, passed **109 outer cases + ACL**, then
failed Cleanup before the later completion/race assertions; metadata exact case
was skipped by fail-fast. Neither run proves successful Cleanup. The new sender
and RawValue peer have **not run on Windows**. Installer success is not native-test
acceptance. No full Tauri IPC, real keyring, systemd/provider or human-flow coverage
is claimed. No live changes or installer execution occurred.

After source acceptance, the original executor may resume the already approved
exact fork/workflow for native verification. Continue the same sequential
worktree ownership; retain first/second failed-run receipts in the final evidence.

Second run is terminal: build succeeded (20m7s), native failed (12m21s). Both
installer downloads were hash/source/version verified and never executed. Latest
download is test.23 from **8279e0dbf**, before the sender repair; it must not be
presented as current passing Cleanup source. All owned processes stopped and
exclusive source/test ownership is released with this documentation commit.

**Shared hostd provenance:** the corrected sender is also used by hostd at
`registration.rs:448` and `deployment.rs:354,434,532`. Conductor identifies prepared
netcup package `c97256e` as predating this correction. Later rollout preparation
must reconcile/refresh its provenance against final accepted source. Assess the
introduced serialization compatibility effect, but do not rebuild/install or
expand this review into deployment work. No current-source hostd artifact claim
is made here.

## Prior handoff: H-1/H-2 re-gate

The same reviewer should re-gate the two findings in the unchanged
[round-0 report](../audits/post-audit/3725-confirmed-hosted-cleanup.md), plus defects
introduced by these repairs. Repair range is
`6d7ca47d2d00560a7c6a221e475a52c107c0aab1..faf59a0b3421c5410f807428d768a2e353e32396`:
H-1 `356b1b7b3` and H-2 `faf59a0b3`, followed by documentation only. Same worktree
and branch below; all executor processes stopped and ownership released.

1. **H-1 current predecessor versus retained replacement.**
   `crates/buzz-paired-owner/src/deployment/cleanup.rs:184` reads every candidate;
   `:225` only tolerates missing previously admitted Cleanup when a later proven
   revision supersedes it. Probe the exact Failed A → unadmitted B → reopen path
   in `cleanup_tests.rs:238`, both Check/Retry reuse and eventual completion.
   Expired B still needs fresh current A proof before explicit renewal. Owner
   regression was measured RED before the production change and GREEN afterward:
   2 deployment cases passed, 14 filtered. No new protocol or expanded matrix.
2. **H-2 stable local commit.** `paired_hosts.rs:90` acquires workspace then
   identity-mutation guards and validates the captured session without recursively
   acquiring the workspace lock. `agents_hosted_cleanup.rs:199` then takes the
   store lock and retains all guards through phase 4. Probe lock order against
   workspace apply and identity import/recovery; no synchronous guard may cross
   network await. `agents_deletion.rs:37` receives captured scope/keys directly;
   it cannot reread the active global retention destination. Ordinary deletion
   still selects the existing best-effort branch via `None`.
3. **Native fixture limitation and introduced defects.**
   `agents_hosted_cleanup_tests.rs:258` extends the existing exact Windows case
   with separate contention refusals and concurrent workspace/identity writers at
   the finalizer boundary. Check channel/thread lifetimes, completion ordering,
   original-only retention and stale-session assertions. The identity writer
   invokes `commit_imported_identity` with synthetic persistence. **This fixture
   remains uncompiled/unrun**; formatting cannot certify Rust types or runtime
   correctness. Existing archive failure/retry coverage remains intact. No real
   keyring or full Tauri dispatch claim is supplied.

Repair fmt, TypeScript, differential size and whitespace gates passed. UI/API and
browser fixture source is unchanged from `6038d0981`, so round-0's independently
measured 32 Node / 4 mock-IPC browser results carry forward only for that unchanged
surface. Exact commands and delta (+112/-45 production, +308/-0 tests) are in the
[repair receipt](../executions/3725-confirmed-hosted-cleanup.md#h-1h-2-repair-handoff--2026-11-10).
No rerun of known-blocked Linux native compilation, Windows push or workflow
dispatch occurred. Human export permission arrived during final handoff: the
conductor relayed “are you waiting on me for that approval? if so it is approved”
in response to the exact fork/branch, Windows tests/build and download question.
The executor must wait for this re-gate and ownership release before resuming the
approved Windows verification. Source re-gating does not substitute for that
required native gate or authorize live work.

The remainder is original implementation context. Its broad review checklist was
assessed in round 0; reopen only on new evidence relevant to the bounded re-gate.

## Where the work lives

Worktree `/home/bryce/services/buzz-wt-3720-host-registration`; branch
`proof/3720-host-registration-events`. Review implementation range
`ee3bbc8d5f77f45634040e41dafa3e6dfb7b159a..6038d0981` (two DCO commits), plus final
handoff documentation. `git diff ee3bbc8d5 6038d0981` isolates this slice from the
already reviewed registration/creation/main-discovery work. Product baseline is
`392c458e8`; preceding spec commit `ee3bbc8d5` was clean at assignment.

## What landed

[The ratified specification](../specs/3725-confirmed-hosted-cleanup.md) now has an
owner Cleanup continuation, permanent local Deploy fence, native shared deletion
coordinator, and setup Delete/recovery UI. Ordinary Delete waits for confirmed
host cleanup before calling the same main finalizer. Pending and failed local
removal retain the protected intent; completed Cleanup also remains as a fence.
No host, relay, schema or deployment feature flag changed.

[Execution receipt](../executions/3725-confirmed-hosted-cleanup.md) records exact
commands, failures and scope adjustments. Owner 2/2 and Node 32 named cases pass;
browser 4/4 is mock-IPC component evidence. Typecheck, test build, scoped format/
Biome, size and whitespace checks pass. **Native source has not compiled here:**
Linux GLib/GIO blocked it, and automatic approval review rejected canary export.

## Your job

Run the independent post-audit on the pinned implementation. Doubt confirmation,
retry and no-resurrection claims at their production boundaries. This executor's
checks are evidence, not an independent gate. Source review can proceed now;
Windows execution must remain a separate required unresolved gate.

**Special instruction:** the native new case is authored but unexecuted. In
particular, do not promote Node IPC mocks into proof of force-flag enforcement or
a real keyring deletion failure. Inspect the native coordinator/finalizer and the
new fixture for compile and runtime defects before grading the packet.

## Specifically look for

1. **One retained operation under races.**
   `crates/buzz-paired-owner/src/deployment/cleanup.rs:63` consumes a privately
   constructed plan under SQLite IMMEDIATE; `:159` obtains fresh operation reads.
   **Probe:** two independently reconciled plans must return the retained winner's
   final sealed ID/event; an intervening unknown Deploy must prevent retention.
   Check the `known`/`reuse` handling for same-revision expiry renewal versus a
   concurrent winner. Cleanup is retained directly at phase 2 because it has no
   creation/publication effects.

2. **Permanent fence across all old Deploy work.**
   `deployment.rs:110`, `:225`, `:425`, `:439`,
   `deployment/continuation.rs:99`, and native `agents_hosted.rs:18` are the guards.
   **Probe:** expiry, failed Cleanup and phase 4 must never reopen old create,
   continuation, recovery or dispatch. Native retention holds the same store lock
   as recovery mutations. Account for already-sent work and relay revision CAS;
   there is no new cancellation protocol.

3. **Confirmation is current, exact and conjunctive.**
   `deployment/cleanup.rs:280` uses the existing operation read/result validation
   and compares stored unit and expected/applied revision. Native
   `agents_hosted_cleanup.rs:135` alone branches into local finalization.
   **Probe:** a host receipt without current read permission, wrong unit/revision,
   mismatched event/result or missing Cleaned evidence must preserve recovery.
   Read the original `continuation::reconcile` and `wire::open_result_body` rather
   than assuming the new boolean supplies their checks. Unknown initial admission
   is refused; explicit renewal requires a proven predecessor/current failure.

4. **Local deletion retries survive an absent record.**
   `agents_hosted_cleanup.rs:181` rechecks session and exact captured record;
   `agents_deletion.rs:22` shares main assignment/save/key/tombstone finalization.
   `agents_pending.rs:67` propagates errors; `:125` preserves an already-enqueued
   archive's persona payload on retry. **Probe:** failure after record removal must
   leave phase 3, remain listed, never rematerialize, and eventually finish through
   the same helper. Ordinary local/provider callers retain best-effort behavior.
   The authored native fixture injects an archive INSERT failure; keyring failure
   is not independently executed by this fixture.

5. **Both entry points, including the force flag.**
   `agents.rs:1205` routes hosted Delete before the ordinary force/provider branch.
   `agents_hosted_commands.rs:318` sends Cleanup and later Check/Retry to the same
   coordinator before catalog/runtime checks. `agents_hosted_cleanup.rs:221`
   derives authority from native saved state/intents. **Probe:** stale renderer
   scope, absent record and identity/community switch must not cause ordinary
   successful deletion. Default-off feature gating must still hold.

6. **Recovery remains visible and honest.**
   `agents_hosted_commands.rs:240` lists the newest Cleanup independently of a
   saved record; completed rows disappear only from active choices.
   `HostedAgentSetup.tsx:32`, `:94`, `:180` hide model/Start/Refresh after Cleanup,
   show pending/local-failure/completion and keep explicit recovery.
   **Probe:** normal pending Delete must reject before roster removal; setup Close
   must retain intent. Review existing generation/latch behavior on deferred results.
   Node cases and the real component/browser fixture exercise these boundaries,
   but their IPC is mocked.

## Spec deviations and decisions

- Conductor approved `agents_pending.rs` reuse of fallible/idempotent enqueue;
  `storage::try_delete_agent_key` already existed, so storage needed no change.
  A shared finalizer sibling avoids a parallel hosted deletion implementation.
- The freshness-to-retention split (`CleanupPlan`) is narrow: it lets native hold
  its synchronous store lock only around the atomic journal mutation, never a
  network await. Judge the read/write recheck rather than assuming the split is
  automatically safe.
- Existing browser fixtures were stale against accepted baseline labels/readiness.
  They now assert Start/Refresh and actual readiness fields; no production guard
  was weakened. One initial 3/4 run failed only on old “Needs login” copy.
- Windows gate was newly authorized by conductor but automatic approval review
  rejected exporting source to the fork without direct human authorization.
  No retry, push or workflow dispatch occurred. The conductor owns that question.

## What NOT to audit / chase

This is the confirmed Cleanup slice, not full #3724/#3725 closure. Registration,
normal trusted host account/HOME/PATH/login/network and accepted creation identity
remain inputs. Relay capability/activation is separately held by #3721; inspector
ops work is outside this checkout. Start/Stop/Restart/reboot, legacy #3767 cleanup,
installer/adoption/root helper/auth broker, provider-v1 lifecycle and live-agent
changes remain deferred. Full CI and human pilot acceptance are not supplied here.

## Deliverables and pre-flight

Return severity-ranked concrete findings to conductor, with a source-pinned
post-audit artifact. Do not merge/push/deploy or mutate taskctl. Recommended source
review is finite G1–G4, including explicit unrun native boundaries; execution
resumes in this same worktree only after reviewer releases ownership.

- Verify clean branch/pin and read spec, prior
  [execute brief](3725-confirmed-hosted-cleanup.md), and execution receipt.
- Owner tests use activated Hermit and `--offline --locked`; the existing socket
  fixture needs loopback permission. No dependency/tool installation.
- Node named-case evidence uses `--test-isolation=none`; ordinary sandbox runner
  yielded only a file-level pass. Browser evidence is explicitly browser-only.
- Preserve source and all original probes. No owned processes remain at handoff.

Requested executor configuration Astra/high; actual served identity not
independently exposed. The two implementation commits add 906/remove 122 production
lines and add 826/remove 71 test lines, including 16 production/47 test lines moved
between scoped files; workflow adds one exact-case line. Details are in receipt.
