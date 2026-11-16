> Historical pause receipt, superseded by the approved continuation and
> [completed implementation review brief](2722-hosted-edit-start-review.md).
> Statements below describe the earlier +370 boundary, not current authorization.

# #2722 paused hosted Edit/Start handoff

## Status

**WIP, incomplete, not release-ready.** The next agent's job is to resume this same
assignment only after conductor/user scope disposition. This is not an independent
review or a merge gate. Requested codecodex Astra/high; runtime identity unverified.

## Where work lives

`/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`; baseline
`bd46993f30c9395d24b971e4520706a7f8f0dba7`. The WIP commit contains seven production
files, one existing test-file extension and this evidence. F2/F3 changes precede
the baseline and remain preserved. Exact file counts, costs and receipts are in
[the execution record](../executions/2722-hosted-edit-start.md).

## What needs doubt

The partial native adapter is not proven. Owner test compilation failed before
execution. Do not treat a signed WIP commit as accepted source. No frontend fix has
landed; ordinary hosted Edit remains unfixed despite native field changes.

## Specifically inspect when authorized to resume

1. `deployment/continuation.rs`, `Admission` and `continue_intent`: does explicit
   Start accept current applied/failed/needs-login evidence without weakening
   setup's model-only or current predecessor/competing peer restrictions?
2. `agents_hosted.rs`, `start`: check scoped re-read after the per-agent lock,
   exact retained identity, latest operation selection, expired admission and
   pending exact retry. Model evidence must match the retained candidate.
3. `agents_hosted.rs`, `candidate`: explicit canonical runtime versus installed
   host mapping remains unproven; command equality alone may be insufficient.
4. `agents_hosted.rs`, `recover`: `existing_start` must preserve later settings,
   refuse missing/moved identities, and avoid weakening old Create recovery.
   Existing Save/profile publication obligations remain separate from Start.
5. `agent_models_update.rs`: inspect structural installed-runtime and scope checks,
   absent-field retention and hosted access-policy refusal. No network validation
   belongs in Save. Scope/key/host movement must remain impossible.
6. `deployment_tests.rs:892`: `Command` is not Clone. The extension did not compile;
   replace both command/candidate clone calls using existing reconstruction, then
   run the actual journey when authorized. Native recovery/offline Save fixtures
   remain unwritten.
7. `AgentDefinitionDialog.tsx:552`: fresh model gating prevents offline Edit. The
   approved file substitution permits a narrow unchanged-selection edit exception;
   keep Create and changed selection truthful. Connect exact clicked instance,
   host context and partial-save errors through existing profile surfaces.

## Deviations

Conductor approved replacing planned `AgentDialog.tsx` work with
`AgentDefinitionDialog.tsx`, using the existing editor/run-location context directly
from `UserProfilePersonaDialogs`. The actual change is pending. Native measured
net +273 leaves 97 lines; remaining frontend estimate is +120–155. Implementation
stopped before crossing +370. No larger budget is approved.

## What not to chase

No live agents/services, relay/hostd/schema changes, helper/wire/state frameworks,
Stop UI, automatic restart, provider tooling, public release, push, taskctl, or
independent partial review campaign. Preserve the accepted F2/F3 predecessor.
Historical 18-file/online-Save/diff-state requirements are superseded by the active
external amendment, linked from the execution record.

## Required next gate and preflight

- Obtain explicit conductor scope disposition; no autonomous expansion.
- Verify the pinned WIP commit and clean worktree; all prior processes are terminal.
- Read the active external authorization/amendment and execution record.
- Complete authorized work and actual focused gates before fresh independent review.
- Native Windows and human acceptance remain separate, not implied by source checks.
