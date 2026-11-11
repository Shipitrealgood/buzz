# #2722 hosted readiness fix — independent post-audit

2026-11-11 UTC. Reviewer `/root/hosted_readiness_review`, independent of executor.
Requested configuration: `gpt-6-astra` / high. Served model and reasoning effort
are not independently exposed; runtime identity is unknown. Rework round 0.

## Verdict and scope

**Source/UI, native Windows gate and test.26 installer artifact ACCEPT; no merge
blockers. C=0 / H=0 / M=0 / L=0.** This bounded repair is merge-safe. Installer
execution and remaining human pilot remain conductor-owned pending work; this is
not full PR readiness or a `buzz-review-completed` attestation. No source repair
or new prerequisite needed. The initial source/UI review is retained below; its
native/artifact NOT RUN entries are superseded by the continuation evidence.
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

## Initial independent source/UI verification ledger

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

## Windows artifact continuation — same review, no source rework

Receipt HEAD `ae335effbfb94357aa43b16fb8bbd90cd517c242`; exact built source
`78232d95173b876babcee0f51b18f7c2a17dac0d`, wrapping accepted product
`25b636dd608467eb1a66e7b61905706767a8feb5`. Read the
[Windows execution receipt](../../executions/2722-hosted-readiness-windows-canary.md)
and saved artifact/API/job records. **Measured locally:** full post-product delta
contains only this source review and the Windows receipt; explicit `git diff
--exit-code` for `desktop crates .github` passes. Source/UI acceptance is reused.
No new build, test suite, source audit, workflow dispatch or live action ran.

Evidence root:
`/home/bryce/services/buzz-agents/releases/2722-hosted-readiness-canary/78232d95173b876babcee0f51b18f7c2a17dac0d/42930493604/`.
Reviewer read `run.json`, `jobs.json`, `artifacts.json`, embedded source/build
receipts, every individual native suite log, UTF-16 ACL log and relevant full
job-log source/version/compiler/runner/build output. These are saved CI records,
not a new independent GitHub retrieval. Native outcomes are **read from CI**;
hashes, archive membership, extracted-byte equality, PE signature, distinct test
count and Git continuity are **independently measured locally** with a reviewer
script, not inferred from executor `verification.json` or `verify.py` success.

[Run 42930493604](https://github.com/Shipitrealgood/buzz/actions/runs/42930493604)
is completed/success, workflow_dispatch, run 26, **attempt 1**, exact built SHA
above. Native job `130227724092` ran 00:20:01Z–00:35:53Z; installer job
`130227724714` ran 00:20:00Z–00:32:52Z on 2026-11-11. Saved jobs and both artifact
workflow_run records agree on source/run. Full logs show Windows-target Cargo
compilation and `.exe` unit runners, plus test.26 NSIS output. Build is the
existing unsigned, updater-disabled canary. No retry is recorded for this run.

Eight native logs contain **111 distinct outer passing test names**, matching
one nonzero summary each, zero failed/ignored: owner 16, shared harness 9,
conversion 73, relay directory 7, paired-host session 3, and these three exact
cases. Count excludes full-log grep echoes and child execution.

| Exact native case | CI result independently read |
| --- | --- |
| `commands::agents::hosted::tests::hosted_native_create_recovery_and_session_fence` | 1 passed, 0 failed/ignored; 9.79s |
| `commands::agents::hosted::tests::hosted_native_cleanup_confirmation_retry_and_session_fence` | 1 passed, 0 failed/ignored; 10.45s |
| `managed_agents::discovery::presets::hosted_metadata_tests::hosted_grok_metadata_uses_static_preset_without_local_binary` | 1 passed, 0 failed/ignored |

Each matches the existing workflow's exact-case inventory
([workflow:284](../../../.github/workflows/windows-canary.yml#L284)). The UTF-16
ACL receipt says the owner used SQLite, another standard user read the control,
and that user was denied DB/WAL/SHM. The full native log contains the same
terminal result and the ACL job step succeeded. This proves the isolated Windows
fixture result, not live host filesystem acceptance.

Installer: **Buzz_0.5.29-test.26_x64-setup.exe**, **56,484,611 bytes**, PE header
checked without execution. Both archives' digest/size equal saved GitHub artifact
metadata. Every member equals its extracted bytes. Installer ZIP contains exactly
installer plus build receipt; native ZIP contains exactly source receipt, eight
suite logs and ACL log. Embedded receipts agree with source, version, target,
filename and installer hash.

| Artifact | Independently recomputed SHA-256 |
| --- | --- |
| Installer executable | `211c5e96db035e0b5fca58e627b598ddf9db30e12b50cc33f3d8d15f0a633144` |
| Installer ZIP, artifact **13816845999**, 56,482,976 bytes | `cf9a67fb714cd4bc52abd53138b9c06aaf8ae5a43d0a80f8349f8050a3630626` |
| Native ZIP, artifact **13816986961**, 5,411 bytes | `f5290973e4a846227274e36f2944c08e33253a519004c803e419cc45f71b5df2` |

Saved metadata expires installer at 2026-11-18T00:32:21Z and native at
2026-11-18T00:35:48Z; original ZIPs and extracted files remain locally retained.
The owner expiry case passed in this first attempt (16-case owner suite: 23.75s).
This does not repair or prove the prior test25 timing hypothesis; the accepted
prior reliability follow-up remains open, without a new finding or rerun here.

Remaining **NOT RUN**: installer execution, actual hosted native IPC/UI status
journey, provider/login/models/chat/service workflow, remaining human pilot,
mutation controls and full `just ci`. Conductor relayed explicit human confirmation
that Claude now appears available after a separate approved host tool-path change.
That is human harness-discovery evidence only; neither this review nor that report
establishes successful Prepare, models, provider execution or conversation.

## Handoff and ownership release

Recommend conductor accept the verified test.26 artifact and proceed to the
separately authorized Windows installation and one disposable Claude Prepare
pilot. Source/UI and bounded native/artifact acceptance stand. Keep registration freshness separate
from deployment catalog readiness; do not add challenge machinery, global cache
changes or provider support work to this repair. No new findings need routing.
Existing prior-review reliability follow-up is not newly evaluated or closed here.

All reviewer test/build invocations reached terminal exit. Playwright completed
its managed server lifecycle; no detached process or source mutation is held.
Only this report is committed with sign-off; final commit and clean-status receipt
are returned to the conductor. The same worktree is released for its next stage.

quality: model=unknown effort=unknown stage=post-audit verdict=merge-safe verify=ran commits=706cedb40a4e8010ba1c3d27610649a0f6966817..25b636dd608467eb1a66e7b61905706767a8feb5 findings=C=0/H=0/M=0/L=0 rework_rounds=0
