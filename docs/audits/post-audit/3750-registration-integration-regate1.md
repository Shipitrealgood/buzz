# #3750 — smoke repair re-gate and Linux service evidence

2026-11-07. **M-1 is resolved; current findings C=0 / H=0 / M=0 / L=0. Source/test review passes and the actual Linux service hold is satisfied. Full #3750 acceptance remains open for complete Windows platform/artifact evidence and the native Windows human journey.** The footer remains `rework`/`partial` because required platform/human obligations remain, not because another source repair is required.

Reviewed clean head `1054d035c8194bd56dffe62678c27a7600931cf7`, repair range `ccf89c9cabcac8370f94cc9c373e44a06bc95a3c..1054d035c8194bd56dffe62678c27a7600931cf7`, test repair `d97823a753b4f14a1634ba2a136ab9e274babd86`, in `/home/bryce/services/buzz-wt-3720-host-registration`. Same assignment, repair round **1**. Requested reviewer configuration: **gpt-6-astra/high**; independently available runtime model/reasoning identity remains **unknown**.

This continuation is limited to the prior [M-1 finding](3750-registration-integration-smoke-addendum.md), defects introduced by its repair, supplied Linux platform receipts, and the conductor-directed operator wording clarification. The [initial review](3750-registration-integration.md), [executor repair handoff](../../briefs/3750-registration-smoke-repair.md) and prior evidence remain preserved. No broad source re-audit, mutation campaign, implementation edits, board changes, push or live access occurred.

## M-1 — resolved (read and independently measured)

The existing `desktop/tests/e2e/paired-hosts.spec.ts` remains registered in the unchanged smoke project (`desktop/playwright.config.ts:25`), without skips. It now binds the approved registration-only UI rather than removed provisioning/freshness behavior:

- At `:169`, two hosts have the same label. Refresh and revoke assert the exact second host/generation plus active owner/relay arguments; cancel emits no revoke IPC, and the first row remains Registered through the second row's terminal transition.
- At `:222`, a supported empty list exposes Add Host. Cancel clears the secret input without IPC; an expired synthetic invitation clears visibly and permits a subsequent successful retry; the actual component renders pending before fixture acknowledgement and then Registered.
- At `:259` and `:274`, unsupported remains explanatory, while unavailable and thrown-error results withhold Add Host and the empty-supported message until a successful Retry.
- At `:302`, the non-Windows surface mounts no registration panel and emits no paired-host IPC.
- The fixture at `:76` records every paired-host call and checks owner/relay; only the four registration commands have successful handlers. `registrationOnly` at `:136` checks absence of provisioning/history/freshness controls, seeded runtime display and forbidden IPC. Thus an accidentally invoked removed command cannot pass through a permissive mock unnoticed.

**Independent run: 6 passed, 0 failed, 27.5 seconds**, no retries, against the retained E2E app build. [Reviewer smoke receipt](../../verification/3750/regate1/smoke.txt). The executor's prior old-case RED points directly to the removed Check host selector; it is retained as executor-measured evidence, not a newly repeated reviewer run. This is production-frontend/browser-mock coverage, not a native Windows journey or native authorization proof.

The repair is confined to this test and documentation. No introduced defect or additional required repair was found. Minimalism/elegance/correctness of this repair: **9/10 each**. Shared mock-helper diagnostics from the supplemental test-inclusive TypeScript check remain baseline-blocked, as recorded in the [executor receipt](../../verification/3750/smoke-repair/supplemental-types.txt); no repair to those helpers is requested or strict green claimed. Normal app build/TypeScript and Biome evidence is retained/read; those checks were not repeated without a changed input.

## Production and platform continuity

**Independently measured:** all **6,580** included Git tree entries match the hosted CI source `6caf7c250f526b8eb4f9630107b632525e7573ef`, including file modes, object IDs, production source, dependency files, workflows and build configuration. Exclusions are only `docs/**` and exactly `desktop/tests/e2e/paired-hosts.spec.ts`. Canonical inventory SHA256: `7f242eee903fba0f88f53dd3366fc30c7d9196e98a44d6f7b8882a5ae5b3ade4`. [Continuity receipt](../../verification/3750/regate1/production-continuity.json).

Under the conductor-approved continuity rule, the test/docs repair does not require new product binaries or repetition of unchanged registration/library suites. Existing product evidence at that CI pin carries forward with its original scope and limitations.

**Linux actual service gate accepted (hosted measurement, independently read and hash-checked):** [workflow 42325003051](https://github.com/Shipitrealgood/buzz/actions/runs/42325003051), job `128258132756`, completed successfully at the exact CI pin. The [copied runtime receipt](../../verification/3750/regate1/linux-registration-rehearsal.txt) shows registration through the isolated TLS relay, actual service restart and owner-journal recovery, second-controller exclusion, revoke and revoked restart, nonroot MainPID, and three actual ExecStartPre namespace checks denying credential/home/helper canaries while permitting state writes. The pinned staging script's outside-unit positive controls ran before startup; its failure would have failed the checked subprocess/job. Cleanup confirms fixture processes, account and credential/state canaries removed.

Artifact `13555041662` archive SHA256 independently verified: `35f715b192b9f5836ab83f2fa89fd7463a9b6c5b2ab24d1780ae5336485a8386`. All five internal manifest hashes were independently recomputed, the source receipt matches the CI pin, and the shipped unit is byte-identical to its reviewed Git object. [Artifact check receipt](../../verification/3750/regate1/linux-platform.json), [workflow receipt](../../verification/3750/regate1/linux-workflow.json). These are debug rehearsal artifacts, not a production deployment image. This closes the exact disposable-service isolation obligation; it makes no per-port network-isolation or #3751 installation claim.

**Windows evidence remains pending review:** the conductor reports native job `128258171608` in run `42325013869` succeeded at the same CI pin, with installer build still running at dispatch. Complete logs/artifacts have not yet been supplied to this reviewer, so that status is attributed rather than independently certified here. Review the complete native counts/ACL results and installer/source/hash receipt when available. The actual native Windows UI → isolated relay/Linux hostd journey and human confirmation remain **NOT RUN** regardless of CI completion.

## Conductor-directed operator clarification

The conductor explicitly ratified and authorized the narrow edits to [windows-journey.md](../../verification/3750/windows-journey.md) and [wsl-fixture-handoff.md](../../verification/3750/wsl-fixture-handoff.md). An existing compatible Ubuntu 24.04 WSL session with dependencies already present is sufficient. The approved acceptance requires disposable fixture resources, not a newly installed distro; actual systemd isolation was separately established in the disposable hosted VM.

The instructions retain fresh loopback-only database/Redis resources, private synthetic state/keys and a dedicated unprivileged fixture identity, no reuse of existing services/data/provider homes, no global package/config changes, and exact owned-resource cleanup. Missing dependencies or loopback reachability return as concrete prerequisites. Cleanup preserves an existing distro. The Windows shared-profile and journal-loss safety boundary is unchanged. This is a conductor-directed documentation clarification, not a product repair or expanded fixture framework.

## Routing and cleanup

No source repair remains. Continue the same #3750 acceptance with the complete Windows receipts and human native journey; return them to this reviewer as continuation. Strict lint/full CI limitations remain as previously disclosed. Do not treat browser mocks or hosted service success as the native journey, and do not mark #3750 complete or authorize #3751 rollout.

[Measured cleanup](../../verification/3750/regate1/cleanup.json): the scoped Playwright command completed; no task browser/server processes or port 4173 listener remain. No relay, Docker fixture, identity secret or host account was created during this re-gate. No temporary product/test mutation occurred. Only the authorized report/receipts and two operator documents were written. Retain the worktree, app bundle and ignored caches. Exclusive write/test ownership is relinquished after the signed-off report commit.

quality: model=unknown effort=unknown stage=post-audit verdict=rework verify=partial commits=ccf89c9cabcac8370f94cc9c373e44a06bc95a3c..1054d035c8194bd56dffe62678c27a7600931cf7 findings=C=0/H=0/M=0/L=0 rework_rounds=1
