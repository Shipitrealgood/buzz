# 2722 — Host discovery replacement: independent review brief

## 1. Assignment

Review the complete pre-Create host discovery replacement against accepted D1.
Executor self-review is evidence, never your independent merge gate. Conductor
owns disposition and any activation. Requested executor configuration Astra/high;
actual runtime identity not independently observable.

## 2. Where the work lives

- Worktree `/home/bryce/services/buzz-wt-3720-host-registration`, branch
  `proof/3720-host-registration-events`.
- Base `93ea3f2a29f663e9c53cd560df33e73a785dc37a`; implementation commit
  `26fac30d27789f5ac11b7d74db9e28f37ea6ef86`. This brief is the following
  documentation-only ownership handoff.
- Approved spec `/home/bryce/services/buzz-agents/docs/specs/2722-host-discovery-replacement.md`.
- [Execution and measured gates](../executions/2722-host-discovery-replacement.md).
- [Failure receipts and limitations](../executions/evidence/2722-host-discovery-replacement-checks.md).
- No original unrelated modifications. Ephemeral test DB/container processes
  are stopped. Sequential exclusive workspace ownership returns to conductor.

## 3. Result and boundaries

Existing Add Agent now discovers selected host models before creating an agent.
Two explicit 43103 operations reuse the private mailbox, protected views and
outbox. The same picker consumes scoped results; the same main creation phases
persist the selected model; the same host runtime rechecks readiness and model
membership immediately before conversation. Old unfinished setups remain usable.
Production: 17 existing files, +649/-189, +460 net, no move discount. Tests grew
+758/-28. This is bounded net growth, not a net-reduction claim.

## 4. Specifically look for

1. **Can revoked model evidence escape through historical registration reads?**
   Probe `paired_hosts.rs:151,299` for duplicate admission and active binding;
   `paired_hosts/registry.rs:214,232` for filtering both evidence and messages.
   Existing ack/history recovery intentionally stays readable. New DB regression
   uses the actual writer and asserts zero operation/placement/ordinary-event growth.
2. **Does correlation depend on encrypted data and signed scope together?**
   Probe `buzz-core/src/host_registry.rs:56,77,457` and owner `registry.rs:858`.
   Require exact parent, ack, nonce, runtime, revision, host and generation, plus
   bounded expiry. The new owner test is a signed protocol peer fixture, not a
   permission-code substitute; permission evidence comes from the DB regression.
3. **Can replay or cancellation create or rerun anything?**
   Probe host `registration.rs:238` and `:436`. The active local grant and exact
   stored response precede probing; no SQLite transaction spans the process.
   The result is journaled before publication. An already admitted process can
   finish after revocation, but relay publication/read must then fail.
4. **Did runner extraction preserve cleanup and final launch enforcement?**
   Probe `runtime_context.rs:260,302` and `capture_finite`, plus unchanged
   `run_user` / `require_ready_selection`. Both discovery callers share capture,
   64 KiB output cap, process-group cleanup and normalization. New actual-runtime
   fixture uses real built ACP/runtime and fake executables; old journey also passed.
5. **Can an old host reply populate a new selection or invent Default?**
   Probe `usePersonaModelDiscovery.ts:249,341,403,470` and
   `AgentDefinitionDialog.tsx:546,579`. Cache keys include full scope, installed ID
   and catalog revision. No wall-clock model TTL. Default is an exact returned ID;
   absent default cannot enable Create. Native query failures stay unavailable.
6. **Does one Create reach main identity creation with selected model?**
   Probe `AgentDialog.tsx:301` through existing `prepareHostedDefinition`,
   native `agents_hosted_commands.rs:171`, `agents_hosted.rs:101`, and explicit
   Windows command export in `agents.rs:422` / registration in `lib.rs:721`.
   The native main fixture now asserts selected initial model, but is **not run**
   here. Inspect that path carefully; Linux checks cannot validate Windows cfg code.
7. **Are retained setups still recoverable?**
   Only new creation loses Prepare wording. Shared `HostedAgentSetup`, Retry,
   PendingLogin wire compatibility, continuation and cleanup remain. New success
   closes the form; a retained error still exposes setup recovery. Do not infer
   that the unrelated live test.27 recovery failure is repaired.

## 5. Deviations and judgment calls

- Added active-binding filtering in existing DB views, with conductor agreement;
  historical registration access was broader than discovery may be.
- Added the required explicit Windows re-export, bringing production count to17.
- Repaired only the approved one-field deletion-test initializer to enable DB tests.
  Obsolete relay root-helper tests remain unchanged and block relay integration.
- Successful-empty host enumeration maps to the shared empty model response;
  failed/unanswered queries throw. Model display has no independent UI state engine.
- Canonical frontend formatting/lint is blocked by existing nested Biome roots;
  cached pinned formatter stdin was used without configuration edits.

## 6. What not to chase

Live Jeff/test1 recovery, native provider/login state, old HTTP cleanup, new app
port, release/build pipelines and relay root-helper fixture redesign are outside
this change. Existing native identity/revocation policy is retained. No new
account isolation, login broker, provider definitions, migrations or generic RPC.
All deployment/installation/live checks remain held for user authorization.

## 7. Deliverable and pre-flight

Return severity-classified findings with concrete failure paths to the conductor;
no executor self-gate, push, merge, board mutation or live activation.

Before reviewing, verify the committed range and clean worktree, read the spec
and execution record, and distinguish measured seam tests from the unproven
end-to-end relay/native journey. Full `just ci` / `just test` are not green:
relay unit-test compilation and native platform prerequisites are still missing.
Fresh Windows verification and a coordinated three-system rollout are required
before a human host-first journey can be accepted.
