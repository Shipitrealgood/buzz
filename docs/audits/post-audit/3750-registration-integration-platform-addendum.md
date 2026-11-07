# #3750 — completed automated platform evidence

2026-11-07. **Automated platform holds are satisfied; no remaining source findings (C=0 / H=0 / M=0 / L=0). Full #3750 acceptance remains open for the actual native Windows application → isolated relay/Linux hostd journey and explicit human confirmation.** This is not a completed native journey or a rollout approval.

Same independent assignment and repair round **1**. Reviewed clean source `41c046e06e0fc98cbfe66eab613c3439a72b75c5`; binary/hosted-test source `6caf7c250f526b8eb4f9630107b632525e7573ef`. Requested reviewer configuration remains **gpt-6-astra/high**; independently available runtime model/reasoning identity is **unknown**. Scope is supplied completed Windows evidence and continuity only. The [prior re-gate](3750-registration-integration-regate1.md) already resolved M-1 and accepted the Linux actual-service evidence; those findings and receipts are preserved, not re-audited.

## Windows evidence accepted

[Workflow 42325013869](https://github.com/Shipitrealgood/buzz/actions/runs/42325013869) completed successfully at the exact binary pin. Both the native-test job `128258171608` and installer-build job `128258172126` succeeded. The reviewer independently read the supplied workflow metadata, complete native result blocks and actual ACL outcome; then recomputed the installer hash and inspected its PE signature. These are hosted Windows measurements independently assessed from receipts, not reviewer-executed local Windows tests.

| Gate | Supported result |
|---|---|
| Owner library on Windows | **14 passed**, zero failed/ignored. Includes invitation validation/exact retry, journal recovery and existing owner authority regressions. |
| Signed directory conversion | **73 passed**, zero failed/ignored. |
| Native relay directory | **7 passed**, zero failed/ignored. |
| Native paired-host commands | **3 passed**, zero failed/ignored. Includes the captured-session lock/workspace-generation test. These unit tests do not exercise the complete native app journey. |
| Actual SQLite ACL probe | Passed. The owner used real SQLite with WAL/SHM present; another nonadministrator standard account successfully read the public control, then received access-denied on the DB, WAL and SHM. The unchanged probe checks the effective SID, expected denial error, positive control and restored identity; its finally block stops SQLite, removes the account and fixture directory, and restores environment. The hosted step completed successfully. |
| Windows canary packaging | Unsigned non-updating NSIS installer built and uploaded; source/version/hash receipt matches the supplied binary. No local installation or human execution is claimed. |

[Sanitized native result excerpt](../../verification/3750/windows-platform-review/native-results.txt), [workflow metadata](../../verification/3750/windows-platform-review/workflow.json), [build receipt](../../verification/3750/windows-platform-review/build-receipt.txt), [independent verification](../../verification/3750/windows-platform-review/verification.json). The verification receipt hashes the original complete native log for provenance; compile/setup noise is omitted from the committed excerpt. Existing compiler warnings do not become a strict-lint pass.

Artifact ID **13555857312**:

- Installer: `Buzz_0.5.29-test.18_x64-setup.exe`.
- SHA256: `66b42dd71cd36165936254731a18c40588cdb2d8be30c48fd24e7b414d868513`.
- Source: `6caf7c250f526b8eb4f9630107b632525e7573ef`.
- Version: `0.5.29-test.18`, derived by the reviewed canary recipe. The installer remains outside the source repository.

## Continuity and remaining gate

Independently repeated the production/build Git inventory comparison from the binary pin through reviewed head `41c046e06`. All **6,580** entries match, excluding only documentation and the repaired smoke spec. Inventory SHA256 remains `7f242eee903fba0f88f53dd3366fc30c7d9196e98a44d6f7b8882a5ae5b3ade4`. Source, dependency/build configuration and workflows are unchanged; the known canary version patch remains part of the reviewed build recipe. No new product build is required for the test/docs-only continuation. [Verification receipt](../../verification/3750/windows-platform-review/verification.json).

The scoped automated evidence now covers the reviewed source/regressions, actual Linux service isolation/lifecycle, Windows native regressions/ACL, and pinned compatible Linux/relay/Windows artifacts. Full repository CI and strict-lint limitations remain as previously disclosed; they were not repaired or relabeled green.

**Remaining acceptance:** prepare the bounded local fixture, install/open the pinned canary under human control, and run the six observations in [windows-journey.md](../../verification/3750/windows-journey.md): empty-list discovery and cancel; invalid/expired invitation and successful registration; scope change during a pending operation; restart and safe journal-loss/authorized-second-desktop recovery; unavailable/retry and revoke with terminal persistence; sanitized source/artifact/observation receipt and explicit human confirmation. Record fixture cleanup. Native TLS-root behavior and app/native command integration remain part of this actual journey, not inferred from CI unit tests.

The personalized local WSL prompt was read for handoff consistency only. Its source/artifact pins, existing-compatible-WSL boundary, fresh resources, dedicated unprivileged identity, dependency prerequisite handling, public-key-only membership input, explicit installer handoff, private invitation handling and shared-profile precautions agree with the committed instructions. No personal identity or filled prompt was copied into this repository. No WSL connection or setup occurred. If safe journal-loss recovery is unavailable in the shared profile, leave that sub-gate explicitly pending; the automated receipts do not waive it.

No implementation repair or new verification framework is recommended. The conductor should proceed to the authorized local fixture/native human journey and return its receipt for this same review's completion. Keep #3750 open and #3751 live changes separately gated.

Cleanup: receipt/hash reads and documentation writes only; no executable was run, no fixture or background process was created, no source/test files changed, and no workflow/board/live action occurred. The report and sanitized receipts are the only committed additions. Worktree ownership is relinquished after the signed-off commit.

quality: model=unknown effort=unknown stage=post-audit verdict=rework verify=partial commits=6caf7c250f526b8eb4f9630107b632525e7573ef..41c046e06e0fc98cbfe66eab613c3439a72b75c5 findings=C=0/H=0/M=0/L=0 rework_rounds=1
