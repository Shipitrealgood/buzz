# #3734 / #3721 — independent netcup preparation review

2026-11-10. **Merge-safe for the bounded P1 preparation packet only.**
C=0 / H=0 / M=0 / L=0. No preparation merge blocker or source repair requested.
WSL relay artifact/compatibility evidence and explicit live approval remain
outstanding; this verdict does not mean the overall rollout is live-ready.

## Pin, authority and inputs

Reviewed `344baa091ad486653ee99aa8b1de64d4839b4ada..354961fe1d78f0338ca63c2e95c3a20c61adca03`
in `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`. Independently confirmed clean input,
full HEAD and branch. The range adds only the execution receipt and review brief
(878 lines); build HEAD remains344baa0. Rework round0 for this assignment.
Reviewer `/root/netcup_rollout_review`, codex-chuck; requested configuration
**gpt-6-astra / high**. Served model/effort metadata is unavailable: unknown.

Applied post-audit and read the portable conductor contract, project AGENTS,
VISION/agent/remote-agent vision, applicable TESTING/ACP guidance and registration
architecture. Read both changed files in full, [P1 specification](../../specs/3734-hetzner-netcup-rollout-preparation.md#L59),
the [six dispatched obligations](../../briefs/3734-netcup-rollout-review.md#L44),
accepted [M-1 re-gate](3721-trusted-host-pilot-preparation.md#L3) and
[final Cleanup review](3725-confirmed-hosted-cleanup.md#L387).
These prior reviews are accepted inputs, not reopened source campaigns.

Exclusive review/report ownership transferred after executor processes stopped.
No implementation/package edits, rebuild, live query/action, real-state/key read,
SSH, provider command/login, relay fixture, Windows run, taskctl, push or merge.
The WSL prompt was read solely for packet consistency; it is an instruction,
not an execution receipt. No second verification framework or mutation campaign.

## Source and artifact continuity

**Measured:** Git929c2cd890fbc2c0993549c3cbc64a2714232832→344baa0 differs in
exactly the six Markdown paths recorded in SOURCE.json and
production-continuity.txt. No production/test/build inputs differ. The retained
scoped diff from c97256e exactly matches host-dependency-delta.patch and changes
only [the typed EVENT sender](../../../crates/buzz-ws-client/src/connection.rs#L179).
Cargo.lock hash matches `060b312fd66fd7cea5c1d8415143dad0bd710691e8fe024c6519fb09fb50d14f`.
Retained Cargo `.d` files associate both host binaries with that changed source;
ACP's recorded source-input set has no changes. All three package binaries match
actual `target/release` output bytes and their SOURCE sizes/hashes. Hostd/runtime
differ from c972; ACP is byte-identical to c972, consistent with unchanged inputs.

**Read provenance, not an independently reproduced build:**
[build.log](/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/344baa091ad486653ee99aa8b1de64d4839b4ada/build.log)
records ws-client and hostd compilation and optimized completion in19.93s.
[SOURCE.json](/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/344baa091ad486653ee99aa8b1de64d4839b4ada/package/SOURCE.json)
records offline/locked command, actual344baa0 build pin, rustc/cargo1.98.1 and
x86_64-unknown-linux-gnu. These receipts plus source/build-output continuity
support accepting this refresh without rerunning the build. Accepted Windows
test.24/111 native cases+ACL remain credited at unchanged929c production source;
they are not reviewer-run tests or human evidence in this pass.

Package root:
`/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/344baa091ad486653ee99aa8b1de64d4839b4ada/package/`.
Independently recalculated all eight manifest hashes and sizes:

| File | Bytes | SHA-256 |
| --- | ---: | --- |
| buzz-hostd | 18,203,680 | `3a31f56b453ec5c6c8d5e1140a5921bd2bfd392b8207c6071f0cd86b3b6f054d` |
| buzz-host-runtime | 3,818,728 | `98404e872a266d94457083bde657e179f3ab19fcd563771ac4e41f994c7bc16a` |
| buzz-acp | 18,545,880 | `b7fe18f1604db51dfeffc8d392676d8755f872662be710b87e84c11fc1e83f33` |
| deployment.json | 917 | `f019bb6dc7b256cfb931f950fa45d53e95a677f6d138e7b3b7321f90c64e376b` |
| buzz-host-registration.service | 952 | `0c5fb6ec7122a38d19d8ca30a0f9d89fdb22e9164072372f872a3e9664702ae0` |
| buzz-hosted-agent@.service | 431 | `6c7759314a7da08cd018f29714edffa1b5f078e4d191f1d5d53993094e686394` |
| OPERATOR.md | 30,043 | `34653012322ebb0ba0c998d7b8fca065aae04d5741e9310d6c6c66e752aaff69` |
| SOURCE.json | 6,789 | `98cc5bc669c6d4b1155d07bbcded82c340dae2a987c939118c45a988d52f8f9b` |

Manifest SHA-256:
`ec1fb606e835ba238a39021a3ad17d90e1f8c3ea933b568f2fa5db78e5671441`.
The old c972 manifest remains
`f8a53553ca83eb4fe56481d9a257d9226840d1908ccbb888c9d4e8829cb94898`,
with all8 old entries independently verified. No moving alias or installed target
was used. The exact staged OPERATOR/config/controller/template bytes equal the
text obtained from the pinned Git receipt, not merely a working-copy comparison.

## Six-obligation assessment

| Obligation | Evidence and outcome |
| --- | --- |
| Actual build/source continuity | PASS measured/read as above; source pin is truthful and all binary bytes match build outputs. |
| Accepted prefix recovery | PASS measured: all9 shell fences equal accepted c972 M-1 after only full release-pin and backup-name substitution; both Python bodies are identical. Read [original/moved/masked branches](../../executions/3734-hetzner-netcup-rollout-preparation.md#L477), [new-controller quiescence](../../executions/3734-hetzner-netcup-rollout-preparation.md#L539), [current-state custody](../../executions/3734-hetzner-netcup-rollout-preparation.md#L612) and final restoration. Same identity/journal,988:988 restoration and private CURRENT snapshot survive. No new interruption fault campaign. |
| Cleanup versus containment | PASS read: [Delete/Check/Retry instructions](../../executions/3734-hetzner-netcup-rollout-preparation.md#L444) require authenticated Cleaned plus local completion, retaining identity/intent while pending. Traced ordinary Delete through [delete_if_hosted](../../../desktop/src-tauri/src/commands/agents.rs#L1205), [HostedAction routing](../../../desktop/src-tauri/src/commands/agents_hosted_commands.rs#L288) and [Cleanup coordination](../../../desktop/src-tauri/src/commands/agents_hosted_cleanup.rs#L131). This matches accepted source. [Exact-unit emergency stop](../../executions/3734-hetzner-netcup-rollout-preparation.md#L584) remains containment and retains pending state. |
| Relay/WSL and flag boundary | PASS read: [packet hold](../../executions/3734-hetzner-netcup-rollout-preparation.md#L125) and [WSL handoff](../../briefs/3734-wsl-relay-preparation.md) agree on exact old image, candidate929c, code+82+literal relay flag, retained82/current data, auto-migrate=false, old-image community-deletion limitation and no substitute old artifact. Same-candidate flag-off is preferred with admitted-attempt limitation;90s ordinary-traffic/readiness threshold is preserved. WSL preparation/live authority and primary-only ownership remain separate. |
| Trusted-account paths/configuration | PASS measured/read: JSON parses; capacity1/runtimes{} and exact package ACP/runtime paths; normal HOME/PATH/projects/shared native login; normal user bus. Config/controller differ from accepted c972 solely in release path. Source/staged/old hosted template bytes match, staged mode0600. [Custody disclosure](../../executions/3734-hetzner-netcup-rollout-preparation.md#L259) remains explicit. No old-agent install target, private HOME or copied provider login was introduced. |
| Checks versus actual operation | PASS with limits below. Initial syntax failures are preserved; retry is syntax only. Historical manager/Grok/agent measurements are expressly subject to approved drift checkpoint. No claim that native credentials, notify, cgroups, provider readiness or human behavior ran here. |

## Independent verification actually run

Hermit activated with self-update disabled for Git operations. All commands ran
to terminal completion; no detached process or temporary source mutation.

| Check | Actual result |
| --- | --- |
| Full pins/branch/clean tree; source/diff and retained dependency inputs | PASS; two changed Markdown files; six Markdown-only production-continuity paths. |
| New/old manifest entry hashes; binary-output equality; JSON/paths; template bytes/mode; pinned committed text | PASS,8/8 each manifest and all3 binaries. |
| Accepted normalized shell comparison; `bash -n`; Python `ast.parse` | PASS9 shell blocks,2 Python bodies. Neither operator shell nor state-validation Python was executed. |
| Staged `buzz-hostd --help`, `registration run --help`, `buzz-acp models --help` | PASS, exit0 each, timeout-bounded; help only. |
| Staged `buzz-host-runtime --help` | Expected exit1,35 output characters; source rejects this argument before runtime entry. No runtime launched. |
| `systemd-analyze --user verify` controller and existing synthetic64-hex instance, sandbox | Both exit1: `SO_PASSCRED failed: Operation not permitted`. Environmental restriction, not a product defect. |
| Same two read-only systemd syntax commands, approved narrow retry | Both exit0, empty diagnostics. No start/notify/credential/cgroup proof. |
| Changed documents' local links/line targets; pinned range `git diff --check` | PASS,35 local targets and clean whitespace. |

Read executor [failed syntax receipt](/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/344baa091ad486653ee99aa8b1de64d4839b4ada/syntax-verification-sandbox.json)
and [successful syntax receipt](/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/344baa091ad486653ee99aa8b1de64d4839b4ada/syntax-verification.json);
their outcomes agree with this independent rerun. Their SHA-256 values are
`ec53c46f1e3d5451b29aed4f90f7f1bc5f738efb22a73b542848690a5db4f0f3`
and `e7f63f182eb3608c8f3fc72bac68838bd812530962f8e9d450841d7675bdf9f9` respectively.
Build log SHA-256 is
`958b33f703e2647d6cef456dc9eaa582bbb9b0fd326509abcd35553f28116e0b`.

**Not run:** build; accepted broad source/Windows suites; full just ci; real
manager/controller/DB/key/provider inspections; native service activation and
credential/notify/cgroup behavior; models/login/chat; hosted creation/Cleanup;
relay compatibility fixtures/live rollout/rollback; reboot/two-host/Android;
human test.24. No PR-readiness or `buzz-review-completed` attestation.

## Routing and release

Recommend conductor acceptance of P1 independently. No finding needs a new task,
architecture ruling or executor rework. Minimalism/elegance/correctness9/10 each
for this bounded packet. Its strongest properties are exact recovery preservation,
truthful build provenance, retained-current-state custody and explicit pending
Cleanup rather than manufactured completion.

Keep the separately owned WSL artifact, exact deployed
`sha256:1da22b2856a998de760a12b0767b30f0daf30e65b580ff193054fa17a4c85044`
on82 compatibility proof and independent WSL review pending. A supplied prompt
does not discharge them. After those gates, explicit approval must precede live
checkpoint/load/migration/restarts/flags/custody/provider/pilot actions. Preserve
the accepted fresh-pilot rule and drift check: unexpected historical deployment
data stops rollout for compatibility assessment. No normal DB restore or schema
rollback. Human test.24, actual service/provider/chat and confirmed pilot Cleanup
remain post-approval acceptance gates ([packet](../../executions/3734-hetzner-netcup-rollout-preparation.md#L137)).

Only this audit artifact is changed by review. All owned commands are terminal;
no background test/build/fixture or package mutation remains. Release the same
workspace at the signed audit commit after clean-status verification. Conductor
owns board dispositions and subsequent approval routing.

quality: model=unknown effort=unknown stage=post-audit verdict=merge-safe verify=ran commits=344baa091ad486653ee99aa8b1de64d4839b4ada..354961fe1d78f0338ca63c2e95c3a20c61adca03 findings=C=0/H=0/M=0/L=0 rework_rounds=0
