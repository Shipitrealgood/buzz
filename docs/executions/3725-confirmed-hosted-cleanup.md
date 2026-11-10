# #3725 confirmed hosted Cleanup execution

Status: H-1/H-2 source repairs accepted; authorized Windows verification active.
Executor releases exclusive write/test ownership with the final documentation commit in
`/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`. Starting clean pin
`ee3bbc8d5f77f45634040e41dafa3e6dfb7b159a`.

Contract: [ratified spec](../specs/3725-confirmed-hosted-cleanup.md), F1–F3/G1–G4.
Requested Astra/high; served identity is not independently exposed.

Conductor approved narrow use of existing fallible key/tombstone operations;
strict hosted finalization keeps phase 3 on failure. No storage API addition was
needed: `try_delete_agent_key` is already crate-visible. Ordinary local/provider
best-effort deletion remains unchanged. Owner-only new Cleanup sibling and native
shared deletion/cleanup siblings keep responsibilities and file size bounded.

Measured early gate: `HERMIT_SELF_UPDATE=false`, activated Hermit,
`cargo test --offline --locked -p buzz-paired-owner deployment -- --test-threads=1`
PASS 2/2 including new Cleanup fixture extension. Initial sandbox run failed at
loopback bind; identical authorized socket-escalated command passed. This is an
intermediate source result, not final acceptance.

Conductor subsequently authorized existing Windows canary verification and the
narrow canary branch fast-forward/dispatch, after coherent local checks. Source
must be committed first; no main/tag/release/live mutation. Native Windows,
browser, final local gates and independent Chuck review remain pending.


## Local coherent implementation checkpoint

Owner commit `bf60e71bb`; native/UI source commit follows this checkpoint.
Measured final owner command above: **2 passed, 0 failed**, 14 filtered; existing
socket fixture includes four admitted-state Cleanup scenarios plus expiry/failed
renewal, duplicate transaction/restart and no-resurrection checks. These are
subcases of the two existing Rust cases, not extra named test counts.

- `pnpm --dir desktop typecheck`: PASS. `pnpm --dir desktop test:e2e:smoke -- hosted-agent-setup.spec.ts`
  built successfully but local server bind was sandbox-denied. Reused that exact
  built source with authorized `pnpm --dir desktop exec playwright test --project=smoke hosted-agent-setup.spec.ts`:
  **4 passed (25.6s)** after fixing one stale existing fixture label. Browser IPC
  is mocked; this is no native/relay execution claim.
- `cd desktop; node --import ./test-loader.mjs --experimental-strip-types --test --test-isolation=none --test-reporter=tap src/features/agents/lib/managedAgentDeletionAvailability.test.mjs`:
  **32 named cases passed**, zero failed/skipped/cancelled, including six hosted
  Agents/profile pending/denied/confirmed cases. The ordinary sandbox child-runner
  invocation reported only a file-level pass; that is not counted as case proof.
- Scoped Biome check: four changed UI/API/test files PASS. Owner cargo fmt and
  scoped native rustfmt (`--edition 2021 --config skip_children=true --check`)
  PASS; `git diff --check` PASS. Differential Desktop size check with
  `CHECK_FILE_SIZES_BASE=ee3bbc8d5f77f45634040e41dafa3e6dfb7b159a` PASS after
  allowing its read-only Git subprocess (initial sandbox spawnSync EPERM).
- Native Linux `cargo test --offline --locked --manifest-path desktop/src-tauri/Cargo.toml --lib commands::agents::hosted::tests:: --no-run`:
  BLOCKED before application compilation by missing GLib/GObject/GIO pkg-config
  dependencies. No installation attempted. Windows supported-platform gate follows.

Fixture alignment is test-only: accepted baseline already says Start/Refresh,
Host readiness and model setup, and checks readiness/availability. The old fixture
used Launch/Refresh models/Needs login and omitted readiness. Conductor approved
alignment; guards and existing behavior assertions remain. Initial rendered run
was 3/4 (only stale Needs login text failed); all four now pass.

Fresh authorized fork read: canary head `23d2dc15c6c45554181938b7956b59c085980455`,
workflow ID `323279417` active. Workflow push filters exclude the canary branch;
no workflow_run trigger. Existing hooks absent; no hook bypass. Native fixture
adds exactly `commands::agents::hosted::tests::hosted_native_cleanup_confirmation_retry_and_session_fence`
to the existing exact-one Windows list, retaining all prior suites and ACL.

## Source handoff and held Windows gate

Implementation commits: `bf60e71bb` (owner) and `6038d0981` (native/UI/tests).
No unrelated baseline or final modifications. All local command sessions ended;
no owned browser/server/build/test process remains. No detached process was used.
The next documentation-only commit records release of source/test ownership to
conductor for fresh independent Chuck. Source review can run while export approval
is pending; the same executor assignment may resume Windows verification after
explicit authorization and reviewer ownership release.

Automatic approval review rejected the attempted canary push/dispatch **before
command execution**. Its reason was:

> The command pushes the private source to an external fork and dispatches its workflow; the user authorized Windows verification but did not explicitly authorize exporting this code to that destination.

The rejection instructed: “Do not bypass this rejection through a workaround or
indirect execution. Continue with a safer alternative, or carry out checks to prove
that the action is authorized or low risk before trying again.” The conductor
requested explicit human approval for exactly `Shipitrealgood/buzz` branch
`build/paired-hosts-canary` and the existing workflow. **No push, workflow run,
installer, source/run association or artifact hash exists for this delta.** Native
Windows tests/build are **NOT RUN**; no old test.21 evidence is substituted.

Measured source delta `ee3bbc8d5..6038d0981`:

| Category | Files | Added | Removed | Net |
|---|---:|---:|---:|---:|
| Production Rust/TS/TSX | 11 | 906 | 122 | +784 |
| Tests/fixtures | 6 | 826 | 71 | +755 |
| Workflow | 1 | 1 | 0 | +1 |
| Docs including feature AGENTS | 3 | 84 | 1 | +83 |

Counts include formatting/reindentation. Known relocation within those counts:
16 production lines (`run_managed_agent_deletion`) and 47 test-helper lines
(`seed_intent`). They are not new independent implementations. Final handoff docs
add documentation only beyond this measured implementation range.

Native fixture assertions are **authored, not executed**: one new exact case
extends the original isolated create fixture, passes real Device/Management into
the common coordinator, confirms admission refusal/pending retention/session
switch/runtime disappearance, and injects archive enqueue failure after record
removal before restart/retry completion. This does not prove the full Tauri
command dispatcher or real keyring failure path. Force routing is source-wired
before the provider force branch; Node verifies hosted IPC sends no force flag or
shutdown. Reviewer should judge these boundaries explicitly.

Full `just ci`, native Windows/canary, real Tauri IPC/UI journey, human confirmation,
live relay/host/provider/login/chat/service tests, installation and deployment are
NOT RUN. No board mutation, source-wide cleanup, protocol/schema/host change,
feature activation, main/tag/release/updater change occurred. Recommended next
gate: [fresh independent source review](../briefs/3725-confirmed-hosted-cleanup-review.md),
then authorized supported-platform verification before claiming native acceptance.

## H-1/H-2 repair handoff — 2026-11-10

The conductor assigned only the two High findings in the unchanged
[round-0 audit](../audits/post-audit/3725-confirmed-hosted-cleanup.md). Repair
started at clean `6d7ca47d2d00560a7c6a221e475a52c107c0aab1` in the same worktree.
No unrelated changes were present or absorbed. Signed repair commits:

- `356b1b7b3c321c89e6b4ae8f1e3294ca87582477`: H-1, reconcile admitted Cleanup
  before retained renewal. All candidates are read; an unadmitted replacement
  cannot hide its current failed predecessor. A missing previously admitted
  Cleanup is accepted as superseded only when a proven later revision exceeds
  its admitted revision. Existing transaction winner checks and fence remain.
- `faf59a0b3421c5410f807428d768a2e353e32396`: H-2, serialize confirmed local
  finalization. Existing workspace, identity-mutation, then managed-agent-store
  guards cover captured-session validation, assignment/record/key effects,
  captured-owner/scoped tombstone enqueue and phase 4. Contention returns a
  retryable error. No guard crosses a network await or recursive `Session::keys`;
  nest regeneration starts after release. Ordinary deletion passes no confirmed
  scope and preserves its existing best-effort behavior.

The narrow `Session::with_local_commit` exposure was explicitly approved by the
conductor. This uses existing writer boundaries, including identity import's
`commit_imported_identity`; it adds no global writer redesign, journal, service,
protocol, relay, host or UI change.

Measured repair verification (Hermit activated, self-update disabled):

| Command / source | Actual result |
|---|---|
| `cargo test --offline --locked -p buzz-paired-owner deployment_current_revision_refresh_launch_and_exact_retry -- --test-threads=1`, new H-1 regression with pre-fix production | Expected RED: **0 passed, 1 failed, 15 filtered**, 10.42s; reopen/retained B fails with “Admission has not been confirmed.” |
| `cargo test --offline --locked -p buzz-paired-owner deployment -- --test-threads=1`, H-1 repaired source | **2 passed, 0 failed, 14 filtered**, 14.42s. Existing loopback fixture permission only. Covers Check and Retry exact B reuse, admission/Cleaned completion, expiry renewal requiring current proof, denial and permanent fence. |
| `cargo fmt --package buzz-paired-owner --check` | PASS. |
| `rustfmt --edition 2021 --config skip_children=true --check` on the five changed native files | PASS; source parsing/formatting only, not typechecking. |
| `pnpm --dir desktop typecheck` | PASS, `tsc --noEmit`, exit 0. |
| `CHECK_FILE_SIZES_BASE=6d7ca47d2d00560a7c6a221e475a52c107c0aab1 node desktop/scripts/check-file-sizes.mjs` | PASS after permitted read-only subprocess retry; sandbox attempt failed at `spawnSync git EPERM`. |
| `git diff --check` | PASS. |

The existing native exact-case fixture now injects contention separately for each
writer guard and races both workspace mutation and real in-memory identity import
at the shared finalizer's before-remove boundary. It asserts phase 3 until effects
complete, phase 4 before writers enter, original-scope kind-5/9035 retention,
absence of retention DBs for switched scope/owner combinations, and stale-session
refusal. A test-only callback exposes that existing boundary. **These native
assertions remain authored, uncompiled and unexecuted.** Identity persistence in
the fixture is a synthetic ephemeral closure, not actual keyring import. The
earlier injected archive failure/retry assertions remain intact.

Native Linux compilation was not repeated: its established GLib/GIO dependency
block precedes application compilation. Windows remains **NOT RUN**, with no
export/workflow retry. During final repair handoff the conductor relayed explicit
human approval: “are you waiting on me for that approval? if so it is approved.”
This answered the exact question authorizing Cleanup source export to
`Shipitrealgood/buzz:build/paired-hosts-canary`, its Windows tests/build and canary
download, with no release/tag/updater or live relay/service/agent change. That
resolves the prior export-authorization condition; the historical rejection above
is preserved. Conductor sequencing still holds execution until the same reviewer
finishes the H-1/H-2 re-gate and releases ownership back to this executor.
Node/browser tests were not rerun for these Rust-only repairs: UI/API/fixture
sources and existing browser build are unchanged from `6038d0981`; the round-0
review independently measured 32 named Node cases and 4 mocked-IPC browser cases.
Those receipts support only their unchanged browser/IPC behavior, not H-2.

Measured repair delta `6d7ca47..faf59a0b3`: production Rust **5 files, +112/-45**;
tests **2 files, +308/-0**. Production-path counts include the test-only boundary
parameter. No between-file relocation, workflow or frontend delta in this repair.
This receipt and the updated brief add documentation only. All owned command
sessions are terminal; no detached build/server/test processes were started.
Exclusive write/test ownership is released at the following documentation commit
for the **same reviewer** to re-gate H-1/H-2 and introduced defects. Independent
acceptance and supported-platform native execution remain outstanding.

## Authorized Windows verification — launched

Same reviewer accepted the H-1/H-2 repairs with C=0/H=0/M=0/L=0 and released
clean `8cec4d7cf5742b7d4fbcb3f093dd2242f6865d21`. The executor resumed exclusive
write/test ownership for this same assignment. Explicit human approval quoted
above covers the exact source export, workflow and download; the automatic
approval review accepted this now-authorized action. No rejection was bypassed.

Fresh fork head was `23d2dc15c6c45554181938b7956b59c085980455`, verified ancestor
of the accepted source. All 31 workflow documents were parsed: push branch
filters exclude the canary branch, tag-only workflows cannot match a branch push,
and there is no `workflow_run` trigger. Existing Windows workflow ID `323279417`
is active. Generated Git hooks remain absent; no hook was disabled or rewritten.

Normal fast-forward updated only `Shipitrealgood/buzz:build/paired-hosts-canary`
to `8cec4d7cf5742b7d4fbcb3f093dd2242f6865d21`. `gh workflow run windows-canary.yml
--repo Shipitrealgood/buzz --ref build/paired-hosts-canary` launched
[run 42862238726](https://github.com/Shipitrealgood/buzz/actions/runs/42862238726).
The existing exact Cleanup native case and all prior native/ACL gates remain.
Owned monitor: `gh run watch 42862238726 --repo Shipitrealgood/buzz --exit-status
--interval 45`. Executor retains ownership until **both** jobs are terminal and
source/artifact receipts are checked. No successful native result is claimed yet.

### First native result and bounded fixture correction

Run `42862238726`, source `8cec4d7cf`, native job `129991147892` completed
**FAILURE** after 14m28s. Native application/tests compiled. Owner 16, shared
harness 9, conversion 73, directory 7, registration 3 and exact hosted-create 1
passed: **109 passed outer cases**, zero ignored in those suites. Exact Cleanup
then failed (0 passed/1 failed), so the later exact metadata case did not run.
The isolated SQLite ACL check passed and failure receipts uploaded successfully.

Measured failure: `agents_hosted_cleanup_tests.rs:121` expected a refusal containing
capitalized “Admission.” Source inspection shows the shared create fixture also
retains an unexpired, unadmitted follow-on Deploy at expected revision 1; the
existing production guard refuses that earlier as “competing setup admission
unresolved.” The failed assertion did not print the returned error. The narrow
correction asserts that exact expected refusal and includes actual/expected values
on failure. Identity/record/no-effects and all subsequent Cleanup assertions are
preserved; production is unchanged. Conductor approved this fixture-only repair.
Scoped rustfmt initially requested its usual trailing comma; applied formatter,
then rustfmt check and whitespace check passed. Supported-platform rerun is still
required; this source explanation alone is not a green test result.

Failure evidence root:
`/home/bryce/services/buzz-agents/releases/3725-windows-canary/8cec4d7cf5742b7d4fbcb3f093dd2242f6865d21/42862238726/`.
Native artifact `13786727514`, 5,320 bytes, expires `2026-11-17T17:25:17Z`;
ZIP SHA-256 `f7b3a1dd8b2a97fe4be44ead43b5f4714e0fda87885caf9a311b42a6256bc38e`
matches GitHub metadata. Nine flat members inspected and extracted to `tests/`.
Original job metadata and raw/ANSI-stripped logs retained. Initial log retrieval
refused terminal escapes; file-only retry allowed them and inspection used the
stripped copy. Installer job `129991147255` remains monitored to terminal; this
failed source/run will not be discarded or represented as native acceptance.
