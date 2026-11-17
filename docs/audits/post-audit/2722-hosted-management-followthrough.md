# #2722 hosted management — consecutive recovery phase re-gate, round 5

**Scoped source correction accepted; open findings C=0 / H=0 / M=0 / L=0.**
Reviewed clean `55545416c6cecc5f9c69dd34e5c57c07e3293ef7` against
`a86cfe66a695618dae3f7a2b039adc2a7eeff585`, same exclusive worktree.
One production line, the existing native fixture and receipts changed. The prior
13 production files/net +481 therefore becomes **13/net +482**, within +500.
Requested Astra/high; actual served reviewer model/effort unknown.

**Confirmed production defect and correction (read, high confidence):** real
[start_existing_deployment](../../../crates/buzz-paired-owner/src/deployment/continuation.rs):108
marks the snapshot and goes through `continue_intent` → `retain_deployment`.
[deployment.rs](../../../crates/buzz-paired-owner/src/deployment.rs):187 retains phase 0;
its advancement guard at lines 237–239 correctly permits only the same or next phase.
The former existing-Start recovery's direct 0→2 was invalid. This source review
previously missed that transition; the native failure is real production evidence,
not a fixture setup error.

The added [agents_hosted.rs](../../../desktop/src-tauri/src/commands/agents_hosted.rs):434
advances to 1 before 2, after session/competing-intent checks and exact saved
pubkey/backend scope/private key/auth tag/relay validation. This branch neither
rewrites main settings nor publishes a replacement profile. `advance` at line 33
reloads the intent and skips an already reached phase: phase-1 retry advances only
to 2, while phase-2/3 retries do not regress. Each real journal advancement retains
the original event/materialization and commits separately; failure of step 2 leaves
phase 1 retryable. The consecutive journal guard is unchanged. Disposition: this
platform-discovered Start defect is **closed in source, pending native execution**.
Prior H-1/M-1/M-2/M-3 remain closed; no introduced blocker found.

**Fixture teeth (read):**
[agents_hosted_tests.rs](../../../desktop/src-tauri/src/commands/agents_hosted_tests.rs):547
asserts the true phase-0 start, then requires the specific `fixture phase two failure`
error, phase 1, unchanged exact signed event and unchanged later-saved main record.
The trigger is dropped and its connection closed before reopening at phase 1 and
again at phase 2. Both recoveries require success, phase 2 and the same event/record.
This cannot pass merely because an unrelated earlier guard refused recovery.
The trigger predicate matches phase-2 updates generally, not an operation ID;
isolation comes from the temporary fixture database, no concurrent writer during
this interval, and production UPDATE's exact community/owner/operation predicates
(`deployment.rs:246`). Only the current operation is updated in this recovery path.

The remaining main fixture sequence was read through the end: linked-definition
model/prompt expectations, runtime refusal, identity/missing-record/session refusals
remain intact. The separate cleanup branch returns before this Start scenario;
its retained phase 2, dispatch/reconciliation phase 3 and finalization phase 4
remain consecutive. The unchanged cleanup fixture's durable failure/reopen,
finalization locks, retained deletion events and stale-session/repeated completion
checks were traced; no analogous phase jump was found. These are source conclusions,
not claims that unexecuted native assertions passed.

**Measured:** exact fixture trigger SQL independently executed against local Python
SQLite: phase 1 commits; phase 2 raises the named error and rolls back; another
operation is unchanged; dropping the trigger allows phase 2. This validates the
failure control only, not Rust recovery. Exact diff/whitespace checks passed.
No suites, mutants or product modifications were made. Prior independent browser
21/21 and owner/host results are retained for unchanged areas; they do not verify
this new native recovery line. Executor rustfmt is receipt evidence only.

**Platform status:** read preserved `native-job.log` lines 2967–2987 for run
43993874579 and its [committed receipt](../../executions/2722-hosted-edit-windows.md).
Desktop build/packaging and SQLite ACL passed; 122 cases passed, but exact hosted
create/recovery failed with `nonmonotonic intent phase`. Cleanup and Grok metadata
cases were skipped. Corrected native recovery/cleanup, other platform checks and
human installed normal-launch/Edit/Stop/Start acceptance remain **not run/pending**.
The earlier installer remains diagnostic. Return to conductor for Windows rerun;
no unconditional merge-safe verdict while required gates remain incomplete.

All reviewer commands terminal; no server or mutation remains. Only this signed
report is committed. No publication, CI dispatch, live action or taskctl operation.
Historical sections below are superseded by this pin and platform status;
`rework / partial` denotes remaining platform/human gates, not further source repair.

Signed: codex-chuck — independent #2722 reviewer, 2026-11-17.

## Historical native fixture correction re-gate — round 4

**Scoped test correction accepted; open findings C=0 / H=0 / M=0 / L=0.**
Reviewed clean `dd3ad12a85e966abaad17f41faf8bab3d03ee4f7` against
`dc6088571d16e970b7839cf2f380ef57a3251636` in the same exclusive worktree.
The exact delta is one existing test fixture and two documentation files;
production is unchanged (previous 13 files/net +481 remains applicable).
Requested reviewer Astra/high; actual served model/effort unknown.

**Source trace (read, high confidence):** the old fixture changed `definitions[0]`.
[personas.rs](../../../desktop/src-tauri/src/managed_agents/personas.rs):342 loads
and merges definitions; `sort_personas` at line 164 places built-ins first, with
`builtin:fizz` first in the built-in order. The test's instance instead links
`one-definition` (fixture lines 90 and 139). Production
[candidate](../../../desktop/src-tauri/src/commands/agents_hosted.rs):87 calls
[build_deploy_payload](../../../desktop/src-tauri/src/commands/agents_deploy.rs):208,
which invokes
[resolve_effective_config](../../../desktop/src-tauri/src/managed_agents/effective_config/mod.rs):247.
That resolver finds the exact `persona_id` and resolves model/prompt from its
definition. The old index edit left that definition at `initial-host-model`.
This explains the observed result without a production precedence defect.

The corrected [fixture](../../../desktop/src-tauri/src/commands/agents_hosted_tests.rs):442
selects the linked ID, verifies its initial model, writes the nondefault model and
prompt, then reloads and asserts both exact values before calling candidate.
The stale instance values remain deliberate. Candidate assertions still demand
`saved/model:2` and `Saved host instructions` (lines 489–499). Later-Save main-record
equality and exact retained-event equality (lines 552–560), plus identity and
missing-record recovery refusals, are unchanged. The correction strengthens setup
verification without weakening model precedence or immutable recovery assertions.
No remaining defect was found in this bounded delta; prior findings remain closed.

**Platform evidence:** read the actual saved-model diagnostic in preserved
`native-job.log`, lines 2956–2988, for Windows run 43988951035. The
[committed CI receipt](../../executions/2722-hosted-edit-windows.md) records successful
Desktop compilation/packaging and SQLite ACL verification, 122 distinct passing
cases, and the failed hosted create/recovery case. Cleanup/session-fence and Grok
metadata cases were skipped after failure. These are CI-worker evidence, not
reviewer-run tests; the installer is diagnostic, not an accepted canary.

**Measured here:** clean pin and exact diff inspected; `git diff --check` passed.
No suites or mutants were rerun for this test-only correction. Previous independent
21/21 browser and owner/host evidence is retained because production is unchanged.
Executor focused rustfmt is receipt evidence only. Corrected native Windows case
is **not run** locally; successful Windows native verification and human installed
normal-launch/Edit/Stop/Start acceptance remain pending. No full merge-safe claim.

Return to conductor for the authorized Windows rerun. No product edits, CI dispatch,
publication, live actions, or taskctl operations. All reviewer commands terminal;
no reviewer server was started. Only this signed report is committed. Historical
sections below are superseded by this pin and platform status. `rework / partial`
continues to denote outstanding native/human acceptance, not another source repair.

Signed: codex-chuck — independent #2722 reviewer, 2026-11-17.

## Historical Windows compiler correction re-gate — round 3

**Scoped source correction accepted. Open source findings C=0 / H=0 / M=0 / L=0.**
Windows acceptance remains pending a successful corrected build and native cases.
No browser/owner suite reruns, new tests, product edits or live actions were made.

Clean reviewed pin: `bd03a6c4845597f1975ac5cd2f8e6ee0aebc8f3c`, same exclusive
worktree. Delta from previously reviewed `21099d751150a219e4c3d342aafea9ff04495f0e`:
one native call correction plus execution/CI receipts; parent `ec804bf9` is docs only.
Independent recount confirms **13 production files, net +481**, below the approved
+500. Requested reviewer Astra/high; actual served model/effort unknown.

**New platform evidence:** Windows run **43986614016** failed building Desktop
with E0308 at `agents.rs:1040`: expected `&str`, found `&ScopedWorkspaceRelay`.
The prior source review missed this type mismatch; its source acceptance did not
establish native compilation. This reviewer read the preserved compiler diagnostic
in `releases/2722-hosted-edit-windows/21099d751150a219e4c3d342aafea9ff04495f0e/build-job.log`
under `/home/bryce/services/buzz-agents` (lines 1550–1572). The
[committed Windows receipt](../../executions/2722-hosted-edit-windows.md) records
39 distinct passing root Rust tests before the failure, no completed exact hosted
Desktop or SQLite ACL cases, and no installer. These are CI-worker evidence,
not tests independently executed by this reviewer.

**Correction trace, read:**
[agents.rs](../../../desktop/src-tauri/src/commands/agents.rs):989 captures the
workspace URL once through `bind_expected_relay_scope`. At line 1041, the correction
passes `Some(reconcile_relay.as_str())` into `assert_expected_relay_scope`, whose
signature requires `Option<&str>`
([relay/scope.rs](../../../desktop/src-tauri/src/relay/scope.rs):13–16).
The existing accessor at lines 40–43 borrows the wrapper's stored String; it does
not consult workspace state. The assertion still normalizes that captured URL and
compares it with the exact hosted record's community. Owner and replay-floor guards
remain immediately afterward. This preserves the checked, captured scope and fixes
the reported type error without adding a reread, conversion policy or fallback.
The same accessor already feeds profile reconciliation immediately above.

**Disposition:** compiler finding corrected in source and accepted for the Windows
rerun; prior H-1/M-1/M-2/M-3 remain closed. No additional defect was found in this
single-call delta. **Measured:** correction whitespace check passed; aggregate size
recount above. Executor Rustfmt evidence is retained in the
[execution receipt](../../executions/2722-hosted-edit-start.md), not rerun here.

**Evidence retained:** independent browser 21/21 and previous owner/host checks
remain applicable because their product/test source is unchanged. **Not run on the
corrected pin:** native Windows build/tests, exact hosted/ACL cases, other native
platform checks, `just ci`, installed normal launch and human/live delivery checks.
Do not interpret the earlier 39 passing tests as a green Windows gate.

Return to conductor for the authorized corrected Windows run. All reviewer processes
are terminal; no mutation or port 4173 listener remains. Only this signed report
addendum is committed. Historical sections below remain evidence and are superseded
by this current pin and platform status. The quality footer's `rework / partial`
continues to mean outstanding platform/human acceptance, not further source repair.

## Historical source re-gate — round 2

**Source accepted. All findings closed: C=0 / H=0 / M=0 / L=0.**
No further implementation correction is recommended. Full merge/release acceptance
remains pending native platform and human gates; no unconditional merge-safe verdict
is claimed while those required checks are unrun. The quality footer retains
`rework / partial` solely for that outstanding acceptance, not for a source defect.

Reviewed 2026-11-17 at clean `571ecf9497a14095c6db9a15682aae53de3df3f0`, same
worktree and branch. Narrow delta from report commit
`84fb2d6b63e45593a586c5952eab75019c22be0d`: one product identifier changes
`StartTarget::Hosted(_)` to `StartTarget::Hosted(_scope)` in the non-Windows arm
of [agents.rs](../../../desktop/src-tauri/src/commands/agents.rs):1063, plus the
executor receipt. Combined reviewed source is
`f95d075b365486a432732d603ba4511cf19fe7ff..571ecf9497a14095c6db9a15682aae53de3df3f0`.
Edit/Start remains **13 production files / net +478**, within the approved +500.
Requested reviewer configuration Astra/high; served model/effort unknown.

| Finding | Final disposition |
| --- | --- |
| H-1 | **Closed.** Windows-only dispatch remains cfg-gated; the non-Windows arm now binds its payload and still refuses Start. Both original name-resolution defect and its denied-warning residual are resolved. |
| M-1 | **Closed**, retaining round-1 source trace and passing cross-harness partial-Save retry case. |
| M-2 | **Closed**, retaining round-1 source trace and passing local Duplicate case. |
| M-3 | **Closed**, retaining round-1 source trace and passing delayed/error/refetch legacy-context cases. |

**Measured:** the identical round-1 compiler reproduction with only `_` changed to
`_scope` passed under the pinned compiler with `-D warnings`, exit 0 and no
warnings (`rustc --edition 2021 --crate-type lib --emit metadata -D warnings`,
stdin source, output `/tmp/2722-start-cfg-probe-fixed.rmeta`). This is a narrow
compiler-semantics check, not a native app build. Correction diff/whitespace check
passed. The production match arm was independently read before the reproduction.

**Preserved evidence:** independent round-1 browser **21/21**, unchanged UI and
fixture; initial independent owner **17/17**, synthetic host **1/1**, service
guards **7 passed / 1 existing ignored**, unchanged owner/host source and fixtures.
No suite rerun or mutation campaign was needed or performed for this identifier.
The executor's Rust formatting receipt remains executor evidence.

**Still not run:** native Windows/Linux/macOS compilation and native tests/clippy,
`just ci`, installed normal Windows launch, live same-agent delivery, and human
acceptance. The known Linux GLib/WebKit gap remains unchanged. Browser checks and
the compiler reproduction cannot prove native persistence or transport. The
conductor's next stage is the existing Windows native workflow, then applicable
human checks; this report grants no live activation authority.

All tool processes are terminal; port 4173 has no listener, and no mutation is held.
Only this signed report update is committed. Minimalism, elegance and source
correctness: **9/10 each** for the bounded change after the four corrections;
platform/runtime confidence remains subject to the explicit unrun gates above.
The round-1 and initial records below are historical evidence, superseded by this
final disposition. No additional source blocker remains.

## Historical re-gate — round 1

**Current verdict: rework. Open findings C=0 / H=0 / M=1 / L=0.**
The original H-1 name-resolution defect and M-1–M-3 are corrected. H-1 retains
one narrow non-Windows lint residual introduced by its correction (below). No unrelated area
was reopened. Windows compilation/tests and human acceptance remain unrun and are
separate subsequent gates; this is not release acceptance.

Reviewed 2026-11-17 in the same exclusive worktree at clean
`102a5c6471da5126fd807e6cfabf238da49a764e`. Correction range:
`0ce26a88cd81caf3e9fe85370a9896ddf22f306f..102a5c6471da5126fd807e6cfabf238da49a764e`;
product correction commit `0e0839640f1af0b024343463926baeb767868271`.
Combined source range remains rooted at `f95d075b365486a432732d603ba4511cf19fe7ff`.
Conductor recorded all original findings as fix-in-lane; the human authorized up
to +500 net lines in the same 13 production files for these four corrections.
Independent recount: **13 files, +570/-92, net +478** for Edit/Start;
combined F2/F3 and Edit/Start: **15 files, +661/-139, net +522**.
Requested reviewer configuration remains Astra/high; actual served model/effort unknown.

## Re-gate dispositions

| Finding | Actual disposition and independent evidence at this pin |
| --- | --- |
| H-1 | **Fix-in-lane, residual remains.** Original unresolved-module defect corrected: `agents.rs:1060–1063` now compiles the hosted call only for Windows and returns an unsupported error otherwise. Source-derived correction; native compilation not run. See H-1 residual below. |
| M-1 | **Closed.** `UserProfilePanelPersonaSubmit.ts:99–120` independently emits the selected metadata's command/default args for each hosted instance write, even when the definition already committed. Absent metadata leaves ordinary offline retention intact. The actual profile case at `hosted-agent-setup.spec.ts:1076` changes Grok to Claude, fails the record write, retries after refetch, and asserts Claude's empty args and exact installed ID. Independently passed. |
| M-2 | **Closed.** `UserProfilePersonaDialogs.tsx:105–111` confines hosted context to id-bearing Edit; its run-location provider is likewise conditional. Duplicate keeps local source/create. The actual Duplicate case at `hosted-agent-setup.spec.ts:1209` passes with host denied, asserting local backend, no destination, and no host discovery/prepare. Independently passed. |
| M-3 | **Closed.** `UserProfilePersonaDialogs.tsx:136–157` freezes the initial resolved runtime per dialog; `:204–212` supplies initialization status. `AgentDefinitionDialog.tsx:790,1083–1087` disables submission and removes the form until resolution while retaining Cancel. Source-key remount handles reopen. Existing cases at `hosted-agent-setup.spec.ts:1256,1310` exercise delayed context, later refetch preserving the draft/runtime, error, Cancel and reopen. Independently passed. |

The modified files are
[agents.rs](../../../desktop/src-tauri/src/commands/agents.rs),
[submit boundary](../../../desktop/src/features/profile/ui/UserProfilePanelPersonaSubmit.ts),
[dialog context](../../../desktop/src/features/profile/ui/UserProfilePersonaDialogs.tsx),
[shared editor](../../../desktop/src/features/agents/ui/AgentDefinitionDialog.tsx), and
[existing browser fixture](../../../desktop/tests/e2e/hosted-agent-setup.spec.ts).
All correction diffs and the affected callers were read. No additional UI correctness
regression was found in the bounded re-gate.

### H-1 (Medium, high confidence) — unsupported-platform arm leaves a denied warning

**Required lint correction; conductor disposition: fix-in-lane.** The original H-1
name-resolution error is fixed, but on Linux/macOS the private local
`StartTarget::Hosted(Scope)` payload at
[agents.rs](../../../desktop/src-tauri/src/commands/agents.rs):994 is never read:
its only consuming arm is compiled out and the replacement at line 1063 matches
`Hosted(_)`. Rust emits `field 0 is never read` (`dead_code`). There is no enclosing
suppression in this module, `commands/mod.rs`, or `lib.rs`. The repository's required
`desktop-tauri-clippy` commands explicitly use `-- -D warnings`
([Justfile](../../../Justfile):222–224), so this warning becomes a failing native
lint gate on those platforms.

**Measured:** the pinned compiler on this Linux host emitted that exact diagnostic
for a minimal local-enum reproduction with the same Windows/non-Windows match
arms (`rustc --edition 2021 --crate-type lib --emit metadata`, stdin source,
output `/tmp/2722-start-cfg-probe.rmeta`). This is a compiler-semantics probe,
**not a native application compile or clippy run**. Reproduction body:

```rust
pub fn start_windows_only(scope: String) -> Result<(), String> {
    enum StartTarget { Local, Hosted(String) }
    let target = if scope.is_empty() { StartTarget::Local }
                 else { StartTarget::Hosted(scope) };
    match target {
        #[cfg(windows)]
        StartTarget::Hosted(scope) => { drop(scope); Ok(()) },
        #[cfg(not(windows))]
        StartTarget::Hosted(_) => Err("Hosted Start is only supported on Windows".into()),
        StartTarget::Local => Ok(()),
    }
}
```

Remedy: consume/bind the payload in the unsupported arm (for example `_scope`)
without enabling the Windows module or weakening refusal. This can be corrected
without adding production lines. Proposed owner: existing executor, as continuation
of the H-1 fix. No broad lint campaign or additional product work is requested.

## Round-1 verification and handoff

- **Measured:** `pnpm exec playwright test --project=smoke hosted-agent-setup.spec.ts`
  independently passed **21/21**, 1.7m, using the executor's final source bundle,
  scoped Chromium/loopback permission, actual components and mocked IPC. Raw local
  log `/tmp/2722-chuck-regate-browser.log`; this report is the durable result index.
- **Measured:** correction-range whitespace check passed; production size independently
  recounted above. Worktree was clean at intake and product/test source is untouched.
- **Reused with source continuity:** prior independent owner 17/17, synthetic host
  1/1 and service guards 7 passed/1 existing ignored. No owner/host implementation
  or fixture changed in this correction range. No new mutation campaign ran.
- **Executor evidence only:** rework build/typecheck/Biome/rustfmt passed per the
  updated [execution receipt](../../executions/2722-hosted-edit-start.md). Those
  checks were not independently rerun. No production change follows that bundle.
- **Not run:** native Windows/Linux/macOS compile, native platform tests/clippy,
  `just ci`, installed normal Windows launch, live same-agent delivery and human
  acceptance. Known Linux GLib/WebKit limitations were not repaired or bypassed.

All re-gate sessions are terminal; no mutation is held and port 4173 has no listener.
Only this signed report update is committed. Route the H-1 residual for its tiny correction,
then a source-only recheck of that arm is sufficient for this reviewer; preserve
21/21 browser evidence if UI source stays unchanged. Conductor owns the subsequent
Windows workflow and human gate. The historical initial report below is retained
as the finding/evidence trail; its former counts and pending dispositions are
superseded by this round-1 section.

## Historical initial review — 2026-11-16

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


quality: model=unknown effort=unknown stage=post-audit verdict=rework verify=partial commits=0ce26a88cd81caf3e9fe85370a9896ddf22f306f..102a5c6471da5126fd807e6cfabf238da49a764e findings=C=0/H=0/M=1/L=0 rework_rounds=1


quality: model=unknown effort=unknown stage=post-audit verdict=rework verify=partial commits=f95d075b365486a432732d603ba4511cf19fe7ff..571ecf9497a14095c6db9a15682aae53de3df3f0 findings=C=0/H=0/M=0/L=0 rework_rounds=2


quality: model=unknown effort=unknown stage=post-audit verdict=rework verify=partial commits=21099d751150a219e4c3d342aafea9ff04495f0e..bd03a6c4845597f1975ac5cd2f8e6ee0aebc8f3c findings=C=0/H=0/M=0/L=0 rework_rounds=3

quality: model=unknown effort=unknown stage=post-audit verdict=rework verify=partial commits=dc6088571d16e970b7839cf2f380ef57a3251636..dd3ad12a85e966abaad17f41faf8bab3d03ee4f7 findings=C=0/H=0/M=0/L=0 rework_rounds=4

quality: model=unknown effort=unknown stage=post-audit verdict=rework verify=partial commits=a86cfe66a695618dae3f7a2b039adc2a7eeff585..55545416c6cecc5f9c69dd34e5c57c07e3293ef7 findings=C=0/H=0/M=0/L=0 rework_rounds=5
