# #3725 confirmed hosted Cleanup — independent review

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
