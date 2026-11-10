# #2722 hosted catalog Windows canary — execution and review brief

2026-11-10 UTC. Executor `/root/hosted_catalog_windows`; requested Astra/high;
runtime model/effort not independently exposed. Existing workflow execution only.

## Pin and ownership

Workspace `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`. Starting clean HEAD and intended build:
`444d18ef03b14354373393cd297323a17b9065e8`. Production-equivalent fix:
`160c8bfb853647ad336f296d0cff68c59cc75e62`; intervening review is docs only.
Own only this report and private artifact directory
`/home/bryce/services/buzz-agents/releases/2722-hosted-catalog-canary/<source>/<run>/`.
Conductor owns board/roadmap; source remains unchanged. No merge, release, tag,
updater, installer execution, deployment or live host/provider/service access.

## Accepted inputs and preflight

[Execution](2722-hosted-catalog-session-fix.md) and
[independent source review](../audits/post-audit/2722-hosted-catalog-session-fix.md)
accept the narrow catalog/session repair with native verification pending.
Applicable repository AGENTS, testing/product/architecture guidance, conductor,
execute and brief instructions were read. This is the combined dispatch-owned
execution/brief path; no additional brief file is owned.

Measured fork branch `Shipitrealgood/buzz:build/paired-hosts-canary` at
`929c2cd890fbc2c0993549c3cbc64a2714232832`, matching successful prior run
[42870540036](https://github.com/Shipitrealgood/buzz/actions/runs/42870540036).
`git merge-base --is-ancestor` passed against intended source. Full delta contains
only the two reviewed Rust files plus documentation. No workflow changes.
All 31 workflow YAML documents parse; push filters exclude the canary branch and
no `workflow_run` trigger exists. Existing Windows Canary is manual-only,
contents-read, unsigned NSIS, updater-disabled, seven-day artifact retention.
Both exact extended hosted cases are already required, with one pass and zero
failed/ignored. Generated pre-push hook absent; none bypassed.

Git commands after initial baseline reads use `HERMIT_SELF_UPDATE=false` and
`bin/activate-hermit`. Initial sandbox network reads failed DNS; explicit
read-only escalation succeeded. No GitHub write has occurred yet.

## Execution ledger

Pending exact-source normal fast-forward push, existing workflow dispatch,
owned monitoring, artifact receipt validation and signed-off report commit.
No process is detached. No native result is claimed while pending.

## Launched run

Normal fast-forward export succeeded from `929c2cd8` to exact `444d18ef`; explicit
Windows Canary dispatch succeeded without approval rejection. GitHub API confirms
[run 42918144439](https://github.com/Shipitrealgood/buzz/actions/runs/42918144439)
at exact accepted SHA, event `workflow_dispatch`, both jobs started 22:50 UTC.
Build job `130185804532`; native job `130185804910`. Monitor is executor-owned
`gh run watch 42918144439 --repo Shipitrealgood/buzz --exit-status --interval 45`.
No native/test/build success is claimed until terminal receipts are checked.

## Native failure — blocker, no repair/rebuild

Native job `130185804910` completed FAILURE after 4m31s. Owner library compiled;
`cargo test --locked --target x86_64-pc-windows-msvc -p buzz-paired-owner --lib`
returned **15 passed, 1 failed, 0 ignored**, exit 101. Exact failing case:
`deployment::tests::protected_main_snapshot_same_key_expiry_tamper_and_terminal_receipt`.
Panic at `crates/buzz-paired-owner/src/deployment_tests.rs:79:10`:
`called Result::unwrap() on an Err value: invalid or unsupported host deployment contract`.
This is measured test failure, not compiler or disk failure. No cause is inferred
from the message alone. The affected desktop repair has not run in native tests.
Shared harness, directory conversion, paired-host and all three exact hosted cases
were skipped after the failed owner step. ACL and failure receipt upload succeeded.

[Native artifact 13810468291](https://github.com/Shipitrealgood/buzz/actions/runs/42918144439/artifacts/13810468291)
is 1,506 bytes, expires `2026-11-17T22:54:34Z`. Downloaded ZIP SHA-256 matches
GitHub metadata: `c1704e512dcdf05164127fdea99f141ede7c2a93971729c201b223cf346e8129`.
Only `source.txt`, `paired-owner-tests.log`, `journal-acl.log` are present; source,
run and target match. Native raw and ANSI-stripped full job logs retained under
private evidence root `444d18ef03b14354373393cd297323a17b9065e8/42918144439/logs/`.
First log fetch was refused for ANSI bytes; explicit file-only retry succeeded.
ACL file is UTF-16; initial UTF-8 read failed, then corrected without rerunning tests.
Installer job remains owned; no repair, rerun or cancellation was performed.

## Scoped retry decision

Conductor inspected the unchanged failing fixture and reports an authority expiry
of `now()+2` before temporary-directory/device setup and cryptographic work; the
first `prepare_deployment` unwrap fails at line 79. Slow startup exhausting that
window is a hypothesis, not a measured cause. Owner suite ran 40.77s total.
Conductor explicitly authorizes one `gh run rerun --failed` on this same source
after original jobs terminate, provided installer succeeds. It must preserve
attempt 1, avoid rebuilding a successful installer, and stop on a second failure.
No fixture/source/workflow changes are authorized here. First-attempt metadata,
ZIP, extracted logs and full native log are copied under `attempt-1/`.

## Installer success and attempt 2

Original build job succeeded in **17m14s**, and watch session `46905` exited 1
naturally because native failed. Terminal attempt-1 run/jobs/artifacts metadata
was preserved before the single authorized `gh run rerun 42918144439 --failed
--repo Shipitrealgood/buzz`. Retry command exited 0; API confirms attempt 2 at
identical `444d18ef`, native job `130194917102`. Watch session `57421` owns it.
GitHub presents the previously successful build as job `130194920520` in attempt
2 with original timestamps/duration; only failed native executes again.

Installer **`Buzz_0.5.29-test.25_x64-setup.exe`**, **56,468,766 bytes**, SHA-256
`c21c0a460901f44ac6f56d1aaf753bdcead49ff8288c3d3894b38eac0f45e159`.
[Installer artifact 13810958555](https://github.com/Shipitrealgood/buzz/actions/runs/42918144439/artifacts/13810958555),
expires **2026-11-17T23:06:41Z**, downloaded ZIP **56,467,134 bytes**, SHA-256
`4881e6d66d4a1aaa8bf37668e13f130203dd199f79794fa67190ec76978748ae` matches GitHub.
ZIP contains exactly installer and `build-receipt.txt`; source/version/name/hash
agree with receipt. PE header checked. Installer never executed. Full build log,
receipt and `installer-verification.json` retained in private evidence directory.

## Final measured result

**Attempt 2 SUCCESS; 111 distinct outer Rust cases passed, zero failed/ignored,
plus ACL.** Native ran `23:08:35Z–23:24:05Z` (**15m30s**); run completed success
`2026-11-10T23:24:06Z`. The installer is reused from successful attempt 1 at the
same exact source; its original build timestamps are `22:50:10Z–23:07:24Z`.
No second installer build occurred. API `run_attempt=2`, source receipts and
both artifact records agree with accepted source `444d18ef` and version
`0.5.29-test.25` (workflow run number 25).

All native Cargo commands used `--locked --target x86_64-pc-windows-msvc`.
Owner/core ran at repository root; desktop suites from `desktop/src-tauri`.

| Native command/filter | Attempt-2 measured result |
| --- | --- |
| `cargo test … -p buzz-paired-owner --lib` | 16 passed; 25.82s, including initially failed case |
| `cargo test … -p buzz-core --lib harness::` | 9 passed |
| `cargo test … --lib nostr_convert::` | 73 passed |
| `cargo test … --lib commands::agent_discovery::relay_directory::` | 7 passed |
| `cargo test … --lib commands::paired_hosts::` | 3 passed |
| Exact `commands::agents::hosted::tests::hosted_native_create_recovery_and_session_fence` | 1 passed; 9.86s |
| Exact `commands::agents::hosted::tests::hosted_native_cleanup_confirmation_retry_and_session_fence` | 1 passed; 10.58s |
| Exact `managed_agents::discovery::presets::hosted_metadata_tests::hosted_grok_metadata_uses_static_preset_without_local_binary` | 1 passed |
| `crates/buzz-paired-owner/windows/test-journal-acl.ps1` | Owner used SQLite; standard user read control and was denied DB/WAL/SHM |

Exact test names and one-pass/zero-failed/zero-ignored summaries were verified
from individual logs; all 111 names are unique. Child execution and grep echoes
in full logs were not counted again. Both hosted tests run the extended
`native_phases` at `desktop/src-tauri/src/commands/agents_hosted_tests.rs:135`,
including empty-operation catalog success, unchanged setup rejection, metadata/
inventory/no-intent checks and stale-session rejection before existing journeys.
Their success is measured; per-assertion tracing and mutation kills are not.

[Successful native artifact 13811888126](https://github.com/Shipitrealgood/buzz/actions/runs/42918144439/artifacts/13811888126)
is **5,400 bytes**, expires **2026-11-17T23:24:00Z**, ZIP SHA-256
`2c7fbf0cf4e64cc50a65a61d3c29d0414f67a6526fda23c902d5e94d68dacf5d`, matching GitHub.
Ten flat members: source receipt, eight suite logs and ACL log. GitHub retains
both same-named native artifacts; select **13811888126** for success and
**13810468291** for first-attempt failure. Local `native.zip`/`native/` are attempt
2; `attempt-1/native.zip`/`attempt-1/native/` preserve original failure.
`verification.json` lists all names, counts, sizes, hashes, URLs, expiry and ACL.
Full retry log is `logs/native-attempt2.log`; first failure remains `logs/native.log`.

## Verification limits and deviations

The existing workflow, source and fixtures were unchanged. One scoped deviation
from the initial stop-on-failure instruction was explicitly authorized by the
conductor: retry failed native once after preserved failure evidence and successful
installer. Retry success demonstrates non-recurrence, **not a proved timing cause**.
The fixture's two-second expiry remains a reliability concern for the conductor
and independent reviewer; no silent test repair or repeated green-seeking occurred.
Nonfatal setup-node unsupported-input and Node 20 deprecation warnings occurred.

**NOT RUN:** failing-before/passing-after mutation control (the authorized workflow
has no such step and source/workflow changes were excluded), full `just ci`, full
Tauri IPC/UI and live status-validation journey, real provider/login/chat/service
acceptance, installer execution, human pilot acceptance or deployment. Existing
native fixtures use isolated boundaries; success does not establish live readiness.
No additional live host software is needed to distribute this desktop-only fix.

## Review handoff

The same independent reviewer should assess this finite native/artifact evidence
against its accepted source review, without another build. Specifically inspect:

1. `windows-canary.yml:284` exact-case selection against individual attempt-2 logs;
   distinguish the actual one-case pass from full-log grep echoes.
2. `agents_hosted_tests.rs:135` production catalog assertions and prior source review;
   measured native passes close compilation/execution, not mutation falsifiability.
3. First-attempt `deployment_tests.rs:79` failure versus second-attempt pass;
   assess the disclosed timing hypothesis without treating it as proven or repaired.
4. Original installer receipt and attempt-1 build timestamps versus attempt-2 test
   receipts; all must bind `444d18ef`, with production-equivalent fix `160c8bfb`.
5. Artifact IDs above: same-named native artifacts have different outcomes; hashes
   and ten-member success ZIP prevent confusing first-attempt failure with success.

Scope remains the catalog repair and this evidence. Live host/controller/relay,
clock-skew, Retry-button, provider and human acceptance remain conductor-owned.
No new product/architecture question or missing foundation was discovered here.
Reviewer returns its native evidence verdict to conductor; executor self-checks
are evidence, not the independent gate.

## Terminal ownership

Watch `46905` exited 1 after original run; retry watch `57421` exited 0. Push,
dispatch, retry and all downloads/metadata commands exited; no owned background
build/test/watch/download process remains. Private artifacts remain under the
absolute evidence root above. Starting checkout was clean; owned delta is this
report only. Source unchanged, unrelated paths preserved. Documentation links,
`git diff --check`, and final scoped status are checked before the signed-off
handoff commit. Receipt is committed locally only; fork remains the built source.
