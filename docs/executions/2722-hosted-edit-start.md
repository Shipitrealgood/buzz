# #2722 hosted Edit + explicit Start — paused WIP

**Incomplete, unverified, not release-ready.** Implementation stopped at the
approved growth boundary before frontend changes. No increased budget is approved.
This is a preservation commit, not an implementation or review acceptance.

## Scope and source

Workspace `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`. Exclusive ownership started at clean
`bd46993f30c9395d24b971e4520706a7f8f0dba7`, after the F2/F3 worker and processes
finished. Those predecessor changes are preserved. No unrelated dirty files existed.
Requested role/settings: codecodex, Astra/high; hidden runtime identity and effort
are unverified. No live, provider, relay, hostd, schema, push, release, CI dispatch,
taskctl or working-agent actions occurred.

Active contract: `/home/bryce/services/buzz-agents/docs/specs/2722-hosted-save-next-spawn.md`,
top Execution authorization and Superseding narrowing amendment. Historical
online Save validation, desired/applied UI and 18-file proposal are superseded.
Ordinary Save uses the existing definition and selected-record writes offline;
partial errors must be honest. Exact installed runtime is optional backend data.
Explicit Start validates current host execution and uses existing revisioned
Deploy, retaining identity and immutable retries. Save does not interrupt a body.
One `existing_start` materialization marker prevents Start recovery from reinstalling
old desired settings. Existing cleanup, revision, peer and setup guards remain.
Approved ceiling: 13 production files, net +370 lines, tests/guidance excluded.

## Measured partial diff against the ownership pin

| Production file | Added | Removed | Net |
| --- | ---: | ---: | ---: |
| `crates/buzz-paired-owner/src/deployment/continuation.rs` | 75 | 9 | 66 |
| `desktop/src-tauri/src/commands/agent_models_update.rs` | 30 | 1 | 29 |
| `desktop/src-tauri/src/commands/agents.rs` | 8 | 4 | 4 |
| `desktop/src-tauri/src/commands/agents_hosted.rs` | 204 | 29 | 175 |
| `desktop/src-tauri/src/commands/agents_hosted_cleanup.rs` | 2 | 8 | -6 |
| `desktop/src-tauri/src/managed_agents/types.rs` | 2 | 0 | 2 |
| `desktop/src-tauri/src/managed_agents/types/requests.rs` | 3 | 0 | 3 |
| **7 production files** | **324** | **51** | **273** |

One existing owner test file adds 132 lines. Documentation is additional.
Normal rustfmt formatting is used; no minification or statement packing was used
to hide growth.

The `agents_hosted.rs` estimate was +75–105; measured +175. Extracting the existing
candidate projection avoided a duplicate settings resolver, but the explicit
Start entry still needs per-agent serialization, fresh scoped session/catalog,
exact retained-context/latest-operation selection, immutable pending retry,
locked current-record read, current model discovery, typed admission, recovery,
dispatch and summary. The old create/open/recover helpers do not expose that whole
existing-agent entry. Marker-aware recovery and scope-only backend identity checks
also add code. This is an underestimated adapter, not evidence of a new wire or
state mechanism. The remaining work cannot honestly be promised within 97 lines.

## Remaining six production files and estimate

Conductor approved one scope-neutral substitution after inspection showed
`AgentDefinitionDialog.tsx:552` requires fresh returned hosted models for both
Create and Edit. That contradicts offline Save. Use the existing definition editor
directly in `UserProfilePersonaDialogs` with the existing run-location provider;
edit `AgentDefinitionDialog.tsx` instead of planned `AgentDialog.tsx`. No frontend
part of that substitution has been implemented.

| Remaining file | Necessary work | Estimated net |
| --- | --- | ---: |
| `desktop/src/shared/api/types.ts` | Installed runtime backend/update fields | 4–6 |
| `desktop/src/features/profile/ui/UserProfilePanel.tsx` | Exact clicked instance/context and submit forwarding | 2–3 |
| `desktop/src/features/profile/ui/UserProfilePersonaDialogs.tsx` | Scoped host catalog/retained context, stale-result fence, installed selection, existing editor/context | 75–95 |
| `desktop/src/features/profile/ui/UserProfilePanelPersonaSubmit.ts` | Canonical/installed split, ordinary Save, truthful partial error and next-start copy | 30–38 |
| `desktop/src/features/agents/ui/AgentDefinitionDialog.tsx` | Narrow unchanged-selection offline Edit gate; preserve Create/changed-selection validation | 6–9 |
| `desktop/src/features/profile/ui/useAgentLifecycleActions.ts` | Hosted Start requested wording | 3–4 |
| **Remaining estimate** | **Not measured or approved expansion** | **120–155** |

Projected complete production delta is net +393–428 before corrections discovered
by verification. No new scope-neutral reduction was established; shared projection
and continuation reuse are already in the held patch. Do not reduce required
identity/revision/recovery guards or compress formatting to fit the budget.

## Verification and actual failure

- `rustfmt --edition 2021` ran on the owned Rust files; formatting completed.
- Final `rustfmt --check --edition 2021 --config skip_children=true` on all eight
  owned Rust files: PASS.
- `git diff --check`: PASS.
- WIP signed commit uses `-c core.hooksPath=/dev/null` to honor the stop
  instruction and avoid implicit extra checks/mutations; commit hooks did not run.
- One targeted test launched before the stop instruction:
  `cargo test -p buzz-paired-owner deployment_current_revision_refresh_launch_and_exact_retry -- --nocapture`.
  **FAIL, exit 101, before tests executed.** New test code calls `command.clone()`
  at `deployment_tests.rs:892`, but `host_deployment::Command` is not Clone.
  A second candidate clone must also be removed/reconstructed when authorized.
  The tool session was already terminal when interrupt/poll returned the compiler
  receipt. No test relaunch or repair followed the conductor's stop instruction.
- No native desktop compile, native Windows tests, browser/typecheck, host inactive
  tests, full owner suite, mutation proof, `just ci` or independent review ran for
  this partial patch. Predecessor F2/F3 evidence does not validate this code.

## Known concerns and next authorization

The patch has not established native compilation or end-to-end correctness.
The new Start candidate compares effective command and arguments to host metadata;
explicit canonical definition-runtime equality still needs review/verification.
Exact runtime/model rejection, stale session/scope, access-policy refusal, cleanup
fencing, expired/nonadmitted Start, immutable retries and preservation of a later
Save need the planned native/owner fixtures. The new owner test is uncompiled and
must not be represented as coverage. No offline Edit or partial-Save UI exists yet.
Required agents guidance has not been updated to claim unfinished behavior.

Next gate is conductor/user scope disposition, not merge or release. Resume only
under an explicitly authorized bounded contract. Then finish implementation,
repair and run focused tests, update guidance, and hand the complete committed
range to a fresh independent reviewer. See
[paused review/handoff index](../briefs/2722-hosted-edit-start-paused.md).

All task-owned tool processes are terminal; no server was started. Worktree is
committed clean at handoff. Hermit used `HERMIT_SELF_UPDATE=false`; no tooling
installation or host-cache changes were made.
