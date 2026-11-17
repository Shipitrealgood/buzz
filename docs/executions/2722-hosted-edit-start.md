# #2722 hosted Edit + explicit Start — execution receipt

Source implementation complete for independent review; native Windows compilation,
independent review, and human acceptance remain pending. This is not release acceptance.

## Consecutive Start recovery phases (2026-11-17)

Intake: clean `a86cfe66a695618dae3f7a2b039adc2a7eeff585`. Windows run 43993874579
reached the saved-model/prompt assertions, then failed with `nonmonotonic intent
phase`; see the [terminal Windows receipt](2722-hosted-edit-windows.md).

Cause is production, not fixture setup: `start_existing_deployment` retains a
normal Deploy intent at phase 0 (`deployment.rs:187`), then native `recover` tried
0→2. `deployment_advance` correctly permits only the current or next phase. The
one-line correction advances phase 1 after existing-record identity verification,
then phase 2. Neither step rewrites main settings or signed command bytes. The
consecutive journal rule remains unchanged. Production: **13 files/net +482**.

The existing native fixture still begins at phase 0. One temporary SQLite trigger
aborts its phase-2 update; assertions require the phase-1 commit, later saved record
and exact retained event to survive. After dropping the trigger, the fixture reopens
and recovers from phase 1, then reopens/repeats at phase 2. Existing saved-model,
identity-change, missing-record and stale-session assertions remain intact.

Full remaining sequence was source-traced: Start retries at phases 1/2/3 do not
regress phase or rewrite settings; identity/session refusal precedes progression.
The separate cleanup fixture branches before this Start scenario. Cleanup retains
phase 2, dispatch/reconciliation advances to 3, and finalization advances to 4;
its repeated-finalization, deletion, retention and session guards remain unchanged.
No analogous phase jump was found there; this is source evidence, not a native pass.

Measured locally: focused rustfmt and diff checks passed. Executing the exact fixture
trigger SQL against an in-memory Python SQLite table confirmed phase 1 persists,
phase 2 aborts, and dropping the trigger permits phase 2. That checks the failure
control only. Corrected native recovery/cleanup tests are **not run locally**; Linux
native prerequisites remain unavailable. No browser/owner reruns, new framework,
installations, publication, CI dispatch or live actions. Same reviewer checks this
bounded correction before conductor's Windows rerun; all executor processes terminal.

## Windows native fixture correction (2026-11-17)

Intake: clean `dc6088571d16e970b7839cf2f380ef57a3251636`. The
[Windows receipt](2722-hosted-edit-windows.md) records run 43988951035 compiling
and packaging Desktop, then failing the saved-model assertion in the exact hosted
create/recovery case. This run is not a green platform gate.

Cause (source trace, reported before editing): the fixture updated `definitions[0]`.
`personas.rs:342` loads and merges built-ins; `sort_personas` at `:164` sorts them
first. Index zero is a built-in, while the fixture instance links `one-definition`.
`build_deploy_payload` uses `resolve_effective_config` (`effective_config/mod.rs:247`),
which finds that exact linked ID, so `initial-host-model` was the correct result
for the incorrectly saved fixture state. This was not an instance-precedence defect.

Test-only correction selects `desired.persona_id`, checks the initial model, saves
its new model/prompt, then reloads and asserts the exact linked values before
candidate projection. Expected candidate `saved/model:2` and saved prompt are
unchanged; later Save preservation and immutable retained-event assertions remain.
No production change: **13 production files/net +481**. No new fixture framework.

Measured local verification: focused rustfmt and `git diff --check` passed.
Corrected Windows native case is **not run here** (Linux native prerequisites remain
unavailable); no native pass is claimed. Browser/owner suites were not repeated for
this fixture-only correction. Next gate: same reviewer checks exact target selection
and preserved expectations, then conductor runs Windows. No live/provider/working-
agent action, installation, publication, CI dispatch or taskctl action occurred.

## Windows compiler correction (2026-11-17)

At clean `ec804bf9dd8563e18433eabc675391f79565599b`, corrected the E0308 recorded
in the [failed Windows receipt](2722-hosted-edit-windows.md): the hosted Start
scope assertion now passes `reconcile_relay.as_str()` instead of a reference to
`ScopedWorkspaceRelay`. This uses the existing accessor on the already captured
relay; scope validation remains intact. Rustfmt expands the call by three lines:
current production total is **13 files/net +481**, within +500.

Focused rustfmt and diff checks passed. No browser/owner suites or native compile
were rerun for this accessor-only change. Failed run 43986614016 recorded 39 Windows
tests passed before desktop compilation failed; exact hosted/ACL cases did not run
and no installer was produced. Reviewer checks this exact delta before the conductor
publishes/reruns Windows CI. All executor processes are terminal; no live/push/CI action.

## Round-1 lint residual (2026-11-17)

At clean `84fb2d6b63e45593a586c5952eab75019c22be0d`, changed only the non-Windows
match binding in `agents.rs:1063` from `Hosted(_)` to `Hosted(_scope)` so its enum
payload counts as read. Unsupported-platform behavior is unchanged. Focused
rustfmt and diff checks passed; no native compilation or test rerun is claimed.
UI/owner source is unchanged, preserving the reviewer's 21/21 browser and prior
owner evidence. Production remains 13 files/net +478. Reviewer owns closure.

## Review corrections (2026-11-17)

Conductor accepted H-1/M-1/M-2/M-3 from the unchanged
[review report](../audits/post-audit/2722-hosted-management-followthrough.md) for
fix-in-lane. Rework began at clean `0ce26a88cd81caf3e9fe85370a9896ddf22f306f`.
Signed correction source commit: `0e0839640f1af0b024343463926baeb767868271`.
The human authorized net +500 for these four fixes only, preserving the same
13 production files. Measured correction: four existing production files,
net +29; aggregate **13 files, +570/-92, net +478**.

- H-1: Windows-only Start dispatch is platform-gated; other platforms return an
  explicit unsupported error without compiling the Windows journal module.
- M-1: hosted selected-record Save converges command, arguments and MCP command
  from the chosen metadata regardless of an already-committed definition. Missing
  metadata preserves offline unchanged-selection behavior. The regression now
  changes Grok to Claude, fails the record write, then retries after definition
  refetch and checks the exact installed ID and Claude's empty argument vector.
- M-2: only id-bearing existing-definition Edit gets hosted context. Duplicate
  uses main's local catalog/create path. The regression verifies local backend,
  immediate spawn intent, and no host discovery/prepare calls with host unavailable.
- M-3: initial legacy context is resolved and frozen per dialog before fields are
  exposed. The existing editor shell/footer shows loading/error and usable Cancel.
  Later query results preserve authored instructions and the initial exact runtime.
  A source key remount prevents the editor's source-change effect from clearing a
  newly seeded hosted selection on reopen.

Rework validation: `pnpm exec playwright test --project=smoke hosted-agent-setup.spec.ts`: **21 passed**, 1.7m. `pnpm build:e2e` (including `tsc`),
`pnpm typecheck`, affected Biome, Rust formatting and diff checks passed. Final
source build has only the existing Vite chunk warnings. Initial regression runs
caught the reopen reset (fixed); Duplicate fixture iterations corrected its local
availability, default-suffixed label and expected empty args, matching main's
`instanceInputForDefinition.ts` local path (args resolve at spawn). No local-path
production behavior was changed to satisfy those test expectations.

Local logs: `/tmp/2722-review-fixes-{build,typecheck,focused,duplicate,browser}.log`.
Owner/host logic is unchanged; earlier green evidence remains applicable and those
suites/mutations were not repeated. Native Windows/Linux compile, Windows tests,
`just ci` and human acceptance remain not run; source inspection is not a compile
claim. Re-gate belongs to the same independent reviewer, followed by conductor's
Windows workflow. No additional feature or live-system change was made.

## Scope and source

Workspace `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`. Ownership began clean at
`bd46993f30c9395d24b971e4520706a7f8f0dba7`; predecessor F2/F3 commits are preserved.
Requested role/settings: codecodex, Astra/high; actual hidden runtime identity and
reasoning configuration are unknown. No live agents, services, provider calls,
relay/hostd production changes, schema, push, release, CI dispatch or taskctl actions.

Active source: `/home/bryce/services/buzz-agents/docs/specs/2722-hosted-save-next-spawn.md`,
top execution authorization and superseding narrowing amendment. The human approved
initial continuation to +450 after the +370 estimate proved too small, then
**13 production files / net +500 for the four specific review corrections**. Historical online Save validation, desired/applied UI and 18-file
proposals are superseded. The approved contract is:

- Edit uses the exact selected hosted instance and its host context. Canonical harness
  metadata stays distinct from the installed runtime ID. Legacy instances resolve one
  exact retained context or fail visibly; no local-machine fallback.
- Save performs ordinary definition and selected-record writes offline, retains an
  omitted installed runtime, preserves the draft on partial failure, and says saved
  changes take effect on next start. Save neither probes deployment nor interrupts.
- Explicit Start projects main's current effective settings and current identity,
  validates current host runtime/model facts, and uses the existing revisioned Deploy.
  Pending retries retain immutable bytes. Current revision, peer, cleanup and scope
  protections remain. The `existing_start` marker prevents recovery from overwriting
  later saved settings. Host active-body refusal and inactive higher-revision reuse
  remain existing behavior; existing `!shutdown` is separate from this change.

## Commits and measured production budget

`8c0603fe8bf57155c5e4250bd6ba2791c7ec4894` preserved the paused native WIP.
`c542979eb` completes native projection/recovery and focused owner/host/native fixtures.
`dcd9658aa` completes the editor, ordinary Save feedback and browser coverage.
All carry DCO sign-off. The WIP deliberately bypassed hooks to honor the stop;
completion commits used ordinary `git commit -s` (no installed pre-commit/commit-msg
hooks in this checkout). Documentation follows in a separate signed commit.

Measured with `git diff bd46993 --numstat`; production excludes existing test files,
AGENTS guidance and docs. Normal rustfmt/Biome formatting; no compression to fit.

| Production file | Added | Removed | Net |
| --- | ---: | ---: | ---: |
| `crates/buzz-paired-owner/src/deployment/continuation.rs` | 75 | 9 | 66 |
| `desktop/src-tauri/src/commands/agent_models_update.rs` | 30 | 1 | 29 |
| `desktop/src-tauri/src/commands/agents.rs` | 16 | 4 | 12 |
| `desktop/src-tauri/src/commands/agents_hosted.rs` | 225 | 30 | 195 |
| `desktop/src-tauri/src/commands/agents_hosted_cleanup.rs` | 2 | 8 | -6 |
| `desktop/src-tauri/src/managed_agents/types.rs` | 3 | 0 | 3 |
| `desktop/src-tauri/src/managed_agents/types/requests.rs` | 3 | 0 | 3 |
| `desktop/src/features/agents/ui/AgentDefinitionDialog.tsx` | 21 | 6 | 15 |
| `desktop/src/features/profile/ui/UserProfilePanel.tsx` | 10 | 2 | 8 |
| `desktop/src/features/profile/ui/UserProfilePanelPersonaSubmit.ts` | 32 | 5 | 27 |
| `desktop/src/features/profile/ui/UserProfilePersonaDialogs.tsx` | 150 | 24 | 126 |
| `desktop/src/features/profile/ui/useAgentLifecycleActions.ts` | 5 | 3 | 2 |
| `desktop/src/shared/api/types.ts` | 2 | 0 | 2 |
| **13 files** | **574** | **92** | **482** |

Approved scope-neutral substitution: `AgentDefinitionDialog.tsx` replaces planned
`AgentDialog.tsx`, since the shared definition editor required fresh models for Edit.
The unchanged saved-selection exception is restricted to hosted Edit; Create and
changed selections retain model evidence requirements. The existing profile wrapper
uses the existing run-location provider directly. Its next-start description is
rendered explicitly because the shared shell did not display its description prop.
The native adapter exceeded its initial estimate because it must compose existing
locking, session/catalog, retained-intent, current-record projection, model admission,
recovery and dispatch seams; no new state/wire/store mechanism was introduced.

## Verification actually run

Commands ran with `HERMIT_SELF_UPDATE=false` and `. ./bin/activate-hermit`.
Local socket fixtures needed sandbox escalation; no external endpoint was used.

| Check | Measured result and scope |
| --- | --- |
| `cargo test -p buzz-paired-owner --lib` | **17 passed**, 0 failed; includes expanded existing revision/exact-retry journey |
| `cargo test -p buzz-paired-owner --lib deployment_current_revision_refresh_launch_and_exact_retry -- --nocapture` | Restored source **1 passed**, 16 filtered, 12.44s |
| `cargo test -p buzz-hostd --lib prepare_replay_restart_and_identity_fences_precede_effects -- --nocapture` | **1 passed**, 31 filtered; synthetic manager verifies same unit/key, active refusal, inactive higher revision |
| `cargo test -p buzz-hostd --lib service_actions::tests:: -- --nocapture` | **7 passed, 1 existing ignored**, 24 filtered; ignored actual ACP case requires `BUZZ_TEST_ACP` |
| `pnpm typecheck` in desktop | **PASS** |
| `pnpm build:e2e` in desktop | **PASS**, including final restored build (`tsc` + Vite); ordinary chunk warnings |
| `pnpm exec playwright test --project=smoke hosted-agent-setup.spec.ts` | **18 passed**, 1.5m; actual UI, mocked IPC boundary |
| Same Playwright command with `-g 'profile hosted Edit'` | Final strengthened and restored case **1 passed**, 13.4s; offline Save, exact runtime, partial failure/draft, Start wording/identity |
| Biome check on six changed frontend production files plus hosted browser spec | **PASS**, 7 files, no fixes |
| `rustfmt --check --edition 2021 --config skip_children=true` on all eleven touched Rust production/test files | **PASS** |
| `git diff --check` | **PASS** |

The full browser suite ran before final test-only strengthening (closing/reopening
profile to discard local model cache and asserting Start). The final case then passed
separately with those assertions, and passed again after mutation restoration. Other
17 cases and production source were unchanged by that strengthening.

Falsification: temporarily bypassing owner `evidence.current` admission made the
existing journey fail at the `advanced` assertion; restored bytes passed. Temporarily
disabling the hosted unchanged-selection exception made the final profile journey
fail at the offline Save enabled assertion (`hosted-agent-setup.spec.ts:1118`). Both
mutations were restored byte-for-byte; restored browser bundle and case passed.
Raw local logs are `/tmp/2722-edit-{owner-tests,owner-mutation,owner-restored,host-test,service-guards,browser-suite,profile-final,browser-mutation,restored-build,restored-browser}.log`;
this committed receipt is the durable sanitized evidence index.

Historical failures: paused WIP owner test attempted `Command.clone()` and did not
compile; replaced with existing serialization/restore and the compiled suite passed.
Initial browser iteration exposed a Create-name reset from an added effect dependency;
the dependency was removed with a narrow lint explanation, and full Create cases passed.
Sandbox loopback denial was environmental; allowed local reruns passed.

## Not run and next gate

Desktop native Linux/Windows compile/tests were **not run** for this implementation.
The known Linux GLib/WebKit dependencies are unavailable; no environment installation
was attempted. The existing Windows-filtered
`hosted_native_create_recovery_and_session_fence` fixture now covers latest definition
projection, exact installed runtime, mapping/inventory rejection, immutable retained
Start recovery, later Save preservation, identity/auth/missing-record refusal and
session fencing. Its cleanup sibling remains in the existing workflow. Source
inspection covered RegisteredHost constructors/callers; this is not compilation proof.
The fixture exercises candidate/recovery seams, not the entire live Start transport.
Browser coverage cannot prove native persistence or network delivery.

`just ci`, native platform gates, real provider/service runs, human acceptance and
independent review are **not run**. The next gate is the same independent reviewer re-gating the four correction
findings, then the existing Windows
workflow as authorized by the conductor. See [review brief](../briefs/2722-hosted-edit-start-review.md).
No product decision remains open within this bounded contract. Final process scan found no task-owned test/build/browser process and `/proc/net/tcp*`
contained no port 4173 listener. All tool sessions were terminal before the signed
documentation commit; clean source is checked again in the final handoff.
