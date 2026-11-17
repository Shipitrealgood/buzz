# #2722 hosted Edit/Start — Windows CI passed

**Windows build and scoped native gates passed. Canary 0.5.29-test.32 is ready for
human acceptance.** This supersedes the failed-run platform status preserved below;
installed normal launch and real Edit/Stop/Start remain pending.

Recorded 2026-11-17 by `codecodex`, assignment `/root/hosted_edit_windows_ci`.
Requested Astra/high; actual served model/reasoning is unknown. Tested source and
clean intake: **`bbf105b5b2a8c213e1aabab5273c40547298b18e`**, same worktree
`/home/bryce/services/buzz-wt-3720-host-registration` and branch
`proof/3720-host-registration-events`. Read the
[round-5 independent source acceptance](../audits/post-audit/2722-hosted-management-followthrough.md).
Conductor performed the authorized fast-forward and single dispatch. This worker
only monitored, collected evidence and updated this document. No source/test/workflow
edits, extra publication or rerun occurred. The receipt commit is documentation
only and **is not the tested CI source SHA**.

## Terminal run

[Windows Canary run 43997403503](https://github.com/Shipitrealgood/buzz/actions/runs/43997403503),
number 32, `build/paired-hosts-canary`: **completed / success**, exact head independently
verified as `bbf105b5b2a8c213e1aabab5273c40547298b18e`.

- [Build job 133752750531](https://github.com/Shipitrealgood/buzz/actions/runs/43997403503/job/133752750531):
  **success**, 19m57s; sidecars, Desktop compilation, NSIS packaging and upload.
- [Native job 133752751073](https://github.com/Shipitrealgood/buzz/actions/runs/43997403503/job/133752751073):
  **success**, 16m3s; all selected Rust suites, exact hosted cases, ACL and receipt upload.

## Measured native acceptance

Downloaded receipts contain **125 distinct Rust passes, zero failures and zero
ignored cases** across the selected suites. Counts come from exact fully qualified
test names in receipt files, not echoed command output or summed duplicate runs.

| Suite | Passed |
| --- | ---: |
| Paired-host owner | 17 |
| Signed deployment | 13 |
| Shared harness facts | 9 |
| Nostr conversion | 73 |
| Relay directory/loopback | 7 |
| Paired-host commands | 3 |
| Exact hosted cases below | 3 |

The three exact cases all passed:

- `commands::agents::hosted::tests::hosted_native_create_recovery_and_session_fence`
  (10.27s): includes linked-definition model/prompt, consecutive recovery phases,
  injected phase-2 failure/retry, unchanged later-saved main record and retained event.
- `commands::agents::hosted::tests::hosted_native_cleanup_confirmation_retry_and_session_fence`
  (10.50s).
- `managed_agents::discovery::presets::hosted_metadata_tests::hosted_grok_metadata_uses_static_preset_without_local_binary`
  (0.00s).

Both native journey fixtures explicitly remove `BUZZ_NATIVE_HOST_DEPLOYMENT_V1`
from their isolated subprocess environment (source read), and both passed on Windows
(measured). This establishes the selected env-absent native fixture behavior, not an
installed Start-menu launch or live provider session.

**SQLite ACL passed:** owner used SQLite; another standard user read the control
and was denied DB, WAL and SHM. ACL is an additional script check, not included in
the 125 Rust-case count. Earlier compile, fixture-identity and recovery-phase failures
are resolved by this run; their evidence remains below.

## Verified installer and receipts

Unsigned **0.5.29-test.32**, never executed or installed by this worker.
Artifact metadata, native `source.txt`, installer `build-receipt.txt`, and run head
agree on source/run identity. Archive filenames were inspected before extraction;
ZIP bytes/digests match GitHub API metadata. Extracted installer name/version/hash
match the build receipt.

- [Installer artifact 14290649962](https://github.com/Shipitrealgood/buzz/actions/runs/43997403503/artifacts/14290649962)
- [Native receipt artifact 14291477163](https://github.com/Shipitrealgood/buzz/actions/runs/43997403503/artifacts/14291477163)

| File | Bytes | SHA-256 |
| --- | ---: | --- |
| `Buzz_0.5.29-test.32_x64-setup.exe` | 56497079 | `720dd24e821b65d6eb11bcfe798f5cc0af8288df538893dea4c2c7fe1c7c0bdc` |
| `installer.zip` | 56495447 | `1c9a9c4c88d81dddb8b528b525a553c8e1933bb9b24ce73e4901033e18b68f10` |
| `native-receipts.zip` | 6102 | `0f60d1b112bc956b6b9967a606b9f5c84c90c7984cfb9ecd5fb7b71c2382286d` |

Local installer:
`/home/bryce/services/buzz-agents/releases/2722-hosted-edit-windows/bbf105b5b2a8c213e1aabab5273c40547298b18e/Buzz_0.5.29-test.32_x64-setup.exe`.
The same directory contains both archives, extracted native receipts, archive
listings, run/artifact metadata, installer verification JSON, deduplicated passed
case list and sanitized build/native/monitor logs. Prior evidence is preserved.
Native log SHA-256: `845f1f9af534bc285dbc5a61cc7afbbdc62fa2818ec382800d975b699fdcd222`.
Raw logs and binaries remain outside git.

## Verification limits and handoff

Measured commands: `gh run view`, attached `gh run watch --exit-status` (terminal
exit 0), artifact/job-log API reads, Python ZIP/source/digest assertions and exact
case-name counting. Receipt local links and `git diff --check` passed. All CI,
monitor and download processes are terminal; no mutation is held. This document is
the sole checkout change; production remains at the accepted source above.

Not run here: installed Windows normal/Start-menu launch, human real Edit/Stop/Start,
live provider/chat behavior, native Linux/macOS checks, clippy or `just ci`. No
installation, live host access, working-agent action, service restart, taskctl
mutation, PR, tag, release or updater publication occurred. Source review is the
independent reviewer's gate; this receipt supplies platform execution evidence.

Next gate: conductor hands the verified canary to the human for installed normal
launch and real same-agent Edit/Stop/Start acceptance. Native fixture success does
not substitute for those human checks. No further source repair is indicated by
this run. No open platform failure or spec deviation remains within this assignment.

Signed: `codecodex` — #2722 Windows CI evidence worker, 2026-11-17.

---

## Historical run 43993874579 — recovery phase failure

**Windows gate failed; installer is diagnostic only.** On the reviewed fixture
correction, the saved-model/prompt assertions were traversed, then the native
create/recovery case failed on a later recovery phase transition. This supersedes
prior platform status; the earlier failed-run receipts remain below.

Recorded 2026-11-17 by `codecodex`, assignment `/root/hosted_edit_windows_ci`.
Requested Astra/high; actual served model/reasoning unknown. Clean intake and tested
source: `8b415dd7d19e5213f7b7d77f305277fadcbb066e`, same worktree/branch. Read the
[round-4 accepted fixture correction](../audits/post-audit/2722-hosted-management-followthrough.md).
Conductor performed the authorized fast-forward and single dispatch; this worker
only monitored, collected evidence and wrote this document. No source/test/workflow
changes or rerun occurred. Receipt HEAD differs from the tested source SHA.

## Terminal run and diagnostic

[Run 43993874579](https://github.com/Shipitrealgood/buzz/actions/runs/43993874579),
number 31, exact source above on `build/paired-hosts-canary`: **completed / failure**.

- [Build job 133741004785](https://github.com/Shipitrealgood/buzz/actions/runs/43993874579/job/133741004785):
  **success**, 21m2s, including Desktop compilation, NSIS packaging and upload.
- [Native job 133741005711](https://github.com/Shipitrealgood/buzz/actions/runs/43993874579/job/133741005711):
  **failure**, 17m17s; exact hosted case failed, then ACL and receipt upload passed.

Measured panic at
[agents_hosted_tests.rs](../../desktop/src-tauri/src/commands/agents_hosted_tests.rs):550:

```text
commands::agents::hosted::tests::hosted_native_create_recovery_and_session_fence
called `Result::unwrap()` on an `Err` value: "nonmonotonic intent phase"
```

This is `context.recover(...).await.unwrap()` after the fixture inserts a retained
phase-0 intent and saves later desired values. The wrapper at line 40 repeats the
child failure; it is one distinct failed case. The preceding saved-model/prompt
assertions completed (inferred from source order and the later panic), but the
whole case did not pass. Full evidence: `native-job.log`, lines 2956–2986.
Neither job was cancelled; no repair or rerun was attempted.

## Measured tests and limits

Native archive/source metadata verified exact tested SHA and run identity.
**122 distinct Rust cases passed; one distinct case failed.** Passing suites:
owner 17, signed deployment 13, shared harness 9, Nostr conversion 73,
relay directory 7, paired-host commands 3. Counted exact names from receipt logs;
parent/child duplicates are excluded. Passing suites report zero failed/ignored.

The create/recovery fixture explicitly removes `BUZZ_NATIVE_HOST_DEPLOYMENT_V1`;
it ran and failed, so **no env-absent native pass** is claimed. Exact cleanup and
hosted Grok metadata cases were unrun after the loop stopped on its first failure.
**SQLite ACL passed:** owner used SQLite; another standard user read the control
and was denied DB, WAL and SHM. No installed launch, real Edit/Stop/Start, provider
or live-agent acceptance is claimed. Native Linux/macOS checks, clippy and `just ci`
remain unrun. No installation, live access, service/agent action or taskctl mutation.

## Verified artifacts and local evidence

Unsigned **0.5.29-test.31** installer was downloaded, never executed. Archive names
were inspected before extraction; ZIP size/hash matched API metadata. Installer
source/version/name/hash matched `build-receipt.txt` and run head.

- [Installer artifact 14289872104](https://github.com/Shipitrealgood/buzz/actions/runs/43993874579/artifacts/14289872104)
- [Native receipt artifact 14290261362](https://github.com/Shipitrealgood/buzz/actions/runs/43993874579/artifacts/14290261362)

| File | Bytes | SHA-256 |
| --- | ---: | --- |
| `Buzz_0.5.29-test.31_x64-setup.exe` | 56495647 | `dfb111ca5c0e628178403700f4f5644263b00930805d93b3679b5d94aadb0128` |
| `installer.zip` | 56494006 | `98d83ffb03ea9033f9d7c1b86565fbef7134866501004807aff11f223073de6d` |
| `native-receipts.zip` | 5583 | `86920230e8a819f85568a0d0ebbe0b0ea3c5ae77ac7b59b75b033b772b220e2e` |

Local installer/evidence directory:
`/home/bryce/services/buzz-agents/releases/2722-hosted-edit-windows/8b415dd7d19e5213f7b7d77f305277fadcbb066e/`.
Contains both archives, extracted receipts, installer verification JSON, run/artifact
metadata, archive listings, deduplicated case lists and sanitized build/native logs.
Native log SHA-256: `ebbf51d3e462d75f0b499784e2ab9eb6baceb82c44d997fefab9457596ecc33e`.
Raw logs remain outside git; historical artifacts are untouched.

Verification: `gh run view`, attached `gh run watch --exit-status` (terminal exit 1),
artifact/job-log API reads, Python ZIP/digest/source assertions and exact-name counts.
Receipt local links and `git diff --check` passed. All monitor/download processes
are terminal; only this receipt changed in the source worktree.

## Handoff

Third unsuccessful Windows attempt: conductor routed a full review of the remaining
fixture/recovery sequence to the original executor rather than another unchecked
single-assertion patch. Preserve durable phase monotonicity, exact retained bytes,
saved main-record values and linked-definition identity. Determine whether the
fixture or production transition violates the intended contract before correcting
it; this receipt makes no causal conclusion beyond the measured error. The same
independent reviewer and a green corrected Windows run remain next gates, followed
by human installed normal-launch and Edit/Stop/Start acceptance. No ready canary is
claimed from this failed run.

Signed: `codecodex` — #2722 Windows CI evidence worker, 2026-11-17.

---

## Historical run 43988951035 — saved-model fixture failure

**Windows gate failed: native saved-model assertion.** The corrected Desktop
compiles and packages successfully, but its hosted native create/recovery case fails.
The installer below is a diagnostic artifact, not an accepted canary. This section
supersedes the platform status of the historical run preserved below.

Recorded 2026-11-17 by `codecodex`, assignment `/root/hosted_edit_windows_ci`.
Requested Astra/high; actual served model/reasoning remains unknown. Intake clean at
`b47be43097cf1f47c2be7918cb8c5cbc9cce192e` in the same worktree and branch. Read the
[round-3 independent acceptance](../audits/post-audit/2722-hosted-management-followthrough.md).
The conductor published and dispatched the corrected source under the existing user
authorization. This worker made no source/test/workflow edits, publication or rerun.
This receipt commit is documentation only and differs from the tested source SHA.

## Corrected run and concrete failure

[Run 43988951035](https://github.com/Shipitrealgood/buzz/actions/runs/43988951035),
number 30, `build/paired-hosts-canary`, independently verified exact head
`b47be43097cf1f47c2be7918cb8c5cbc9cce192e`: **completed / failure**.

- [Build job 133724287848](https://github.com/Shipitrealgood/buzz/actions/runs/43988951035/job/133724287848):
  **success**, including sidecars, Desktop compilation, NSIS packaging and upload.
- [Native job 133724288097](https://github.com/Shipitrealgood/buzz/actions/runs/43988951035/job/133724288097):
  **failure**, exact hosted create/recovery test; ACL and receipt upload then passed.
  Both jobs finished naturally; no cancellation was requested for this run.

Measured failure at
[agents_hosted_tests.rs](../../desktop/src-tauri/src/commands/agents_hosted_tests.rs):471:

```text
commands::agents::hosted::tests::hosted_native_create_recovery_and_session_fence
assertion `left == right` failed
  left: Selected { id: "initial-host-model" }
 right: Selected { id: "saved/model:2" }
```

The candidate configuration's model differs from the fixture's expected saved model.
The wrapper at line 40 reports its child failure; it is the same failing Rust case,
not a second failure. The owner of the correction must investigate production and
fixture precedence before deciding which is wrong. No cause beyond the measured
assertion is claimed here. Full diagnostic: `native-job.log`, lines 2956–2988.

## Actual native verification

Downloaded receipts contain **122 distinct passing Rust cases and one distinct
failing case**: owner 17, signed deployment 13, shared harness facts 9, Nostr
conversion 73, relay directory 7, paired-host commands 3. Counted exact fully
qualified names from receipt files, deduplicating parent/child and echoed rows;
all six passing suites report zero failed/ignored.

The exact create/recovery case ran with `BUZZ_NATIVE_HOST_DEPLOYMENT_V1` explicitly
removed by its fixture subprocess, but **failed** as above: no env-absent native
pass is claimed. Exact cleanup/session-fence and hosted Grok metadata cases were
**not run**, because the workflow loop exited after the first failure.

**Measured ACL pass:** the owner used SQLite; another standard user read the control
and was denied the DB, WAL and SHM. Verified from `journal-acl.log` and job outcome.
This isolated runner check does not prove installed Start-menu launch or live
provider behavior. Human installed normal launch and real Edit/Stop/Start remain
pending. Native Linux/macOS checks, clippy and `just ci` were not run.

## Verified diagnostic artifacts

Artifact metadata, native `source.txt`, installer `build-receipt.txt` and run head
agree on source `b47be43097cf1f47c2be7918cb8c5cbc9cce192e` and run 43988951035.
Both archives were inspected for filenames before extraction. ZIP sizes/digests
match the GitHub API; the extracted installer hash matches its build receipt.
Installer version: **0.5.29-test.30**, unsigned; never executed or installed.

- [Installer artifact 14287476767](https://github.com/Shipitrealgood/buzz/actions/runs/43988951035/artifacts/14287476767)
- [Native receipts artifact 14288131427](https://github.com/Shipitrealgood/buzz/actions/runs/43988951035/artifacts/14288131427)

| File | Bytes | SHA-256 |
| --- | ---: | --- |
| `Buzz_0.5.29-test.30_x64-setup.exe` | 56506597 | `828fd4efec911798a09241b8d09b44fa4c3fc5b9650320fd3729c58ef88fa0f1` |
| `installer.zip` | 56504963 | `472f067ae4b6d4963a5cedefe6f2a685684083d051acd4c686d0832f958ee76d` |
| `native-receipts.zip` | 5607 | `665a62c534b2dcb97a5d169846f830d2ffa59cf3e49fddf5fbddf52279ecd2ce` |

Local installer and evidence directory:
`/home/bryce/services/buzz-agents/releases/2722-hosted-edit-windows/b47be43097cf1f47c2be7918cb8c5cbc9cce192e/`.
Alongside the installer are both ZIPs, extracted native receipts, archive listings,
initial/terminal run and artifact JSON, installer verification JSON, deduplicated
passed/failed case lists, attached monitor output and sanitized job logs. Raw logs
remain outside git. `native-job.log` SHA-256:
`c5a717a79450bb967f9217d5cc22e057d41cf4aa00433439515fb85592b562a6`.

## Handoff

Verification used `gh run view`, attached `gh run watch --exit-status` (terminal
exit 1), artifact/job-log API reads, Python ZIP inspection/SHA-256/source assertions
and exact test-name counting. Receipt local links and `git diff --check` passed.
All monitor/download processes are terminal; no mutation is held. Only this receipt
is changed in the source checkout; prior run artifacts and receipt history remain.

Return to the conductor and original executor for the saved-model mismatch, then
the same independent reviewer. Preserve model precedence, saved configuration and
pending exact-byte recovery invariants; do not weaken the assertion without proving
the intended contract. Do not widen into unrelated workflow warnings or tooling.
No live access, installation, service/agent action, taskctl mutation or extra
publication was performed. A green corrected Windows run and human acceptance are
still required.

Signed: `codecodex` — #2722 Windows CI evidence worker, 2026-11-17.

---

## Historical run 43986614016 — compile failure

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
