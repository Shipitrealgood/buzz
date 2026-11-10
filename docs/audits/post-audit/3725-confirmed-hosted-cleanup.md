# #3725 confirmed hosted Cleanup — independent post-audit

**Verdict: merge-block.** Two High correctness findings require fixes in lane.
Supported-platform native execution is also still a required pending gate.
This is round 0, reviewed 2026-11-10; C=0 / H=2 / M=0 / L=0.

## Pin, contract and exclusions

Workspace `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`, clean review handoff
`fe99afea8dc89719cb81df28e94f922b48eb8ae7`. Implementation range:
`ee3bbc8d5f77f45634040e41dafa3e6dfb7b159a..6038d09813a664dc5c4aac2a7a47c87a7d0dfa18`.
The later handoff commit changes documentation only. Contract inputs are the
[ratified spec](../../specs/3725-confirmed-hosted-cleanup.md),
[review dispatch brief](../../briefs/3725-confirmed-hosted-cleanup-review.md) and
[execution receipt](../../executions/3725-confirmed-hosted-cleanup.md), read with
root/feature AGENTS, product vision, testing guidance and the portable conductor
contract. Review used post-audit and gate-dispatch; the locking analysis below
also applies big-brain within this artifact, without creating another framework.

This review covers retained owner Cleanup, permanent Deploy fencing, shared
native finalization and the existing Delete/setup UI. Never-admitted refusal,
runtime-independent cleanup, ordinary local/provider semantics and default-off
native enablement are settled. Relay/host/schema source, live systems, existing
agents, installation, deployment, export, workflow dispatch and broader lifecycle
work are excluded. No prior 99-source/110-Windows result is represented as running
against this delta. Requested reviewer configuration: **gpt-6-astra / high**;
served model and reasoning identity are not independently exposed.

## Findings

### H-1 (High, high confidence) — failed-Cleanup renewal cannot resume before admission — fix-in-lane

**Measured failure, explained by source.**
[cleanup.rs:185](../../../crates/buzz-paired-owner/src/deployment/cleanup.rs#L185)
finds the greatest retained Cleanup expected revision;
[:192](../../../crates/buzz-paired-owner/src/deployment/cleanup.rs#L192) then skips
every older Cleanup before reading current placement evidence. A retained row is
not necessarily an admitted operation.

Concrete sequence: Deploy is historical; Cleanup A (`expected_revision=1`) is
admitted Failed at placement revision 2. Explicit Retry retains B
(`expected_revision=2`). The app exits or the connection fails before B reaches
the relay. On reopen, A is still the current placement, but planning skips it
because B has the larger expected revision. B reads unadmitted, so `current`
stays empty and [:222](../../../crates/buzz-paired-owner/src/deployment/cleanup.rs#L222)
returns “Admission has not been confirmed.” All native Check/Retry/Delete routes
plan before dispatch
([coordinator:112](../../../desktop/src-tauri/src/commands/agents_hosted_cleanup.rs#L112)),
so none can send B. Expiring B does not restore the skipped evidence. Identity is
retained, but the promised recovery path is stranded.

The existing fixture proves renewal retention but immediately moves on
([cleanup_tests.rs:221](../../../crates/buzz-paired-owner/src/deployment/cleanup_tests.rs#L221)).
I extended that existing scenario temporarily after `after_failed` was retained:

```rust
let after_failed_id = &after_failed.protected.authority.operation_id;
responses.lock().unwrap()["by_operation"][after_failed_id] = missing.clone();
drop(device);
let mut device = Device::at(&dir.path().join("renew.sqlite")).unwrap();
let retry = management
    .plan_cleanup(&mut device, &signer, after_failed_id, true)
    .await;
assert!(retry.is_ok(), "retained failed-Cleanup renewal must resume: {:?}", retry.err());
```

The exact production fixture failed at this assertion with
`Some(Admission has not been confirmed; check status before deleting this hosted setup)`:
0 passed / 1 failed / 15 filtered. The unmodified baseline passed before and after
the probe. No production code was mutated. The restored fixture and its saved
original both SHA-256 to
`ebe72dcf37abc18fec7aeae9cbcddd3b54178b174f35875854a69cf31c4ca3ea`.

**Fix:** reconcile the current Cleanup predecessor even when a higher-revision
renewal is retained but unadmitted. Distinguish the selected latest candidate
from the current admitted placement. Reuse B's exact sealed ID/event while
unexpired; require proven current revision for explicit expiry renewal. Preserve
the transaction winner checks and permanent fence. Extend this same fixture
through reopen, identical B reuse and eventual admission/completion, with Check
as well as Retry covered. No new protocol is needed.

### H-2 (High, high confidence) — finalization can write its tombstone into a switched workspace — fix-in-lane

**Source-derived interleaving; native race not executed.**
[finish:189](../../../desktop/src-tauri/src/commands/agents_hosted_cleanup.rs#L189)
holds `managed_agents_store_lock`, then calls `session.keys()` once. However,
[Session::keys:80](../../../desktop/src-tauri/src/commands/paired_hosts.rs#L80)
only holds `workspace_apply_lock` until that call returns. The finalizer then
does synchronous record/assignment/key I/O and
[re-reads active retention scope](../../../desktop/src-tauri/src/commands/agents_pending.rs#L72)
through the [shared helper](../../../desktop/src-tauri/src/commands/agents_deletion.rs#L31).
[apply_workspace:208](../../../desktop/src-tauri/src/commands/workspace.rs#L208)
serializes on the workspace lock, but its
[relay/key mutation:256](../../../desktop/src-tauri/src/commands/workspace.rs#L256)
does not take the managed-agent store lock.

Concrete sequence: A's fresh Cleaned proof reaches `finish`; its session check
passes and releases the workspace guard. While finalization performs disk/keyring
I/O, another thread applies workspace B (same owner is enough). The record and
agent key are removed, then `active_retention_scope` resolves B and enqueues the
tombstone/archive there. Finally
[finish:215](../../../desktop/src-tauri/src/commands/agents_hosted_cleanup.rs#L215)
marks A's Cleanup phase 4 using the captured A keys. A has no tombstone and no
listed pending Cleanup; the trailing session check may reject the response but
cannot undo the durable removal or phase 4. With an identity switch, the helper
can additionally sign using the new owner. This violates F2's captured-session
and correctly scoped completion contract.

**Fix direction:** serialize the entire synchronous finalization with the
existing workspace/identity mutation boundaries, validate the captured session
while those guards are held, and use captured scope/keys for tombstone enqueue.
Use a consistent lock order and avoid recursively calling `Session::keys()`
while holding its non-reentrant workspace mutex. Native identity import also
mutates keys under `identity_mutation`
([identity.rs:458](../../../desktop/src-tauri/src/commands/identity.rs#L458)); the
fix must account for that writer or otherwise establish an equivalent stable
signer boundary. Hold no synchronous guard over network awaits.

The alternatives are: hold the existing guards for the finite local commit
(recommended); pass only captured scope (prevents retargeting but alone does not
serialize mutation against a switch); or add a new transactional finalization
journal (unnecessary scope and persistence complexity). Another check after I/O
cannot repair already-written effects. Extend the existing native fixture at the
finalization boundary so a concurrent switch cannot change destination or retire
the old retry witness prematurely. Its current generation-switch scenario only
switches during an operation query
([cleanup_tests:77](../../../desktop/src-tauri/src/commands/agents_hosted_cleanup_tests.rs#L77)).

## Finite obligation assessment

| Dispatch probe | Independent assessment and limits |
|---|---|
| 1. Atomic retained operation, restart, renewal | SQLite IMMEDIATE winner handling and seal-derived ID are present ([cleanup.rs:74](../../../crates/buzz-paired-owner/src/deployment/cleanup.rs#L74)). Measured duplicate concurrent retention, reopen, four admitted Deploy states and renewal fixture; H-1 disproves complete restart recovery. Fresh-plan/intervening unknown Deploy and capacity failure are source-reviewed guards, not separately measured new cases. |
| 2. Permanent Deploy fence | Read transaction checks in retain/advance, before and after dispatch reconciliation, continuation, and native recovery/publication checks ([deployment.rs:110](../../../crates/buzz-paired-owner/src/deployment.rs#L110), [hosted.rs:18](../../../desktop/src-tauri/src/commands/agents_hosted.rs#L18)). Owner fixture measures old prepare/continue/dispatch refusal and phase-4 fence. Native recovery uses the same store lock as Cleanup retention. Already-sent network work remains subject to existing relay CAS; this is not cancellation. |
| 3. Exact fresh confirmation | Read `operation` → `reconcile` → `wire::open_result_body` → `DeploymentResult::validate`, plus stored-unit check ([cleanup.rs:280](../../../crates/buzz-paired-owner/src/deployment/cleanup.rs#L280), [wire.rs:337](../../../crates/buzz-core/src/host_deployment/wire.rs#L337)). Measured pending/cleaned, wrong revision/unit/event/result/state, denied fresh read and expired signed completion. Full authority equality binds scope/action/digest/expiry; no cached receipt or admission-only success. No runtime query in the Cleanup connection/coordinator. |
| 4. Shared finalization, absent-record retry | Read strict key/tombstone propagation, phase 4 after success, exact record comparison, and tombstone/archive payload preservation ([deletion:22](../../../desktop/src-tauri/src/commands/agents_deletion.rs#L22), [pending:125](../../../desktop/src-tauri/src/commands/agents_pending.rs#L125)). H-2 blocks session-safety acceptance. Authored native fixture exercises archive INSERT failure after record removal and reopen, but is unrun and disables the real keychain. No actual keyring-failure claim. |
| 5. Entry points, force flag, default off | Normal Delete branches to hosted coordinator before provider force logic ([agents.rs:1205](../../../desktop/src-tauri/src/commands/agents.rs#L1205)); setup Check/Retry/Delete route before runtime open ([commands:318](../../../desktop/src-tauri/src/commands/agents_hosted_commands.rs#L318)). Native derives scope from record/intents, rejects absent matching authority, and `run` checks enablement. Source review finds no force bypass. Node mocks prove only emitted IPC and follow-on behavior; authored native case invokes coordinator, not actual Tauri command/force dispatch. |
| 6. Honest UI and discoverable recovery | Native list reconstructs Cleanup independently of a saved record and omits only phase-4 active rows ([commands:240](../../../desktop/src-tauri/src/commands/agents_hosted_commands.rs#L240)); AgentDialog lists retained setups independently of runtime discovery. Real-component mock-IPC browser fixture measures Close/reopen, refusal, unavailable recovery and local-pending/completion labels. Node hook tests measure no early roster removal, force flag or shutdown on pending/denied hosted Delete. DTO is an explicit secret-free projection; component retains generation/latch fencing. |

The small shared finalizer and owner Cleanup sibling are reasonable reuse; no
concrete unnecessary subsystem or source duplication warrants another finding.
Minimalism 9/10, elegance 9/10, correctness 7/10 because H-1/H-2 violate recovery
and scope integrity. Ordinary local/provider deletion still uses its best-effort
branch. Scope/fixture changes do not weaken runtime readiness guards.

## Verification receipts

All commands used `HERMIT_SELF_UPDATE=false` and activated `bin/activate-hermit`.
No dependency/tool installation, live traffic, source export or external workflow
was performed. Loopback-only owner/browser fixtures used narrow approved sandbox
escalation. Exact commands and actual results:

| Command | Result |
|---|---|
| `cargo test --offline --locked -p buzz-paired-owner deployment -- --test-threads=1` | Initial sandbox 1 passed/1 failed at loopback bind (`deployment_tests.rs:354`, EPERM), environmental. Identical permitted run: **2 passed, 14 filtered**. After restoring the temporary probe: **2 passed, 14 filtered**, 13.55s. |
| `cargo test --offline --locked -p buzz-paired-owner deployment_current_revision_refresh_launch_and_exact_retry -- --test-threads=1` with the inline H-1 probe above | **Expected RED: 0 passed, 1 failed, 15 filtered**, 10.27s, exact failure above. This is a regression scenario on unchanged production, not a mutation kill claim. |
| In `desktop`: `node --import ./test-loader.mjs --experimental-strip-types --test --test-isolation=none --test-reporter=tap src/features/agents/lib/managedAgentDeletionAvailability.test.mjs` | **32 named cases passed**, 0 failed/cancelled/skipped, 6.15s. |
| `pnpm --dir desktop exec playwright test --project=smoke hosted-agent-setup.spec.ts` | **4 passed**, 24.1s. Reused executor's existing E2E build: reviewed production pin unchanged, only temporary owner test changed/restored. No independent rebuild claim. Real React component, mocked IPC, local browser only. |
| `pnpm --dir desktop typecheck` | **PASS**, `tsc --noEmit`, exit 0. |
| `git diff --check` and original-fixture SHA-256 comparison | PASS after restoration. Only this audit artifact is retained as reviewer output. |

**NOT RUN:** Windows native new/existing suites, canary build/workflow/artifacts,
real Tauri IPC journey, real keyring failure, human/pilot acceptance and full
`just ci`. Linux native compilation was already blocked by GLib/GIO before app
compilation; deliberately not repeated. The single new Windows exact case is
registered in [windows-canary.yml:291](../../../.github/workflows/windows-canary.yml#L291)
and inspected for control flow, but remains authored/unexecuted. Source review
does not certify compilation. Executor formatting/Biome/size receipts remain
executor evidence, not independently rerun claims. No broad relay/host battery or
new fault framework was launched.

## Disposition and handoff

| Finding / obligation | Owner and next action |
|---|---|
| H-1 — fix-in-lane | Conductor ruled 2026-11-10: original executor repairs current-predecessor reconciliation and extends existing restart fixture. |
| H-2 — fix-in-lane | Conductor ruled 2026-11-10: original executor repairs existing scope/locking boundary with valid lock order and captured authority; no new framework. |
| Required Windows gate — pending | Conductor owns export authorization and later supported-platform execution. Automatic approval review previously rejected fork export/workflow before execution; this review does not retry it. |

After fixes, return the updated committed pin to this reviewer for H-1/H-2 and
introduced-defect re-gating. Do not re-open settled broader lifecycle or relay
activation work. This report grants no merge, push, installation or deployment
authority. The reviewer restores all temporary test edits and releases exclusive
write/test ownership at the signed audit commit after all owned commands finish.

quality: model=unknown effort=unknown stage=post-audit verdict=merge-block verify=partial commits=ee3bbc8d5f77f45634040e41dafa3e6dfb7b159a..6038d09813a664dc5c4aac2a7a47c87a7d0dfa18 findings=C=0/H=2/M=0/L=0 rework_rounds=0
