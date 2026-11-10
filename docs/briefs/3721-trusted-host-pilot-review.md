# M-1 re-gate: trusted netcup pilot preparation (#3721)

2026-11-10. Conductor `/root` → same independent codex-chuck reviewer.
Executor `/root/netcup_trusted_pilot_prepare`, codecodex, requested Astra/high;
served model/reasoning not exposed. **Preparation only; NOT READY TO ACTIVATE.**

## 1. Where the repair lives

Same exclusive worktree `/home/bryce/services/buzz-wt-3720-host-registration`,
branch `proof/3720-host-registration-events` (not main), clean repair baseline
`c6c1ae869b7d41545f48fe851ca20c107a85223b`. Review baseline through the signed
repair commit returned by conductor. Only execution packet and this brief changed;
independent report remains intact. No baseline dirt or detached process.

```sh
cd /home/bryce/services/buzz-wt-3720-host-registration
export HERMIT_SELF_UPDATE=false
. ./bin/activate-hermit
git diff c6c1ae869b7d41545f48fe851ca20c107a85223b..HEAD -- docs/executions/3721-trusted-host-pilot-preparation.md
```

Package root remains
`/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/c97256e20893b2688c1cc1c6bc2e23361aab707e/`.
Current package manifest SHA256:
`f8a53553ca83eb4fe56481d9a257d9226840d1908ccbb888c9d4e8829cb94898`.
Original `02e2ffe9…5490` manifest, runbook and SOURCE are preserved under
`history/pre-M1/`. Three binaries and three config/template files are byte-unchanged.

## 2. What changed

[Execution packet](../executions/3721-trusted-host-pilot-preparation.md#m-1-repair--interruption-aware-rollback-2026-11-10)
contains the exact repaired OPERATOR.md text and finite check ledger.
The repair implements explicit observed-prefix branches, absent-user handling,
stopped current-state snapshot, metadata validation/numeric custody restoration,
exact-owned-file removal and verified original-service restore. No live command,
product code, rebuild, provider check, new utility or installer was added.
Governing input is your [M-1 report](../audits/post-audit/3721-trusted-host-pilot-preparation.md).

## 3. Your job

Re-gate M-1 and defects introduced by this repair only. Previously accepted package,
source, trusted-account and blocker conclusions remain evidence; do not reopen
architecture or broaden runtime tests. Both relay43104 and confirmed owner cleanup
remain independent activation holds. Preparation acceptance cannot close them.

## 4. Specifically inspect

1. **Archive failure before unit move:** rollback section1 chooses `original`,
   verifies the installed regular-unit hash and stops the old controller. Section3
   allows the unchanged988:988/private tree without requiring a successful archive.
   **Probe:** trace an out-of-space failure after disable/stop; no saved-unit lookup,
   unlink or stale DB extraction should occur in this branch.
2. **Unit moved, mask missing:** section1 chooses `moved` and verifies the saved
   regular unit. Section4 restores it only into a truly absent destination.
   **Probe:** trace interruption between forward mv/ln and rollback unlink/mv;
   neither branch may overwrite an unexpected destination or start after failed mv.
3. **Partial ownership:** section3 accepts only988:988 or1002:1003 private regular
   files/directories, checks root entry names, then snapshots CURRENT stopped state
   before chown/chmod and verifies all988:988/0700/0600.
   **Probe:** trace a mixture after interrupted chown and a failed new snapshot.
   No old start may bypass validation/snapshot failure; no archive is extracted.
4. **Absent/partially installed user controller:** section2 handles not-found only
   with absent path and quiescence. A known exact file awaiting reload is inspected
   before reload; a cached loaded unit is stopped even if rollback removed its file.
   **Probe:** trace no installation, only controller installed, and removal-before-
   reload. A file mismatch or unknown manager status must stop for assessment.
5. **Restart exclusion and exact removal:** old/new controller MainPID/state/cgroup
   are checked before custody; exact pilot stop is separate and optional only by
   recorded placement. Controller file removal requires owner/mode/byte match;
   template remains with instance drop-ins. Final original restore rechecks hashes,
   no active new controller and no active old controller before enable/start.
   **Probe:** confirm previous enabled/active state is restored, and no absent-unit
   operation or unexpected unlink is silently ignored.
6. **Evidence and limits:** new nine shell fences use `bash -n`, two Python heredocs
   use AST parsing, and seven prefixes were statically inspected.
   **Probe:** `m1-repair-checks.json` records syntax/static evidence, not native
   execution. Exact runbook/config text and current manifest must match committed
   packet. Old binary/config hashes must stay equal to original manifest.

## 5. Decisions/deviations

No architecture deviation. Recovery is selected from actual filesystem/manager
state rather than assumed last command. A fully unchanged early prefix can restore
without a completed archive; after custody changes a fresh stopped-state snapshot
is required. Unexplained files/modes/processes refuse recovery. These are explicit
fail-closed operator branches, not an automatic recovery framework.

## 6. What not to chase

Existing accepted99 source/110 Windows+ACL evidence, unchanged binaries/config,
relay implementation/activation, native provider behavior and lifecycle redesign
are outside re-gate. Missing narrow owner Cleanup remains #3724/#3725; #3767 is
legacy cleanup. Emergency stop retains journal/placement and is not teardown.
No live manager queries/actions, private-state/key/provider reads, SSH, board writes,
new test campaign or service/provider invocation is authorized.

## 7. Return and preflight

Verify clean pinned worktree; read M-1 and exact repair; compare new and historical
manifests and exact text; inspect prefix branches; use syntax-only checks if useful.
Update your existing independent report with M-1 re-gate and any repair-introduced
findings. Return signed report pin, remaining activation holds, actual checks and
ownership release. All executor commands completed; no owned process remains.
