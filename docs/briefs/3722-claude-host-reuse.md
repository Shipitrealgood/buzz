# #3722 — main discovery/readiness planning handoff

**Date:** 2026-11-10. **Next owner:** conductor for plan review, then a fresh
executor only after dispatch. Requested `gpt-6-astra` / `high`; actual served
model/reasoning not independently exposed. This brief supersedes the earlier
recommendation to extract main auth orchestration. Native provider login stays
outside Buzz, on netcup.

## Where the work lives

Worktree `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`; clean source input
`3169659626044819e17c7f62fe2d060b890c760e`. Activate Hermit with
`HERMIT_SELF_UPDATE=false` before Git. Reuse this worktree for sequential source
execution and independent review, handing over ownership between stages.

## What is in the tree

This assignment adds [the bounded spec](../specs/3722-main-discovery-native-login.md)
and replaces this brief only. Product/test source stays at the input pin. The
[accepted audit](../audits/pre-audit/3722-claude-host-reuse.md) remains historical:
H1/H2/M1 findings stand, but its proposed auth extraction no longer does.
[Selected T](../specs/3719-host-online-readiness.md) remains normal networking
with private per-agent state, authorization and approved runtime controls.
Conductor owns roadmap, pilot plan and task mutations; none belong to this worker.

## Sequence and suggested commit boundaries

1. **Resolve native-entry design first.** The conductor/operator specifies a
   concrete exact-service native-login approach by source/OS design, with no live
   action. If significant privileged interactive machinery is required, return
   that decision before coding the dependent path. Demonstration is later, in
   the approved disposable pilot.
2. **S1: shared main facts/classifiers.** Put only portable registry/discovery/
   readiness decisions in existing `buzz-core`; adapt Desktop to consume them.
   Leave Tauri, installers, global caches and login orchestration in Desktop.
   This boundary makes semantic reuse independently reviewable before host wiring.
3. **S2–S4: one coherent host/contract/UI increment.** Resolve host candidates,
   preserve catalog approval, evaluate same-service readiness, extend existing
   private observations and update existing setup/model UI. Retire the custom
   host login command and receipt gate together with their replacement. New
   clients read old observations as unknown; old clients reject new optional
   fields, so activation requires matched gated host/client builds.
4. **S5: explicit T policy commit.** Pair a named root-approved online policy with
   an exact reviewed service configuration; retain offline defaults. Keep this
   separable from discovery/readiness for policy review and rollback.
5. **Source verification and independent review.** Use finite affected tests from
   the spec, then return committed evidence. No live activation follows by default.
6. **Conductor prepares rollout approval.** One approved disposable Grok pilot
   demonstrates the agreed native-entry procedure and the complete journey.

## Pre-flight

- Read the new spec and latest dispatch before treating older task/audit prose as
  authority. This is a plan-only handoff, not permission to implement.
- Preserve checkpoint evidence for unchanged source; do not restart broad audits.
- Confirm exclusive write/test ownership and exact committed baseline; preserve
  any unrelated edits discovered on handoff.
- Read root/feature instructions and applicable test guidance before edits.
- Record that exact native login entry and actual service/provider gates are open.

## Things to flag back before powering through

1. **Exact service login is missing.** [Unit:10](../../crates/buzz-hostd/deploy/buzz-paired-agent@.service#L10)
   uses DynamicUser/private state; [context:156](../../crates/buzz-hostd/src/runtime_context.rs#L156)
   has null stdin. Neither an operator login nor a guessed numeric UID is the
   agent's context. If native OS tooling cannot supply the exact context without
   new interactive machinery, return the prerequisite before dependent coding;
   default is hold that work and the pilot.
2. **Single owner of facts.** [Main registry:50](../../desktop/src-tauri/src/managed_agents/discovery/catalog.rs#L50)
   and [presets:128](../../desktop/src-tauri/src/managed_agents/discovery/presets.rs#L128)
   remain the semantic source. A parallel hostd provider table or whole-desktop
   relocation is scope drift. Move shared facts; keep callers thin.
3. **Additional native CLI approval.** [Runtime:63](../../crates/buzz-hostd/src/lib.rs#L63)
   pins only harness/adapter. Claude/Codex status commands need approved native
   executable closure; missing approval is not permission to search operator HOME.
4. **Auth truth.** [Grok metadata:115](../../desktop/src-tauri/src/managed_agents/discovery/presets.rs#L115)
   says NotApplicable. Do not manufacture login success or infer entitlement from
   ACP models. Unknown and probe failures block; NotApplicable is explicitly not
   authenticated. Richer unsupported configurations remain unsupported.
5. **Wire rollout.** [Observation:62](../../crates/buzz-core/src/host_deployment/observation.rs#L62)
   denies unknown fields. If rolling mixed-version interoperability is required,
   return that decision; matched gated activation is the proposed minimum.

## What NOT to do

Scope is source reuse and truthful existing UI/transport projections. Keep live
Claude/Grok/Scout, pairing, operator homes/credentials and services untouched.
No custom login flow, terminal/PTY, broker, per-provider hostd adapters, installer,
adoption, namespace/veth/nft, relay schema/resources, HTTP cleanup, main catch-up,
new project or prolonged standalone fixture platform. No test proves entitlement
unless an explicitly approved native provider journey actually ran.

## Done criteria and verification state

Planning endpoint: spec states file-level changes, explicit context/approval,
readiness versus model semantics, compatibility limit, native-entry blocker and
finite gates. It is ready for human plan review; implementation and the native
journey are not accepted by this document.

This assignment performed bounded source inspection, preflight and spec
self-review. Relative document/source links and `git diff --check` must pass
before its docs-only DCO commit. Builds, product tests, native service/provider
checks, full CI and human acceptance are **NOT RUN**. No long-running process was
started. Return commit and final status; source execution requires a new dispatch.
