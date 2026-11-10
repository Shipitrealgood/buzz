# #3725 confirmed hosted Cleanup execution

Status: source implementation handed off for independent review; Windows gate held.
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
