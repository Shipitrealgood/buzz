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
