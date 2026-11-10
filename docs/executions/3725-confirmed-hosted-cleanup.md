# #3725 confirmed hosted Cleanup execution

Status: implementation in progress. Executor owns exclusive write/test work in
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
