# #2722 reply validator Windows canary — execution and review brief

2026-11-12 UTC. Worker `/root/reply_validator_windows_canary`; requested
`gpt-6-astra` / high. Actual served model and effort not independently exposed.
This combined execution/brief is the sole owned documentation path.

## 1. Pins and ownership

Workspace `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`. Clean starting source
`1bf985337776ed03551128d63ef3c849b887457a`, accepted implementation
`af9994e8560e5229705472a1218c6d9b87e75040`. Build source after the scoped CI gate
addition: `000d9c5e8bb1cb0d1314ecd2e96d136ad4be6916`.

[Source execution](2722-reply-validator-fix.md),
[independent source acceptance](../audits/post-audit/2722-reply-validator-fix.md),
and [test26 receipt](2722-hosted-readiness-windows-canary.md) are accepted inputs.
Read project AGENTS/releasing/product/testing guidance, portable conductor
contract and execute/brief skills. No source re-audit is claimed.

## 2. Prepublish checks and scoped CI correction

Measured fork branch `Shipitrealgood/buzz:build/paired-hosts-canary` at
`78232d95173b876babcee0f51b18f7c2a17dac0d` (test26), with clean source and valid
ancestry. Reviewed complete delta: validator fix and separately reviewed #3721
credential quoting source/tests plus documentation. The packaging list and
conservative local runtime/build dependency closure exclude `buzz-hostd` from
the Windows Desktop and its five bundled sidecars. No other production delta.
Scoped secret-pattern scan found no private key/token matches; heuristic only.

Existing workflow excluded the signed-wire regression via its `harness::` core
filter. Conductor authorized and independently read the one-step, ten-line CI
addition at `.github/workflows/windows-canary.yml:262`. It runs the actual
`buzz-core --lib host_deployment` suite and requires 13/0/0, 13 named passes and
the exact signed-reply regression. YAML parsing, Bash syntax and comparison of
all pre-existing steps passed. Exact case/count matched accepted local test log.
Owner full-library selection already includes the new error-category journey.
All native exact-case, shared harness, ACL and artifact gates are retained.

All workflow YAML parsed; push branch filters exclude this branch and no
workflow_run trigger exists. Canary remains manual-only, contents-read, unsigned
NSIS, updater-disabled, with seven-day artifact retention. Hermit activated with
`HERMIT_SELF_UPDATE=false`. Pre-push/pre-commit/commit-msg hooks absent; none
bypassed. Sandbox DNS failed initially; narrowly approved network access worked.

## 3. Execution ledger

Normal fast-forward push and exactly one dispatch succeeded. Run
[43127107116](https://github.com/Shipitrealgood/buzz/actions/runs/43127107116)
completed successfully on its first attempt, run number 27. Worker-owned
`gh run watch --exit-status --interval 45` (session 11954) terminated with exit 0.
Run created `2026-11-12T00:24:59Z`, terminal update `00:46:18Z`.
Native job `130881877895`: `00:25:02Z–00:40:46Z`, success, **15m44s**.
Build job `130881878230`: `00:25:03Z–00:46:17Z`, success, **21m14s**.
No rerun, retry or source repair was required.
Evidence root:
`/home/bryce/services/buzz-agents/releases/2722-reply-validator-canary/000d9c5e8bb1cb0d1314ecd2e96d136ad4be6916/43127107116/`.

## 4. Final measured outcome

**SUCCESS: 124 distinct outer Rust tests, zero failed or ignored, plus ACL.**
All native Cargo commands used `--locked --target x86_64-pc-windows-msvc`.
Owner/core run at the repository root; Desktop suites in `desktop/src-tauri`.

| Command/filter | Measured result |
| --- | --- |
| `cargo test … -p buzz-paired-owner --lib` | 16 passed |
| `cargo test … -p buzz-core --lib host_deployment` | 13 passed |
| `cargo test … -p buzz-core --lib harness::` | 9 passed |
| `cargo test … --lib nostr_convert::` | 73 passed |
| `cargo test … --lib commands::agent_discovery::relay_directory::` | 7 passed |
| `cargo test … --lib commands::paired_hosts::` | 3 passed |
| Exact `commands::agents::hosted::tests::hosted_native_create_recovery_and_session_fence` | 1 passed |
| Exact `commands::agents::hosted::tests::hosted_native_cleanup_confirmation_retry_and_session_fence` | 1 passed |
| Exact `managed_agents::discovery::presets::hosted_metadata_tests::hosted_grok_metadata_uses_static_preset_without_local_binary` | 1 passed |
| `crates/buzz-paired-owner/windows/test-journal-acl.ps1` | Owner used SQLite; another standard user read control and was denied DB/WAL/SHM |

Exact wire regression
`host_deployment::wire_tests::result_binding_and_permit_challenge_are_not_interchangeable`
and owner journey
`deployment::tests::protected_main_snapshot_same_key_expiry_tamper_and_terminal_receipt`
are included in the respective logs. The former covers actual signed status,
operation and permit boundaries; the latter includes the new fixed error
categories. Every individual suite summary and name was checked, including
0 failed/ignored and all three exact hosted paths. Counts exclude full-job-log
grep echoes and child execution. Source/category test coverage is retained from
the accepted review; this is actual Windows execution of those same tests.

Installer **`Buzz_0.5.29-test.27_x64-setup.exe`**, **56,478,143 bytes**, SHA-256
**`de7d9b9ee43a9805ddc0be71129207b7b7961c86099458fd7997ba5c8540c2f2`**.
[Download installer artifact 13903788220](https://github.com/Shipitrealgood/buzz/actions/runs/43127107116/artifacts/13903788220),
expires **2026-11-19T00:45:21Z**. ZIP **56,476,504 bytes**, SHA-256
`744c0ca566fc08d1421ee08a5ab719d2e4bc35f2a55a5f9817ee7f8cd8b71891`.
Exactly two members: installer and `build-receipt.txt`. Embedded source, version,
filename and installer hash agree. PE header and `file` identify an i386/PE32
NSIS wrapper (`0x14c`); that describes the installer stub, while build target,
filename and source receipt identify the x64 package. Payload binaries were not
unpacked or executed; no stronger binary-architecture claim is made.

[Native artifact 13905095124](https://github.com/Shipitrealgood/buzz/actions/runs/43127107116/artifacts/13905095124),
expires **2026-11-19T00:40:41Z**. ZIP **6,069 bytes**, SHA-256
`e885442c508198d7a464735113971dc78fd1495cb028b68e87f0d3bd7393ebd2`.
Exactly eleven flat members: source receipt, nine suite logs, ACL receipt.
Both ZIP sizes/digests match GitHub metadata. Exact member sets, duplicates,
traversal, symlink attributes and size limits were checked before extraction.
Run and artifact metadata bind source/run; native source receipt binds target;
both full job logs independently report version test27 and source `000d9c5e8`.

Under the private evidence root (directory mode 0700): `installer/` and `native/`
retain extracted artifacts; original ZIPs remain. `run.json`, `jobs.json` and
`artifacts.json` are terminal API records. `logs/*-raw.log` retain full job logs;
corresponding `.log` copies strip ANSI bytes. `verification.json` records
source, version, artifact expiry/URLs/hashes and every distinct test name/count.
`python3 verify.py` exited 0. `evidence-index.json` records each retained file's
size/SHA-256. This committed document is the sanitized shareable evidence index.

## 5. Decisions, deviations and limits

Only deviation: the necessary core Windows gate addition described above,
authorized and read-accepted by the conductor before publication. Product code
is unchanged from its accepted source review. Known nonfatal Node20/action-input
warnings persisted. No approval rejection occurred. Full job-log downloads used
`--allow-escape-sequences` only for file output, with stripped copies retained.
No live fixed claim. Current
`BUZZ_NATIVE_HOST_DEPLOYMENT_V1` opt-in unchanged.

**NOT RUN:** full `just ci`, repeated strict clippy (accepted source receipt
records existing failures), extra source/UI re-audit, native mutations, installer
execution, full native IPC/UI and live status-validation journey, provider/login/
chat/service pilot, human acceptance or deployment. Previous missing local Linux
GLib/WebKit prerequisites were not installed. No origin push, release/tag,
auto-updater, test1/Jeff, live service/config or board action occurred.

## 6. Conductor review handoff

Conductor checks finite run/artifact evidence independently; no extra source
review campaign is requested. Review probes:

1. **Exact source:** compare `run.json`, both artifacts' `workflow_run`,
   `native/source.txt`, `installer/build-receipt.txt` and version lines in full
   job logs. The later receipt commit is not the built source.
2. **Archive provenance:** compare `verification.json` and `evidence-index.json`
   to original ZIPs and `artifacts.json`, including their distinct expiries.
3. **New Windows gate:** `.github/workflows/windows-canary.yml:262` is the sole
   workflow delta; `native/host-deployment-tests.log` must contain 13 named
   passes and the exact wire regression.
4. **Owner coverage:** `native/paired-owner-tests.log` contains 16 passes and
   the actual deployment journey; accepted source review describes its fixed
   category assertions. First-attempt expiry-case success is not a flake repair.
5. **Desktop scope:** inspect the three exact hosted logs (1/0/0 each), the
   other suite logs, and the 124 distinct names. Fixtures do not prove live UI.
6. **ACL and package:** inspect UTF-16 `native/journal-acl.log`, successful ACL
   step, installer receipt/hash and PE-wrapper distinction above.
Prior owner expiry reliability concern is not closed by an isolated green run.
Live test1/Jeff validation requires explicit user authorization; installation,
provider/login/chat work, live config/service mutation and broad CI are outside
this assignment.

## 7. Ownership release / next gate

All owned processes are terminal: publication/dispatch 70759, monitor 11954,
native download/log 19515, installer download 94012 and metadata 14938. No
source/test mutation is held. Starting worktree was clean; final owned changes
are the ten-line CI commit and this receipt only. No unrelated changes existed
or were altered. Scoped whitespace, link, source-continuity and status checks
pass before the signed receipt commit. The receipt is local only; fork remains
exact built source `000d9c5e8bb1cb0d1314ecd2e96d136ad4be6916`.

Recommended next gate: conductor independently checks the finite run/artifact
receipt and provides test27 to the user. Ask the user before test1 validation.
No new product question or prerequisite. Source review acceptance stands.
