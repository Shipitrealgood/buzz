# #2722 F2/F3 — independent review handoff

## 1. Where the work lives

Worktree: `/home/bryce/services/buzz-wt-3720-host-registration`.
Branch: `proof/3720-host-registration-events` (not main).
Base: `f95d075b365486a432732d603ba4511cf19fe7ff`, initially clean.
Implementation: `996203e6cb2dce39420854cdd26116a2cf35b19a` (one DCO-signed-off
implementation commit after base; a second documentation commit contains this brief).
Review source range `f95d075b365486a432732d603ba4511cf19fe7ff..996203e6cb2dce39420854cdd26116a2cf35b19a`.
The final handoff HEAD includes the documentation commit; conductor receives its
exact hash. All test/build sessions terminated; no remaining worktree test process
or port-4173 listener, and no held mutation.

```sh
cd /home/bryce/services/buzz-wt-3720-host-registration
git diff f95d075b365486a432732d603ba4511cf19fe7ff..HEAD
```

## 2. What landed

Only F2/F3 of the [approved specification](../specs/2722-hosted-management-followthrough.md).
The shared saved-agent affordance becomes Host status. The panel separates saved
runtime/model, deployment evidence, and exact-key conversational availability;
missing/expired evidence no longer asserts installation/login/model failure.
Windows proceeds through the existing native gates without the pilot environment
variable. Source measurement: three production files, net +44; tests/guidance and
this [execution record](../executions/2722-hosted-status-normal-launch.md) are separate.
Typecheck, focused Biome, Rustfmt and diff checks passed; final focused browser suite
passed **17/17**. One status-predicate mutation failed at the intended assertion,
then the exact source was restored and the final suite passed.

## 3. Your job

Independently review the committed range for correctness and agreed-contract
violations; executor verification is evidence, not the independent merge gate.
The conductor owns disposition, board updates, merge and any later Windows release.

Special instruction: Windows native cases are source-updated, not executed here.
The native observation-expiry branch is source-reviewed only: its existing phases
fixture does not exercise the live Management operation reader. The conductor
accepted no new harness for this three-line condition. Do not label that branch
runtime-proven by the browser fixture.

## 4. Specifically look for

1. Does missing evidence remain neutral?
   Probe `HostedAgentSetup.tsx:109` and `:136`: expired observations are excluded;
   only a current applicable catalog can mark the selected model unavailable.
   `currentHostedModels` remains unchanged and still binds scope, identity,
   operation, runtime, revision, readiness and expiry before Start.

2. Can presence accidentally become deployment authority?
   Probe `HostedAgentSetup.tsx:40`, `:125`, `:243`: exact initial pubkey goes to
   the shared availability reader; Online/Away add a Start block. Unknown and
   Offline do not change the native launch fence. The fixture returns Online
   for other requested keys, while the target's own presence can be Offline.

3. Is actual unfinished recovery accessible?
   Probe `HostedAgentSetup.tsx:42`, `:166`, `:179`, `:220`: recoverable states
   retain the picker and Refresh; installation/sign-in require current matching
   evidence. Check/Retry/Delete remain accessible independently of presence.
   Existing error/conflict/cleanup rendering has not been hidden by Online.

4. Did native expiry handling swallow a real error?
   Probe `agents_hosted_commands.rs:122`: only the additional refresh advice is
   limited to needs-login/failed/unadmitted. Invalid observations are still
   dropped, prior errors retained, and operation read errors still exposed at
   `:134`. Applied must not acquire advice for a disabled Refresh action.

5. Did normal launch bypass any authority?
   Probe `agents_hosted.rs:161`, `:175`, `:444`: enabled() still guards the same
   entry points; only its predicate changes to Windows. Session/signer capture,
   relay capabilities, catalog/registration, protected journal and build policy
   are unchanged. `agents_hosted_tests.rs:95` now requires env-absent token success,
   exact invalid-scope denial, no publication connection and no saved records.

6. Are browser regressions bound to actual components?
   Probe the existing `hosted-agent-setup.spec.ts` ordinary hosted journey:
   retained/unverified Online, disconnect Unknown, explicit applied check,
   unfinished login/refresh, Online/Away Start block, expiry preserving the ID,
   and failed launch/retry. The existing cleanup case and seven form outcomes
   remain. The selected-model predicate is mutated once; see execution receipt.

## 5. Spec deviations

- Existing older journeys still assumed the removed Prepare form. Their submission
  entry steps now use existing persona-dialog-submit and a shared returned-model
  selection helper, including after explicit harness changes. This is
  fixture drift from the accepted discovery replacement, not a new product flow.
- No native projection test seam was added: the conductor explicitly accepted the
  bounded UI/source-review evidence instead of constructing a new harness.
- Native Windows and installed normal-launch checks remain not run on Linux.

## 6. What NOT to audit / chase

F1/P1 linked-definition Edit and Save/Start application remain a separate planned
integration; Save was neither disabled nor claimed fixed. This patch does not
change relay, hostd, lifecycle APIs, schemas, provider login, isolation, root
helpers or PATH. Existing Claude/Grok/Scout and Jeff/test1 were not acted on.
The accepted live d7714be cohort and previous real-relay evidence remain unchanged;
no new live/runtime acceptance is inferred from them.

## 7. Deliverables and pre-flight

Return independent findings and a gate disposition to the conductor, with the
reviewed revision. Read the spec and execution record, verify the branch/pin and
clean handoff, and check that no source mutation or test server remains. The next
platform obligation is the existing Windows workflow's exact hosted native cases,
then a fully quit/start-from-menu human check without a manual environment flag.
The eventual PR `just ci` and human testing requirements remain outstanding.
