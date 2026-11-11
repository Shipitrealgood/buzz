# #2722 hosted readiness fix — independent post-audit

2026-11-11 UTC. Reviewer `/root/hosted_readiness_review`, independent of executor.
Requested configuration: `gpt-6-astra` / high. Served model and reasoning effort
are not independently exposed; runtime identity is unknown. Rework round 0.

## Verdict and scope

**Source/UI ACCEPT; no merge blockers. C=0 / H=0 / M=0 / L=0.** This bounded
repair is merge-safe. Updated Windows artifact, native app execution and human
pilot remain conductor-owned pending work; this is not full PR readiness or a
`buzz-review-completed` attestation. No source repair or new prerequisite needed.
Minimalism 9/10, elegance 9/10, correctness 9/10 within the reviewed scope.

Workspace `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`; clean at entry. Reviewed exact range:
`706cedb40a4e8010ba1c3d27610649a0f6966817..25b636dd608467eb1a66e7b61905706767a8feb5`.
Only this report is reviewer-owned. Production delta is 13 additions / 6 removals
in `WhereToRunSection.tsx`; other changes are its existing jsdom fixture, existing
hosted browser fixture, feature guidance and execution brief. Full diff, changed
source/test files, callers and relevant unchanged boundary code were read.

Contract inputs: bounded conductor dispatch, [execution/review brief](../../executions/2722-hosted-readiness-fix.md),
[hosted-flow specification](../../specs/2722-windows-hosted-flow.md),
[accepted native catalog review](2722-hosted-catalog-session-fix.md), project and
feature AGENTS, VISION/agent/remote-agent vision, architecture overview, applicable
TESTING rules, and portable conductor contract. Post-audit skill applied.
No implementation, live state, credential, service, provider, board, push or merge
mutation. Goose configuration support and operator Claude PATH repair excluded.

## Acceptance evidence

1. **Exact registration remains necessary (read and tested).**
   [selector:73](../../../desktop/src/features/agents/ui/WhereToRunSection.tsx#L73)
   filters the current owner/community before matching the destination key;
   [key:5](../../../desktop/src/features/agents/lib/hostDestination.ts#L5)
   includes community, owner, host and generation. Supported, available and
   error-free directory plus `state === "paired"` are all required. `fresh:false`
   alone no longer rejects a valid registration. Freshness is neither rewritten
   nor used to claim Online. Current rows say Registered; retained rows after
   failed/unavailable/unsupported reads say Saved registration · unavailable.
   [jsdom:99](../../../desktop/src/features/agents/ui/WhereToRunSection.jsdom-test.mjs#L99)
   and [jsdom:187](../../../desktop/src/features/agents/ui/WhereToRunSection.jsdom-test.mjs#L187)
   test both freshness values, pending/expired/revoked/unknown states, missing rows,
   failed refetch with retained data, unavailable/unsupported replies and scope
   mismatch. The query's existing scoped cache/refresh policy is unchanged; no
   new cache lifetime or registration challenge is implied.

2. **Registration remains insufficient (read and tested).**
   [intent:65](../../../desktop/src/features/agents/ui/whereToRunIntent.ts#L65)
   still requires hostAvailable AND hostedReady.
   [dialog:223](../../../desktop/src/features/agents/ui/AgentDialog.tsx#L223)
   cancels obsolete catalog completions, checks exact returned scope, keeps the
   one-shot expiry timer, and requires a token plus current catalog.
   [catalog:137](../../../desktop/src/shared/api/hostedAgents.ts#L137)
   requires all four scope fields, current availability, supported capability,
   and strictly future expiry. The real form still checks trimmed name, selected
   available runtime and shared configuration validity
   ([form:511](../../../desktop/src/features/agents/ui/AgentDefinitionDialog.tsx#L511));
   submit rechecks readiness and requires metadata, availability and command
   ([submit:354](../../../desktop/src/features/agents/ui/AgentDialog.tsx#L354)).
   Missing runtime metadata is filtered from the hosted picker at dialog:253;
   no fallback native capability table was added.

3. **Reported case and negative browser cases (measured).**
   [fixture:137](../../../desktop/tests/e2e/hosted-agent-setup.spec.ts#L137)
   returns native-shaped `fresh:false`/zero lifetime; the real form selects
   `native/grok`, uses Fizz, and enables Prepare with catalog/token. Empty name
   disables; restoring it enables; advancing past catalog expiry disables without
   interaction or catalog refetch
   ([case:365](../../../desktop/tests/e2e/hosted-agent-setup.spec.ts#L365)).
   Missing token, rejected catalog, wrong generation, stale catalog and explicitly
   selected unavailable Grok remain blocked
   ([cases:407](../../../desktop/tests/e2e/hosted-agent-setup.spec.ts#L407)).
   The unavailable-runtime fixture truly selects the menu item at :353–362;
   the prior automatic-selection assumption was corrected, not ignored.
   Four existing continuation/cleanup journeys still pass, including latch,
   identity retention, exact installed-ID retry, lost response, channel retry,
   late closed-setup result, denied evidence and cleanup recovery.

4. **Scope/race coverage is stated at its actual layer.** The 14-case intent
   suite checks each catalog scope field and exact expiry. The selector jsdom
   suite tests a late old owner/community directory response
   ([case:137](../../../desktop/src/features/agents/ui/WhereToRunSection.jsdom-test.mjs#L137)).
   The browser tests late results after closing/reopening continuation
   ([case:686](../../../desktop/tests/e2e/hosted-agent-setup.spec.ts#L686)), not a
   delayed initial-catalog response during an owner/community switch. Initial
   catalog cancellation is unchanged and read-verified at dialog:223–242.
   Browser catalog/token values are mocked IPC outputs; these results do not
   prove cryptographic verification or native/live behavior.

5. **Native authority continuity (measured unchanged diff, read boundary).**
   `git diff --exit-code <base>..<head> -- crates desktop/src-tauri` passed.
   Native catalog and Prepare still use guarded session capture and current
   nonce-bound status query, then host-signed status opening
   ([open:165](../../../desktop/src-tauri/src/commands/agents_hosted.rs#L165),
   [transport:7](../../../crates/buzz-ws-client/src/host_deployment.rs#L7)).
   Relay reads still require current authorization and exact active registration
   ([binding:34](../../../crates/buzz-db/src/store/paired_hosts/deployment.rs#L34),
   [query:194](../../../crates/buzz-db/src/store/paired_hosts/deployment.rs#L194)).
   Prepare retains inventory validation before identity generation
   ([create:435](../../../desktop/src-tauri/src/commands/agents.rs#L435)).
   The prior accepted native evidence is credited for unchanged code, not
   described as freshly executed Windows verification.

## Independent verification ledger

All commands completed at the reviewed pin. Hermit activation was attempted and
failed because its metadata cache is read-only in this sandbox; existing pinned
Node 24.15.0 and Biome 2.4.7 binaries were used directly. No install/toolchain edit.
Commands below run from `desktop/` with that Node directory first in PATH.
Report commit subsequently activated Hermit successfully with
`HERMIT_SELF_UPDATE=false`; this did not require a test rerun.

| Command/check | Measured outcome |
| --- | --- |
| `node node_modules/typescript/bin/tsc --noEmit` | PASS, exit 0 |
| `node --import ./test-loader.mjs --experimental-strip-types --test-isolation=none --test src/features/agents/ui/whereToRunIntent.test.mjs` | 14 passed, zero failed/skipped |
| `node --import ./test-jsdom-setup.mjs --import ./test-loader.mjs --experimental-strip-types --test-force-exit --test-isolation=none --test src/features/agents/ui/WhereToRunSection.jsdom-test.mjs` | 5 passed, zero failed/skipped |
| Same imports/strip-types/force-exit, normal per-file isolation, `--test` for `WhereToRunSection.jsdom-test.mjs` and `PairedHostsSection.jsdom-test.mjs` | 2 files passed, zero failed/skipped |
| Biome `check` for selector TSX, selector jsdom fixture and hosted browser spec | PASS, 3 files; no fixes |
| `node node_modules/vite/bin/vite.js build --mode e2e` | PASS, 4.05s; existing chunk-size warning |
| `node node_modules/@playwright/test/cli.js test --project=smoke tests/e2e/hosted-agent-setup.spec.ts --workers=1 --reporter=list` | 10 passed, 41.8s, exit 0 |
| Pinned native-source zero diff and `git diff --check <base>..<head>` | PASS |

Browser's first attempt stopped before tests because sandbox denied socket
creation. Approved rerun started the local static server and Chromium; all ten
cases ran. No failed test was suppressed or retried in that successful run.

**Read, not independently rerun:** executor's failing-before receipt records
2/5 selector failures against unchanged old production, followed by 5/5 after
repair. The committed diff confirms replacement of the old `selectedHost.fresh`
predicate and final source has no held mutation. Executor's combined non-isolated
jsdom bridge collision and initial 9/10 browser fixture failures remain disclosed
in the [execution receipt](../../executions/2722-hosted-readiness-fix.md#7-verification-and-handoff).
No new mutation campaign was dispatched or performed.

**NOT RUN:** full `just ci`, Rust/native Windows suites, Windows installer build
or execution, real hosted IPC/relay/provider/login/chat/service workflow and human
acceptance. Those exclusions are explicit in this bounded dispatch. The changed
frontend was built and tested; new Windows artifact and pilot remain next gates.

## Handoff and ownership release

Recommend conductor accept the source/UI repair and proceed to the separately
authorized Windows artifact and human pilot. Keep registration freshness separate
from deployment catalog readiness; do not add challenge machinery, global cache
changes or provider support work to this repair. No new findings need routing.
Existing prior-review reliability follow-up is not newly evaluated or closed here.

All reviewer test/build invocations reached terminal exit. Playwright completed
its managed server lifecycle; no detached process or source mutation is held.
Only this report is committed with sign-off; final commit and clean-status receipt
are returned to the conductor. The same worktree is released for its next stage.

quality: model=unknown effort=unknown stage=post-audit verdict=merge-safe verify=ran commits=706cedb40a4e8010ba1c3d27610649a0f6966817..25b636dd608467eb1a66e7b61905706767a8feb5 findings=C=0/H=0/M=0/L=0 rework_rounds=0
