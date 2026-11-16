# #2722 F2/F3 — hosted status and normal Windows launch

## Scope and source

Execution of F2/F3 only from the [approved spec](../specs/2722-hosted-management-followthrough.md).
Workspace `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`; starting clean at
`f95d075b365486a432732d603ba4511cf19fe7ff`. No unrelated changes existed or were
staged. Role: codecodex, requested Astra/high; authoritative runtime model/effort
metadata is not exposed by this harness. Final source/commit/process receipt below.

F1/P1 Edit/Save/Start belongs to a later executor. No lifecycle API, relay, hostd,
schema, provider login, isolation, root helper, PATH, live service, deployment,
agent action, taskctl, push or canary dispatch changed or ran.

## Implementation

- `desktop/src/features/agents/ui/HostedAgentSetup.tsx`: saved-agent Host status /
  Hosted agent labels; neutral unverified deployment; separate exact-key relay
  availability; saved runtime/model; only fresh observations supply readiness
  guidance. Installation/sign-in advice requires corresponding evidence in an
  actual recoverable state. A current applicable catalog must exclude the selected
  model before it is labelled unavailable. Online/Away add the shared Start block.
  Existing model expiry timer, currentHostedModels and native launch gates remain.
  Recovery, errors, conflict and cleanup stay independent of presence.
- `desktop/src-tauri/src/commands/agents_hosted_commands.rs`: invalid observations
  are still dropped, but expiry/Refresh advice is limited to recoverable setup
  states. Applied no longer receives advice for an unavailable Refresh action.
  Existing read-denied/error propagation is unchanged.
- `desktop/src-tauri/src/commands/agents_hosted.rs`: enabled() remains at the same
  boundaries and permits Windows without BUZZ_NATIVE_HOST_DEPLOYMENT_V1. Other
  platforms fail closed. Session/signer, relay capability, catalog, registration,
  ownership/build-policy and protected Windows journal checks remain unchanged.

Measured source delta: **3 production files, +91/-47, net +44**.
Tests: **2 files, +161/-58, net +103**. Required agents guidance: **+10/-2, net +8**.
Execution and review brief are additional documentation. No new state framework,
fixture harness, provider process or test matrix was introduced.

## Verification and receipts

All commands activated Hermit with `export HERMIT_SELF_UPDATE=false` and
`. ./bin/activate-hermit`. Without the exported setting a first Biome invocation
failed against the read-only Hermit metadata cache; no package/tool installation
or permission changes were made. An initial JSX replacement also affected the
loading paragraph; Biome caught it before tests and the exact local edit was fixed.

| Command / boundary | Result |
| --- | --- |
| `cd desktop && pnpm typecheck` | PASS, exit 0 (`tsc --noEmit`) |
| `cd desktop && pnpm exec biome check src/features/agents/ui/HostedAgentSetup.tsx tests/e2e/hosted-agent-setup.spec.ts` | PASS, 2 files, no fixes; rerun after final test adaptation |
| `rustfmt --check --edition 2021 --config skip_children=true desktop/src-tauri/src/commands/agents_hosted.rs desktop/src-tauri/src/commands/agents_hosted_commands.rs desktop/src-tauri/src/commands/agents_hosted_tests.rs` | PASS, exit 0 |
| `git diff --check` | PASS, exit 0 |
| `cd desktop && pnpm test:e2e:smoke hosted-agent-setup.spec.ts` | E2E TypeScript/Vite build PASS; Playwright initially blocked by sandbox loopback socket permission |
| `cd desktop && pnpm exec playwright test --project=smoke hosted-agent-setup.spec.ts` against restored E2E bundle | PASS, **17 passed (1.3m)**, exit 0 |
| One source-predicate mutation and same real-component journey | Expected FAIL, 1 test, at selected-model unavailable assertion; restored source byte-for-byte |

The browser uses existing IPC/connection/time boundaries and actual components.
The extended ordinary hosted journey covers an Online retained identity with no
observation, neutral unverified state, saved model retained, disconnected Unknown,
explicit applied check, genuine unfinished logged-out guidance, Refresh/exact
model selection, Online/Away Start block, Offline subject to native guards,
automatic expiry disabling Start without erasing the model, and failed launch /
retry. Existing denied/current-catalog miss, contextual close/reopen/late-result,
cleanup and seven Create outcome cases remain in the same suite.

The first suite exposed baseline test drift: old cases expected Prepare while
accepted source already used Add/Create and required a returned model. Initial
adaptation yielded 16/17 PASS; contextual Create then exposed model reset on an
explicit harness change. The third symptom was routed to the conductor, who
resumed this assignment with authorization to complete the existing fixture
adaptation. The class-level fix uses existing `persona-dialog-submit` and one
returned-model selection helper, including after explicit harness changes.
Missing-model evidence now returns a different nonempty catalog, matching native
successful catalog semantics. No assertions were weakened to accept any error.

The mutation replaced the corrected catalog-presence predicate with
`!catalog?.models.some((entry) => entry.id === model)` and rebuilt using
`pnpm exec vite build --mode e2e`. Command:
`pnpm exec playwright test --project=smoke hosted-agent-setup.spec.ts --grep 'ordinary hosted form separates'`.
Measured receipt: **1 failed**, expected substring **not `(unavailable)`**;
received **Availability: Online / Deployment status not checked / Selected model:
opaque/model:fixture (unavailable)**. Restored source SHA-256:
`8f22e2c59aace8f3575101b09b78fc78e59d035b3bd1e6e41b0db237d767dda7`;
`cmp` passed against the pre-mutation copy. Restored Vite E2E build passed.
The final suite uses that restored artifact. The usual chunk-size warning remains.

## Not run and evidence limits

- Native Windows tests/build and installed Start-menu launch: **NOT RUN** on Linux.
  The existing env-absent native fixture now requires token success, exact
  `invalid or unsupported host deployment contract` rejection for owner==host,
  no publication connection and no saved records. Existing create/recovery/session/
  cleanup assertions remain. Run the two exact hosted native cases via
  `.github/workflows/windows-canary.yml:293` on the changed source.
- Native applied/expired projection branch: **source-reviewed only**, not a new
  runtime claim. The existing phases fixture never calls the live Management
  setup reader; conductor explicitly accepted UI coverage plus this source review
  instead of introducing a new harness.
- `just ci`, full native Linux build, broader integration/provider matrix,
  deployment and human testing: **NOT RUN**. Known GLib/WebKit prerequisites were
  not installed or retried. Prior actual-relay/native/live d7714be evidence is
  credited only for unchanged paths, not these new Windows behavior claims.
- Existing Claude/Grok/Scout and unfinished Jeff/test1 were not accessed or acted
  on. Ordinary Edit and Save/application behavior is not claimed fixed.

## Handoff

Implementation commit: `996203e6cb2dce39420854cdd26116a2cf35b19a`
(`git commit -s`, DCO trailer verified). Documentation follows in a separate signed-off commit.
Final Playwright session 15485 terminated exit 0; mutation session 6752 terminated
exit 1 as expected; both Vite build sessions terminated exit 0. The interrupted
initial browser run left its Python server; its cwd was verified as this worktree
before stopping only its owned PIDs 3398160/3398161. Final host process scan
(excluding the checker) returned **Remaining worktree test/build processes: []**;
`ss -ltnp 'sport = :4173'` returned no listener. No held mutation remains. All
changes belong to this assignment; no unrelated baseline/final dirty state exists.
No product/spec deviation; test drift and bounded native coverage adjustment are
explicit above. Next gate is the conductor's fresh independent reviewer using the
[review brief](../briefs/2722-hosted-status-normal-launch-review.md). The subsequent
Windows gate and human fully-quit/Start-menu launch without a manual environment
flag remain necessary. Executor self-review is not an independent gate.
