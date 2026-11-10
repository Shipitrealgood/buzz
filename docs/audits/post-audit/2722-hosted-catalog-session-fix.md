# #2722 hosted catalog session fix — independent source and native review

2026-11-10 UTC. Reviewer `/root/hosted_catalog_fix_review`, independent of the
executor. Requested configuration: `gpt-6-astra` / high. Runtime model and
reasoning configuration are not independently exposed; requested settings are
not runtime identity evidence. Same review resumed for native receipts; source
rework round 0 (no implementation changes).

## Verdict and routing

**Source, native Windows gate and installer artifact ACCEPT.** Findings:
**Critical 0 / High 0 / Medium 0 / Low 1**. No merge blockers, missing foundation,
or catalog implementation rework. The prior pending native obligation is closed
by independently inspected exact-source Windows receipts below. `merge-safe`
applies to this bounded catalog repair review; it does not attest full repository
PR readiness, installer execution or live/human acceptance. The conductor owns
those remaining pilot steps and optional L-1 reliability routing.

Minimalism 9/10, elegance 9/10, source correctness 9/10: one existing projection
is extracted, the wrong operation guard is replaced by the existing session
guard, and the existing journey gains the regression. Native fixture execution
is now evidenced; the full live UI workflow remains unmeasured.

## Pin, scope, and contract

Workspace `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`; clean at review start. Exact source range:
`d5cfb0d1c245ff0956d6a00289cf1f79e3cb73c7..160c8bfb853647ad336f296d0cff68c59cc75e62`.
The entire commit diff and both changed Rust files were read, including unchanged
creation/recovery steps and callers. Only this audit artifact is reviewer-owned.

Inputs: [execution and review brief](../../executions/2722-hosted-catalog-session-fix.md),
[accepted hosted-flow contract](../../specs/2722-windows-hosted-flow.md), root
AGENTS, VISION and VISION_REMOTE_AGENTS, applicable TESTING/CONTRIBUTING guidance,
architecture overview, personal instructions and the shared conductor contract.
The current bounded dispatch governs the source gate; prior broader acceptance
does not become fresh runtime evidence.

Excluded: relay/auth re-audit, clock-skew and Retry-button work, live services,
providers, agent identities or journals, environment/toolchain repair, workflow
dispatch, push, merge and board mutation. No implementation file was changed.

## Acceptance evidence

All behavior claims below are **read-derived**, not executed native results.

1. **Read without a setup operation.** The actual command still passes empty
   operation/runtime strings to `open`, then calls the tested `catalog` helper
   ([commands:149](../../../desktop/src-tauri/src/commands/agents_hosted_commands.rs#L149)).
   The helper only projects existing status/canonical metadata, checks the session
   and clones status. It creates no token, identity or deployment intent. Its
   projection logic is otherwise unchanged. `open` still reads status with
   `operation_id: None` and verifies it through `wire::open_status` before
   constructing `Create` ([open:162](../../../desktop/src-tauri/src/commands/agents_hosted.rs#L162)).
   `open_status` checks the signed host event, decrypted scope/header, owner,
   capability/inventory and expiry
   ([wire:404](../../../crates/buzz-core/src/host_deployment/wire.rs#L404)).
   The test uses synthetic Status, so it does not itself exercise remote status
   verification; preservation of that unchanged path was reviewed in source.

2. **Correct guard retained.** Final `Session::keys` propagates failure
   ([commands:194](../../../desktop/src-tauri/src/commands/agents_hosted_commands.rs#L194));
   its existing implementation rejects a switch in progress, changed generation
   or origin, incomplete reset and changed signer
   ([session:78](../../../desktop/src-tauri/src/commands/paired_hosts.rs#L78),
   [checked keys:109](../../../desktop/src-tauri/src/commands/paired_hosts.rs#L109)).
   Actual creation still uses `Create::check`, including deployment-intent and
   authority checks ([create:19](../../../desktop/src-tauri/src/commands/agents_hosted.rs#L19)).
   Empty operation remains invalid
   ([intent:44](../../../crates/buzz-paired-owner/src/deployment.rs#L44)).

3. **Meaningful production-bound regression.** Both existing fixture cases call
   `native_phases` ([tests:5](../../../desktop/src-tauri/src/commands/agents_hosted_tests.rs#L5)).
   The added steps assert empty-operation rejection by the actual create guard,
   success through the same helper called by the command, one `native/grok`
   entry, canonical command/arguments, available inventory, unknown auth, status
   preservation and no deployment intents. They then change generation and
   require the exact session error
   ([tests:135](../../../desktop/src-tauri/src/commands/agents_hosted_tests.rs#L135)).
   Canonical metadata is a pure existing builtin/preset lookup
   ([discovery:1031](../../../desktop/src-tauri/src/managed_agents/discovery.rs#L1031));
   Grok arguments agree with the shared preset
   ([preset:74](../../../crates/buzz-core/src/harness/presets.rs#L74)).
   JSON expectations match camelCase outer DTOs, snake_case metadata fields and
   tagged auth ([types:644](../../../desktop/src-tauri/src/managed_agents/types.rs#L644),
   [metadata:674](../../../desktop/src-tauri/src/managed_agents/types.rs#L674)).
   Status already implements Clone; helper visibility, immutable borrowing and
   recapture assignment show no source-level type/borrow incompatibility.

4. **Prior journey and failure propagation survive.** Session recapture and
   restoration of the original operation/runtime precede the unchanged rejection,
   create, durable recovery and cleanup steps
   ([tests:181](../../../desktop/src-tauri/src/commands/agents_hosted_tests.rs#L181)).
   No prior test is removed, ignored or bypassed. The subprocess wrapper requires
   successful exit and a passed-test result; new assertions propagate failures.
   The unchanged canary requires both exact test names to report one passed,
   zero failed, zero ignored
   ([workflow:284](../../../.github/workflows/windows-canary.yml#L284)).

**Falsifiability, inferred only:** restoring `create.check()?` in the helper
would fail catalog success with `invalid operation`; deleting the final session
guard would fail the stale-generation assertion. Neither mutation was executed,
and neither is claimed as a measured kill against a green native baseline.

## Initial independent verification ledger

Historical source-gate results at `444d18ef`; native NOT RUN entries below are
superseded only by the subsequent Windows evidence section, not erased.

Commands used `HERMIT_SELF_UPDATE=false` and `. ./bin/activate-hermit` after
reading the repository instruction; initial read-only Git discovery preceded it.

| Check | Reviewer result |
| --- | --- |
| `rustfmt --edition 2021 --check desktop/src-tauri/src/commands/agents_hosted_commands.rs desktop/src-tauri/src/commands/agents_hosted_tests.rs` | PASS; parser/format check only |
| `git diff --check 160c8bfb8^ 160c8bfb8` | PASS |
| Pinned `git diff --exit-code` for `agents_hosted.rs`, `paired_hosts.rs`, owner `deployment.rs`, and `windows-canary.yml` | PASS; all four unchanged |
| `CHECK_FILE_SIZES_BASE=d5cfb0d1c245ff0956d6a00289cf1f79e3cb73c7 node desktop/scripts/check-file-sizes.mjs` | PASS, exit 0 after read-only escalation; first sandbox run failed `spawnSync git EPERM` |
| Native Windows compile and both extended hosted cases | NOT RUN |
| Native failing-before/passing-after controls | NOT RUN |
| Full `just ci`, live pilot and human acceptance | NOT RUN; outside this dispatch |

The module is explicitly Windows-gated
([agents:418](../../../desktop/src-tauri/src/commands/agents.rs#L418)).
The [executor environment receipt](../../executions/2722-hosted-catalog-session-fix.md#verification-ledger)
reports only `x86_64-unknown-linux-gnu` installed and missing GLib/GObject/GIO,
GTK and WebKit packages. Those are executor measurements, not fresh reviewer
measurements. Per initial dispatch, no environment retry, installation, Linux
substitute test run or whole-toolchain build was attempted. At that stage the
fixture had not been typechecked; prior Windows receipts concerned earlier source.

## Native and installer evidence disposition

Continuation input: [Windows execution receipt](../../executions/2722-hosted-catalog-windows-canary.md).
Clean incoming HEAD `684a7376f9d14adfff1c941c7d66101346ea2d65` adds only that receipt
after the prior audit commit. **Measured locally:** `git diff --stat` from
`160c8bfb` to incoming HEAD contains only two documents; explicit zero diffs
confirm command/test source, workflow and owner expiry fixture unchanged.
Built/tested source is `444d18ef03b14354373393cd297323a17b9065e8`, production-equivalent
to the reviewed fix `160c8bfb853647ad336f296d0cff68c59cc75e62`.

Evidence root (local retained artifacts, not independently fetched again):
`/home/bryce/services/buzz-agents/releases/2722-hosted-catalog-canary/444d18ef03b14354373393cd297323a17b9065e8/42918144439/`.
Reviewer read saved run/jobs/artifact metadata, original failure logs, exact-case
logs, source/build receipts and relevant full native/build log lines. Execution
outcomes below are **read from CI records**; hash, archive-byte comparison, PE
signature and distinct-case counts are **measured independently locally**.

[Run 42918144439](https://github.com/Shipitrealgood/buzz/actions/runs/42918144439)
attempt 2 completed successfully at the exact accepted SHA. Full logs show checkout,
native `x86_64-pc-windows-msvc` compilation and test execution, not a Linux filter
or zero-match result. Eight individual logs contain **111 distinct outer Rust
cases**, all passed, zero failed/ignored: owner 16, harness 9, conversion 73,
directory 7, paired-host session 3, plus the three exact cases below. The UTF-16
ACL receipt reports successful owner SQLite access and denial of DB/WAL/SHM to
the standard-user control. Counts exclude echoed grep output and child duplication.

| Exact native case | CI receipt independently read |
| --- | --- |
| `commands::agents::hosted::tests::hosted_native_create_recovery_and_session_fence` | 1 passed, 0 failed/ignored; 9.86s |
| `commands::agents::hosted::tests::hosted_native_cleanup_confirmation_retry_and_session_fence` | 1 passed, 0 failed/ignored; 10.58s |
| `managed_agents::discovery::presets::hosted_metadata_tests::hosted_grok_metadata_uses_static_preset_without_local_binary` | 1 passed, 0 failed/ignored |

These two unchanged journey entry points run the reviewed catalog assertions and
continue through existing recovery/cleanup. This closes compilation/execution
and the original native gate; it does not prove mutation kills or remote IPC/UI.
Successful native evidence is artifact **13811888126**, not the similarly named
first-attempt artifact **13810468291**.

The original installer job succeeded at `22:50:10Z–23:07:24Z`. Attempt-2 jobs
metadata repeats these exact original build timestamps while native executes
at `23:08:35Z–23:24:05Z`; this supports reuse of the successful original installer,
not a second source/build. Full build log records the NSIS output and the same
SHA/version as `installer/build-receipt.txt`. Installer is
**Buzz_0.5.29-test.25_x64-setup.exe**, **56,468,766 bytes**, PE signature checked.
The existing workflow produces an unsigned, updater-disabled canary. It was not
executed by the reviewer or executor.

| Artifact | Independently recomputed SHA-256 |
| --- | --- |
| Installer executable | `c21c0a460901f44ac6f56d1aaf753bdcead49ff8288c3d3894b38eac0f45e159` |
| Installer ZIP, artifact 13810958555, 56,467,134 bytes | `4881e6d66d4a1aaa8bf37668e13f130203dd199f79794fa67190ec76978748ae` |
| Attempt-2 native ZIP, artifact 13811888126, 5,400 bytes | `2c7fbf0cf4e64cc50a65a61d3c29d0414f67a6526fda23c902d5e94d68dacf5d` |
| Preserved attempt-1 native ZIP, artifact 13810468291, 1,506 bytes | `c1704e512dcdf05164127fdea99f141ede7c2a93971729c201b223cf346e8129` |

ZIP hashes/sizes match saved GitHub artifact metadata; executable hash matches
the build receipt. Every ZIP member matches extracted bytes. Installer ZIP has
exactly the installer and build receipt; successful native ZIP has source receipt,
eight suite logs and ACL log. Failure ZIP has source, failed owner and ACL logs.
Source/run/target receipts agree. Artifact IDs/retention and local paths are
indexed in the committed execution receipt; full raw files remain at the root
above. No claim of independent fresh GitHub retrieval is made.

### L-1 (Low, high confidence) — existing expiry fixture has a startup timing dependency; rides #2722

The unchanged owner fixture sets `expires_at = now()+2` before crypto and temporary
Device setup, then unwraps the first prepare
([expiry:27](../../../crates/buzz-paired-owner/src/deployment_tests.rs#L27),
[prepare:66](../../../crates/buzz-paired-owner/src/deployment_tests.rs#L66)).
There is no recorded timestamp at that call. Attempt 1 measured 15 passes and one
failure at line 79 (`invalid or unsupported host deployment contract`), before
the hosted tests ran; full suite duration was 40.77s. The authorized single
same-source retry passed all 16 owner cases in 25.82s and then the native gate.
These durations do not establish the failing call's elapsed time. Expiry is a
plausible explanation, **not a proved cause or a repaired defect**.

Conductor disposition received in this review: retain reliability follow-up on
**#2722** (conductor cites comments 11519/11520); nonblocking for this bounded fix
and pilot, with no code repair authorized in this increment. The reviewer did not
independently query the board. If pursued, distinguish setup-time
validity from the deliberate later expiry step without weakening expiry checks.
Keep the first failure available for future recurrence. No new test campaign,
source repair, board task or additional rerun was performed by this reviewer.

Remaining NOT RUN: before/after mutation controls, full `just ci`, installer
execution, actual hosted IPC/UI/live status flow, real provider/login/chat/service
behavior and human acceptance. Mutation execution was excluded from this finite
workflow dispatch; initial source falsifiability remains an inference. These
limits do not reopen the completed bounded native/artifact gate.

## Handoff

Next action: conductor can hand off this verified canary artifact for its
authorized Windows pilot and human acceptance, keeping L-1 as optional reliability
routing. No new architectural decision or catalog repair is needed. Acceptance
does not authorize runtime activation or attest repository PR readiness.

Only this report is staged for its signed-off commit. All reviewer tool processes
are terminal; no background process or source mutation is held. Exclusive
write/test ownership returns with the report commit and clean-status receipt
provided in the reviewer return.

quality: model=unknown effort=unknown stage=post-audit verdict=merge-safe verify=ran commits=d5cfb0d1c245ff0956d6a00289cf1f79e3cb73c7..160c8bfb853647ad336f296d0cff68c59cc75e62 findings=C=0/H=0/M=0/L=1 rework_rounds=0
