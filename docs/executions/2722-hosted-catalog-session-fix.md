# #2722 hosted catalog session fix — execution and review brief

2026-11-10 UTC. Executor `/root/hosted_catalog_fix`; requested Astra/high;
runtime model/effort not independently exposed. Scope: catalog guard and the
existing native fixture only. No live-system access or change is authorized.

## Where the work lives

Worktree: `/home/bryce/services/buzz-wt-3720-host-registration`.
Branch: `proof/3720-host-registration-events`.
Starting HEAD: `d5cfb0d1c245ff0956d6a00289cf1f79e3cb73c7`; measured clean.
Owned paths are this report, `agents_hosted_commands.rs` and
`agents_hosted_tests.rs` under `desktop/src-tauri/src/commands/`.
Conductor owns board/roadmap work; preserve any concurrent document changes.

## Contract and current ledger

Accepted dispatch is the narrow repair specification. Existing source contract:
[Windows hosted flow](../specs/2722-windows-hosted-flow.md).
Read and confirmed: catalog opens `Create` with empty operation/runtime, projects
verified status, then incorrectly invokes `Create::check`, whose
`Device::deployment_intent` call rejects the empty operation. Catalog requires
the existing session identity/community fence, without setup-intent validation.
Keep `Create::check` and deployment validation unchanged for actual setup paths.

## What landed

`agents_hosted_commands.rs:155` calls the extracted `catalog` projection; its
final guard at line 194 checks `Session::keys` only. Projection logic is unchanged;
status is cloned because the helper borrows the fixture/production `Create`.
`agents_hosted_tests.rs:135` starts with empty operation/runtime, checks the real
setup guard still refuses them, then exercises the production catalog projection.
It asserts one canonical installed Grok runtime, matching status/inventory,
available metadata, unknown auth and an empty intent journal. Generation change
rejects at line 177; recapturing the session resumes the existing create/recovery
and cleanup journeys. No additional test name or workflow entry is required.

Measured diff: production **+7/-2**, tests **+43/-2**, plus this report.
`agents_hosted.rs`, `paired_hosts.rs` and owner `deployment.rs` are unchanged.
The first two are under `desktop/src-tauri/src/commands/`; the owner file is
`crates/buzz-paired-owner/src/deployment.rs`.

## Verification ledger

All commands ran in the pinned worktree with `HERMIT_SELF_UPDATE=false` and
`. ./bin/activate-hermit` before toolchain/Git commands. Initial read-only Git
baseline preceded discovering the repository's Hermit instruction.

| Gate | Measured result |
| --- | --- |
| `rustfmt --edition 2021 --check desktop/src-tauri/src/commands/agents_hosted_commands.rs desktop/src-tauri/src/commands/agents_hosted_tests.rs` | PASS, exit 0; parser/format check, not typecheck |
| `git diff --check` | PASS |
| `CHECK_FILE_SIZES_BASE=d5cfb0d1c245ff0956d6a00289cf1f79e3cb73c7 node desktop/scripts/check-file-sizes.mjs` | PASS, exit 0 after authorized read-only subprocess retry; sandbox first blocked `spawnSync git` with EPERM |
| `git diff --exit-code -- desktop/src-tauri/src/commands/agents_hosted.rs crates/buzz-paired-owner/src/deployment.rs desktop/src-tauri/src/commands/paired_hosts.rs` | PASS; setup guard, operation validation and session implementation unchanged |
| `rustup target list --installed` | Only `x86_64-unknown-linux-gnu` |
| `pkg-config --modversion glib-2.0 gobject-2.0 gio-2.0 gtk+-3.0 webkit2gtk-4.1` | All five packages unavailable |
| Native Windows tests and typecheck | NOT RUN: affected module is `cfg(windows)` at `agents.rs:418`; no installed Windows target/runner. A Linux test filter cannot exercise it. No toolchains or GTK installed. |
| Failing-before / passing-after mutation execution | NOT RUN for the same native limitation; do not promote the source proof to measured runtime evidence |
| Full `just ci`, live pilot, human acceptance | NOT RUN; outside this dispatch |

Read-based self-review caught and corrected initial test JSON assumptions:
nested catalog metadata uses snake_case `default_args`, and `auth_status` is a
tagged object. Final assertions match the serializers in
`managed_agents/types.rs:644` and `:674`; no executed test result is claimed.
Existing [Windows canary evidence](2722-shared-discovery-windows-canary.md)
proves the fixture's prior version could execute, not that this new change passes.

## Your job: independent review

Fresh @codex-chuck should review the scoped commit after `d5cfb0d1c`; this report
is the authorized combined execution/brief artifact, not an independent gate.
Use the pinned revision supplied in the executor return, not an unpinned branch
head. Decide source acceptance separately from pending native verification.

Special instruction: the new fixture has not been compiled or run. Inspect
serialization assertions and borrow/type compatibility before granting a source
gate; conductor owns the subsequent unchanged Windows canary workflow.

## Specifically look for

1. **Does the real command use the tested path?** Probe
   `agents_hosted_commands.rs:154`: `open` still supplies empty strings and calls
   `catalog`; the helper contains the original whole projection and final guard.
2. **Is the session fence intact?** Probe `agents_hosted_commands.rs:194` and
   `paired_hosts.rs:78,109`: keys checks workspace generation/origin and signer.
   Removing that guard should fail the fixture's community-change assertion at
   `agents_hosted_tests.rs:177` when the native test executes.
3. **Are setup protections unchanged?** Probe `agents_hosted.rs:19` and
   `crates/buzz-paired-owner/src/deployment.rs:50`: invalid operation must still
   reject. The assertion at `agents_hosted_tests.rs:148` exercises that path.
4. **Is the success case meaningful?** Probe `agents_hosted_tests.rs:149`:
   installed ID, command/default arguments, availability, auth, status, inventory
   and zero intent assertions use the existing supported Grok status fixture.
   Restoring `create.check()?` at the projection's final guard should fail the
   success assertion with `invalid operation`; this is inferred, not measured.
5. **Does the prior journey survive?** Probe `agents_hosted_tests.rs:181`:
   recapture after generation increment and restore operation/runtime before
   existing rejection, recovery, cleanup and stale-session assertions. The
   shared `native_phases` serves both exact existing workflow cases.

## Spec deviations and boundaries

No contract deviation. The permitted minimal projection helper allows testing
the production branch without new IPC machinery; borrowing adds one status clone.
This report includes the review brief in the dispatch-owned execution path rather
than adding an unowned `docs/briefs` path. No missing foundation was worked around.

Review is limited to the two Rust files and this report. Clock skew, retry-button
behavior and the earlier unproven private-deployment-response cause remain with
the conductor's separate routing. Existing agents, provider adapters, service
control, runtime sources, credentials and live journals were not accessed or
changed. No push, workflow launch, deployment, merge or taskctl mutation occurred.

## Deliverables and next gate

Return source findings to conductor with exact commit pin and distinguish native
NOT RUN from green static checks. After independent source review, conductor can
run the existing Windows canary, including both exact hosted fixture names in
`.github/workflows/windows-canary.yml:290`. Native success and live/human pilot
acceptance remain outstanding. No product or architecture open question.

Preflight: confirm branch/pin and clean status; read this report and the linked
hosted-flow contract; check the two-file diff before testing. Before/after native
mutation proof remains recommended when a suitable runner is available.

All owned tool processes are terminal; no child/build/test/watch process was
launched in the background. Baseline was clean and no unrelated edits appeared
during execution. Exclusive write/test ownership releases with the signed-off
commit named in the final return; no additional source mutation is planned.
