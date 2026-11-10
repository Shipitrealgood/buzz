# Review scope: refreshed netcup rollout packet (#3734 / #3721)

**Date:** 2026-11-10. **Requester:** conductor for Bryce.
**Next role:** fresh independent codex-chuck preparation reviewer, requested Astra/high.
**Executor:** `/root/netcup_rollout_prepare`, codecodex, requested Astra/high;
served configuration not exposed. WSL relay lane is independently pending.

## 1. Where the work lives

Use `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events` (not main), same exclusive workspace.
Review range `344baa091ad486653ee99aa8b1de64d4839b4ada..HEAD` contains one signed
execution/brief commit; conductor supplies the full final hash. Build HEAD remains
344baa0 even after the documentation commit. Starting tree was clean.

```sh
export HERMIT_SELF_UPDATE=false
. ./bin/activate-hermit
git log --oneline 344baa091ad486653ee99aa8b1de64d4839b4ada..HEAD
```

Artifact root `/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/344baa091ad486653ee99aa8b1de64d4839b4ada/`.
Its `package/` contains all three binaries, config/controller/template, OPERATOR,
SOURCE and manifest. Manifest SHA256:
`ec1fb606e835ba238a39021a3ad17d90e1f8c3ea933b568f2fa5db78e5671441`.

## 2. What landed

The [execution receipt](../executions/3734-hetzner-netcup-rollout-preparation.md)
binds exact staged text and hashes to an offline locked release build at344baa0.
All nine shell command blocks preserve the accepted M-1 mechanics after path
substitution; narrative now carries test.24 and supported confirmed Cleanup.
Only two Markdown files changed in Git; no product/live changes. Source contract:
[ratified specification](../specs/3734-hetzner-netcup-rollout-preparation.md) P1;
[executor dispatch brief](3734-hetzner-netcup-rollout-spec.md).

## 3. Your job

Independently review this finite preparation delta and its evidence. Challenge
source/artifact continuity, exact-text fidelity and whether revised instructions
preserve the accepted custody/recovery contract. Return preparation acceptance or
concrete rework; do not call the overall rollout live-ready without the separately
owned WSL receipt and user approval. No fresh broad source/native campaign.

**Special instruction:** syntax checks passed via a narrow read-only sandbox retry;
they say nothing about actual credential delivery, notify or cgroup behavior.
No fresh real-agent/controller metadata was queried. Treat old measured metadata
as historical until the approved live checkpoint revalidates it.

## 4. Specifically look for

1. **Does actual build344baa0 truly carry accepted production929c/test.24?**
   **Probe:** [receipt:31](../executions/3734-hetzner-netcup-rollout-preparation.md:31),
   SOURCE.json, production-continuity.txt and host-dependency-delta.patch. Six
   Markdown-only differences from929c; only [typed sender:179](../../crates/buzz-ws-client/src/connection.rs:179)
   changed scoped host dependencies sincec972. Match three binaries to build
   outputs and manifest; ACP is legitimately unchanged because cached inputs are
   unchanged, not evidence that an old host package was copied blindly.
2. **Did any prefix-recovery mechanics drift under narrative refresh?**
   **Probe:** [receipt:477](../executions/3734-hetzner-netcup-rollout-preparation.md:477),
   original/moved/masked branches and [custody:612](../executions/3734-hetzner-netcup-rollout-preparation.md:612).
   Normalize full release pin and backup3734-trusted-344baa0 to old values and
   compare all9 shell blocks. Both embedded Python bodies must remain unchanged.
   Compare against accepted [M-1 re-gate](../audits/post-audit/3721-trusted-host-pilot-preparation.md:3),
   not the historical initial failed review. No new interruption campaign needed.
3. **Does supported Cleanup replace only the stale source blocker?**
   **Probe:** [receipt:444](../executions/3734-hetzner-netcup-rollout-preparation.md:444),
   pending same-agent Check/Retry through authenticated Cleaned/local completion,
   emergency stop retaining state. Accepted [native review](../audits/post-audit/3725-confirmed-hosted-cleanup.md:394)
   credits111 native cases+ACL; human/provider/pilot acceptance remains unrun.
   Current [HostedAction:288](../../desktop/src-tauri/src/commands/agents_hosted_commands.rs:288)
   supports Cleanup and routes Check/Retry against retained Cleanup.
4. **Are code/schema/flag and WSL ownership truthful?**
   **Probe:** [receipt:125](../executions/3734-hetzner-netcup-rollout-preparation.md:125)
   and OPERATOR prerequisites. f316 image on82 remains missing proof, not a
   flag-only rollout or assumed additive-schema compatibility. Match old image
   full ID, retained schema82, auto-migrate=false, community-deletion limitation,
   same-candidate deployment=false preference and90s ordinary-traffic threshold.
5. **Are staged paths exact and trusted-account scope preserved?**
   **Probe:** [config:157](../executions/3734-hetzner-netcup-rollout-preparation.md:157),
   [controller:181](../executions/3734-hetzner-netcup-rollout-preparation.md:181),
   [template:210](../executions/3734-hetzner-netcup-rollout-preparation.md:210).
   Compare staged bytes, parsed capacity1/runtimes:{}, pinned package binaries,
   normal HOME/PATH and source template mode0600. No existing ACP/agent install
   target or private HOME/root-helper/copy-login addition.
6. **Are finite checks, failures and live limits distinguished?**
   **Probe:** [checks:99](../executions/3734-hetzner-netcup-rollout-preparation.md:99),
   preparation-checks.json and syntax-verification{,-sandbox}.json. Initial
   SO_PASSCRED failures remain; approved read-only retry exit0 is syntax only.
   Runtime --help deliberately exits1 without starting runtime. Shell/Python
   bodies were parsed, never executed. Oldc972 manifest still verifies8/8.

## 5. Spec deviations

No deviation from P1. The permitted retry only lifted the sandbox restriction
for the same two read-only systemd-analyze verification commands. No operator
commands, provider/model probes or real-state reads were used to obtain green.
No source defect or missing architectural foundation was discovered.

## 6. What NOT to audit / chase

Review the bounded host artifact/runbook delta. Accepted source/Windows gates,
existing agents, native login and live controller/keys remain outside this pass.
WSL owns relay image/archive and exact-old-image compatibility proof using the
[standalone WSL prompt](3734-wsl-relay-preparation.md); no duplicate netcup relay
build, integration fixture, SSH or task messaging is needed. Reboot/two-host/
Android/full release validation remain separate. Conductor owns taskctl/routing.

## 7. Deliverables and pre-flight

Return an independent report to conductor with pinned commit/artifact hashes,
findings, actual read/run limits and live-gate disposition. No merge/push/deploy.
Suggested report `docs/audits/post-audit/3734-netcup-rollout-preparation.md`.

Before review: confirm conductor's final signed pin and clean tree; confirm all
executor processes are terminal and ownership transferred; read spec/this brief
and accepted M-1/Cleanup reviews; verify exact new manifest before any safe probe.
The current executor's gates pass, but independent acceptance is still pending.
