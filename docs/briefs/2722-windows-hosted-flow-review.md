# Independent review scope: Windows hosted Add Agent (#2722)

**Date:** 2026-11-10 UTC. **Requester:** conductor `/root`.
**Next owner:** fresh independent reviewer; executor cannot supply the gate.
**Requested executor configuration:** gpt-6-astra / high; runtime identity unavailable.

## 1. Where the work lives

Worktree: `/home/bryce/services/buzz-wt-3720-host-registration`.
Branch: `proof/3720-host-registration-events` (not main).
Review base: `0f7927f24bb79f9a3592dc80a5dbe56ed0f13717`.
Functional source head: `10e9ecd21e8f993ee0e9996b3ccb295a809b5466` (four source commits).
The subsequent evidence commit adds public native API comments, this brief,
execution receipt and a synthetic screenshot; conductor returns its exact final pin.

```sh
cd /home/bryce/services/buzz-wt-3720-host-registration
export HERMIT_SELF_UPDATE=false
. ./bin/activate-hermit
git log --oneline 0f7927f24bb79f9a3592dc80a5dbe56ed0f13717..HEAD
git status --short
```

Exclusive writer/test ownership is released after the final evidence commit;
reuse this same workspace. No owned test/fixture process remains. Baseline was
clean and no unrelated user changes were found or staged.

## 2. What landed

The ordinary create form can consume approved host-installed metadata while
retaining its shared definition/configuration validation and model picker. Native
main owns the single setup identity, existing protected intent journal, profile
publication and exact replay; follow-on preparation/model selection retains that
identity. A shared setup view is reachable after creation, through retained drafts,
and from the owned saved agent's Runtime tab. The feature stays native default-off;
registration, local/provider create and management provenance remain separate.

Inputs: [ratified spec](../specs/2722-windows-hosted-flow.md),
[execution handoff](2722-windows-hosted-flow.md),
[execution receipt](../executions/2722-windows-hosted-flow.md).
Accepted destination and checkpoint-2 reviews linked there remain inputs.

Measured production +2,122/-278 across 25 files (embedded Rust test excluded),
within the conductor's revised +1,700–2,500/-200–400 provisional range.
Original test estimate was low: +1,756/-18 test/fixture/config lines. Reuse and
exact-line movement accounting are in the receipt; no savings claim is implied.

## 3. Your job

Independently review the pinned range against F1–F4 and finite V1–V5. Doubt the
identity/revision/recovery claims and whether the real UI reaches the guarded
native path. Report concrete correctness, security or agreed-contract failures;
source acceptance does not authorize rollout.

**Special instruction:** Windows-only native code was formatted but neither
compiled nor executed here. The native test calls real main phases/recovery with
isolated storage, but it is not top-level Tauri command/native service acceptance.
Do not infer Windows readiness from the Linux owner/browser results. The final
expired-peer fix uses verified remote absence, not missing local receipt, as its
supersession evidence; scrutinize the transaction binding.

## 4. Specifically look for

1. **Metadata authority and native default-off boundary.**
   Probe `desktop/src-tauri/src/managed_agents/discovery.rs:1084`,
   `discovery/presets.rs:619`, `commands/agents_hosted.rs:38` and
   `commands/agents_hosted_commands.rs:131`. Confirm Grok is projected from the
   preset, installed IDs remain distinct, command/args/environment restrictions
   match the accepted payload, and default-off precedes effects. The renderer
   cannot enable the native flag. No acp/ prefix assumption remains in this route.

2. **Journal winner, exact replay and expiry/admission distinction.**
   Probe `crates/buzz-paired-owner/src/deployment.rs:76`, `:129`, `:400` and
   `deployment/continuation.rs:84`, `:156`. Two different key candidates for one
   token must leave exactly one phase-zero winner. Expired previously admitted
   operations must reconcile, never reset to revision zero. Every expired peer
   ignored at insertion must have an exact verified-absent read; new/unverified
   peers must still fail. Existing query deadlines are five seconds each, cap
   1,024 rows; there is no aggregate action deadline or retry loop.

3. **Latest intent and main record/model recovery.**
   Probe `desktop/src-tauri/src/commands/agents_hosted.rs:259`, `:318` and
   `commands/agents_hosted_tests.rs:6`. Reopening an older root must select the
   latest exact scoped operation and refuse conflicting placement. Only the
   journaled model may change in a follow-on record. Frozen draft, key, auth tag,
   backend/scope, profile input and persona identity must remain aligned. Test
   seeding of a signed follow-on row is fixture input, not proof of full native
   transport execution. Native generation fencing across real awaits remains a
   Windows-runner obligation; the owner signer-after-await test does execute.

4. **Public projections and authority freshness.**
   Probe `commands/agents_hosted_commands.rs:9`, `:61`, `:188` and
   `desktop/src/shared/api/hostedAgents.ts:107`. IPC must not expose the main
   create response key, auth tag, ciphertext or raw recovery snapshot. Local
   retained listing makes no network fan-out and grants no launch authority.
   Observation/unit projection requires current placement; launch additionally
   requires current runtime mapping/catalog, exact model, scope, revision, login
   and expiry. Failed admitted operations use their admitted revision.

5. **Actual shared form, latches and retained definition.**
   Probe `ui/AgentDialog.tsx:174`, `:356`, `ui/AgentDefinitionDialog.tsx:152`,
   `:517`, `ui/usePersonaModelDiscovery.ts:443`, and
   `lib/prepareHostedDefinition.ts:27` under `desktop/src/features/agents/`.
   Both ordinary submit callers enter the shared hosted helper before local
   runtime availability. Same-tick latch precedes avatar/persona effects;
   reply-loss retry reuses definition/token. Hosted mode must not discover local
   auth/models or silently drop unsupported authored configuration. Initial
   local-mode discovery is intentionally preserved until a host is selected.

6. **Reachable continuation, honest states and channel scope.**
   Probe `ui/HostedAgentSetup.tsx:23`, `:210`, `lib/hostDirectory.ts:58`,
   `useCreatedAgentChannelAttachment.ts:8` and
   `desktop/src/features/profile/ui/UserProfilePanelSections.tsx:536`.
   Closed-view delayed results must not replace a new view. Model disappearance
   stays visible but cannot launch. Registered labels use the existing scoped
   cache without row queries. Normal linked agents use the Runtime-tab summary;
   Edit still opens their definition. Membership failure keeps the identity and
   retries with captured signer/community, `ensureRunning:false`. The existing
   toast action is used after closing the setup modal; no global toast redesign.

## 5. Spec deviations and implementation judgments

- Five focused production siblings instead of the original estimated 2–4; they
  separate existing-owner continuation, native sanitized projection, shared submit
  helper, shared setup view and typed IPC. No new persistence/transport system.
- Dedicated hosted commands reuse main create phases; ordinary main create keeps
  refusing host destinations instead of returning a secret-bearing main response.
- Retained setup enumeration is local/validated. One unavailable host cannot hide
  other setups; explicit continuation reads current permission/revision. Ambiguous
  same-revision candidates can require multiple bounded reads with no global
  deadline. Is that limitation clearly acceptable for this source checkpoint?
- Latest lookup and repeated-expiry peer validation were tightened after finite
  test/self-check evidence. Public source acceptance must retain the lost-admission
  negative; missing local phase/result is insufficient proof of absence.
- Shared summary additionally renders in the owned Runtime tab because linked
  definitions bypass the instance editor. This was a real rendered reachability
  correction approved by the conductor, not a change to definition Edit semantics.
- Membership scope uses ws(s) URL spelling to match existing caller/fixture
  conventions; native accepts the equivalent canonical HTTP community origin.

## 6. What NOT to audit / chase

Hostd, relay, schema, core wire/ACP and accepted checkpoint-2 lifecycle foundations
are unchanged. Carry their accepted evidence; broad reruns are not requested.
Scope excludes N1 online policy (#3719), native Grok/service acceptance (#3722),
installer/adoption, HTTP removal, main catch-up and general lifecycle controls.
No live relay/provider/real pairing, existing Claude/Grok/Scout homes, credentials,
services or deployment actions are authorized. Native feature activation remains
outside this source packet. Do not install a Windows toolchain to erase a gap.

## 7. Deliverables and pre-flight

Return independent findings and exact source pin to the conductor. Use the
project's post-audit artifact convention and distinguish source acceptance from
native/full-CI/human readiness. Executor evidence is not the independent gate.

- Verify final pin/clean workspace and read the spec/brief/receipt end to end.
- Available final evidence: owner 2/2, Node 65/65, jsdom 3/3, 47 distinct browser
  cases, TypeScript/Vite and scoped format/size PASS. Failed intermediate commands
  and their corrected outcomes are recorded explicitly in the receipt.
- Native Windows tests/build, full CI, human and live acceptance are NOT RUN.
- Synthetic screenshot: `docs/executions/evidence/2722-hosted-setup-models.png`.
- Do not revive old owner processes; no test/server is handed over running.
