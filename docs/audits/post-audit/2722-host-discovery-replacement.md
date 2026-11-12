# 2722 — Host discovery replacement: independent post-audit

## Current disposition — re-gate round 1

**Scoped source delta accepted; no unresolved findings: C0/H0/M0/L0.** H-1 and
M-1 are resolved at `797c5169683af2512b80ebcdc0a8513f9ff4b649`. Independent
focused checks passed (browser 7, command consistency/API 2). No defect introduced
by the repair was found. Native, actual-relay and human journey requirements
remain unproven, so this is **conditional source acceptance, not merge-safe or
release readiness**. The quality footer uses `rework` for those outstanding
acceptance obligations; it does not request another implementation rewrite.

Reviewed repair range:
`8496d726dda6c0402500bedb39e14cc3d8f9c57b..797c5169683af2512b80ebcdc0a8513f9ff4b649`.
The original source review and evidence below are retained; its initial blockers
are historical. See the re-gate evidence at the end for current routing.

## Initial review scope and findings

Initial verdict was merge-block: C0/H1/M1/L0. The bounded host-first design was
otherwise consistent with accepted D1. Source, native integration, and
coordinated rollout remain distinct obligations.

Reviewed base `93ea3f2a29f663e9c53cd560df33e73a785dc37a` through implementation
`26fac30d27789f5ac11b7d74db9e28f37ea6ef86`, with documentation handoff HEAD
`13aaa9907891e5562e83805ec96043da08dceb89`, in
`/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`. Initial worktree was clean. First review,
rework rounds 0. Requested reviewer configuration: **gpt-6-astra/high**; actual
served model/reasoning identity is not independently observable.

Inputs: the current header and D1 in
`/home/bryce/services/buzz-agents/docs/specs/2722-host-discovery-replacement.md`,
[review dispatch brief](../../briefs/2722-host-discovery-replacement-review.md),
[execution](../../executions/2722-host-discovery-replacement.md), and
[prior failure receipts](../../executions/evidence/2722-host-discovery-replacement-checks.md).
Read the applicable contributor/product/testing/database guidance and portable
conductor/post-audit contracts. No product changes, board changes, live/provider
traffic, installed-app build, publication, or existing-agent access. No new
verification framework or mutant campaign. Trusted native account, two explicit
43103 operations, main identity creation, retained old setups and deferred bulk
cleanup are settled scope, not reopened findings.

## Findings

### H-1 — High, high confidence — missing platform guard breaks non-Windows desktop — resolved in re-gate

**Read/inferred compile failure:** [lib.rs:722](../../../desktop/src-tauri/src/lib.rs#L722)
adds `hosted_models` to `tauri::generate_handler!` without `#[cfg(windows)]`.
Its only implementation/module and re-export are behind that guard in
[agents.rs:418](../../../desktop/src-tauri/src/commands/agents.rs#L418).
The adjacent commands each have their own guard; the attribute on
`hosted_catalog` does not cover the following command.

Concrete failure: on macOS/Linux the hosted module and generated command macros
are absent, while the handler still references them. The cached Tauri macro
source (`tauri-macros-2.6.3/src/command/handler.rs:155–177`) confirms attributes
are attached to individual match arms. This is a new source-level build defect,
independent of the pre-existing unavailable Linux GLib/WebKit environment. A
native build was **not run**, so no compiler diagnostic is claimed measured.

**Fix:** put the same Windows guard immediately before `hosted_models` in the
handler. Verify command registration and export together on the relevant targets
when native gates are available. This needs no cross-platform hosted feature.

### M-1 — Medium, high confidence — Create dismisses a verified failed deployment — resolved in re-gate

**Read/traced user-visible failure:**
[AgentDialog.tsx:423](../../../desktop/src/features/agents/ui/AgentDialog.tsx#L423)
closes the form whenever `submitted.hosted.error` is falsy. That field means a
transport/recovery error, not successful deployment. Native
[setup:85](../../../desktop/src-tauri/src/commands/agents_hosted_commands.rs#L85)
sets `response.state = read.state` (or `conflict`) on a successful read, and only
sets an error for other conditions. A valid failed receipt can have no observation
and produce `{ state: "failed", error: null }`.

Concrete sequence: Create publishes successfully; the host rejects launch (for
example the selected model disappeared after discovery); the final operation
read sees the valid failed receipt. `setup` returns that failure state without an
error, and the new UI branch dismisses it as success. The same state can be
returned on exact retry: [dispatch:450](../../../crates/buzz-paired-owner/src/deployment.rs#L450)
explicitly allows a previously admitted failed terminal operation. The existing
owner test constructs such a valid failed receipt and reads it successfully at
[deployment_tests.rs:629](../../../crates/buzz-paired-owner/src/deployment_tests.rs#L629).
The retained setup still exists, so this is concealed failure/recovery feedback,
not identity loss. Before this change the returned setup was displayed.

**Fix:** retain the existing recovery view for `failed`, `conflict`, and
`needs-login` even when `error` is null; do not treat unavailable/unverified state
as successful completion. An admitted pending create may retain the agreed
asynchronous UX, with no claim that the agent is running. Do not add polling.
Extend the actual submit-boundary journey with failed/needs-login/conflict results
and an accepted pending/applied result; the hook test alone cannot catch this.
Native failed-state projection and the UI predicate must agree.

| Finding | Disposition / owner / next action |
|---|---|
| H-1 | **landed / resolved** at `797c516`: matching Windows guard read; exported-command consistency test independently passed. |
| M-1 | **landed / resolved** at `797c516`: close only pending/applied without error; seven real-form outcome tests independently passed. |

## Acceptance analysis

- **Read:** pre-Create UI discovery goes through `hostedModels` → native
  `hosted_models` → `open` (session/status/device read) → owner `Registry::models`.
  It does not call `prepareHostedDefinition`, `create_managed_agent_phases`, or
  deployment intent preparation. Those remain behind Create. See
  [native models](../../../desktop/src-tauri/src/commands/agents_hosted_commands.rs#L159),
  [open](../../../desktop/src-tauri/src/commands/agents_hosted.rs#L168),
  [shared Create](../../../desktop/src/features/agents/lib/prepareHostedDefinition.ts#L29).
  The DB test measures no ordinary-event/agent-operation/placement growth;
  host runtime fixture measures no service staging before Create. These real
  seams substantiate the absence of agent effects beyond a mocked UI call count.
- **Read + measured tests:** codec checks signed/encrypted scope and exact header
  equality; owner checks ack, nonce, runtime, revision, request parent and expiry
  at [registry.rs:858](../../../crates/buzz-paired-owner/src/registry.rs#L858).
  DB active-binding admission occurs before duplicate acceptance, and protected
  messages/evidence filter model traffic after revocation while retaining ack
  recovery ([writer:151](../../../crates/buzz-db/src/store/paired_hosts.rs#L151),
  [reads:210](../../../crates/buzz-db/src/store/paired_hosts/registry.rs#L210)).
  New operations retain transient pruning and do not replace registration status.
- **Read + measured runtime journey:** host checks grant/catalog/configured runtime,
  ends its transaction before discovery, and journals the exact response before
  publication ([receive_models:238](../../../crates/buzz-hostd/src/registration.rs#L238)).
  Replay after restart returns that response without rerunning. Both callers use
  [discover_models:302](../../../crates/buzz-hostd/src/runtime_context.rs#L302) and
  the existing bounded capture/process-group cleanup. No remote executable or
  environment is accepted. Revocation blocks new reads/writes; an already admitted
  bounded probe may finish. Final conversation rechecks current catalog, readiness
  and exact selected model at [run_user:593](../../../crates/buzz-hostd/src/runtime_context.rs#L593).
- **Read + measured hook:** cache and pending results use full host scope/runtime/
  catalog revision. Unavailable throws while empty is explicit. Blank Default
  maps only to an actual returned default ID; explicit IDs are membership-checked
  at [dialog:546](../../../desktop/src/features/agents/ui/AgentDefinitionDialog.tsx#L546).
  Selected model passes the existing definition/instance projection to native
  `Selected` at [prepare:101](../../../desktop/src-tauri/src/commands/agents_hosted.rs#L101).
  Native main-phase test is present but unrun on this platform. Local/provider
  mapping tests pass. Shared latch, protected retry intent, legacy continuation,
  PendingLogin and deletion surfaces remain; M-1 concerns only new dismissal.

**Initial scores: minimalism 9/10; elegance 9/10; correctness 7/10.** The implementation reuses
existing mailbox, picker, main creation and finite runner; no new production
service/table/kind/framework or whole-file cleanup. The reported production
budget (17 existing files, +649/-189, net +460) is consistent with the reviewed
change shape and remains below its explicit ceiling; test growth is separate.
Correctness falls below 9 specifically because H-1 and M-1 break a platform build
and hide an observed deployment failure. Fixes are narrow and do not invalidate
D1 or justify another architecture campaign. No optional cleanup findings.

## Independent verification

All commands executed in the pinned worktree, with Hermit and
`HERMIT_SELF_UPDATE=false`; Cargo used `--offline --locked -j2` and sequential
process tests. Cargo reported cached test builds; no installer or environment
installation. Existing runtime/ACP and DB test binaries are the executor-built
artifacts retained in this worktree; source is unchanged since that implementation
(the subsequent handoff commit only changed documentation). This is independent
execution of those seam tests, not an independent native release build.

| Gate actually run | Result and limits |
|---|---|
| `cargo test … -p buzz-core -p buzz-paired-owner --lib -- --test-threads=1` | Core **325 passed**. Owner initially **13 passed / 4 socket-EPERM failures**; this is not green. |
| Same owner-only command with narrow local-socket permission | **17 passed**, including signed models correlation fixture and existing continuation/retry test. |
| `cargo test … -p buzz-hostd --lib -- --test-threads=1` with synthetic process/socket permission | **28 passed, 4 ignored**. |
| `BUZZ_TEST_RUNTIME=$PWD/target/debug/buzz-host-runtime BUZZ_TEST_ACP=$PWD/target/debug/buzz-acp cargo test … -p buzz-hostd --lib user_service::tests::actual_ -- --ignored --test-threads=1` | **2 passed**: old continuation and new pre-Create discovery through real runtime/ACP with fake providers; includes later model/login refusal. Expected negative child `managed runtime unavailable` messages occurred. |
| `scripts/3720-registration-proof.sh run target/debug/deps/buzz_db-8990db044ccf2a6b store::paired_hosts::postgres_tests::registry_models_admission_and_revoked_reads_have_no_agent_effects --ignored --exact --nocapture` | **1 passed** on fresh isolated network-none PG/Redis fixture; DB dropped and all three containers removed. No default/shared DB. |
| Desktop Node real hook, jsdom setup + test loader, `--experimental-strip-types --test-isolation=none --test …/usePersonaModelDiscovery.jsdom-test.mjs` | **3 passed**. |
| Desktop Node loader, same flags; `usePersonaModelDiscovery.test.mjs`, `agentDefinitionSubmitPayload.test.mjs`, `instanceInputForDefinition.test.mjs`, `hostDestination.test.mjs` | **57 passed**. |
| `pnpm --dir desktop typecheck` | **Passed**. TypeScript does not compile the native handler or test state-dependent dismissal. |
| `cargo fmt --all --check`; `cargo fmt --manifest-path desktop/src-tauri/Cargo.toml --all --check`; `git diff --check` | **Passed**. |

**Not run / not accepted:** Windows native compilation and updated main-phase
fixture; non-Windows native compilation; full actual owner → relay → hostd
mailbox integration; full `just ci` / `just test`; native UI/human journey. The
executor's production relay check, advisory lint and differential size receipts
were read, not rerun here. Strict clippy and canonical frontend Biome are not
green per the recorded unchanged warning/configuration failures. The old relay
test fixture is still incompatible (`external_infra_deployment_tests.rs:390`
missing `Catalog.local`, `:412` removed `Helper::at`); no root helper, ignored test,
or rewritten fixture was introduced to conceal that gap. No native dependencies
were installed to get around the missing Linux prerequisites.

**Initial routing (superseded by re-gate below):** executor fixes H-1/M-1 in this same worktree, then
this reviewer checks those findings and any introduced delta. Source acceptance
can only be conditional until the outstanding native and relay requirements are
explicitly resolved. The future integration gate must exercise a real isolated
relay with the actual owner adapter and host reconciliation, then the native
Windows Add Agent flow: registered host → installed harness → returned model →
one main Create → host launch → conversational response. Verify failure and
retry feedback too. Coordinated relay + hostd + desktop versions are required;
an installer alone or successful compilation cannot establish this user outcome.
Any repair of the obsolete relay fixture belongs to a separately authorized
bounded follow-up; do not restore the root architecture. Human verification and
live rollout remain for the conductor/user, with existing agents untouched.

At initial report handoff all reviewer-started commands were terminal. No background
process, isolated DB, or container remains. Only this review artifact is modified;
product source stayed pinned. Commit this report with DCO signoff and return its
exact SHA separately, because a commit cannot contain its own SHA.


## Re-gate evidence and final routing

**Read:** the repair is two production edits only: a Windows guard immediately
before `hosted_models` ([lib.rs:722](../../../desktop/src-tauri/src/lib.rs#L722))
and the existing submit branch's allowlist
([AgentDialog.tsx:426](../../../desktop/src/features/agents/ui/AgentDialog.tsx#L426)).
Only error-free `pending`/`applied` close; all other states retain the same setup
view. Pending makes no running claim. There is no new polling or state framework.
The feature guidance now matches that behavior. The native export remains gated;
Tauri attaches the added attribute to the correct handler arm. H-1 and M-1 are
therefore resolved, with no newly supported defect in this delta.

**Measured:** independently reran from `desktop` with Hermit:

- `node --import ./test-loader.mjs --experimental-strip-types --test-isolation=none --test src/shared/api/hostDestination.test.mjs`:
  **2 passed**. The added consistency check derives all six hosted exports and
  requires exactly one individually Windows-gated handler for each. This checks
  source consistency, not native compilation.
- `pnpm exec playwright test --project=smoke tests/e2e/hosted-agent-setup.spec.ts --grep 'Create submit outcome' --max-failures=1`:
  **7 passed, 27.8 seconds**. Used the retained executor-built e2e bundle, cached
  Chromium, synthetic Tauri bridge and a temporary server explicitly bound to
  `127.0.0.1:4173`. Server startup refused an occupied port and cleanup was trapped.
  The command exited successfully and the owned server was reaped.
- `git diff --check`: passed. No product source changed during re-gate.

**Read / test validity:** the browser cases at
[hosted-agent-setup.spec.ts:843](../../../desktop/tests/e2e/hosted-agent-setup.spec.ts#L843)
drive the actual host picker, model selection and Add agent submit, then assert
recovery or dismissal. They do not copy the production predicate. Each asserts
exactly one native prepare call carrying `opaque/model:fixture`. Boundary-only
mocking returns failed, conflict, needs-login, unverified, pending-with-error,
pending-without-error and applied-without-error. The failed/no-error case would
hit the original M-1 branch and dismiss, contradicting its recovery assertion.
No mutant run was added. Native result projection remains source-reviewed only.

Accepted continuity: the original independent core/owner/hostd/DB/runtime/hook
checks above remain applicable because this repair changes none of their source.
Executor typecheck, e2e bundle build, formatting and size receipts were read, not
repeated as a new campaign. The source delta is +7/-2 production lines; current
original-baseline budget remains 17 production files, +654/-189 (net +465).
Current source assessment: **minimalism 9/10, elegance 9/10, correctness 9/10**,
subject to the explicit integration limits; no optional findings.

**Still not run:** native Windows and non-Windows compilation, native main-phase
fixture, actual owner → real relay → hostd mailbox integration, remaining legacy
Prepare-oriented browser cases, full CI and human app journey. The prior obsolete
relay fixture and environment/lint gaps are unchanged. Focused browser success
cannot prove native registration, authenticated transport, launch or conversation.

**Routing:** return source ownership to the conductor with H-1/M-1 closed. No
further product fixes are requested by this re-gate. Resolve the named native and
real-relay prerequisites before claiming full acceptance, and coordinate matching
relay + hostd + desktop for the eventual real select-host → harness → models →
Create → launch → chat journey. A standalone installer/canary is not that gate.
No live rollout, installation, existing-agent access, board action or publication
was performed or authorized by this review.

All reviewer commands are terminal; the temporary HTTP server was reaped. No
containers were started in re-gate. Worktree was clean at the repair handoff and
only this report changed. Report commit SHA is returned separately. Requested
configuration remains **gpt-6-astra/high**; runtime identity remains unknown.

quality: model=unknown effort=unknown stage=post-audit verdict=rework verify=partial commits=93ea3f2a29f663e9c53cd560df33e73a785dc37a..797c5169683af2512b80ebcdc0a8513f9ff4b649 findings=C=0/H=0/M=0/L=0 rework_rounds=1
