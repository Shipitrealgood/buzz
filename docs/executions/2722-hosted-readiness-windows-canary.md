# #2722 hosted readiness Windows canary — execution and review brief

2026-11-11 UTC. Executor `/root/hosted_readiness_windows`; requested
`gpt-6-astra` / high. Actual runtime model and effort are not independently
exposed. Combined execution/brief at the dispatch-owned path.

## 1. Where the work lives

Workspace `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`. Starting clean HEAD and exact build source:
`78232d95173b876babcee0f51b18f7c2a17dac0d`. Product commit:
`25b636dd608467eb1a66e7b61905706767a8feb5`.
Own this receipt only, plus private artifacts below
`/home/bryce/services/buzz-agents/releases/2722-hosted-readiness-canary/`.
Root owns board/roadmap. Prior processes and ownership were released.

## 2. Accepted inputs and preflight

[Execution](2722-hosted-readiness-fix.md),
[independent source/UI ACCEPT](../audits/post-audit/2722-hosted-readiness-fix.md),
and [prior test25 receipt](2722-hosted-catalog-windows-canary.md) are inputs.
Applicable repository AGENTS, conductor contract, execute/brief skills, product
and testing guidance read. Source/UI acceptance is preserved without rerunning.

Measured fork branch `Shipitrealgood/buzz:build/paired-hosts-canary` at
`444d18ef03b14354373393cd297323a17b9065e8`. Ancestry passes to exact intended SHA.
Native source and every workflow are unchanged from that prior source. Full delta
is reviewed selector/fixtures/guidance plus documentation. All 31 workflow YAML
files parse. Push filters exclude the branch; no workflow_run trigger. Windows
Canary is manual-only, contents-read, unsigned NSIS, updater-disabled, seven-day
retention. Exact hosted cases require one pass and zero failed/ignored.
Hermit activated using `HERMIT_SELF_UPDATE=false`; no pre-push hook exists.
Initial sandbox API read failed network access; read-only escalation succeeded.

## 3. Execution ledger (initial state)

Pending normal fast-forward push of exact accepted SHA, one existing workflow
dispatch, owned monitoring and receipt verification. No retry is pre-authorized.
Prior test25 owner expiry failure remains a disclosed reliability concern;
timing cause was hypothesized, not proved. No source/workflow repair is owned.
No detached process or live state operation exists.

Normal fast-forward push and single workflow dispatch succeeded without approval
rejection. API confirms [run 42930493604](https://github.com/Shipitrealgood/buzz/actions/runs/42930493604),
run number 26, event workflow_dispatch, exact accepted source `78232d95173b876babcee0f51b18f7c2a17dac0d`.
Initially expected version `0.5.29-test.26`; verified below.
Monitor is executor-owned `gh run watch 42930493604 --repo Shipitrealgood/buzz
--exit-status --interval 45`. Pending native/build outcome is not a pass.

## 4. Final measured result and artifacts

**SUCCESS, attempt 1; 111 distinct outer Rust tests, zero failed or ignored,
plus ACL.** No retry, source edit or workflow edit. Run completed
`2026-11-11T00:35:53Z`. Native job `130227724092` ran
`00:20:01Z–00:35:53Z` (15m52s); build job `130227724714` ran
`00:20:00Z–00:32:52Z` (12m52s). `gh run watch` session `63499` exited 0.

Installer **`Buzz_0.5.29-test.26_x64-setup.exe`**, **56,484,611 bytes**,
SHA-256 **`211c5e96db035e0b5fca58e627b598ddf9db30e12b50cc33f3d8d15f0a633144`**.
[Installer artifact 13816845999](https://github.com/Shipitrealgood/buzz/actions/runs/42930493604/artifacts/13816845999),
expires **2026-11-18T00:32:21Z**. ZIP **56,482,976 bytes**, SHA-256
`cf9a67fb714cd4bc52abd53138b9c06aaf8ae5a43d0a80f8349f8050a3630626`.
ZIP has exactly the installer and `build-receipt.txt`. Embedded receipt source,
version, filename and installer hash agree. PE header checked; never executed.

[Native artifact 13816986961](https://github.com/Shipitrealgood/buzz/actions/runs/42930493604/artifacts/13816986961),
expires **2026-11-18T00:35:48Z**. ZIP **5,411 bytes**, SHA-256
`f5290973e4a846227274e36f2944c08e33253a519004c803e419cc45f71b5df2`.
Exactly ten flat members: `source.txt`, eight suite logs and `journal-acl.log`.
Both ZIP sizes/digests match GitHub artifact metadata. API run and artifact
workflow_run metadata bind exact accepted SHA and run; native source receipt
binds source/run/target. Both full job logs independently report test26/source.
There is one native artifact and one installer artifact, from this single attempt.

All native Cargo commands used `--locked --target x86_64-pc-windows-msvc`.
Owner/core run at repository root; desktop suites in `desktop/src-tauri`.

| Native command/filter | Measured result |
| --- | --- |
| `cargo test … -p buzz-paired-owner --lib` | 16 passed; 23.75s |
| `cargo test … -p buzz-core --lib harness::` | 9 passed |
| `cargo test … --lib nostr_convert::` | 73 passed |
| `cargo test … --lib commands::agent_discovery::relay_directory::` | 7 passed |
| `cargo test … --lib commands::paired_hosts::` | 3 passed |
| Exact `commands::agents::hosted::tests::hosted_native_create_recovery_and_session_fence` | 1 passed; 9.79s |
| Exact `commands::agents::hosted::tests::hosted_native_cleanup_confirmation_retry_and_session_fence` | 1 passed; 10.45s |
| Exact `managed_agents::discovery::presets::hosted_metadata_tests::hosted_grok_metadata_uses_static_preset_without_local_binary` | 1 passed |
| `crates/buzz-paired-owner/windows/test-journal-acl.ps1` | Owner used SQLite; another standard user read control and was denied DB/WAL/SHM |

Every individual suite summary and test name was checked, including zero failed/
ignored and exact hosted paths, against expected counts. All 111 outer names are
unique. Full-log grep echoes and child execution are not counted twice. Prior
expiry case `deployment::tests::protected_main_snapshot_same_key_expiry_tamper_and_terminal_receipt`
passed in this first attempt. This establishes non-recurrence here; the prior
timing hypothesis remains unproved and the reliability follow-up remains open.

Private evidence root (directory mode 0700):
`/home/bryce/services/buzz-agents/releases/2722-hosted-readiness-canary/78232d95173b876babcee0f51b18f7c2a17dac0d/42930493604/`.
`installer/` and `native/` retain extracted artifacts; original ZIPs remain.
`run.json`, `jobs.json`, `artifacts.json` are terminal API records.
`logs/build-raw.log` and `logs/native-raw.log` preserve complete jobs; corresponding
`.log` files remove ANSI bytes. `verification.json` records artifact hashes,
URLs/expiry, installer hash/size and all test names/counts. `evidence-index.json`
indexes retained files with SHA-256/size. `python3 verify.py` exited 0; it verifies
single-attempt success, metadata binding, exact member sets, installer receipt/PE,
and all eight nonzero test suites. ACL text and native step success were inspected.
This committed document is the sanitized shareable index; raw logs stay private.

## 5. Decisions / deviations / limits

No contract deviation or new product/architecture decision. Combined brief stays
in the sole dispatch-owned document; no extra brief file or board update.
Initial build-log API fetch refused ANSI bytes; explicit file-only fetch with
`--allow-escape-sequences` succeeded, then a stripped copy was produced. This is
log retrieval, not a test rerun. Nonfatal setup-node unsupported-input and Node20
deprecation warnings match prior runs. No automatic approval denial occurred.

**NOT RUN:** frontend source/UI re-audit or suite rerun (accepted review retained),
full `just ci`, native mutation controls, installer execution, full native IPC/UI
and live status-validation journey, provider/login/chat/service pilot, human
acceptance, deployment. Native fixtures exercise isolated boundaries; this run
does not establish live readiness. The conductor reports Claude is available for
the next human pilot after a separate host PATH correction; that observation is
not evidence produced or verified by this worker. No live host change is needed
within this desktop artifact assignment.

## 6. Review handoff — specifically inspect

The same independent readiness reviewer should assess these artifact/native
receipts against its accepted source/UI review, without another build or broad
source re-audit. Return the finite evidence verdict to the conductor.

1. Does API and embedded provenance select this exact source? Probe `run.json`,
   both artifacts' `workflow_run`, `native/source.txt`, `installer/build-receipt.txt`
   and job-log version lines. Product `25b636dd` is wrapped by source/UI review
   commit `78232d95`; this receipt commit is not the built source.
2. Do the ZIP hashes/member sets prevent stale artifact substitution? Probe
   `verification.json` against installer ID `13816845999` and native ID
   `13816986961`, including the two distinct expiry timestamps.
3. Are exact hosted cases real nonzero passes? Probe individual native files,
   not duplicate grep echoes in the full job log; compare existing
   `.github/workflows/windows-canary.yml:284` selection. Each exact case is 1/0/0.
4. Does the total claim stay at its actual layer? Probe all eight suite logs and
   `verification.json`'s 111 unique names; fixtures remain isolated native tests,
   and the accepted frontend/browser results remain separate evidence.
5. Is the prior flaky owner case disposition honest? Probe current owner log's
   16/0/0 versus prior test25 receipt. First-attempt success here does not repair
   or prove the two-second-expiry timing hypothesis.
6. Has ACL actually executed? Probe UTF-16 `native/journal-acl.log` and successful
   native job step; standard-user control readability plus DB/WAL/SHM denial are
   the measured claim, not live host filesystem acceptance.

## 7. Ownership and next gate

Only this report is changed in the checkout. Starting state was clean; source,
workflow and unrelated paths remain unchanged. Push session `55742`, dispatch
`95713`, installer download `94043`, monitor `63499`, and all metadata/log/native
artifact commands reached terminal exit. No owned build/test/watch/download
process remains, and no mutation is held. Documentation links, diff whitespace,
source continuity and scoped status are checked before the signed-off commit.
Receipt is local only; fork remains exact built source `78232d95`.

Recommended next gate: same independent reviewer accepts or rejects this finite
artifact evidence, then conductor directs Windows installation/human Claude pilot.
Source/UI acceptance stands. Live host/controller/relay, clock/retry reliability,
provider work and full PR-readiness attestation remain outside this assignment.
No new named product question; prior reliability follow-up is not closed.
