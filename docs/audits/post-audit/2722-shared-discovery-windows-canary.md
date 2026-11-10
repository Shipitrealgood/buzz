# #2722 shared discovery Windows canary — independent review

2026-11-10 UTC. **Merge-safe for the bounded Windows native/artifact unit.**
C=0 / H=0 / M=0 / L=0; no merge blockers. The pending Windows execution obligation
for the accepted M-1 repair is satisfied. This verdict does not certify whole-repo
merge readiness, installation, activation or live end-to-end behavior.

## Scope and continuity

Reviewed `da4ee105a34ff45e84d5e44e64c3e31d3a3c9651..7e8f870e14ab47cc9d2b349357f0d6d50decb955`
in `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`. Clean input pin confirmed. Built source
`23d2dc15c6c45554181938b7956b59c085980455` contains both the accepted baseline and
M-1 repair `5d72b08d3e5ab7bbcd52711e17a43572e184e47b`; the later input commit
changes only the two handoff documents. The reviewed behavioral delta is eleven
workflow lines, with no native source correction.

Inputs: [bounded brief](../../briefs/2722-shared-discovery-windows-canary-review.md),
[execution record](../../executions/2722-shared-discovery-windows-canary.md),
[accepted source re-gate](3721-host-discovery-windows.md#re-gate-round-1--source-repair-accepted-native-verification-pending),
and [prior test.20 review](2722-windows-native-canary.md). Read applicable
AGENTS/VISION, agent/remote-agent vision, testing and portable conductor guidance;
applied post-audit and gate-dispatch. Accepted architecture and Linux/Node evidence
were carried forward, not re-audited. Requested configuration: **gpt-6-astra/high**.
Served runtime model/effort: **unknown**; no authoritative runtime identity was
exposed to this reviewer. Rework rounds for this unit: **0**.

## Independent source assessment

**Read:** [workflow:263](../../../.github/workflows/windows-canary.yml#L263)
runs `cargo test --locked --target "$TARGET" -p buzz-core --lib harness::` on
Windows and requires nine named success lines plus exactly nine passes and zero
failed/ignored. The recorded names match the nine source tests in
[main harness:268](../../../crates/buzz-core/src/harness/mod.rs#L268),
[runtime metadata:198](../../../crates/buzz-core/src/harness/runtime_metadata.rs#L198)
and [installer strings:104](../../../crates/buzz-core/src/harness/windows_install.rs#L104).
The six installer cases, vendor metadata, registry/default arguments and shared
classifiers all execute. Existing owner, conversion, directory, registration,
exact hosted/metadata and ACL gates remain. Bash `set -euo pipefail` propagates
Cargo failure through `tee`; empty, ignored or missing named results cannot meet
the new predicates. Exact hosted checks retain both the exact name and one-pass,
zero-failed/ignored requirements at [workflow:284](../../../.github/workflows/windows-canary.yml#L284).

The executor describes five mock-Cargo scenarios in the
[execution record](../../executions/2722-shared-discovery-windows-canary.md#launched-run).
No separate raw probe receipt was retained in the supplied evidence. These remain
executor-described measurements, independently assessed through the Bash predicates
and actual native logs, **not reviewer-run probes or mutant kills**. The conductor
confirmed this disposition; no unresolved failure-propagation concern warranted a
repeat or additional controls.

**Read + remote evidence:** the [outer hosted test:6](../../../desktop/src-tauri/src/commands/agents_hosted_tests.rs#L6)
launches its exact child test with disposable HOME/app-data and requires successful
child exit plus a pass. The child reaches the unconditional
[four-scenario loop:145](../../../desktop/src-tauri/src/commands/agents_hosted_tests.rs#L145):
missing inventory, missing runtime, unavailable and unsupported. Each invokes real
creation phases, requires the inventory error, and checks no intent, saved agent,
queued publication, accepted loopback connection or tracked process. The
[original recovery/session path:215](../../../desktop/src-tauri/src/commands/agents_hosted_tests.rs#L215)
follows the loop. The current native pass therefore covers these assertions;
these are **four subcases of one outer test**, not four additional Cargo cases.
The early [inventory guard:435](../../../desktop/src-tauri/src/commands/agents.rs#L435)
remains before creation effects. No native compiler/test repair was needed.

**Read + measured:** manual dispatch and fork/ref restrictions, credential-free
checkout, read-only contents permissions and disabled updater artifacts remain in
[the workflow](../../../.github/workflows/windows-canary.yml#L10). No release/tag
or main-merge action exists in it. The installer PE certificate table is empty,
consistent with the unsigned build. The [deployment flag:143](../../../desktop/src-tauri/src/commands/agents_hosted.rs#L143)
still requires exact value `1`; default-off behavior is unchanged. This preserves
the bounded product contract. Minimalism, elegance and correctness: **9/10 each**.

## Terminal evidence and independent local measurements

Fresh read-only GitHub API calls confirmed [run 42803199965](https://github.com/Shipitrealgood/buzz/actions/runs/42803199965)
was manual, attempt 1, run number 21, terminal SUCCESS at source `23d2dc15c`.
Both jobs have that head SHA: native **129785693525**, SUCCESS in **15m6s**;
build **129785692135**, SUCCESS in **21m40s**. All required native/build steps
succeeded. Both checkout logs and canary version lines agree with the metadata.

Evidence root: [/42803199965/](/home/bryce/services/buzz-agents/releases/2722-windows-canary/23d2dc15c6c45554181938b7956b59c085980455/42803199965/).
Read staged run/jobs/artifact/verification JSON, source/build receipts and actual
native/build logs. Fresh API responses are `review-run.json`, `review-jobs.json`
and `review-artifacts.json` there. Native log result lines are 707, 769, 2192,
2432, 2667, 2918 and 3152; ACL success is line 3165. Build artifact digest/ID are
lines 1781–1784. Individual `tests/` receipts avoid duplicate grep echoes.

| Windows gate | Independently counted recorded result |
| --- | --- |
| Owner library | 16 passed |
| Shared harness | 9 passed; exact source-name set matched |
| Conversion | 73 passed |
| Relay directory | 7 passed |
| Registration | 3 passed |
| Exact hosted create/recovery/session fence | 1 passed, 9.64s |
| Exact hosted Grok metadata | 1 passed |
| SQLite ACL | Owner use succeeds; secondary standard account reads control, denied DB/WAL/SHM |

**110 unique outer Rust cases, zero failed/ignored, plus ACL.** These are actual
remote Windows measurements independently inspected, not locally rerun native
suites. They apply to the newer source; test.20 outcomes were not substituted.

**Measured locally:** recomputed both ZIP SHA-256 values and sizes against fresh
GitHub metadata; ZIP CRC checks pass; every member matches its extracted file
byte-for-byte. Native ZIP has nine flat text members. Installer ZIP has exactly
the EXE and build receipt. Installer source/version/name/hash match the receipt;
MZ/PE headers and empty certificate table checked without execution.

- Installer **`Buzz_0.5.29-test.21_x64-setup.exe`**, **56,407,044 bytes**,
  SHA-256 `6c4ad18e50bb8fd6b2e798d36865ebc69ef386b0b18726a410f3cc58e4c37208`.
- [Installer artifact 13759200363](https://github.com/Shipitrealgood/buzz/actions/runs/42803199965/artifacts/13759200363):
  unexpired at inspection; expires **2026-11-17T11:15:44Z**. ZIP **56,405,392 bytes**,
  SHA-256 `69d522c9a2211b3e342d3bfe6096a8cc5cb675f801f9008ee6edcf9af8fd5424`.
- [Native artifact 13758431634](https://github.com/Shipitrealgood/buzz/actions/runs/42803199965/artifacts/13758431634):
  unexpired at inspection; expires **2026-11-17T11:10:33Z**. ZIP **5,036 bytes**,
  SHA-256 `cf83c81a9bf05f35c98bb520e2369c74875cdc63e424a278d88b654dae3270b4`.

## Routing, limitations and release

Recommend accepting this bounded automated Windows fixture/build packet and
recording the prior M-1 native verification gap as closed. No new source fix or
prerequisite is proposed. Initial API access failed under network restriction;
the narrow authorized read-only retry succeeded. No CI rerun, push, installer
execution, live access or activation occurred during review.

**NOT RUN:** full `just ci`, full native Tauri IPC/UI journey, human acceptance,
real systemd/cgroup/user-manager/reboot behavior, provider/native-login/chat,
live host/relay/pairing, installation or deployment. Accepted Linux/Node suites
were not repeated. Native phase fixtures and packaging do not establish these
outcomes. No `buzz-review-completed` attestation or live-readiness claim is made.

`git diff --check`, source ancestry and report-link checks pass. Only this report
is committed by the reviewer with DCO. All reviewer commands are terminal; no
reviewer build, test, watch or download process remains. Executor terminal process
receipts are preserved in its handoff. Workspace ownership returns to conductor
at the report-only commit after the clean-status check.

quality: model=unknown effort=unknown stage=post-audit verdict=merge-safe verify=ran commits=da4ee105a34ff45e84d5e44e64c3e31d3a3c9651..7e8f870e14ab47cc9d2b349357f0d6d50decb955 findings=C=0/H=0/M=0/L=0 rework_rounds=0
