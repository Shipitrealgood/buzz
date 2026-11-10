# Independent review: shared host discovery and Windows Add Agent (#3721)

## Current handoff: M-1 re-gate only

The conductor accepted the review's M-1 and authorized this bounded repair.
Review `f15313756c4b3f4e28d442f275532602e60e076f..5d72b08d3e5ab7bbcd52711e17a43572e184e47b`
in the same worktree. The original scope below is historical context, not a
request to restart the independent 99-test gate. Same reviewer owns this re-gate.

- `commands/agents.rs:434`: the first hosted check now validates inventory before
  phase 1 and every creation effect; probe whether any effect can precede it.
- `commands/agents_hosted.rs:222`: record-independent `check_inventory` reuses the
  session, scoped Status, availability/config and canonical metadata checks.
  `check_runtime` still calls it before comparing the retained record's argv.
- `commands/agents_hosted_tests.rs:135`: four rejected drafts assert no intent,
  record, publication queue entry or loopback connection; the original valid
  identity/retry/recovery case follows after closing the disposable listener.
  Probe whether the fixture would fail if the new entry check were removed.

Production delta +12/-3, fixture +96/-6; no architecture or scope deviation.
Fmt/diff/differential-size and static ordering checks PASS. Native test build is
blocked before app compilation by missing GTK GLib/GIO metadata; the Windows-only
fixture remains NOT RUN. See the [repair receipt](../executions/3721-host-discovery-windows.md#m-1-repair--2026-11-10).
No provider/live/dependency action or test-platform workaround was added. All
processes exited and the committed checkout is released for this re-gate.

---

## Original S2/S3 handoff (context)


Date: 2026-11-10. Requester: conductor. Next owner: fresh independent reviewer.
Requested executor configuration: Astra/high; actual served identity not exposed.

## 1. Where the work lives

Worktree `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events` (not main).
Review source range:
`4149810598334eb8bff18f3c65d014cba3092a59..7720b41a853775eae23ce103944de904f626865b`
(two DCO source commits: `cbe4bef84`, `7720b41a8`). Documentation follows separately.
Baseline and final tree are clean; no unrelated changes. Executor processes exited;
exclusive write/test ownership is released with the documentation commit.

## 2. What landed

Main's portable tables, availability/version/auth interpretation and original pure
metadata tests moved into existing core. Hostd resolves those candidates in its
configured normal account and runs bounded probes/models before guarded execution.
Existing encrypted DTOs, owner continuation and Windows Add Agent consumers carry
remote facts; native login stays external. Read the [execution receipt](../executions/3721-host-discovery-windows.md)
for exact counts, commands, failure corrections and native limitations.

Source of authority: [S2/S3 spec](../specs/3722-main-discovery-native-login.md),
[prior audit](../audits/pre-audit/3722-claude-host-reuse.md), and accepted
[S1 cleanup re-gate](../audits/post-audit/3721-trusted-host-executor.md).

## 3. Your job

Independently review the bounded source change. Doubt the mapping, readiness and
start-retry claims against production paths and the finite fixture evidence.
The executor does not supply the independent gate.

**Special instruction:** Desktop Rust did not reach application compilation because
GObject metadata is absent. The actual runtime fixture execs an instrumented
conversation harness; real ACP is used for models only. Neither is native Windows,
real systemd or a provider conversation proof. Do not upgrade these limits.

## 4. Specifically look for

1. **One main definition owner, actual account resolution.**
   Probe [shared candidates](../../crates/buzz-core/src/harness/mod.rs:197) and
   [host resolution](../../crates/buzz-hostd/src/discovery.rs:30).
   Does `runtimes: {}` plus local `buzz_acp` discover fake Claude/Grok from account
   PATH without authored provider rows? Do explicit rows retain their local ID and
   restrictions? Does Desktop retain its own cache/install/orchestration behavior?
   [Fixture](../../crates/buzz-hostd/src/user_service_tests.rs:498) compares two PATHs.

2. **Truthful current readiness and bounded children.**
   Probe [inspection](../../crates/buzz-hostd/src/discovery.rs:77),
   [readiness](../../crates/buzz-hostd/src/discovery.rs:121), and
   [capture](../../crates/buzz-hostd/src/discovery.rs:165).
   Check missing CLI/adapter/version, unsupported required config, timeout/output
   cap and process-group cleanup. Grok NotApplicable must never become LoggedIn.
   Shared parse diagnostics must not leak through host observations.

3. **Actual runtime launch and false-ready prevention.**
   Probe [run_user](../../crates/buzz-hostd/src/runtime_context.rs:513),
   [final gate](../../crates/buzz-hostd/src/runtime_context.rs:589), and
   [launch](../../crates/buzz-hostd/src/runtime_context.rs:477).
   Ensure current execution binding/model membership/readiness precede exec;
   internal recheck sends no READY and creates no untracked cleanup artifact.
   Distinct key, existing policy and configured context must reach main launch.
   [Actual-entry fixture](../../crates/buzz-hostd/src/user_service_tests.rs:641)
   checks these seams and the same-account logged-out reversal.

4. **Retry and cleanup retain S1's repaired boundary.**
   Probe [start](../../crates/buzz-hostd/src/user_service.rs:282) and
   [cleanup](../../crates/buzz-hostd/src/user_service.rs:418).
   Active exact retry must not start a second execution; changed identity/revision
   cannot borrow that receipt. Cleanup remains independent of deleted launcher,
   moved HOME/workspace and unusable current catalog. Restart=no and no enable
   preserve explicit shutdown; reboot lifecycle policy is intentionally deferred.

5. **Optional fields do not weaken existing transport fences.**
   Probe [observation predicate](../../crates/buzz-core/src/host_deployment/observation.rs:116),
   [wire tests](../../crates/buzz-core/src/host_deployment/wire_tests.rs:375), and
   [owner gate](../../crates/buzz-paired-owner/src/deployment/continuation.rs:181).
   Old missing evidence is unknown; historical success is irrelevant; 64-entry and
   existing byte limits remain. Matched rollout is required: old deny-unknown readers
   may reject new fields. Existing operation/scope/revision/catalog/nonce checks stay.

6. **Existing Windows flow uses host facts without local fallback.**
   Probe [catalog projection](../../desktop/src-tauri/src/commands/agents_hosted_commands.rs:159),
   [native inventory gate](../../desktop/src-tauri/src/commands/agents_hosted.rs:241),
   [model predicate](../../desktop/src/shared/api/hostedAgents.ts:115), and
   [setup copy](../../desktop/src/features/agents/ui/HostedAgentSetup.tsx:105).
   Unknown/unsupported metadata is not selectable, runtime availability stays actual,
   version labels appear only with a parsed host check, and sign-in guidance is native.
   Preserve scoped metadata/model clearing and ordinary key/config/model-picker paths.

## 5. Spec decisions/deviations

No architecture deviation. Local optional `buzz_acp` is the concrete auto-discovery
entry; explicit local rows are intentional allowlisting. Richer required config is
unsupported rather than silently borrowed or synthesized. Pure metadata tests moved
with the source so they execute without Tauri; orchestration tests remain native.
Conductor clarified lifecycle scope: no reboot/explicit-stop persistence subsystem.

## 6. What not to chase

Live netcup/Hetzner, existing agents/provider sessions, credential inspection,
Windows installer publication, root/private-account redesign, delegated host
management, new transport/schema, main catchup and generic lifecycle policy are
outside this source gate. Legacy root helper remains solely for its old caller.
Pre-existing clippy warnings are documented; no unrelated sweep is warranted.

## 7. Deliverable and pre-flight

Return findings and independently run/not-run evidence to conductor; source approval
and native rollout are separate. Use the same pinned checkout for this sequential
review. Read the spec, receipt and prior S1 re-gate before testing. Recorded passes:
shared metadata 9; core wire 13; hostd library 26 plus 2 explicitly run ignored
fixtures, activation 1, controller 13; owner 2; Node 33; tsc/Biome/fmt/size/diff.
Desktop Rust is blocked by missing GObject; full CI, Windows, real services/provider
chat and human acceptance remain unrun. No merge/push/deploy/board authority is added.
