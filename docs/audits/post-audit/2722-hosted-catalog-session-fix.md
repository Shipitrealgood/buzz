# #2722 hosted catalog session fix — independent source review

2026-11-10 UTC. Reviewer `/root/hosted_catalog_fix_review`, independent of the
executor. Requested configuration: `gpt-6-astra` / high. Runtime model and
reasoning configuration are not independently exposed; requested settings are
not runtime identity evidence. Initial review, rework round 0.

## Verdict and routing

**Source ACCEPT; native Windows verification pending.** Findings:
**Critical 0 / High 0 / Medium 0 / Low 0**. No concrete source defect, missing
foundation, or implementation rework is recommended. This is not end-to-end
acceptance or a merge-safe verdict: native compilation and the two extended
Windows test cases have not run at this pin. The conductor owns the next existing
Windows canary run and subsequent pilot/human acceptance. The quality record uses
`rework` to retain that verification obligation, not to request a source fix.

Minimalism 9/10, elegance 9/10, source correctness 9/10: one existing projection
is extracted, the wrong operation guard is replaced by the existing session
guard, and the existing journey gains the regression. Runtime correctness remains
unmeasured here.

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

## Independent verification ledger

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
measurements. Per dispatch, no environment retry, installation, Linux substitute
test run or whole-toolchain build was attempted. The fixture has not been
typechecked by this review; prior Windows receipts concern earlier source.

## Handoff

Next action: conductor runs the existing Windows canary on a revision containing
the reviewed source and checks exact-case receipts, then owns live/human routing.
There are no proposed new tasks or architectural decisions. Source acceptance
does not authorize runtime activation or attest repository PR readiness.

Only this report is staged for its signed-off commit. All reviewer tool processes
are terminal; no background process or source mutation is held. Exclusive
write/test ownership returns with the report commit and clean-status receipt
provided in the reviewer return.

quality: model=unknown effort=unknown stage=post-audit verdict=rework verify=partial commits=d5cfb0d1c245ff0956d6a00289cf1f79e3cb73c7..160c8bfb853647ad336f296d0cff68c59cc75e62 findings=C=0/H=0/M=0/L=0 rework_rounds=0
