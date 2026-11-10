# Review scope: shared discovery Windows canary (#2722)

2026-11-10 UTC. Requester: conductor. Next owner: fresh independent reviewer.
Executor requested Astra/high; served model/effort not independently exposed.

## 1. Where the work lives

Worktree `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`, starting clean at
`da4ee105a34ff45e84d5e44e64c3e31d3a3c9651`.
Built source `23d2dc15c6c45554181938b7956b59c085980455`; later receipt commits are
listed in the terminal handoff. Review `git diff da4ee105..HEAD` here.
Both jobs are terminal SUCCESS: native 15m6s, build 21m40s. The native gate
measured 110 Rust cases plus ACL; the hosted fixture ran 9.64s. Watcher and
download exited 0. Exclusive write/test ownership returns to conductor after
the final documentation commit; no unrelated edits or owned running processes.

## 2. What landed and governing chain

The existing Windows Canary workflow now additionally executes nine shared
`buzz-core` harness cases moved by S2/S3 extraction, requiring exactly nine named
passes and zero failures/ignored. Existing owner, conversion, directory,
registration, exact hosted native/metadata and disposable-account ACL gates remain.
No production source was changed for this addition. Accepted source and M-1 repair
are already in the baseline; architecture remains ordinary trusted nonroot host
account HOME/PATH/native login/network and distinct Buzz identity/config.

Read [execution receipt](../executions/2722-shared-discovery-windows-canary.md),
[accepted source re-gate](../audits/post-audit/3721-host-discovery-windows.md),
[source execution](../executions/3721-host-discovery-windows.md), and
[prior canary review](../audits/post-audit/2722-windows-native-canary.md).
The original successful `test.20` receipt is preserved separately; this run must
prove the newer source, not borrow its Windows result.

## 3. Your job

Independently assess this bounded workflow/evidence packet and any concrete native
failure correction recorded below. Check source-to-run-to-artifact identity and
actual native outcomes. Existing accepted S1/S2/S3 Linux/Node evidence stands for
unchanged scopes; restarting those broad suites is outside this gate.
The executor's local gate probes are evidence, not an independent verdict.

## 4. Specifically look for

1. **The relocated tests actually execute.**
   Probe `.github/workflows/windows-canary.yml:263`: existing `buzz-core` library
   filter `harness::`, exact nine result and nine named success lines. Match those
   names to `crates/buzz-core/src/harness/mod.rs:268`,
   `crates/buzz-core/src/harness/runtime_metadata.rs:198` and
   `crates/buzz-core/src/harness/windows_install.rs:104`.
   Native metadata projection remains a separate exact Desktop case.
2. **M-1's rejection evidence is not inflated.**
   Probe `desktop/src-tauri/src/commands/agents_hosted_tests.rs:145`: all four
   scenarios assert empty intent, record, publication queue, no loopback connect
   and no process. They run within one child of one outer exact Rust test.
   A successful exact test proves those assertions ran; it is not four separately
   reported cargo cases, real hosted IPC, provider or human acceptance.
3. **Mandatory gates retain failure semantics.**
   Probe `.github/workflows/windows-canary.yml:255` and `:284`: owner/directory
   suites retain nonzero checks; exact hosted paths retain one pass and zero
   failures/ignored; `set -euo pipefail` retains cargo failure propagation.
   ACL runs unless cancelled and its receipt is separate from Rust totals.
4. **The canary stays artifact-only.**
   Probe workflow `:10`, `:137`, `:154`, `:181`: manual dispatch, fork/ref guard,
   credential-free checkout, read-only contents permissions, unsigned NSIS and
   updater artifacts disabled. No release, tags or primary branch pushes.
   `desktop/src-tauri/src/commands/agents_hosted.rs:143` still requires exact feature flag value `1`.
5. **Receipt consistency and limitations.**
   Probe staged run/artifact metadata, native source and test receipts, full logs,
   installer receipt, ZIP members, hashes and expiry. The final receipt gives
   exact local paths. Count outer cases once; child/grep echoes are duplicates.
   Preserve any failed attempt separately and require the accepted artifact's
   own source to have both successful jobs.

## 5. Decisions and deviations

No product or architecture deviation. Conductor confirmed the nine moved core
cases belong in this existing Windows workflow. The initial trigger-check script
conservatively rejected tag-only push filters; corrected inspection treats those
as excluding branch pushes. Generated pre-push/pre-commit hooks were absent at
baseline, not bypassed or installed. Linux native dependency installation and
full local CI remain excluded. Local shared-gate probes use temporary mock cargo
only; they are not substitute native measurements.

## 6. What not to audit or chase

Live netcup/relay/pairing, existing Claude/Grok/Scout, provider/login/service state,
installer execution, feature activation and human acceptance remain separate.
Accepted normal-account architecture, S1 review and unchanged Linux/Node suites
are inputs. Repository-wide CI, Linux GTK installation, workflow modernization,
main catchup and new test frameworks are outside this bounded follow-on.

## 7. Deliverable and pre-flight

Return a committed independent report and explicit actual/not-run gate verdict
to conductor. This packet grants no merge, rollout or board authority.
Before review: verify final clean handoff pin; read receipt and prior re-gate;
confirm all owned jobs/watchers/downloads are terminal; check exact built source
contains `da4ee105`; verify both jobs and artifact hashes. The execution receipt contains final counts, hashes, URLs and expiry; no new
native run is needed for these docs-only changes.

## Terminal artifact index

Run [42803199965](https://github.com/Shipitrealgood/buzz/actions/runs/42803199965),
source `23d2dc15c6c45554181938b7956b59c085980455`, terminal success at
2026-11-10 11:17:13 UTC. No native failure or source correction occurred.
Installer [13759200363](https://github.com/Shipitrealgood/buzz/actions/runs/42803199965/artifacts/13759200363):
`Buzz_0.5.29-test.21_x64-setup.exe`, 56,407,044 bytes, expires
2026-11-17 11:15:44 UTC. Installer SHA-256
`6c4ad18e50bb8fd6b2e798d36865ebc69ef386b0b18726a410f3cc58e4c37208`.
ZIP SHA-256 `69d522c9a2211b3e342d3bfe6096a8cc5cb675f801f9008ee6edcf9af8fd5424`.

Evidence root:
`/home/bryce/services/buzz-agents/releases/2722-windows-canary/23d2dc15c6c45554181938b7956b59c085980455/42803199965/`.
Use `run.json`, `jobs.json`, `artifacts.json`, `verification.json`, `tests/`,
`installer/`, both ZIPs and ANSI-stripped build/discovery logs. Both ZIP digests
match GitHub metadata; installer source/version/digest matches its receipt.
The native archive has nine flat text members, seven Rust suite logs totaling
110 cases, one source receipt and one UTF-16 ACL receipt. No installer execution,
feature activation, full CI, human or live/provider acceptance is implied.
