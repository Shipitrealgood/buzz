# #2722 Windows native canary execution

**Final status: Windows native gates and unsigned canary build PASS.**
[Run 42764261281](https://github.com/Shipitrealgood/buzz/actions/runs/42764261281)
completed successfully at source `c4eda2615b72acc5761ef34bb14aed86509c9a14`:
101 Rust cases, zero failed/ignored, plus ACL; build 37m9s, tests 14m9s.
The installer and ZIP hashes are verified below. Independent final evidence
review is next; full repository CI, human and live/service acceptance are NOT RUN.
No feature activation, live-system mutation or installer execution occurred.
Earlier preparation/run-state notes below are historical and superseded here.

Requested executor: Astra/high;
served configuration is not exposed. Exclusive source/test ownership is held in
`/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`. Clean baseline:
`bc827df8c6109eb6972ca2ee8786095bda5c9ee3`.

Accepted input: [source re-gate](../audits/post-audit/2722-windows-hosted-flow.md),
[contract](../specs/2722-windows-hosted-flow.md), and
[prior execution](2722-windows-hosted-flow.md). This bounded follow-on runs the
existing Windows artifact-only workflow plus the two existing hosted native tests.
No product/architecture change is planned. Full CI, human and live acceptance
remain not run. No runtime flag, live service, provider, pairing or agent home
is activated or touched.

Read fixture isolation: the hosted child removes inherited Buzz credentials and
runtime flag; sets temporary home variables; pins nest and the mock app-data
identifier to its temporary directory; bypasses managed-agent keychain storage;
uses a generated in-memory signer and temporary owner journal; pins profile
publication to closed loopback port 9; and requests no spawn. The metadata test
uses a no-local-binary closure and the production static projection. Existing
owner/directory/registration suites and disposable-account SQLite ACL test remain.

Workflow additions require each fully qualified hosted test to appear as passing
and report exactly one pass, zero failures and zero ignored. Per-suite test logs
and a source/run receipt are retained seven days. These logs contain fixture test
names/outcomes, not real credentials. The build stays unsigned NSIS, updater
artifacts disabled, with no release/tag/publication step.

Push-trigger inspection: all 31 workflow YAML documents parsed; push branches are
main, release or verify/paired-hosts-staging, with separately filtered release tags.
None matches build/paired-hosts-canary, and no workflow_run trigger exists.
Only a fast-forward push to the existing fork branch and explicit Windows Canary
dispatch are authorized. The recorded fork head f316fb46d89c3e16eedc6173ae06e85fe224a1a9
is an ancestor of the baseline; remote freshness will be checked before push.

Next: conductor ratification of the minimal patch, then dispatch/monitor to
terminal status. Source changes from actual Windows failures require a fresh
independent review before the resulting artifact is accepted.

## Run ledger

Conductor ratified workflow commit `1bec0dc72c5228b3fcfe73879a5910307596dac4`.
Fresh remote equality and ancestry checks passed; normal fast-forward push
updated only `Shipitrealgood/buzz:build/paired-hosts-canary`. Manual dispatch:
[run 42762758173](https://github.com/Shipitrealgood/buzz/actions/runs/42762758173),
API-confirmed exact head `1bec0dc72c5228b3fcfe73879a5910307596dac4`. Both Windows
jobs started 2026-11-10 05:32 UTC; executor retains monitor ownership.
Preparation gates: YAML parse, embedded Bash syntax, branch-trigger exclusion
check, source ancestor check and `git diff --check` passed. No Windows result
is claimed until the terminal receipt below.

## First native failure and bounded correction

Run 42762758173 test job 129652105479 compiled Windows Tauri successfully, then
passed owner **16**, nostr conversion **73**, relay directory **7**, paired-host
registration **3** tests (all zero failures/ignored) and the SQLite ACL probe.
The hosted phases case ran exactly once and failed at its first phase call:
`agents_hosted_tests.rs:112`, `invalid or unsupported host deployment contract`.
The metadata case was skipped after that failure; it is not counted as passed.

Read diagnosis: `paired_host::canonical_origin` requires `Host::Domain`; the
fixture used the rejected IP-literal scope `https://127.0.0.1:9`. This was a
fixture prerequisite error, not a Windows compiler or production validator defect.
The correction uses `https://localhost:9` / `wss://localhost:9`, asserts the scope
passes production validation before the phases, and replaces only the fixture's
HTTP client with an explicit 127.0.0.1:9 resolution, no proxy and no redirects.
The production canonical-origin rule and all main creation/recovery guards remain
unchanged. Conductor ratified this bounded fixture correction. Scoped Rustfmt
and diff whitespace checks passed; actual correction verification awaits Windows.
The original installer job remains monitored and is not accepted independently.

Reviewer precision note (optional, no behavior change): the fixture replacement
is `state.http_client` for query/policy reads. Profile POST uses the separately
constructed `state.media_fetch_client`, already pinned to localhost with redirects
disabled, without an explicit `no_proxy`. No blanket proxy-immunity claim is
made. This Windows run receives no provider/relay credentials or proxy secrets.

Correction run: [42764261281](https://github.com/Shipitrealgood/buzz/actions/runs/42764261281),
API-confirmed source `c4eda2615b72acc5761ef34bb14aed86509c9a14`, dispatched after
a normal fork-only fast-forward. Test job 129656910917 and build job 129656911283
are monitored by the same executor; the original run remains separately owned.

## Corrected native measurements

Run 42764261281, Windows job 129656910917: **SUCCESS**, 14m9s.
All commands use `--locked --target x86_64-pc-windows-msvc`; Tauri suites run
from `desktop/src-tauri`, owner suite from repository root.

| Gate | Actual outcome |
| --- | --- |
| `cargo test … -p buzz-paired-owner --lib` | 16 passed, 0 failed/ignored |
| `cargo test … --lib nostr_convert::` | 73 passed, 0 failed/ignored |
| `cargo test … --lib commands::agent_discovery::relay_directory::` | 7 passed, 0 failed/ignored |
| `cargo test … --lib commands::paired_hosts::` | 3 passed, 0 failed/ignored |
| `cargo test … --lib commands::agents::hosted::tests::hosted_native_create_recovery_and_session_fence -- --exact` | 1 passed, 0 failed/ignored; 12.61s |
| `cargo test … --lib managed_agents::discovery::presets::hosted_metadata_tests::hosted_grok_metadata_uses_static_preset_without_local_binary -- --exact` | 1 passed, 0 failed/ignored |
| `./crates/buzz-paired-owner/windows/test-journal-acl.ps1` | PASS: secondary standard account read control, denied DB/WAL/SHM |

Total **101 distinct outer Rust cases**, plus ACL probe. The confined hosted
child is not counted twice; grep echoes each named successful line but does not
rerun it. This proves the existing Windows main-phase/recovery fixture and static
metadata projection. Full Tauri IPC/service/provider/user acceptance is not claimed.

Native artifact [13739438372](https://github.com/Shipitrealgood/buzz/actions/runs/42764261281/artifacts/13739438372):
`buzz-windows-native-tests-c4eda2615b72acc5761ef34bb14aed86509c9a14`, 4,527 bytes,
expiry `2026-11-17T06:01:04Z`, ZIP SHA-256
`5e5a0aed8c86c5ea9b54276c7fb1637daa833351a1a850b6b6ade42af96ab8af`.
Downloaded digest matches GitHub metadata. Its eight flat members were checked
before extracting only text receipts under `tests/`. Full ANSI-stripped job output
is `logs/discovery.log`. Local evidence root:
`/home/bryce/services/buzz-agents/releases/2722-windows-canary/c4eda2615b72acc5761ef34bb14aed86509c9a14/42764261281/`.

Original run 42762758173 is **terminal FAILURE**: build SUCCESS (29m16s), tests
FAILURE (11m37s). Its installer exists remotely but is not accepted/downloaded
as a ready canary. Original watch process exited 1 naturally. Failure log and
hash-verified test ZIP are retained under the analogous `1bec0dc72…/42762758173/`
root. Failed-test artifact 13738747338 expires `2026-11-17T05:43:58Z`, SHA-256
`814daff4fd10aa870036f88d0cf0a695659d9fa3dd9c413014ea0e6ca673f6f3`.
Corrected installer job remains active; executor retains monitor ownership.

## Terminal installer and closure

Run `42764261281` completed **SUCCESS** at 2026-11-10 06:24:09 UTC. Both jobs
are terminal success; build duration 37m9s, tests 14m9s. Source is exactly
`c4eda2615b72acc5761ef34bb14aed86509c9a14` (the later evidence commit is docs-only).

- Installer: `Buzz_0.5.29-test.20_x64-setup.exe`, version `0.5.29-test.20`,
  56,348,905 bytes; unsigned NSIS, updater artifacts disabled.
- Installer SHA-256: `47940334b97c5589c6efeb6f3ea51cd488d5305a632387ab4925bfa3d706eb9b`.
- [Installer artifact 13739854472](https://github.com/Shipitrealgood/buzz/actions/runs/42764261281/artifacts/13739854472),
  name `buzz-windows-canary-c4eda2615b72acc5761ef34bb14aed86509c9a14`,
  expires **2026-11-17 06:21:05 UTC** (seven-day retention).
- Artifact ZIP: 56,347,221 bytes; SHA-256
  `2470e8535ccf5c2e154a680c39ec00ee22514f3e4d9ca5fa343d34c708f1d083`,
  matched GitHub artifact metadata. Listing contains only the named installer
  and `build-receipt.txt`. Source/version/filename and installer digest match
  that receipt. PE header checked; installer was never executed.
- Local installer, ZIP, build receipt, `verification.json`, terminal `run.json`,
  artifact metadata, native receipts and ANSI-stripped build/test logs are under
  `/home/bryce/services/buzz-agents/releases/2722-windows-canary/c4eda2615b72acc5761ef34bb14aed86509c9a14/42764261281/`.

Scoped Rustfmt, YAML parsing/embedded Bash syntax, trigger exclusion and
`git diff --check` passed. Differential size gate
`CHECK_FILE_SIZES_BASE=bc827df8c6109eb6972ca2ee8786095bda5c9ee3 node desktop/scripts/check-file-sizes.mjs`
passed after sandbox `spawnSync git EPERM` required approved read-only subprocess
escalation. Existing setup-node unsupported-input and Node 20 deprecation
warnings are non-fatal and unchanged; no adjacent workflow modernization.

Owned source changes: workflow +39/-6, existing test fixture +13/-4;
**zero production Rust/UI changes**. No missing foundation or new architecture.
No commit was pushed to origin/main, no tag/release/PR/deployment was created,
and no relay/provider/pairing/existing agent home was accessed. Native feature
remained default-off; no activation env was introduced even in this fixture.

Both watcher processes exited naturally (original status 1, corrected status 0);
all artifact download commands completed. No owned build/test/monitor process
remains. Starting worktree was clean; only owned files changed. Final docs-only
signed-off commit releases exclusive writer/test ownership in the same worktree.
Next gate: fresh independent review of the final evidence and fixture correction,
using [the handoff](../briefs/2722-windows-native-canary-review.md). No further CI
run is required for this documentation-only receipt update.
