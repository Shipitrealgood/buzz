# #3750 M-1 smoke repair receipt

Measured 2026-11-07 in `/home/bryce/services/buzz-wt-3720-host-registration`.
Clean start: `ccf89c9cabcac8370f94cc9c373e44a06bc95a3c`.
Signed-off test repair: `d97823a753b4f14a1634ba2a136ab9e274babd86`.
Requested executor configuration: gpt-6-astra/high; runtime identity independently unknown.

| Check | Result and receipt |
| --- | --- |
| Existing E2E app build, including normal app TypeScript check | Passed; [build.txt](build.txt). Existing bundle-size warning retained. |
| Discriminating unchanged old case | Failed at removed `Check host`, as expected; [red.txt](red.txt). |
| Complete revised existing paired-host smoke spec | 6 passed, 38.9 seconds; [green.txt](green.txt). Earlier complete revised run also passed 6/6; final receipt supersedes it. |
| Scoped Biome check | Passed, no fixes; [format.txt](format.txt). |
| Supplemental test-inclusive TypeScript check | Baseline-blocked: two unchanged shared `tests/helpers/bridge.ts` diagnostics; [supplemental-types.txt](supplemental-types.txt). Baseline/current output compared byte-for-byte. No shared-helper cleanup. |
| CI production-input continuity | Exact 6,580-entry inventory match against `6caf7c250`; [production-continuity.json](production-continuity.json). Exclusions are all docs and exactly the changed smoke spec. Includes workflows, dependency and build configuration. |
| Local cleanup | No matching compiler/browser/server processes in worktree; no 4173 listener; temporary baseline spec removed; [cleanup.json](cleanup.json). |

Commands and outcomes are retained in individual receipts. Activate Hermit from `desktop/` with `. ../bin/activate-hermit`. Existing Playwright dependencies/browser were reused. `CI=1` makes Playwright start and stop its own localhost test server. The tests use synthetic identities and intercepted IPC; no relay, credentials, host registration, service install, or Docker fixture was used in this repair. Network-shaped fixture names are synthetic. HTTP request noise and terminal escapes are removed from receipts. Ignored `target/3750-smoke-repair` retains raw local text logs; Playwright's ignored result directory may retain generated synthetic test artifacts.

Only the scoped smoke spec changed: **product +0/-0; test +240/-246**. Documentation and receipt additions are separate. No config registration was removed, no skip was introduced, and no new framework was added. Existing jsdom journeys were not rerun because their mocks and production bindings were unchanged. Full app suites, native suites, hosted service tests and native Windows human journey were not rerun here.

Platform status is **conductor-supplied**, not newly certified by this worker: Linux hosted systemd workflow `42325003051` passed; archive/internal hashes verified under `/home/bryce/services/buzz-agents/releases/3750-6caf7c250/linux`. Windows native/discovery/ACL job `128258171608` passed in workflow `42325013869`; installer was still building at the latest supplied update. The continuity receipt supports carrying their production evidence across this test/docs delta under the approved rule. Independent platform receipt review and actual native Windows UI-to-isolated-relay/Linux-hostd human journey remain open.

Use the existing [Windows journey](../windows-journey.md) and [WSL fixture handoff](../wsl-fixture-handoff.md) for the coordinated operator handoff after reviewed artifacts are pinned. Canary installation shares the normal app identifier; no separate profile is assumed. This repair does not introduce new fixture prerequisites or request identity material.

Next: same independent Astra/high reviewer re-gates M-1 and platform evidence; conductor owns #3750 acceptance and external actions. This receipt does not mark #3750 complete.
