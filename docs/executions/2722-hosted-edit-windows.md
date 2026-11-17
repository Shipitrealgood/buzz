# #2722 hosted Edit/Start — Windows CI failure receipt

**Windows gate failed; no installer produced.** Recorded 2026-11-17 by
`codecodex`, assignment `/root/hosted_edit_windows_ci`. Requested configuration:
`gpt-6-astra` / high; actual served model/reasoning is not exposed in authoritative
runtime metadata. Source acceptance is superseded by the concrete compiler defect
below; this receipt does not supply an independent review gate.

## Source and publication

- Tested source: `21099d751150a219e4c3d342aafea9ff04495f0e`.
- Workspace: `/home/bryce/services/buzz-wt-3720-host-registration`, branch
  `proof/3720-host-registration-events`; intake clean, no unrelated changes.
- Read: [accepted source review](../audits/post-audit/2722-hosted-management-followthrough.md)
  and [existing workflow](../../.github/workflows/windows-canary.yml).
- Measured preflight: exact public fork URL `https://github.com/Shipitrealgood/buzz.git`,
  canary branch at `d7714be862f4514822c303b124a347f5e07dc8c5`, fast-forward ancestry,
  and no installed pre-commit, commit-msg or pre-push hooks. Hermit activated with
  `HERMIT_SELF_UPDATE=false`.
- The worker's combined publication/dispatch request was rejected by automatic
  approval review because direct human authorization was absent from its context.
  No mutation occurred from that request. The conductor then used the original
  conversation's direct user authorization, completed the fast-forward to the tested
  source, verified the remote SHA, and dispatched once. No worker workaround or retry.
- This receipt is the only repository change after CI. Its commit is a documentation
  revision, **not the tested source SHA**; no product, test or workflow edits occurred.

## Terminal run and measured failure

[Windows Canary run 43986614016](https://github.com/Shipitrealgood/buzz/actions/runs/43986614016),
run number 29, branch `build/paired-hosts-canary`, exact head SHA independently
verified as above. Overall status: **completed / cancelled**.

| Job | Terminal result |
| --- | --- |
| [Build Windows canary, 133716296888](https://github.com/Shipitrealgood/buzz/actions/runs/43986614016/job/133716296888) | Failure; sidecars passed, NSIS build failed |
| [Test Windows host discovery, 133716297227](https://github.com/Shipitrealgood/buzz/actions/runs/43986614016/job/133716297227) | Cancelled during desktop native compilation |

Measured compiler diagnostic from the completed build job:

```text
error[E0308]: mismatched types
  --> src\commands\agents.rs:1040:60
crate::relay::assert_expected_relay_scope(Some(&reconcile_relay), &scope.community)?;
expected `&str`, found `&ScopedWorkspaceRelay`
error: could not compile `buzz-desktop` (lib) due to 1 previous error
```

The paired native job builds this same production Desktop library, so completing
its remaining tests required a source correction. After capturing the build log,
the worker requested cancellation under the dispatch's explicit failure rule and
monitored through terminal completion. No rerun or source repair was attempted.

## Tests and artifact limits

Measured from timestamped native job stdout, counting distinct fully qualified
passing test names: **39 passed** — owner 17, signed host deployment 13, shared
harness facts 9. The log has 40 matching success rows because the workflow's exact
`grep` repeats one deployment result; that duplicate is excluded. Each suite reports
zero failed and zero ignored. This partial pass is evidence from a failed run.

Desktop directory/conversion/paired-host suites did not complete. Both exact hosted
native cases (`hosted_native_create_recovery_and_session_fence` and
`hosted_native_cleanup_confirmation_retry_and_session_fence`), hosted Grok metadata,
and isolated Windows SQLite ACL checks were **not run**. Source read confirms the
hosted fixture removes `BUZZ_NATIVE_HOST_DEPLOYMENT_V1`; no env-absent native pass
is claimed because those cases were skipped.

Both jobs derived **0.5.29-test.29**, but this is only an attempted build version.
The run artifacts API returned `total_count: 0`. There is no installer, ZIP,
downloadable artifact URL, artifact ID, installer size or binary hash to report.
The receipt upload was skipped following cancellation.

Not run: installed Windows/Start-menu launch, real Edit/Stop/Start, live provider
behavior, native Linux/macOS compilation, native clippy and `just ci`. No installation,
live-system access, agent action, service restart, taskctl mutation, PR, tag, release
or updater publication occurred.

## Durable evidence and next gate

Local evidence directory:
`/home/bryce/services/buzz-agents/releases/2722-hosted-edit-windows/21099d751150a219e4c3d342aafea9ff04495f0e/`.
It contains initial/terminal run JSON, zero-artifact JSON, attached monitor log,
completed build/native job logs with terminal controls stripped, and the deduplicated
`distinct-passed-tests.txt`. Raw logs remain outside git.

| Evidence | Bytes | SHA-256 |
| --- | ---: | --- |
| `build-job.log` | 232379 | `f32b9b9b62271f65017a80e69b1a000a655446229c01bda3d22bccc813a4023b` |
| `native-job.log` | 139825 | `6089c65482c98f971176aca121d50753794ea41bf8b3877e2ef6ef3e230a66d1` |
| `distinct-passed-tests.txt` | 3272 | `07916a0dd1e25bcec3c96789e30c6e1fafbb35d5cb2fe69896d4fc26cf2325d7` |

Verification commands: `git status --porcelain`, `git merge-base --is-ancestor`,
`git ls-remote`, `gh run view --json ...`, `gh run watch --exit-status`, job-log and
run-artifact `gh api` reads, Python distinct-name counting/SHA-256, and
`git diff --check`. The attached monitor exited 1 after cancellation; all tool
processes are terminal. Documentation content/link/whitespace checks passed.

Handoff to conductor: return the measured E0308 at
[agents.rs](../../desktop/src-tauri/src/commands/agents.rs):1040 to the original
executor, then the same independent reviewer checks the bounded correction.
[ScopedWorkspaceRelay](../../desktop/src-tauri/src/relay/scope.rs):40 already exposes
`as_str()` (read); preserve the pinned relay and scope checks. After that review,
publish and run the corrected source under the existing authorization. Do not widen
this repair into unrelated workflow warnings or tooling changes. Successful Windows
CI and subsequent human installed-launch/Edit/Stop/Start acceptance remain required.

Signed: `codecodex` — #2722 Windows publication/CI evidence worker, 2026-11-17.
