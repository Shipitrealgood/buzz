# #3721 trusted netcup pilot preparation — independent review

2026-11-10 UTC. **Rework: one Medium preparation defect, M-1.**
C=0 / H=0 / M=1 / L=0. The package matches its recorded source and bytes, but
the operator rollback must cover incomplete cutover prefixes before preparation
acceptance. **Not ready to activate**, independently of this repair: relay
deployment capability and owner-confirmed cleanup remain separate holds.

## Pin, authority and inputs

Reviewed `c97256e20893b2688c1cc1c6bc2e23361aab707e..a28772c9c9d88a109837f19ac60c177f2c7edf7f`
in `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`. Clean input verified. The delta contains
only the [execution packet](../../executions/3721-trusted-host-pilot-preparation.md)
and [review brief](../../briefs/3721-trusted-host-pilot-review.md); no product code.
Reviewer `/root/netcup_trusted_pilot_review`, role codex-chuck. Dispatch requested
`gpt-6-astra/high`; actual served model/effort metadata is not exposed: unknown.
Rework round 0.

Read the portable conductor contract, applicable AGENTS/VISION/agent and remote
agent vision, TESTING and registration architecture, the full changed documents,
the [accepted S1 review](3721-trusted-host-executor.md),
[S2/S3 review](3721-host-discovery-windows.md), and
[Windows test.21 gate](2722-shared-discovery-windows-canary.md). Applied post-audit.
Prior source/native tests and the conductor-supplied human smoke are inputs;
none is recast as a reviewer-run native pilot.

Scope is the six preparation obligations in the brief: cutover and interruption,
actual baseline/custody compatibility, normal-account configuration, artifacts,
truthful relay/cleanup holds, and bounded rollback preserving the existing agents.
No source edits, build, source-suite rerun, fault-fixture campaign, live manager
query, provider execution, SSH, authenticated relay access, private-state read,
service mutation, board write, merge, push or deployment occurred. Only staged
package files, source and recorded sanitized evidence were inspected/executed.

## M-1 (Medium, high confidence) — rollback only covers a completed mask — fix in lane

**Read/inferred control-flow defect; no live fault injection.** The
[cutover sequence](../../executions/3721-trusted-host-pilot-preparation.md#L330)
uses `set -eu`, stops/disables the old controller at line 336, backs up state and
binary/config, then moves the regular system unit at line 349 and creates its
mask at line 350. Each successful prefix is therefore a reachable interruption
state. The [rollback entry](../../executions/3721-trusted-host-pilot-preparation.md#L426)
unconditionally disables a USER unit that may not have been installed, and the
only [system restoration commands](../../executions/3721-trusted-host-pilot-preparation.md#L466)
require a `/dev/null` mask and a saved unit. The final interruption paragraph
says to keep the controller stopped; it supplies no missing recovery branch.

**Concrete trigger:** the state archive runs out of space at line 340. The old
controller is stopped/disabled, its original regular unit remains in `/etc`, the
backup unit does not exist, and no USER controller exists. Obeying the rollback
mask check refuses recovery; executing its unlink/move commands regardless would
remove the original unit and then fail to restore a nonexistent backup. Similarly,
interruption between `mv` and `ln -s` leaves an absent system-unit path, which is
not the required mask. This is an incomplete executable recovery contract, not a
claim that the runbook tells an operator to ignore its comments.

After custody changes, [lines 434–442](../../executions/3721-trusted-host-pilot-preparation.md#L434)
also leave the snapshot, numeric ownership/mode restoration and exact user-unit
removal as prose. There is no command branch for a partially completed recursive
chown or partially installed user files. Custody is load-bearing: both the
[current loader](../../../crates/buzz-hostd/src/store.rs#L58) and the recorded
rollback source `0bacd58f5369f61a4da1fd794c26d25503117a23:crates/buzz-hostd/src/store.rs`
require the state directory/database to be private and owned by the process UID.
The restored system service's StateDirectory behavior must not substitute for
explicit, verified custody restoration.

**Small bounded fix:** update staged OPERATOR.md and its exact committed copy,
with explicit shell commands/checks selected by the completed prefix. Keep the
existing human-operated runbook; do not build an installer or recovery framework.

| Completed prefix | Required recovery branch |
| --- | --- |
| Before old stop, or stop result uncertain | Inspect the actual old process/cgroup and completed operations; do not infer successful stop or transfer custody. |
| Old stop, original regular system unit still present | Verify its original hash and unchanged 988:988 state; keep that unit in place. Skip absent USER-unit operations only after confirming absence/quiescence. Restore the original enabled/running state without requiring a nonexistent backup unit. |
| Unit moved, mask not created | Verify the saved original unit and that the destination really is absent; restore the saved unit directly. Do not unlink an absent path or overwrite an unexpected path. |
| Mask installed; ownership/install partially or fully completed | Verify/retain the mask. Stop any installed/running USER controller and exact pilot unit before state operations. Give the private stopped-state snapshot, safe tree validation, explicit numeric 988:988 custody restoration and mode checks, including partial 988/1002 ownership and the known new private files. Preserve the latest database, sidecars, deployment journal and retained hosted state. Remove only owned, exact package user-unit/template files under the stated no-instance condition; reload the user manager as appropriate. |
| Ready to restore original system service | Verify saved original unit before removing an exact mask; restore only in the matching branch, then reload/start and check binding/hash/old-agent continuity. Unknown state or failed stop remains stopped for assessment, without identity reset or stale archive restoration. |

The root recovery block must stop on failed checks/commands, including a missing
backup; it must not continue into a start after a failed restore. Rehash the
package and update the manifest/brief/execution references after changing its
runbook. No changes to product source, current live services or shared login
files are required. Re-gate through source/prefix inspection, exact-text/hash
checks and shell syntax; no additional runtime failure campaign is requested.

| Finding | Owner / disposition / next action |
| --- | --- |
| M-1 | **fix in lane**, ruled by conductor during this review. Original preparation executor supplies the bounded runbook repair in this same checkout; this reviewer re-gates M-1 and defects introduced by that repair. |

## Remaining obligation assessment

| Obligation | Evidence and result |
| --- | --- |
| Single controller and identity | **Read:** normal forward ordering stops the system controller, saves its regular unit and masks it before chown/user start. Same `/var/lib/buzz-host-registration` is retained; no init/new identity, second state or root-helper drop-in. [CLI lock](../../../crates/buzz-hostd/src/registration_cli.rs#L110) and [store ownership](../../../crates/buzz-hostd/src/store.rs#L58) support the plan. Interrupted recovery remains M-1. |
| Current binary/unit and compatibility | **Read recorded evidence:** [November 9 rollout receipt](/home/bryce/services/buzz-agents/releases/3750-refresh-de05655071c5f6af/deployment/receipt.json) names installed `de05655071c5f6af3451cc70116969f53da0df0ff64448593a0fc913bf10d3ee`, source `0bacd58f5369f61a4da1fd794c26d25503117a23`, unit hash `9cb7a6d7b9df3e18b39d46916d1d7b32140e0567e123305dfe8685a5e8a03bce`, UID/GID 988 and unchanged public binding. The packet uses this later baseline, not the superseded #3751 binary. **Read/inferred compatibility:** old/current loaders accept version 1; [deployment schema](../../../crates/buzz-hostd/src/deployment.rs#L89) only adds tables. Preserve current DB instead of restoring stale state. No native rollback was run. |
| Trusted account/config/template | **Measured + read:** exact JSON sets capacity 1, `runtimes:{}`, matched `local.buzz_acp`, normal HOME/PATH and UID1002 runtime directory. [Discovery](../../../crates/buzz-hostd/src/discovery.rs#L30) expands main candidates. Exact staged template bytes equal pinned source and mode is 0600, satisfying [private template check](../../../crates/buzz-hostd/src/user_service.rs#L388). Current agent binaries/config and shared login files are outside the change. |
| Source/build/artifacts | **Measured:** Windows source `23d2dc15c6c45554181938b7956b59c085980455` to package source c97256e changes only three canary Markdown files. All eight manifest entries, manifest hash, binary hashes/sizes, Cargo.lock hash, exact staged/committed runbook/config and template compare successfully. Binary bytes also match the existing `target/release` outputs. Build provenance is the executor's [SOURCE.json](/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/c97256e20893b2688c1cc1c6bc2e23361aab707e/package/SOURCE.json) and build.log, not an independently reproduced build. |
| Relay hold | **Read recorded public response:** [public-nip11.json](/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/c97256e20893b2688c1cc1c6bc2e23361aab707e/public-nip11.json) has pinned key/registry v2 and lacks host_deployment. [NIP-11](../../../crates/buzz-relay/src/nip11.rs#L423) and [configuration](../../../crates/buzz-relay/src/config.rs#L678) require deployment enabled plus registry admission; [environment parse](../../../crates/buzz-relay/src/config.rs#L1471) accepts exact `true`. Absence cannot distinguish old code from disabled configuration. Separate serving-source/schema/config verification and compatible rollout remain required; no flag-only assumption. |
| Cleanup hold and emergency stop | **Read:** [HostedAction](../../../desktop/src-tauri/src/commands/agents_hosted_commands.rs#L256) has Check/Retry/Refresh/Launch only; [Close](../../../desktop/src/features/agents/ui/HostedAgentSetup.tsx#L197) dismisses. [Owner retention](../../../crates/buzz-paired-owner/src/deployment.rs#L96) rejects non-Deploy; [continuation](../../../crates/buzz-paired-owner/src/deployment/continuation.rs#L226) builds Deploy. [Lifecycle](../../../desktop/src-tauri/src/commands/agents.rs#L1037) and [Delete](../../../desktop/src-tauri/src/commands/agents.rs#L1253) refuse RegisteredHost. [Host Cleanup](../../../crates/buzz-hostd/src/deployment.rs#L453) exists underneath. The packet correctly calls emergency stop containment with retained state, not confirmed teardown. Route the owner/setup gap through #3724/#3725, not legacy #3767. |

## Independent checks and limits

Evidence/package root is
`/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/c97256e20893b2688c1cc1c6bc2e23361aab707e/`.
Original manifest SHA256:
`02e2ffe9adcb06a8c83b51b51921c0c83e93654de6ad0b222f2b4318b1655490`.

| Check actually performed | Result |
| --- | --- |
| Hermit activated with `HERMIT_SELF_UPDATE=false`; Git pin/status/diff and docs-only continuity | PASS; correct clean input and exact two-doc delta. |
| `sha256sum -c MANIFEST.sha256`, independent Python hashes/sizes/byte comparisons | PASS, 8/8 entries; binary sizes 18,201,640 / 3,818,744 / 18,545,880 bytes and hashes match the execution packet. |
| Exact staged OPERATOR/config/unit text, source template bytes/mode, JSON structure and local document links | PASS. This does not invoke production Catalog validation against future live paths. |
| Six OPERATOR shell fences through `bash -n` | PASS; syntax only, zero runbook commands executed. |
| Staged hostd help, registration-run help, ACP-models help | PASS exit 0 each, no state/relay/provider invocation. |
| Staged runtime `--help` | Expected refusal exit 1; no help mode and no runtime launched. |
| `systemd-analyze --user verify` on staged controller and synthetic hosted instance | Both blocked locally, exit 1: `SO_PASSCRED failed: Operation not permitted`. No escalation or manager action. The executor's two exit-0 [syntax receipts](/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/c97256e20893b2688c1cc1c6bc2e23361aab707e/syntax-verification.json) were read, not promoted to reviewer passes. |
| Recorded existing-agents-before/after byte comparison | PASS. This verifies the recorded continuity evidence, not a fresh live observation. |
| Current rollback source loader vs package source loader | READ; same version-1 acceptance, additive deployment tables. |
| `git diff --check` on reviewed range | PASS. |

Reading the older root-owned #3751 post-pair receipt was permission-denied; no
escalation was requested. The readable later #3750 rollout receipt includes both
public bindings and installed hashes, so no baseline claim depends on that read.
No required product test was skipped: the dispatch excludes rerunning accepted
source/Windows suites and a new runtime campaign. Native systemd credential
delivery/notify/cgroups, providers/login/models/chat, Windows hosted creation,
confirmed cleanup, actual rollback, reboot and full CI remain **NOT RUN**.
The blocked optional syntax rerun is disclosed above; syntax never certifies
native activation. No PR-readiness or `buzz-review-completed` attestation.

## Routing and handoff

Return M-1 to the original executor for the bounded runbook repair, then re-gate
the repaired pin. Retain both independent activation holds after preparation
acceptance; the conductor owns their routing and the later exact live-approval
request. No new architecture/specification stage or task is proposed.

Minimalism and elegance: 9/10. Correctness: 8/10 for the missing prefix recovery;
the repair above closes that concrete defect. Strong parts of the packet are its
explicit shared-account host-key custody, correct later rollback binary, retained
current journal, exact artifact text and truthful cleanup/capability limits.

Only this report is changed by the reviewer. All local review commands are
terminal; no background build/test/provider process was launched. Exclusive
write/test ownership returns to the conductor with the signed report commit and
clean checkout; keep the same worktree for repair.

quality: model=unknown effort=unknown stage=post-audit verdict=rework verify=partial commits=c97256e20893b2688c1cc1c6bc2e23361aab707e..a28772c9c9d88a109837f19ac60c177f2c7edf7f findings=C=0/H=0/M=1/L=0 rework_rounds=0
