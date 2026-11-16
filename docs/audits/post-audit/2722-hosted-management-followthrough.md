# #2722 hosted management — independent post-audit

**Verdict: merge-block. C=0 / H=1 / M=3 / L=0.** Initial review, rework round 0.
Source acceptance requires H-1 and M-1–M-3 to be resolved or explicitly ruled on by
the conductor. Native Windows and human acceptance remain outstanding separately.

Reviewer: codex-chuck. Dispatch requested `gpt-6-astra`, high. Actual served model
and reasoning effort are not exposed by authoritative runtime metadata; unknown.
Reviewed on 2026-11-16. Claims below distinguish measured checks from source traces.

## Pin and contract

Worktree: `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`. Clean source received at
`1eb2ce93116cad9a242dd82137c1ec4b43d96de5`. Combined review:
`f95d075b365486a432732d603ba4511cf19fe7ff..1eb2ce93116cad9a242dd82137c1ec4b43d96de5`.
Edit/Start subrange begins at `bd46993f30c9395d24b971e4520706a7f8f0dba7`.
This report commit changes no product or test source.

Governing inputs: the approved continuation and superseding narrowing amendment in
`/home/bryce/services/buzz-agents/docs/specs/2722-hosted-save-next-spawn.md`, the
[Edit/Start brief](../../briefs/2722-hosted-edit-start-review.md) and
[execution receipt](../../executions/2722-hosted-edit-start.md), and the
[F2/F3 brief](../../briefs/2722-hosted-status-normal-launch-review.md) and
[receipt](../../executions/2722-hosted-status-normal-launch.md). Read applicable
AGENTS, vision, architecture and testing guidance, plus the portable conductor and
post-audit contracts. The original management specification's stronger Save
transaction and online-validation requirements are superseded.

Accepted scope is ordinary offline definition/record Save, truthful partial-save
feedback, exact host/runtime selection, explicit same-identity Start through the
existing deployment journal, non-overwriting retained Start recovery, truthful
status, and normal Windows enablement. No new settings manager, automatic Stop or
restart, relay/hostd production change, schema, login or isolation work. No live
agent/service, external provider, board, push, CI dispatch or deployment action was
taken. Conductor retains all repair and release routing authority.

**Measured size:** Edit/Start is 13 production files, +539/-90, **net +449**;
combined range is 15 distinct production files, +630/-137, **net +493**. F2/F3's
separate +44 accounts for the difference; `agents_hosted.rs` overlaps. Test files,
required AGENTS guidance and docs are excluded. Normal formatting is retained;
no compressed-code budget workaround was found.

## Findings

### H-1 (High, high confidence) — unconditional reference to Windows-only module

**Read/inferred, merge blocker.**
[agents.rs](../../../desktop/src-tauri/src/commands/agents.rs):418–420 declares
`hosted` under `#[cfg(windows)]`. The new match arm at line 1060 unconditionally
calls `hosted::start`; its enum variant and branch at 994 and 1039 are also
unconditional. Linux/macOS compilation removes the module but still resolves the
match arm, producing an unresolved-module error. The runtime `enabled()` predicate
inside the removed module cannot guard name resolution.

Remedy: platform-gate the hosted dispatch and retain an explicit unsupported-platform
error, following the existing gated hosted creation/deletion seams in the same
file. Do not remove platform admission or compile Windows-only journal code on
other platforms. This is a source-derived compile defect; no native compilation
was claimed or attempted in the missing-dependency environment.

### M-1 (Medium, high confidence) — cross-harness Save retry drops required args

**Read/inferred, required correctness fix.**
[UserProfilePanelPersonaSubmit.ts](../../../desktop/src/features/profile/ui/UserProfilePanelPersonaSubmit.ts):99–119
derives the instance patch using `previousPersona`, then saves the selected
installed ID. The existing helper only emits harness args when the **definition**
runtime changed:
[UserProfilePanelUtils.ts](../../../desktop/src/features/profile/ui/UserProfilePanelUtils.ts):317–332.
But `updatePersona` invalidates and awaits both persona and managed-agent queries
([hooks.ts](../../../desktop/src/features/agents/hooks.ts):533–554), and the next
submit uses the refreshed `resolvedPersona` as `previousPersona`
([UserProfilePanel.tsx](../../../desktop/src/features/profile/ui/UserProfilePanel.tsx):527–559).

Concrete sequence: edit an existing Grok agent to an installed Claude harness.
Definition Save succeeds; the instance write fails. The preserved draft is retried
after refetch. Both previous and submitted persona now say `claude`, so the retry
omits `agentArgs` while successfully saving the Claude installed ID/model and
showing “Saved. Takes effect on next start.” The record still contains Grok's
`["agent", "--always-approve", "stdio"]` args
([presets.rs](../../../crates/buzz-core/src/harness/presets.rs):74–77).
Native Start rejects them against Claude's empty default args at
[agents_hosted.rs](../../../desktop/src-tauri/src/commands/agents_hosted.rs):105–125.
Reopening and resaving the same selection also has no definition-runtime delta.

Remedy: make the hosted selected-record patch converge to the explicitly chosen
canonical harness metadata independently of whether definition Save already
committed. Preserve offline unchanged-selection behavior when metadata is absent;
do not add remote validation or a Save journal. Extend the existing partial-Save
case with two *different canonical* harnesses. Its current two Grok IDs cannot
expose this failure, and its mocked Start does not run native validation
([hosted-agent-setup.spec.ts](../../../desktop/tests/e2e/hosted-agent-setup.spec.ts):1021–1132).

### M-2 (Medium, high confidence) — hosted Edit context also captures Duplicate

**Read/inferred, required regression fix.**
[UserProfilePersonaDialogs.tsx](../../../desktop/src/features/profile/ui/UserProfilePersonaDialogs.tsx):105–143
chooses hosted source from `managedAgent.backend` for every non-null dialog state,
without checking whether the input is an existing-definition edit. Duplicate is
reachable for hosted managed profiles and creates a create-mode state
([UserProfilePanel.tsx](../../../desktop/src/features/profile/ui/UserProfilePanel.tsx):561–563,835–836;
[personaDialogState.ts](../../../desktop/src/features/agents/ui/personaDialogState.ts):65–93).

Consequently Duplicate now presents host-installed choices and requires host model
discovery, but submission's create branch ignores `hosted` and calls
`createManagedAgentForPersona`
([UserProfilePanelPersonaSubmit.ts](../../../desktop/src/features/profile/ui/UserProfilePanelPersonaSubmit.ts):130–139).
That callback resolves local available runtimes and builds an instance without a
host destination ([UserProfilePanel.tsx](../../../desktop/src/features/profile/ui/UserProfilePanel.tsx):424–444).
With the same harness installed locally, the duplicate starts on Windows despite
configuration being sourced from the host. Without it, the new definition is saved
and local creation fails. A host outage also newly blocks an otherwise local
duplicate flow.

Remedy: limit this wrapper's hosted source, runtime remapping and run-location
context to the authorized existing-instance Edit intent. Preserve Duplicate's
previous local behavior; adding hosted duplication is a separate product change.

### M-3 (Medium, high confidence) — delayed legacy context resets authored draft

**Read/inferred, required agreed-contract fix.** Legacy records omit
`installed_runtime`, so their initial exact ID comes asynchronously from
`listHostedSetups`
([UserProfilePersonaDialogs.tsx](../../../desktop/src/features/profile/ui/UserProfilePersonaDialogs.tsx):116–143).
The editor nevertheless opens immediately at line 219, and only mutation state
is passed as `isPending` at line 195. Name and instructions remain editable while
the context read is unresolved. When the exact retained context arrives,
`installed` changes, creating a new `initialValues` object. The shared editor's
`[initialValues, open]` effect restores all original fields and clears dirty state
([AgentDefinitionDialog.tsx](../../../desktop/src/features/agents/ui/AgentDefinitionDialog.tsx):218–252).

Concrete sequence: open a legacy hosted Edit while its local retained-journal IPC
is delayed, type instructions, then let the IPC complete. The new instructions
are silently replaced by the saved seed. This violates the required stable draft
and “resolve initial context before seeding editor” behavior. An error paragraph
does not disable these inputs.

Remedy: resolve and freeze the exact initial legacy context before exposing the
editable form, while keeping a visible loading/error/Close state; subsequent
query results must not reseed an authored draft. Keep the existing query and
editor, with no parallel state framework. A delayed response in the existing
profile fixture is sufficient regression coverage.

## Proposed dispositions

All findings remain **pending conductor ruling**; these are recommendations, not
board changes or accepted dispositions.

| Finding | Proposed owner and next action |
| --- | --- |
| H-1 | Existing executor: platform-gate the Start route, then source re-gate and required platform checks. |
| M-1 | Existing executor: converge the selected-record harness fields on retry; focused cross-harness partial-Save case. |
| M-2 | Existing executor: restrict hosted remapping to Edit; preserve Duplicate routing. |
| M-3 | Existing executor: gate initial legacy editor seeding; delayed-context case. |

Keep the human's 13-file/+450 Edit/Start ceiling. If the necessary corrections
cannot fit through ordinary reuse, return the measured overrun to the conductor;
this report grants no budget expansion. Minimalism 9/10: existing owners reused,
no new store or transport. Elegance 8/10 and correctness 7/10 because H-1 and
M-1–M-3 leave platform/intent/lifecycle boundaries inconsistent; the remedies above
are the concrete changes needed before acceptance.

## Coverage and verification

Full combined diff and relevant producers/callers were inspected. The native
entry trace was `start_managed_agent` → per-agent locked `hosted::start` → current
record/effective payload → exact catalog/model validation → typed owner admission
→ marker-aware recovery → existing dispatch. Source review confirms the new
marker branch compares current scope/key/auth/relay and refuses a missing record
without reinstalling its snapshot
([agents_hosted.rs](../../../desktop/src-tauri/src/commands/agents_hosted.rs):422–438).
Owner continuation retains current-revision, competing-intent, expiry,
identity and cleanup guards; setup admission still restricts changes to its
model-only snapshot ([continuation.rs](../../../crates/buzz-paired-owner/src/deployment/continuation.rs):138–309).
No additional concrete native identity or cleanup defect was found.

F2's neutral missing/expired evidence, exact-key presence and reachable recovery
were independently exercised by the existing browser suite. F3's env-absent native
fixture and remaining session/signer/capability/registration gates were read,
not executed. No extra architecture, permission framework, mutation campaign or
test matrix was introduced. Conductor expressly chose source-only findings for
the additional edge cases before executor repair.

| Independent command / evidence | Result and boundary |
| --- | --- |
| `cargo test -p buzz-paired-owner --lib` | **17 passed**, 12.46s, after scoped loopback permission. First sandbox run: 13 passed/4 socket PermissionDenied; environmental, retained in log. |
| `pnpm exec playwright test --project=smoke hosted-agent-setup.spec.ts` in desktop | **18 passed**, 1.5m. Existing restored E2E bundle; actual components, mocked IPC. Includes final strengthened offline profile case. |
| `cargo test -p buzz-hostd --lib prepare_replay_restart_and_identity_fences_precede_effects -- --nocapture` | **1 passed**, 31 filtered. Synthetic same-unit/key and active-body-refusal fixture; no real service. |
| `cargo test -p buzz-hostd --lib service_actions::tests:: -- --nocapture` | **7 passed / 1 existing ignored**, 24 filtered, after scoped permission. Initial sandbox run: 6 passed/1 socket PermissionDenied/1 ignored. Ignored case requires `BUZZ_TEST_ACP`. |
| `git diff --check f95d075..1eb2ce9`; production numstat recount | **PASS**; measured counts above. |

Local raw logs: `/tmp/2722-chuck-owner{,-allowed}.log`,
`/tmp/2722-chuck-browser.log`, `/tmp/2722-chuck-host.log`,
`/tmp/2722-chuck-service{,-allowed}.log`. This committed report is their durable
sanitized result index.

Executor typecheck/build/Biome/rustfmt evidence is retained from the linked
receipts, **not independently rerun**. Mutation receipts were inspected: owner
current-revision bypass failed and restored owner case passed; browser offline
selection bypass failed and restored case passed; F2 catalog-presence mutation
and restored suite are recorded in its receipt. No reviewer mutant was run.
Continuity: no product changes since the executor's restored bundle; the final
test-only profile strengthening is present at this pin and independently passed
in the full 18-case run. Independent owner results also establish the green
unmutated source baseline; no mutation-kill claim is attributed to this reviewer.

**Not run:** native Windows build/tests, native Linux/macOS compile, installed
normal Windows launch, real Save/Stop/Start provider/service delivery, `just ci`,
human acceptance. Linux GLib/WebKit prerequisites are known unavailable; no
installation or workaround was attempted. The existing Windows workflow selects
the extended native phases and cleanup cases
([windows-canary.yml](../../../.github/workflows/windows-canary.yml):295–309),
but that fixture currently exercises candidate/recovery seams rather than the
whole live Start transport. Browser success cannot substitute for native proof.

All reviewer tool sessions terminated. No product/test mutation was made or held;
no port 4173 listener remained at handoff. Only this report is committed with DCO
sign-off. Next action: conductor routes the four findings to the existing executor,
then this reviewer re-gates the updated pin; Windows/human gates remain subsequent
obligations, with no release approval implied.

quality: model=unknown effort=unknown stage=post-audit verdict=merge-block verify=partial commits=f95d075b365486a432732d603ba4511cf19fe7ff..1eb2ce93116cad9a242dd82137c1ec4b43d96de5 findings=C=0/H=1/M=3/L=0 rework_rounds=0
