# #3725 confirmed hosted Cleanup — independent post-audit

**Current verdict: merge-block pending successful Windows native verification.**
Round 2 accepts the exact-wire sender repair and fixture corrections; the third
existing Windows run may proceed after ownership release. No remaining in-scope
source finding was found (C=0 / H=0 / M=0 / L=0). The first two Windows runs
compiled native tests but failed Cleanup before completion/race assertions; the
latest sender source has not yet run on Windows. See
[round 2](#round-2--windows-failures-and-exact-wire-sender-repair) for current
evidence and the historical-envelope compatibility boundary.
The round-0 findings and receipts below are preserved as history and their source
line references describe implementation `6038d0981`, before repair.

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

## Round 1 — H-1/H-2 source re-gate

**2026-11-10. Source repairs accepted; required native verification pending.**
Same reviewer and worktree. Clean handoff pin
`ee43656e4750183a659d20f0b9839c5cc3ca0972`; repair implementation range
`6d7ca47d2d00560a7c6a221e475a52c107c0aab1..faf59a0b3421c5410f807428d768a2e353e32396`.
Commits `356b1b7b3c321c89e6b4ae8f1e3294ca87582477` and
`faf59a0b3421c5410f807428d768a2e353e32396` repair H-1/H-2 respectively;
`ee43656e4` changes only handoff documentation. Read the updated brief and receipt,
full repair diff, repaired control flow and relevant workspace/import writers.
Scope is prior findings and defects introduced by these fixes, not a new broad
review. No additional source finding was supported.

| Finding | Current disposition and evidence |
|---|---|
| H-1 — closed | **Measured fixed.** [cleanup.rs:186](../../../crates/buzz-paired-owner/src/deployment/cleanup.rs#L186) now reads older Cleanup candidates instead of confusing retained order with current admission. A missing previously admitted Cleanup is tolerated only behind a fresh strictly later admitted revision at [:222](../../../crates/buzz-paired-owner/src/deployment/cleanup.rs#L222), consistent with the unchanged relay's replacement pruning. The existing fixture now exercises retained B → reopen → Check and Retry both reuse B's exact event/ID → admission → Cleaned, then expired B → Check preserves B / explicit Retry renews only from current A ([cleanup_tests.rs:238](../../../crates/buzz-paired-owner/src/deployment/cleanup_tests.rs#L238)). I ran the repaired owner suite independently: 2 passed, 0 failed, 14 filtered. Existing transaction winner checks and permanent fence remain. |
| H-2 — source fix accepted; native proof pending | **Read-derived correction.** [Session::with_local_commit:90](../../../desktop/src-tauri/src/commands/paired_hosts.rs#L90) takes workspace then identity-mutation guards, validates the captured generation/origin/signer through `checked_keys` without recursive workspace locking, and retains both guards across its synchronous callback. [finish:199](../../../desktop/src-tauri/src/commands/agents_hosted_cleanup.rs#L199) acquires the store lock inside that callback and keeps all guards through phase 4. The captured intent origin and signer form the retention scope at [:226](../../../desktop/src-tauri/src/commands/agents_hosted_cleanup.rs#L226); [shared finalizer:37](../../../desktop/src-tauri/src/commands/agents_deletion.rs#L37) enqueues using that scope directly. Nest regeneration is after guard release. Workspace apply and identity import/recovery use the respective writer locks; initial contention refuses rather than blocking while holding the other guard. No network await occurs in the guarded region. Ordinary local/provider callers pass `None` and retain best-effort behavior. |

The new native fixture was inspected for thread/channel lifetimes and lock order
([cleanup_tests.rs:258](../../../desktop/src-tauri/src/commands/agents_hosted_cleanup_tests.rs#L258)).
It separately tests contention refusal, then launches workspace and identity
writers at the actual shared finalizer's before-remove boundary. Assertions bind
guard contention, phase 3 during effects, phase 4 before writers enter, original
retention destination and subsequent stale-session refusal. Scoped threads are
joined; coordination receives have five-second deadlines. The identity writer
uses `commit_imported_identity` with synthetic persistence. These assertions are
**authored and read, not compiled or executed here**. They remain no proof of real
keyring failure, full Tauri command dispatch, or a human native workflow.

Independent round-1 verification, with Hermit activated and self-update disabled:

- `cargo test --offline --locked -p buzz-paired-owner deployment -- --test-threads=1`:
  **PASS, 2 cases / 0 failed / 14 filtered, 14.68s**, existing loopback permission.
- `cargo fmt --package buzz-paired-owner --check`: **PASS**.
- `rustfmt --edition 2021 --config skip_children=true --check` on the five changed
  native files (`agents.rs`, `agents_deletion.rs`, `agents_hosted_cleanup.rs`,
  `agents_hosted_cleanup_tests.rs`, `paired_hosts.rs`): **PASS**, formatting only.
- `git diff --check`: **PASS**. Verified no UI/API/browser-fixture delta from
  `6038d0981`; round-0 independent 32 Node/4 browser results carry forward only
  for that unchanged surface. No rerun or new native inference from those tests.

**Still NOT RUN:** supported Windows native suites/new exact case, canary
build/artifact association, real native workflow, real keyring-failure test and
human acceptance. Known Linux GLib/GIO compilation block was not repeated. No
additional mutant campaign, dependency installation, source export, workflow
dispatch, build artifact or live operation was performed by this reviewer.

Conductor now records explicit human authorization for the exact fork export and
Windows workflow; this supersedes the earlier authorization hold, but not the
required gate itself. Recommended routing: original executor resumes the approved
Windows verification after this ownership release, then this reviewer assesses
that evidence and any native fixes. No further source repair is requested on this
pin. All review commands are terminal, no source/test changes were retained, and
the clean signed audit commit releases exclusive write/test ownership.

Requested reviewer configuration remains **gpt-6-astra / high**; served identity
and reasoning remain unknown. Current open source counts C=0/H=0/M=0/L=0;
historical findings total H=2. Overall merge-safe status is withheld because the
required native gate is unrun.

quality: model=unknown effort=unknown stage=post-audit verdict=merge-block verify=partial commits=6d7ca47d2d00560a7c6a221e475a52c107c0aab1..faf59a0b3421c5410f807428d768a2e353e32396 findings=C=0/H=0/M=0/L=0 rework_rounds=1

## Round 2 — Windows failures and exact-wire sender repair

**2026-11-10. Source accepted for the third existing Windows run; native gate
still pending.** Same reviewer, worktree and bounded assignment. Clean handoff
`2e992cbbf7a78d93dbe992a0e914357325a27120`; implementation range
`8cec4d7cf5742b7d4fbcb3f093dd2242f6865d21..d093d6dcd556a6ef6017cf8a2a70c109c14627e8`.
`8279e0dbf9b02048bd33e593e0f9d92de39ed6fc` corrects a native fixture assertion;
`d093d6dcd` repairs the shared sender and exact-wire fixtures; `2e992cbbf` is
documentation only. Read the updated execution/brief, full delta and sender,
relevant journal/relay/host callers, and saved terminal Windows evidence.
No new in-scope source finding or introduced defect was supported.

### Source assessment

The old sender converted a typed `Event` to `serde_json::Value` through `json!`,
which reordered object keys. The journal constructs its event string directly
from typed Event ([Deploy:168](../../../crates/buzz-paired-owner/src/deployment.rs#L168),
[Cleanup:131](../../../crates/buzz-paired-owner/src/deployment/cleanup.rs#L131)).
The real relay extracts the incoming event as
[RawValue:96](../../../crates/buzz-relay/src/handlers/host_deployment.rs#L96) and
[stores those bytes](../../../crates/buzz-db/src/store/paired_hosts/deployment.rs#L140).
Thus semantic JSON equality in the previous fixture concealed a real transport
defect: owner reconciliation's exact retained-string comparison could not pass.

[send_event:179](../../../crates/buzz-ws-client/src/connection.rs#L179) now serializes
the typed `("EVENT", &event)` tuple directly to the same WebSocket text send.
Event ID, signed content, public method signature, error conversion, logging,
buffer behavior, acknowledgement matching and timeout remain unchanged. The
outer tuple serializes as the same two-element JSON array; only the unnecessary
Value conversion is removed. No receiver normalization or exact-byte guard
weakening was introduced.

The owner fixture now captures the incoming raw event element and compares all
three initial/denied/retried sends to the original retained string
([deployment_tests.rs:372](../../../crates/buzz-paired-owner/src/deployment_tests.rs#L372),
[:466](../../../crates/buzz-paired-owner/src/deployment_tests.rs#L466)). It exercises
the production sender. The executor's preserved pre-fix RED receipt is accepted
as executor evidence; I independently ran the repaired suite GREEN. No redundant
external mutation was performed. The native peer likewise preserves RawValue at
[cleanup_tests:43](../../../desktop/src-tauri/src/commands/agents_hosted_cleanup_tests.rs#L43);
it no longer rewrites the evidence it returns. Other response event/result fields
remain embedded retained/typed strings. The first fixture correction now asserts
the actual unexpired follow-on Deploy refusal
([:122](../../../desktop/src-tauri/src/commands/agents_hosted_cleanup_tests.rs#L122));
it does not weaken identity/no-effect assertions. Later state assertions now
include error details. H-1/H-2 production guards/finalizer are unchanged.

### Windows evidence assessed, not re-executed by reviewer

I read each saved `run.json`, `jobs.json`, exact Cleanup log, suite terminal
counts, UTF-16 ACL receipt, `verification.json` and installer build receipt from:

- `/home/bryce/services/buzz-agents/releases/3725-windows-canary/8cec4d7cf5742b7d4fbcb3f093dd2242f6865d21/42862238726/`
- `/home/bryce/services/buzz-agents/releases/3725-windows-canary/8279e0dbf9b02048bd33e593e0f9d92de39ed6fc/42866083928/`

| Run / source | Read result and limits |
|---|---|
| [42862238726](https://github.com/Shipitrealgood/buzz/actions/runs/42862238726), `8cec4d7cf` | Both jobs terminal: installer success, native failure. Native compiled; 109 outer cases passed plus ACL. Cleanup failed its admission-message expectation at line 121. Metadata exact case was not reached. |
| [42866083928](https://github.com/Shipitrealgood/buzz/actions/runs/42866083928), `8279e0dbf` | Both jobs terminal: installer success, native failure. Same 109 passed plus ACL; corrected refusal assertion passed. Cleanup returned `unavailable` instead of `cleanup-pending` at line 180; attached error was not logged, so its exact error text is not a measured Windows fact. Portable raw-wire regression and source trace establish the sender defect independently. Later completion/race assertions and metadata case were not reached. |

Independently recalculated extracted installer SHA-256 and size match the saved
source/version/hash receipts: test.22, 56,479,037 bytes,
`e70675e36e1021d543a0ef14bc6115dcde67dcd3ed30cb71ba11037ce806f8cc`;
test.23, 56,434,987 bytes,
`093ed6573b414b2ece998ab5936dd594d0f754b88bf7bcf87cf0bccf2ef5e93b`.
This is local artifact verification, not fresh GitHub metadata retrieval. Neither
installer contains the sender repair or represents successful Cleanup acceptance;
neither was executed. Earlier statements that the native fixture was uncompiled
are superseded for these two older pins, not for current-source native acceptance.

### Compatibility boundary and routing

The shared sender is used by owner commands and hostd registration/status/result
publication. Registration dedup compares parsed event content
([paired_hosts.rs:163](../../../crates/buzz-db/src/store/paired_hosts.rs#L163));
deployment command and terminal-result dedup intentionally compare exact raw
strings ([deployment.rs:89](../../../crates/buzz-db/src/store/paired_hosts/deployment.rs#L89),
[:157](../../../crates/buzz-db/src/store/paired_hosts/deployment.rs#L157)).
Changing serialization preserves normal signed-event semantics, but does **not**
promise transparent upgrade of already-admitted old reordered envelopes. Such
commands already fail owner reconciliation; a host upgraded after publishing an
old reordered terminal result can have its typed duplicate rejected by the
unchanged terminal-result equality gate. Do not relax those guards as a shortcut.

Named question returned to conductor: limit this gate to isolated verification
and fresh disposable pilot, with historical-envelope compatibility as rollout
preflight, or expand into upgrade recovery. **Conductor ruled the former** on
2026-11-10. The accepted, user-supplied
[WSL receipt](/home/bryce/services/buzz-agents/releases/3738-relay-readonly-20261110/wsl-readonly-receipt.json)
records f316, schema 1–81, no deployment implementation/flag, and all three
deployment tables absent; no deployed host-deployment history exists on that
inspected relay at that receipt. This is read evidence, not my live inspection.
Conductor also confirms the current netcup controller remains registration-only
and prepared `c97256e` was never activated. The historical-envelope limitation is
therefore a future rollout condition, not an in-scope repair request for current
fresh-pilot acceptance. Check drift and reconcile final artifact provenance before
rollout. The `c97256e` package predates this shared dependency change and must not
be labeled current-source; no rebuild/install was done by this reviewer.

### Independent verification and handoff

Hermit activated with self-update disabled; existing loopback fixtures only:

- `cargo test --offline --locked -p buzz-ws-client --lib`: **5 passed**, 0 failed,
  0 ignored, 0.10s. These cover existing header/auth/timeout behavior.
- `cargo test --offline --locked -p buzz-paired-owner deployment -- --test-threads=1`:
  **2 passed**, 0 failed, 14 filtered, 14.39s, including actual exact wire bytes.
- Owner/ws-client package fmt, scoped native-fixture rustfmt, `git diff --check`
  and audit citation checks: **PASS**. Windows workflow unchanged from accepted
  `8cec4d7cf`; unchanged Node/browser suites were not rerun.

Current sender/native-peer Windows execution, full native IPC/keyring and human
workflow remain **NOT RUN**. The two preserved failed Windows runs are not green
gates. No third workflow, source export, dependency install, host artifact build,
live operation or board mutation was performed in review.

Recommendation: original executor may proceed with the third already-authorized
existing Windows run after this clean signed review commit releases ownership.
Require successful exact Cleanup and remaining native/ACL cases plus current
source/run/artifact association, then return evidence and any fixes to the same
reviewer. No source change is requested on this pin. All owned commands are
terminal; no temporary source/test edits or detached processes remain.

Requested configuration: **gpt-6-astra / high**; served identity/effort unknown.
No open in-scope source findings; overall merge-block remains the required native
verification gate, not a new counted source finding.

quality: model=unknown effort=unknown stage=post-audit verdict=merge-block verify=partial commits=8cec4d7cf5742b7d4fbcb3f093dd2242f6865d21..d093d6dcd556a6ef6017cf8a2a70c109c14627e8 findings=C=0/H=0/M=0/L=0 rework_rounds=2
