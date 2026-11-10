# Execute scope: Windows hosted Add Agent (task #2722)

**Date requested:** 2026-11-10 UTC.
**Requester:** conductor `/root`, under Bryce's checkpoint-3 source authorization.
**Next owner:** fresh executor after conductor dispatch; independent reviewer follows at a committed pin.
**Related tasks:** #3721 accepted source bridge; #3719 online policy; #3722 native Grok acceptance; #3738/#3739 existing deployment foundations.
**Requested specifier settings:** `gpt-6-astra` / high. Actual runtime metadata unavailable.

## Where the work lives

Worktree `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`. Source audited at
`fb31164e90584ae50553546059c2c937e992f19f`. Reuse this worktree sequentially;
the docs commit handed back by the conductor is the next source pin.

```bash
cd /home/bryce/services/buzz-wt-3720-host-registration
export HERMIT_SELF_UPDATE=false
. ./bin/activate-hermit
git status --short
git rev-parse HEAD
```

## What's in the working tree

The source baseline was clean. This assignment adds only
[the contract](../specs/2722-windows-hosted-flow.md) and this brief, committed
with sign-off. No product code, tests, board changes or live actions. No owned
test/build/background process exists. The executor receives exclusive writer/test
ownership only after conductor handoff; preserve any other work found on entry.

Accepted reviews are inputs: [destination](../audits/post-audit/2722-hosted-destination-step1.md),
[checkpoint 2 re-gate](../audits/post-audit/3721-host-service-discovery.md).
The latest top section of the [mission](/home/bryce/services/buzz-agents/docs/specs/2722-disposable-grok-pilot.md)
supersedes its lower historical “next checkpoint” headers. Do not redo host/relay audits.

## Sequence and commit boundaries

1. **Native catalog/read/intent seam with its tests.** Project approved installed
   runtime → canonical Rust harness metadata, then extend existing protected
   intents for continuation and nonzero-revision Deploy. Preserve main native
   create/save/profile phases. Commit together so the UI consumes a complete,
   tested native contract. Do not expose the current create response's private
   key (`desktop/src-tauri/src/commands/agents.rs:914`).
2. **Ordinary form and continuation with rendered tests.** Host mode supplies
   catalog/models to the real AgentDialog/AgentDefinitionDialog and existing
   PersonaModelField. Add one shared setup component for in-form and saved-agent
   continuation, update the existing submit callers and channel attachment, then
   host placement summary. Commit the UI behavior with its tests and feature
   AGENTS guidance; local/provider behavior stays intact.
3. **Finite verification receipt and review handoff.** Run V1–V5 where supported,
   record exact counts/boundaries and remaining native gates, commit evidence and
   a review brief, and release the same worktree. The executor cannot provide
   its own independent gate. Source acceptance does not enable the feature.

Estimated production scope is 16–24 existing files plus 2–4 focused siblings,
+900–1,500 / −100–250 lines including roughly 100–200 moved lines; 800–1,300
new/rewritten lines. Tests estimated 350–650. These are estimates for conductor
scope control, not incentives to omit invariants. A foundation/protocol/storage
expansion must return to the conductor before implementation.

## Pre-flight

- Read the full contract, target AGENTS/visions/testing and feature AGENTS.
- Confirm source equals the docs handoff with no other writer or active test.
- Keep `BUZZ_NATIVE_HOST_DEPLOYMENT_V1` unset outside isolated tests. Renderer
  flags cannot enable native deployment; Add Host registration is independent.
- Use current accepted checkpoint-2 source; Grok is a preset and installed IDs
  need not be `acp/<harness>` (`discovery/presets.rs:157`, core `host_deployment.rs:58`).
- Native Windows runner is absent; prior Linux Tauri compile lacks GObject.
  Keep V3/Windows build explicit NOT RUN if unavailable; no environment install.

## Things to flag back before powering through

1. **A new journal/query contract appears necessary.** Current operation replies
   already include unit/revision (`crates/buzz-db/src/store/paired_hosts/deployment.rs:210`).
   Scoped intent enumeration uses the existing 1024-row cap. Return a concrete
   missing invariant before adding storage, wire fields, a queue or a polling loop.
2. **Runtime metadata depends on Windows probing.** The checked main form and
   discovery/preset seams equal local main `e3a0f3d59`; host availability comes from
   signed Status. Factor pure Rust projection instead of copying facts to TS or
   simulating an installed local binary. Unknown mapping is unsupported.
3. **Reopen can regress to the root create snapshot.** Later model-save recovery
   uses the latest reconciled intent, not whichever historical receipt succeeded.
   A historical result can still be fresh while its placement revision is old.
   Bind current revision == expected+1 before presenting launchable models.
4. **Main caller retries create instead of resuming.** `usePersonaActions.ts:174`
   and `useAgentManagement.ts:153` currently create persona then agent and expect
   boolean completion. The hosted branch must retain the same definition/key and
   return continuation state. Channel helper's `ensureRunning:true` default must
   not reach hosted preparation (`useCreatedAgentChannelAttachment.ts:15`).
   Allocate one native opaque setup token before enabling first Prepare; it is
   the existing operation ID and stays fixed across duplicate calls. Existing
   SQLite intent insertion chooses one durable identity before external effects;
   losing transient key candidates are discarded/recovered, not published.
   The shared submit latch starts before avatar/definition effects; React state
   alone cannot stop same-tick double submits. Retain/reuse the definition ID.
5. **Tests only exercise a fake coordinator.** V4 must render the real form and
   production submit/source logic. Mock IPC/network boundaries only. Owner tests
   establish library behavior; they cannot stand in for Windows Tauri execution.

## What NOT to do

Scope is the native bridge, existing intent API, real shared form and continuation.
Host/relay/DB/schema/ACP behavior changes need a conductor ruling. A narrow named
core follow-on validator is allowed with core tests. Existing agent
identities/services/homes and real pairing are never inputs. No online policy,
installer, provider contact, remote login protocol, generic lifecycle UI, adoption,
HTTP retirement, catch-up, new platform fixture, or automatic retry framework.

Keep the existing shared cloud provenance predicate: locally inventoried hosted
agents are still managed here. Use Cloud + “Runs on <host>” in the placement summary
instead (`docs/agent-management-provenance.md:3`, `ui/RunOnSummarySection.tsx:36`).
Keep login outcome, models, deployment application and relay presence distinct.
Native login guidance refers only to the new prepared context on the host.

## Deviations and verification

No deviation from the bounded dispatch. Conductor explicitly ratified continuation
lookup in existing intents, follow-on revision support, shared setup UI, placement
cloud semantics and current-revision checks. No unresolved architecture ruling or
new prerequisite is proposed; four Medium integration gaps are covered by F1–F4.

Specifier performed source/evidence tracing, clean baseline and local-main comparison,
document path/line checks and diff whitespace validation. No behavioral test/build
ran. V1–V5, actual Windows/native/systemd/provider/human gates are NOT RUN here.
Measured docs gate: Python checked both artifacts, all 11 local Markdown link
targets, trailing whitespace and terminal newlines: PASS. Hermit-activated
`git diff --cached --check`: PASS; staged scope is exactly these two documents.
Conductor receives a signed-off clean docs pin and released workspace; next action
is bounded execution, followed by a fresh independent review.
