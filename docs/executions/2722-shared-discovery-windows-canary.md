# #2722 shared discovery Windows native canary — execution

2026-11-10 UTC. Executor `/root/windows_shared_discovery_canary_exec`;
requested configuration Astra/high, served model/effort not independently exposed.
Exclusive workspace `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`; clean starting source
`da4ee105a34ff45e84d5e44e64c3e31d3a3c9651`. No unrelated changes at baseline.

## Contract and pre-launch ledger

Execute the existing unsigned Windows native canary workflow including accepted
S1/S2/S3 source and M-1 repair `5d72b08d3`. Inputs: [current source review](../audits/post-audit/3721-host-discovery-windows.md),
[source execution](3721-host-discovery-windows.md),
[source brief](../briefs/3721-host-discovery-windows-review.md),
[previous canary](2722-windows-native-canary.md), and its
[independent review](../audits/post-audit/2722-windows-native-canary.md).
Accepted design and Linux/Node evidence stand for unchanged scopes; they are not
rerun or promoted into native proof. No design/audit stage is restarted.

Read: hosted metadata and native fixture paths remain valid. The native fixture
includes four zero-effect rejection scenarios (missing inventory, missing runtime,
unavailable, unsupported) before its existing available recovery/session checks.
These are subcases of one exact Rust test, not four independent test cases.
Nine portable harness cases moved to `buzz-core`; add that existing suite to the
workflow with exact nine named passes and zero failed/ignored. Owner, conversion,
directory, registration, exact hosted/metadata and SQLite ACL gates remain intact.
No production source change is planned. No new test framework.

Read hook baseline after Hermit activation: `core.hooksPath` unset, generated
pre-commit/pre-push hooks absent in `/home/bryce/services/buzz/.git/hooks`.
`lefthook.yml` and `bin/.lefthookrc` reviewed; no hook is disabled or rewritten.
Full local CI/Tauri remain outside this bounded run (Linux GTK/GIO unavailable).
All Git commands henceforth use `HERMIT_SELF_UPDATE=false` and `bin/activate-hermit`.
Fresh remote head read: `c4eda2615b72acc5761ef34bb14aed86509c9a14`.
Only normal fast-forward push to `Shipitrealgood/buzz:build/paired-hosts-canary`,
explicit Windows Canary dispatch, monitoring and artifact download are authorized.

Runtime flags remain default-off. No SSH, live relay/netcup, provider/login/service,
existing-agent or host-account change is authorized. No primary branch push, tags,
release, updater artifact, installation or deployment. Full CI, human and live
acceptance are NOT RUN. Executor retains monitoring until every owned job is terminal.

## Launched run

Workflow/source commit `23d2dc15c6c45554181938b7956b59c085980455` includes baseline
`da4ee105`; API confirmed [run 42803199965](https://github.com/Shipitrealgood/buzz/actions/runs/42803199965)
at that exact SHA, launched 2026-11-10 10:55:28 UTC. Normal fast-forward updated
only the authorized fork branch from `c4eda2615`. All 31 workflow YAML documents
parsed; push branch filters exclude the canary branch and no `workflow_run`
trigger exists. Tag-only push workflows were explicitly treated as excluding
branch pushes after the initial conservative checker rejected that shape.
Embedded Bash syntax passed. Generated Git hooks were absent; no bypass used.

New shared-suite gate was exercised with a disposable mock cargo: nine named
passes exits 0; zero cases, ignored cases and missing named pass exit 1; cargo
exit 101 propagates even with a successful-looking summary. This checks gate
failure behavior, not native test outcomes. `git diff --check` passes. Existing
differential size gate against `da4ee105` passes after a narrowly approved
read-only Git subprocess retry (`spawnSync git EPERM` under sandbox initially).

Monitor: `gh run watch 42803199965 --repo Shipitrealgood/buzz --exit-status
--interval 45`, owned by executor. Native job ID `129785693525`; terminal receipt
and build identity follow after completion. No result is claimed while active.

## Native Windows result

Job `129785693525` completed **SUCCESS**, 15m6s. All Cargo commands used
`--locked --target x86_64-pc-windows-msvc`; owner/core run at repository root,
Desktop suites from `desktop/src-tauri`.

| Native command/filter | Measured result |
| --- | --- |
| `cargo test … -p buzz-paired-owner --lib` | 16 passed |
| `cargo test … -p buzz-core --lib harness::` | 9 passed, nine unique named cases |
| `cargo test … --lib nostr_convert::` | 73 passed |
| `cargo test … --lib commands::agent_discovery::relay_directory::` | 7 passed |
| `cargo test … --lib commands::paired_hosts::` | 3 passed |
| Exact `commands::agents::hosted::tests::hosted_native_create_recovery_and_session_fence` | 1 passed, 9.64s |
| Exact `managed_agents::discovery::presets::hosted_metadata_tests::hosted_grok_metadata_uses_static_preset_without_local_binary` | 1 passed |
| `crates/buzz-paired-owner/windows/test-journal-acl.ps1` | Successful owner SQLite use; secondary standard account reads control but denied DB/WAL/SHM |

**110 distinct outer Rust cases, zero failed/ignored**, plus ACL. Counts verified
from individual native logs; grep echoes and child execution are not extra cases.
The nine core cases are six moved Windows installer-string tests, one moved vendor
metadata test, main registry/default-argv projection and shared availability/version/
auth classification. The exact hosted fixture compiles and executes on Windows;
its four inventory rejection scenarios are within that one test. All precede the
original available-runtime recovery/session assertions. Native success establishes
the assertions executed; there are no separate per-scenario cargo output lines.
No mutation run or full Tauri IPC/provider/human proof is claimed.

Native artifact [13758431634](https://github.com/Shipitrealgood/buzz/actions/runs/42803199965/artifacts/13758431634),
`buzz-windows-native-tests-23d2dc15c6c45554181938b7956b59c085980455`, 5,036 bytes,
expires `2026-11-17T11:10:33Z`. ZIP SHA-256
`cf83c81a9bf05f35c98bb520e2369c74875cdc63e424a278d88b654dae3270b4`
matched GitHub metadata. Nine flat members inspected before extraction.

Local evidence root:
`/home/bryce/services/buzz-agents/releases/2722-windows-canary/23d2dc15c6c45554181938b7956b59c085980455/42803199965/`.
Contains ZIP, GitHub artifact/native-job metadata, individual `tests/` receipts,
ANSI-stripped `logs/discovery.log` and `verification.json`. Log download initially
refused ANSI output; a file-only download explicitly allowed it and the inspected
copy strips ANSI. Local receipt verification initially expected literal PASS in
ACL output; corrected to its actual descriptive success sentence and completed.
Neither inspection retry was a native gate failure. No compiler/test defect or
source repair arose. Installer job `129785692135` is still owned and monitored.

## Terminal installer and ownership release

[Run 42803199965](https://github.com/Shipitrealgood/buzz/actions/runs/42803199965)
completed **SUCCESS** at `2026-11-10T11:17:13Z`, exact source
`23d2dc15c6c45554181938b7956b59c085980455`. Both jobs started 10:55:32 UTC:
native finished 11:10:38 (**15m6s**), build finished 11:17:12 (**21m40s**).
Terminal `run.json` and `jobs.json` agree with native/build source receipts.

- Installer **`Buzz_0.5.29-test.21_x64-setup.exe`**, version `0.5.29-test.21`,
  **56,407,044 bytes**. Unsigned NSIS; updater artifacts disabled.
- Installer SHA-256:
  `6c4ad18e50bb8fd6b2e798d36865ebc69ef386b0b18726a410f3cc58e4c37208`.
- [Installer artifact 13759200363](https://github.com/Shipitrealgood/buzz/actions/runs/42803199965/artifacts/13759200363),
  `buzz-windows-canary-23d2dc15c6c45554181938b7956b59c085980455`, expires
  **2026-11-17 11:15:44 UTC** (seven-day retention).
- ZIP **56,405,392 bytes**, SHA-256
  `69d522c9a2211b3e342d3bfe6096a8cc5cb675f801f9008ee6edcf9af8fd5424`,
  matches GitHub metadata. Exactly installer and `build-receipt.txt` members;
  source/version/filename/installer hash agree with that receipt. PE header checked.
- Local installer is under the evidence root's `installer/`; `installer.zip`,
  `native.zip`, `verification.json`, GitHub run/job/artifact metadata and complete
  ANSI-stripped `logs/build.log` and `logs/discovery.log` are retained alongside.
  The installer was never executed.

No native compiler/test failure occurred, so no source repair was needed. Owned
code delta is **11 workflow lines**, plus these docs; production Rust/UI unchanged.
Existing setup-node unsupported-input and Node 20 warnings remain nonfatal;
no adjacent modernization. Runtime feature flags remain default-off; no activation
variable, release/tag/primary push, provider/login/service or live-system action.
Only the source commit was fast-forwarded to the fork canary branch; final docs
are local handoff commits and do not change the built source.

**NOT RUN:** full `just ci`, full native Tauri IPC/UI journey, human acceptance,
live host/relay/pairing, real provider/native-login/chat/service acceptance,
installer execution or deployment. Prior accepted Linux/Node checks were retained
for unchanged scopes and not repeated. This packet closes the bounded automated
Windows fixture/build obligation, not those separate acceptance gates.

Watcher session 6999 exited naturally **0**; installer download session 43870
exited **0**; all other commands are terminal. No owned build/test/watch/download
process remains. Starting checkout was clean; no unrelated edits were present or
changed. Final documentation/link/diff checks passed before the DCO handoff commit.
Exclusive source/test ownership is released to conductor with that clean commit.
Next gate: fresh independent review using [the review brief](../briefs/2722-shared-discovery-windows-canary-review.md).
