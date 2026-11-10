# Execute scope: confirmed hosted Cleanup (#3725 narrow slice)

**Date:** 2026-11-10. **Requester:** conductor `/root`. **Next agent:** fresh codecodex, then fresh independent Chuck on the committed range. **Related:** #3724, #3721; not their full closure.

## Where the work lives

Worktree `/home/bryce/services/buzz-wt-3720-host-registration`; branch `proof/3720-host-registration-events`. Product baseline `392c458e83c37b42e1d58168d11499818d38558f`. Reuse this worktree sequentially. This brief and [spec](../specs/3725-confirmed-hosted-cleanup.md) are documentation-only additions; no product code changed by the specifier. Conductor owns board and ratification.

## Current state

Clean source at assignment; existing trusted-host package reviewed but not activated. Accepted [preparation review](../audits/post-audit/3721-trusted-host-pilot-preparation.md#L3) still holds relay capability and confirmed Cleanup as pilot prerequisites. Canary test.21/source `23d2dc` validates prior creation work, not this Cleanup delta. Another inspector may author a separate ops document; preserve it and stage only owned files.

## Sequence

1. Read the spec end-to-end. Implement F1 with actual Device/Management fixture coverage first: one exact Cleanup retained before effects, current revision/authenticated receipt, duplicate/restart reconciliation and permanent no-Deploy fence. Small owner sibling is allowed; no new table/protocol.
2. Implement F2/F3 together: native Delete and setup Delete share one coordinator; pending never returns successful normal deletion; confirmed Cleanup goes through main local finalization. Extend existing native/browser deletion tests with this coherent behavior. Commit with DCO at a reviewable boundary; do not expose UI wired to a permissive placeholder.
3. Run the finite available G1–G4 checks; report exact unavailable native gates as NOT RUN. Write a source-pinned execution receipt and independent-review brief. Transfer exclusive write/test ownership after processes finish. Conductor dispatches Chuck; executor does not self-gate.

## Pre-flight

- Confirm expected branch/source and existing changes with `git status`; preserve unrelated documentation.
- Read root AGENTS/VISION/TESTING and feature `desktop/src/features/agents/AGENTS.md:350` onward.
- Reuse accepted Cleanup finding, not a new broad audit. Preflight is `skipped-self-contained`.
- The conductor ratified unadmitted refusal and runtime-independent Cleanup; no user approval request is needed for those source choices.
- Keep native feature gating default-off. No activation accompanies code acceptance.

## Flag back before powering through

- **No placement:** relay forbids initial Cleanup without a placement (`crates/buzz-db/src/store/paired_hosts/deployment.rs:112`). Keep never-admitted/ambiguous setup, name its admission-reconciliation failure, and preserve identity. Do not add local cancellation to make the demo pass.
- **Journal ID:** Cleanup ID changes during `wire::seal_command` (`crates/buzz-core/src/host_deployment/wire.rs:210`). Deduplication must return one final retained event under transaction, not merely reuse a renderer token.
- **Catalog trap:** `agents_hosted.rs:157` currently loads status and `agents_hosted_commands.rs:276` calls runtime validation before actions. Cleanup must remain possible after runtime/model/readiness disappears; current registry permission/placement is its authority.
- **Recovery trap:** `agents_hosted.rs:339` can recreate a record, and setup listing skips Cleanup at `agents_hosted_commands.rs:234`. Every old Prepare/Retry/Refresh/Launch/recover/dispatch must see the permanent Cleanup fence, even after expiry or local finalization. Pending Cleanup stays discoverable without a record.
- **Deletion trap:** `agents.rs:1274` is the main removal path; use it after proof, idempotently across interrupted local deletion. `useManagedAgentActions.ts:342` removes channel membership after native success, so pending must not resolve as success.
- **Missing proof or foundation:** if safe renewal cannot be established from current operation queries, stop that transition and return exact evidence. Do not broaden relay/schema/host or invent a generic lifecycle subsystem. Existing native fixture is the test boundary; a new test platform needs rerouting.

## What NOT to do

Scope is owner journal/continuation, native hosted/main deletion, existing setup/API and affected tests/docs. Existing creation identities/configuration, provider-v1/local behavior and trusted host cleanup remain the foundation. Broad Start/Stop/Restart, host installer/broker/account changes, adoption, legacy cleanup, root helpers, HTTP/manual database operations, `!shutdown`, existing agents, live relay/host actions and source-wide test/build campaigns are deferred. No arbitrary unit/path or secret in IPC. No board mutation by the worker.

## Done criteria and verification receipt

- F1–F3 satisfy all G1–G4 invariant rows with production-seam assertions and explicit native gaps.
- One logical Cleanup survives duplicate clicks, reply loss, process restart and local-finalization retry; no stale Deploy resurrects it; current authenticated Cleaned precedes main key/record removal.
- Signed implementation commits, sanitized exact commands/counts/source receipt and review brief returned to conductor; all processes finished, workspace status declared.
- This is not full #3724/#3725 completion or pilot activation permission.

Specifier checks: starting source and clean status measured; targeted source/accepted-artifact reads performed; 31 local document links/line-anchor bounds PASS; staged `git diff --cached --check` PASS. Two owned documents, 150 added lines. Behavioral tests, native Windows execution, builds/full CI and live actions NOT RUN. Requested model/effort Astra/high; actual served identity not independently exposed. No subprocess was detached. Next ready action is conductor ratification, then bounded implementation; source writer ownership releases at the documentation commit.
