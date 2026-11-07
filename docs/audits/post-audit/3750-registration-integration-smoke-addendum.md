# #3750 — existing smoke regression suite continuation

2026-11-07. **Verdict: required test rework. C=0 / H=0 / M=1 / L=0.** This supersedes the initial report's no-source-rework recommendation. No production behavior defect has been established. Platform and human obligations remain separate.

Continuation of the same independent review, on clean HEAD `6caf7c250f526b8eb4f9630107b632525e7573ef`, product range `621c777b886dad8aed7eeee4b428d57c153829e5..d67ecff347bf81392adad6345e05c43634b4d9d1`. The conductor supplied new evidence of an existing, directly affected smoke suite missed in the first review. This was not a broad re-audit. Requested reviewer configuration remains gpt-6-astra/high; runtime model and reasoning identity are independently unknown. Repair rounds: 0, before executor repair.

## M-1 (Medium) — registered paired-host smoke tests still require removed v1 behavior

**Confidence: high. Evidence: read; runtime failure inferred from mandatory selectors/assertions, not claimed as measured.**

`desktop/playwright.config.ts:25` explicitly includes `**/paired-hosts.spec.ts` in the `smoke` project's testMatch. `desktop/package.json:21` invokes that project through `test:e2e:smoke`. The suite therefore remains an executable regression obligation, not an archived document.

Concrete incompatible expectations in `desktop/tests/e2e/paired-hosts.spec.ts`:

- Lines 183–205: the first journey waits for **Check host**, **Host confirmed**, **inventory cached**, **Pair host**, **Unpair**, and **Confirm unpair**. The changed production component renders **Refresh registration**, **Add Host**, **Revoke registration**, and **Confirm revoke**, and makes no freshness claim (`desktop/src/features/agents/ui/PairedHostsSection.tsx:146`). The first mandatory click cannot resolve against this UI.
- Lines 208–220: an unsupported relay must have zero **My hosts** regions. The production component now deliberately retains that region and explains unsupported registration (`PairedHostsSection.tsx:286`). This assertion contradicts accepted honest-state behavior.
- Lines 224–251 require the removed inventory/freshness confirmation UI.
- Lines 254 onward select **Host for new agent**, open **Add agent**, and drive provisioning/resume/history forms. Those forms were intentionally removed from this registration-only surface. The corresponding v1 mock branches at lines 108–165 no longer model this UI's contract.

**Impact:** running the existing paired-host smoke spec against this candidate cannot pass its intended assertions. The new 2/2 jsdom receipt does not close this existing affected regression obligation. The first report reviewed the new tests but missed this registered consumer. This is an agreed-contract/test maintenance defect under #3750's relevant touched-regression acceptance, not evidence that the removed provisioning feature should return.

**Proposed narrow fix:** update `desktop/tests/e2e/paired-hosts.spec.ts` and its local mock data/handlers to the current v2 registration contract. Preserve the useful duplicate-label identity-routing and non-Windows suppression checks. Exercise empty-supported Add Host, cancel/retry, honest unsupported/unavailable states, registration/revocation state transitions and exact selected-host command arguments. Replace obsolete freshness/provisioning journeys with assertions that online/runtime/provisioning controls and provisioning IPC remain absent. Remove dead v1-only mock branches. Keep this spec registered in the smoke project; skipping it or deleting its registration does not resolve the finding. No new test framework or broad suite expansion is required.

| Finding | Proposed owner and next action |
|---|---|
| M-1 | **fix-in-lane, rides #3750** — conductor ruled required test/mock-only repair by the existing Astra/high executor. Repair this smoke spec, run the complete affected spec and retain its outcome; rerun the existing two jsdom journeys if mock/production bindings change. Return a pinned test-only delta for the same reviewer. Do not restore removed provisioning UI. |

## Verification and platform continuity

This continuation read the complete smoke spec, its Playwright registration, package command, and the current production UI. No discriminating browser run was necessary to establish that the required controls are absent and the unsupported assertion is inverted. **No Playwright runtime pass or failure is claimed.** The original Linux/browser/library test measurements remain as recorded in [the initial report](3750-registration-integration.md) and [run index](../../verification/3750/review/run-index.json); they were not rerun.

The conductor has separately supplied a successful Linux hosted workflow at `6caf7c250` and its artifact/receipt paths under `/home/bryce/services/buzz-agents/releases/3750-6caf7c250/linux`. Those files were preserved and are not modified or newly certified by this focused continuation. Windows CI was still in progress at dispatch. Combined platform evidence review remains a later continuation.

If the repair changes only test/mock and review documentation files, existing binary/platform measurements may carry forward with an explicit exact production-input continuity check. A production behavior or build-input change would require the affected build/gates again. The conductor explicitly accepted this continuity rule for test/docs-only repairs; no build rerun is required solely because those files change.

No source/test edits, fixture processes, external mutations or board changes occurred. Only this addendum and the initial report's continuation pointer were changed. The worktree is relinquished after the signed-off report commit; the conductor owns executor routing and #3750 state.

quality: model=unknown effort=unknown stage=post-audit verdict=rework verify=partial commits=621c777b886dad8aed7eeee4b428d57c153829e5..6caf7c250f526b8eb4f9630107b632525e7573ef findings=C=0/H=0/M=1/L=0 rework_rounds=0
