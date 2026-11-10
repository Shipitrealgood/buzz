# #3721 S2/S3 — shared discovery and Windows integration post-audit

Date: 2026-11-10. Reviewer: `/root/host_discovery_windows_review`, `codex-chuck`.
Requested configuration: `gpt-6-astra` / `high`; actual served model and reasoning
metadata are not independently exposed. Rework round: **0**.

**Verdict: rework for one required Medium source finding (M-1).**
C=0 / H=0 / M=1 / L=0. Native Desktop compilation and the Windows journey remain
unverified; they are separate from this concrete source defect. No architecture
prerequisite or additional verification campaign is requested.

## Pin and contract

Worktree `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`, clean starting HEAD
`c9226502a2b922e3361d28b7217d814fae5921d6`.
Reviewed source range
`4149810598334eb8bff18f3c65d014cba3092a59..7720b41a853775eae23ce103944de904f626865b`;
source commits `cbe4bef846affba2a0cc34546af2c8ebe3f2aee5` and
`7720b41a853775eae23ce103944de904f626865b`. Subsequent input commits are docs.

Inputs: [ratified S2/S3](../../specs/3722-main-discovery-native-login.md),
[dispatch brief](../../briefs/3721-host-discovery-windows-review.md),
[executor receipt](../../executions/3721-host-discovery-windows.md),
[prior reuse audit](../pre-audit/3722-claude-host-reuse.md),
[accepted S1 re-gate](3721-trusted-host-executor.md), supplied current task
acceptance/amendments, portable conductor contract, post-audit skill,
applicable AGENTS, VISION/agent/remote vision, ARCHITECTURE host contract and
repository/ACP TESTING guidance.

Scope is the shared main registry/classifiers, configured-account discovery,
guarded runtime launch, optional encrypted facts and existing Windows consumers.
The shared trusted non-root account/HOME/native login/tool/network decision is
settled. S1 is reviewed only for regressions from this delta. Testing used fake
commands, disposable directories/sockets and existing synthetic peers. No product
source was edited, no mutation campaign ran, and no real service, provider,
existing agent, credential inspection, live relay, board, push, merge, deployment
or installer publication was performed.

## M-1 (Medium, high confidence) — inventory rejection follows creation effects

**Read-derived control-flow finding; not a measured Windows reproduction.**
The new [inventory guard](../../../desktop/src-tauri/src/commands/agents_hosted.rs#L241)
belongs to `Create::check_runtime`. For a new intent, however,
[`create()` invokes main creation phases first](../../../desktop/src-tauri/src/commands/agents_hosted.rs#L441)
and reaches [that guard only afterward](../../../desktop/src-tauri/src/commands/agents_hosted.rs#L463).
The earlier [`Create::prepare`](../../../desktop/src-tauri/src/commands/agents_hosted.rs#L38)
checks the canonical mapping and supported payload shape, but never checks
`inventory`. Its call to the owner journal reaches
[`Status::permits`](../../../crates/buzz-core/src/host_deployment.rs#L146),
which validates the inventory's shape, scope, freshness and runtime membership;
it does not require a present, available, configuration-supported entry.

Main consequently [retains the intent, persists the agent record and advances
phase 1](../../../desktop/src-tauri/src/commands/agents.rs#L788), enqueues the
owner publication, and [attempts profile publication](../../../desktop/src-tauri/src/commands/agents.rs#L855)
before this new refusal. `create()` then returns the `check_runtime` error through
`?`; it neither rolls these effects back nor returns the retained setup response.
The native phase test currently supplies
[only an available inventory](../../../desktop/src-tauri/src/commands/agents_hosted_tests.rs#L89),
so it does not challenge that ordering.

**Realistic trigger:** the form reads an available Claude runtime; its adapter is
removed before Prepare opens the current native status. Host status now
[advertises all candidates, including unavailable ones](../../../crates/buzz-hostd/src/deployment.rs#L305).
The fresh status remains `Current` with a valid canonical mapping and a
`NotInstalled` fact. Main creates a distinct identity/retained intent/local record
and can publish the profile, then the caller receives an inventory error instead
of setup. Closing the failed form leaves a created agent that was never dispatched
to the host. Reopening can list that retained setup, but this does not undo the
unexpected creation. Missing inventory (an older host) or `config_supported=false`
has the same ordering for a known mapping with an otherwise supported draft.
The ordinary route still refuses host launch; this is an unintended creation and
failure-response defect, not a demonstrated unauthorized runtime execution.

**Bounded repair:** validate current scoped inventory for the selected installed
ID before entering main creation phases. Keep the later mapping/use-time checks;
avoid making the early check depend on an already-created record. Extend the
existing native phase fixture to assert no retained intent, agent record or
publication attempt for missing, unavailable and unsupported inventory, and retain
its available-runtime identity/retry case. No new cleanup subsystem, transport or
platform is needed. If that native fixture cannot run here, state that explicitly
and preserve the established native gate.

| Finding | Disposition / proposed owner and next action |
| --- | --- |
| M-1 — fix in lane | Conductor ruled fix-in-lane during review. Existing S2/S3 executor repairs the early guard and fixture; this reviewer re-gates M-1 and defects introduced by that repair. |

## Finite obligation assessment

| Obligation | Evidence and conclusion |
| --- | --- |
| One main definition owner, actual account discovery | **Read + measured.** Desktop re-exports core tables and calls the shared availability, version and auth classifiers. [`candidates`](../../../crates/buzz-core/src/harness/mod.rs#L197) derives builtin/preset argv/probe facts; [`resolve_catalog`](../../../crates/buzz-hostd/src/discovery.rs#L30) consumes them using the configured account PATH. Automatic empty rows, two PATH contexts, missing adapter/underlying CLI and outdated Codex pass. Explicit rows retain IDs and restrict candidates when automatic mode is omitted; unsupported args/config refuse readiness. Desktop keeps cache, resolver, install and auth orchestration. |
| Truthful current readiness and finite capture | **Read + measured.** [`readiness`](../../../crates/buzz-hostd/src/discovery.rs#L121) runs main probe argv in `Context::command`; [`capture`](../../../crates/buzz-hostd/src/discovery.rs#L165) bounds bytes/time and kills its process group. Tests pass logged-in without history, logged-out/config-invalid/timeout/oversize refusal despite history, sanitized output, and Grok NotApplicable without claiming authentication. Models are not entitlement. |
| Actual preparation/models/selected launch | **Read + measured.** The [actual runtime fixture](../../../crates/buzz-hostd/src/user_service_tests.rs#L641) executes compiled `run_user`, real ACP models against a synthetic peer, then an instrumented conversation harness. It checks identity/auth tag/owner/policy/HOME/PATH/workspace/strict model, no early READY or untracked recheck artifact, refusal after logout and absent selected ID, exact retry and preserved shared files. [`require_ready_selection`](../../../crates/buzz-hostd/src/runtime_context.rs#L589) is used before exec. This is not real ACP conversation or native systemd proof. |
| S1 retry and cleanup boundaries | **Read + measured.** Active exact retry checks retained conversation identity; changed revision/identity cannot reuse it. The existing lifecycle and [obsolete-launch-config cleanup regression](../../../crates/buzz-hostd/src/user_service_tests.rs#L399) pass. Cleanup still uses saved manager/unit custody rather than current launch configuration. The source introduces no reboot or idle-autostop policy. |
| Bounded optional facts and consumers | **Read + measured.** [Status validation](../../../crates/buzz-core/src/host_deployment.rs#L131), encrypted wire bounds, [Observation binding/current checks](../../../crates/buzz-core/src/host_deployment/observation.rs#L124), and [owner continuation](../../../crates/buzz-paired-owner/src/deployment/continuation.rs#L178) retain existing scope/catalog/revision/operation/freshness checks. Old omitted readiness is unknown; old readers reject new fields, requiring matched rollout. Existing 64-entry and 8 KiB/2 KiB limits remain. No relay schema/resource change. |
| Windows projection and shared flow | **Read + Node/TS measured; native not run.** [`hosted_catalog`](../../../desktop/src-tauri/src/commands/agents_hosted_commands.rs#L159) applies host facts to canonical metadata and suppresses unsupported/unknown metadata, without local resolution. Existing `AgentDialog` host-source switches and async scope/expiry clearing remain. [`currentHostedModels`](../../../desktop/src/shared/api/hostedAgents.ts#L115) checks current readiness and exact identity/scope/revision; setup reuses the picker and external native-login guidance. Main key/config/profile/channel/cloud paths are retained. M-1 is the concrete creation-order exception. |

The extraction is semantic reuse, not just relocation: both consumers call the
shared functions, while hostd supplies resolution/execution context. The receipt's
872 relocated/adapted lines and net +1224 lines (including tests/docs) are consistent
with the rename-aware diff and removed Desktop definitions; they are not 1806
lines of new platform. No hidden second provider matrix was found. Minimalism and
elegance: 9/10 each. Correctness: 8/10 pending M-1's early guard. The fix belongs in
the existing creation boundary, not in another architecture stage.

## Independent verification

All Cargo commands used `HERMIT_SELF_UPDATE=false`, the repository Hermit
environment, `--offline`, and serial tests. Local output logs are
`target/3721-review-s2-*.log`; this committed ledger records the actual outcomes.

| Command | Independent result |
| --- | --- |
| `cargo test --offline -p buzz-core --lib harness -- --test-threads=1` | PASS, 9 |
| `cargo test --offline -p buzz-core --lib host_deployment -- --test-threads=1` | PASS, 13 |
| `cargo test --offline -p buzz-hostd --all-targets -- --test-threads=1` | PASS with narrow socket access: library 26 / 2 ignored; activation 1; controller 13; other targets 0 cases |
| `cargo test --offline -p buzz-paired-owner --lib deployment -- --test-threads=1` | PASS, 2 |
| `cargo build --offline -p buzz-hostd --bin buzz-host-runtime -p buzz-acp --bin buzz-acp` | PASS, current source rebuilt |
| `BUZZ_TEST_RUNTIME="$PWD/target/debug/buzz-host-runtime" BUZZ_TEST_ACP="$PWD/target/debug/buzz-acp" cargo test --offline -p buzz-hostd --lib actual_ -- --ignored --test-threads=1` | PASS, both ignored cases explicitly run, 2; expected negative launches print only `managed runtime unavailable` |
| Desktop `node --import ./test-loader.mjs --experimental-strip-types --test --test-isolation=none --test-reporter=tap src/features/agents/ui/whereToRunIntent.test.mjs src/features/agents/lib/instanceInputForDefinition.test.mjs` | PASS, 33 |
| Desktop `pnpm exec tsc --noEmit` | PASS |
| `cargo fmt -p buzz-core -p buzz-hostd -p buzz-paired-owner -- --check`; `git diff --check` | PASS |

**99 behavior tests independently passed.** The initial sandbox hostd run had
24 passes/2 socket-bind EPERM failures/2 ignored; owner had 1 pass/1 socket-bind
EPERM failure. The same bounded suites passed after approved local fixture-socket
access. No product failure was discarded. No named mutants were dispatched.

Preserved evidence, explicitly **not rerun**: executor clippy (existing warnings),
scoped Biome, Desktop Rust fmt and differential size checks apply to the identical
reviewed source; this review changes docs only. Prior accepted ACP strict-model
and model-switch tests remain evidence for unchanged ACP implementation; the actual
models/runtime-entry tests above were independently rebuilt and rerun. No unchanged
relay/database proof suite was expanded or represented as newly measured.

**Native gap:** the [executor Desktop Rust check](../../verification/3721-host-discovery/desktop-build-blocker.txt)
stopped before application compilation because `gobject-2.0 >= 2.70` metadata is
missing. Native Desktop tests, Windows build/UI journey and the new ordering
scenario are **not run**. No dependencies or platforms were installed. Full
`just ci`, real systemd credential/cgroup/user-manager/reboot behavior, native
provider login/status/entitlement/chat, live relay/database and human acceptance
also remain **not run**. Earlier canary evidence does not prove this changed path.

## Return and release

Recommend repairing M-1 in this checkout with the same executor, then re-gating
that finding and defects introduced by its repair. The existing report supplies
the bounded repair handoff; no additional spec, board task or platform campaign
is requested. Native activation stays separately gated. This reviewer changed
only this report. All review build/test sessions finished; ownership is released
with the DCO documentation commit after clean-status and process checks.

quality: model=unknown effort=unknown stage=post-audit verdict=rework verify=ran commits=4149810598334eb8bff18f3c65d014cba3092a59..7720b41a853775eae23ce103944de904f626865b findings=C=0/H=0/M=1/L=0 rework_rounds=0
