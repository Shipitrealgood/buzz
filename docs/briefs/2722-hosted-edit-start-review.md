# Independent review: hosted Edit + explicit Start (#2722)

Requested by conductor, 2026-11-16; next role: fresh Astra/high Chuck reviewer.
Executor requested Astra/high; actual runtime model/effort is unknown.

## Correction re-gate (2026-11-17)

Same review assignment; re-gate H-1/M-1/M-2/M-3 and defects introduced by their fixes.
Starting review pin `0ce26a88cd81caf3e9fe85370a9896ddf22f306f`; correction source pin
`0e0839640f1af0b024343463926baeb767868271`. The prior reviewer report remains unchanged. Current aggregate
is **13 production files, net +478**, within the human's four-fix +500 allowance.

- `desktop/src-tauri/src/commands/agents.rs:1060`: Windows dispatch guard and explicit
  non-Windows error. Native compilation still awaits the conductor's platform gate.
- `UserProfilePanelPersonaSubmit.ts:113`: metadata-derived command/args/MCP convergence
  on every hosted record Save, independently of definition-runtime delta. Missing
  metadata does not invent new facts or trigger network validation.
- `UserProfilePersonaDialogs.tsx:105`: id-bearing Edit intent limits host source and
  run-location provider; Duplicate keeps local main behavior.
- Same file `:141` freezes the initial runtime for the dialog. At `:191`, the editor
  key separates local/host source lifetimes; at `:204`, unresolved context shows the
  existing editor's loading/error status. `AgentDefinitionDialog.tsx:1083` hides
  editable fields until resolved, leaving Cancel usable. Refetch must not reseed.

Existing browser fixture now covers cross-canonical partial Save retry, unavailable
host/local Duplicate, delayed context, later refetch preservation, error/Cancel and
retry. `pnpm exec playwright test --project=smoke hosted-agent-setup.spec.ts`: **21 passed**, 1.7m. Build/typecheck/format passed. No owner mutation or
broader test campaign repeated; previous owner/host evidence remains unchanged.
See the execution receipt for exact results and test-iteration explanations.

## 1. Where work lives

Worktree `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events` (not main). Edit/Start source range is
`bd46993f30c9395d24b971e4520706a7f8f0dba7..dcd9658aa` (three signed commits,
including paused WIP `8c0603fe8`, native completion `c542979eb`, UI `dcd9658aa`).
This brief/receipt follows in a documentation commit. Conductor's combined review
also includes F2/F3 from `f95d075b365486a432732d603ba4511cf19fe7ff..HEAD`.

```bash
cd /home/bryce/services/buzz-wt-3720-host-registration
git diff bd46993..dcd9658aa --stat
```

## 2. What landed

The existing profile editor now uses exact host context and installed runtime IDs.
Save uses ordinary definition and selected-instance writes offline with partial-error
and next-start feedback. Explicit Start reuses main's effective projection and existing
revisioned Deploy; retained Start recovery preserves later edits. Measured production
scope at initial review was **13 files, net +449**; current correction count is above.

Active contract: `/home/bryce/services/buzz-agents/docs/specs/2722-hosted-save-next-spawn.md`
top authorization/amendment, summarized in the [execution receipt](../executions/2722-hosted-edit-start.md).
The earlier [pause brief](2722-hosted-edit-start-paused.md) is historical only.

## 3. Your job

Find concrete contract failures in the combined range; the executor supplies no
independent gate. Pay particular attention to native compile/type correctness and
native/UI seam differences: Linux could run owner and browser fixtures, but not the
Tauri fixture. Browser mocked IPC is not proof of native persistence or delivery.

## 4. Specifically look for

1. **Current predecessor and exact retries** — `crates/buzz-paired-owner/src/deployment/continuation.rs:108`.
   Probe `Admission::Start` through shared revision/peer/cleanup checks; setup must
   retain its model-only restriction. Applied/failed/needs-login and expired
   nonadmission should admit a new revision; pending retry must retain exact bytes.
   Existing journey extension is in `deployment_tests.rs:870` onward.
2. **Latest effective settings with fixed identity** — `desktop/src-tauri/src/commands/agents_hosted.rs:78` and `:477`.
   Probe selected installed runtime against canonical host mapping, main effective
   model/prompt, current key, scope and supported fields. The locked current record
   feeds candidate projection; current model evidence precedes owner admission.
   Inspect races with Save and the existing per-agent deployment lock.
3. **Recovery does not undo Save** — `desktop/src-tauri/src/commands/agents_hosted.rs:422`.
   Probe the `existing_start` marker under store lock, same scope/key/auth/relay,
   missing-record failure and phase advance without snapshot reinstall. Ordinary
   Create recovery must retain its original behavior. Native fixture extension is
   `agents_hosted_tests.rs:421`; this source still awaits Windows compilation.
4. **Offline structural persistence** — `desktop/src-tauri/src/commands/agent_models_update.rs:328`.
   Probe optional installed-runtime retention, scope/owner checks, access-policy
   refusal and rejection on nonhost backends. No fresh execution facts belong in
   Save. `UserProfilePanelPersonaSubmit.ts:85` must expose second-write failure after
   definition success and keep authored input available for retry.
5. **Exact clicked host editor** — `desktop/src/features/profile/ui/UserProfilePersonaDialogs.tsx:103`.
   Probe scope/query keys, unambiguous legacy fallback, canonical versus installed
   mapping and unavailable-catalog behavior. Initial values must remain stable as
   query/record objects change; local runtime discovery must stay excluded.
6. **Narrow offline exception and truthful UI** — `desktop/src/features/agents/ui/AgentDefinitionDialog.tsx:547`.
   Probe unchanged saved runtime/model only, Create/changed-selection guards,
   visible next-start description at `:818`, and Start-requested wording. The
   final browser journey at `desktop/tests/e2e/hosted-agent-setup.spec.ts:1021`
   closes/reopens profile before offline Save to discard local model evidence.
7. **Host reuse proof stays synthetic** — `crates/buzz-hostd/src/user_service_tests.rs:219`.
   Probe the extension's same-unit/key assertions, active refusal, and inactive
   higher revision. `service_actions_tests.rs` adds nonzero MainPID/ControlPID/
   TasksCurrent checks. Hostd production is untouched; no actual service ran.

## 5. Spec deviations

- Approved scope-neutral substitution: edit `AgentDefinitionDialog.tsx` instead of
  `AgentDialog.tsx`, using existing editor/context directly in the profile wrapper.
  Fresh-model gating in the shared editor otherwise prevented offline Save.
- Human approved +450 after the native adapter exceeded its +75–105 estimate.
  Actual aggregate +449 includes ordinary formatting and every production module.
- Shared shell did not display its description prop; hosted Edit renders the existing
  next-start description explicitly. No new settings store, wire or state manager.

## 6. What not to chase

Review scope is the combined source patch and its existing fixtures. Live agents,
provider accounts/binaries, relay/hostd production, database/schema, automatic restart,
Stop, new journals, desired/applied UI and historical online-Save proposals remain
excluded. Preserve the accepted F2/F3 contextual status/ordinary Windows launch work.
No publish, push, workflow dispatch, taskctl or service operations belong to this review.

## 7. Deliverables and preflight

Return concrete findings and source pin to conductor; conductor owns the next gate.
Read the active amendment and execution receipt before judging superseded requirements.
Confirm clean source at the handed-off HEAD and exclusive ownership before tests.

Measured evidence: owner **17 passed**; full hosted browser **18 passed** plus final
strengthened/restored profile case **1 passed**; synthetic host **1 passed**; service
guards **7 passed/1 existing ignored**; typecheck/build/Biome/rustfmt/diff checks passed.
Owner revision and browser offline-gate mutations failed as intended and restored
cases passed. Exact commands/limits are in the receipt. Native Windows, `just ci`,
independent review and human acceptance remain pending; the existing Windows workflow
already selects the extended native fixture and its cleanup sibling.
