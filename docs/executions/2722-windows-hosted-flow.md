# #2722 Windows hosted flow execution

Status: source packet complete; independent review pending. Windows/native, full CI
and human acceptance remain NOT RUN. Executor owns source/tests exclusively in
`/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`. Clean starting revision:
`0f7927f24bb79f9a3592dc80a5dbe56ed0f13717`.

Requested model/reasoning: Astra/high. Runtime identity is not exposed by the
harness. Scope and acceptance: [ratified contract](../specs/2722-windows-hosted-flow.md)
and [execution handoff](../briefs/2722-windows-hosted-flow.md).

Native bridge/owner intents and tests first; real shared form/continuation and
tests second. Default-off remains enforced natively. No live, board, relay,
hostd, schema, installation, push, merge or PR actions are authorized.

Historical initial checkpoint (superseded by the final matrix below): all
verification gates were NOT RUN. Windows/native,
full CI and human gates require separate evidence; browser fixtures are source
workflow evidence only.

## Scope checkpoint (conductor ruling, #2722 comment 11384)

The original +900–1,500 / −100–250 production estimate was low. After the
first formatted native/owner packet measured +1,041 / −165 (net +876), the
conductor approved a revised provisional **+1,700–2,500 / −200–400** production
range for the same F1–F4 contract. This is scope accounting, not a quota.
No store/protocol/hostd/relay/schema expansion was introduced. Approximately
70 metadata-constructor lines were factored/shared; about 80 existing recovery
lines were rewritten (inferred allocation, not a measured net saving).

The two production siblings are `crates/buzz-paired-owner/src/deployment/continuation.rs`
and `desktop/src-tauri/src/commands/agents_hosted_commands.rs`. They contain
existing-journal continuation/current-operation validation and sanitized native
DTO/command projection. The native list was revised to read validated local
intents without network fan-out: unavailable hosts do not hide other setups.
Explicit actions alone obtain current permission/evidence. A Windows-only
native test sibling uses the existing confined-child/main-create fixture pattern.

Measured owner gate: `cargo test -p buzz-paired-owner deployment -- --test-threads=1`
passed **2 tests**, including real Device and Management with an isolated loopback
WebSocket peer. First run failed at sandbox loopback bind; approved escalation
allowed the fixture. A fixture reply incorrectly used a 15-second lifetime;
production correctly rejected it against the existing 10-second bound. Corrected
the fixture to `wire::PERMIT_SECONDS`, then both tests passed. After atomic
phase/result transaction changes both passed again (3.58 seconds test runtime).

Native checks are NOT RUN: installed MinGW compiler `/usr/bin/x86_64-w64-mingw32-gcc`
exists but the Rust 1.98.1 Windows GNU target library directory does not. No target
or dependency installation was attempted. Prior Linux Tauri GLib/GObject blocker
remains; Linux would not compile Windows-gated commands anyway. Windows-only
authored tests are not native acceptance evidence until run on Windows.

First-group final available checks: owner deployment suite **2 passed, 0 failed**
(3.88 seconds), now including same-token concurrent retention and missing-model /
stale / revoked / unavailable negatives. A test initially attempted `Command::clone`
(the secret-bearing command deliberately is not Clone); corrected it to restore the
candidate through the production protected-command function. Scoped Rustfmt,
Tauri formatting and `git diff --check`: PASS. Differential desktop file-size
check with `CHECK_FILE_SIZES_BASE=0f7927f24bb79f9a3592dc80a5dbe56ed0f13717`:
PASS after approved escalation for sandbox-denied Node→read-only-git subprocess.
These checks do not compile or execute Windows-only source. Historical first-group
status: V2/V3 coverage was incomplete at that point. The final matrix below
supersedes V2; native V3 execution remains NOT RUN.


## Native follow-on verification checkpoint

First source commit: `5b0404522` (native bridge/metadata/existing-intent continuation).
The expanded existing owner journey now passes **2/2** (4.68 seconds): real
WebSocket EVENT replay after denied publication and reopen; exact signed bytes;
expired unadmitted replacement; expired admitted receipt reconciliation without
resend; no revision-zero reset after prior admission; expired observations;
post-await signer change; bounded 1,024-row capacity refusal; and two simultaneous
different identity candidates for one new operation with exactly one journal
winner at phase zero. This remains a transport-boundary fixture, not a live relay.
The additional expired-admitted guard checks retained phase/result as well as
initial revision. Native setup projection also withholds unit/observation on
historical placement or stale current runtime/catalog mapping.

Windows native tests now author interrupted follow-on model-save recovery using
an exact signed/encrypted retained fixture row, actual main record recovery and
same key/auth-tag assertions, including refusal to recover the older root.
Execution of this native case remains **NOT RUN**. It does not independently
exercise the Windows top-level transport command or establish native acceptance.


Final native expiry correction: the previous transaction's peer check blocked
repeat replacement after successive expired/unadmitted initial attempts. Current
continuation explicitly reads each expired same-revision peer through the existing
operation query and passes only verified-absent IDs to the transaction. Local
phase/expiry/no-receipt alone never proves non-admission. The transaction still
rejects any new/unverified peer. The lost-admission-reply negative stores the
admitted peer receipt and refuses replacement. Latest native lookup can select an
expired/verified-absent candidate for explicit refresh, but rejects known/historical
admission conflicts. Owner gate **2/2 PASS**, 5.24 seconds. Each existing query has
a five-second deadline; enumeration is capped at 1,024 retained intents. There is
no aggregate action deadline or automatic retry: pathological same-revision peer
sets can require many sequential reads, and an unavailable read fails the action.


## Frozen source and final available evidence

Functional source pin: `10e9ecd21e8f993ee0e9996b3ccb295a809b5466`.
Four signed-off source commits above the clean dispatch baseline:
`5b0404522` native bridge/intent contract; `758d0b75d` recovery cases;
`747f55d7a` verified expired-peer replacement; `10e9ecd21` shared form/setup UI.
The final evidence commit adds this receipt, review brief, synthetic screenshot,
and public-API doc comments only. Source review range begins at
`0f7927f24bb79f9a3592dc80a5dbe56ed0f13717`.

All commands used `HERMIT_SELF_UPDATE=false` and the activated repository Hermit.
The shell sandbox cannot bind loopback or reliably run Node child processes;
those isolated checks used approved escalation. No dependency was installed.

| Gate | Exact command / result | Boundary |
|---|---|---|
| V1 / affected JS | From desktop: `node --import ./test-loader.mjs --experimental-strip-types --test src/features/agents/lib/instanceInputForDefinition.test.mjs src/features/agents/ui/whereToRunIntent.test.mjs src/features/agents/ui/usePersonaModelDiscovery.test.mjs` — **65 passed**, 0 failed | Includes host readiness, exact scope/expiry/revision/model table and ordinary payload regressions. Native pure Grok metadata test authored, NOT RUN. |
| V2 | `cargo test -p buzz-paired-owner deployment -- --test-threads=1` — **2 passed**, 0 failed, 5.24s | Real Device/Management, disposable SQLite and signed/encrypted loopback peer. Test names: `protected_main_snapshot_same_key_expiry_tamper_and_terminal_receipt`, `deployment_current_revision_refresh_launch_and_exact_retry`. |
| V3 | `hosted_native_create_recovery_and_session_fence` and `hosted_grok_metadata_uses_static_preset_without_local_binary` authored | **NOT RUN** on Windows. No supported Windows target libraries/runner. Main-phase fixture is not full IPC/native service evidence. Native generation fencing across real awaits still requires Windows execution; owner signer-after-await negative did run. |
| V4 / V5 destination UI | From desktop: `node --import ./test-jsdom-setup.mjs --import ./test-loader.mjs --experimental-strip-types --test-force-exit --test src/features/agents/ui/WhereToRunSection.jsdom-test.mjs` — **3 passed**, 0 failed | Existing real selector tests include delayed old owner/community query, unavailable/unsupported/empty states. |
| Shared UI build | `pnpm --dir desktop typecheck`; final `pnpm --dir desktop build:e2e` — **PASS** | TypeScript + fixture-mode Vite build; existing chunk-size warnings. Not a Windows native build. |
| V4 / V5 browser batch | `pnpm --dir desktop test:e2e:smoke -- hosted-agent-setup.spec.ts paired-hosts.spec.ts onboarding-agent-defaults.spec.ts where-to-run-config.spec.ts` — 40 passed, 1 contextual fixture failure, subsequently corrected | 26 onboarding, 8 registration, 4 provider-form cases passed; 2 hosted cases passed. The failed profile step used an incomplete synthetic raw record. |
| Contextual/local regression follow-up | `pnpm --dir desktop test:e2e:smoke -- hosted-agent-setup.spec.ts welcome-agent-modal-screenshots.spec.ts --grep 'contextual hosted|channel-targeted creation'` — 8 passed, 1 contextual reachability failure, subsequently corrected | Package-script argument forwarding did not apply grep: 6 existing welcome/local-create cases and 2 hosted cases passed. No claim of a green aggregate invocation. |
| Final contextual correction | `pnpm --dir desktop exec playwright test --project=smoke --grep 'contextual hosted' hosted-agent-setup.spec.ts` — **1 passed**, 0 failed, 9.2s | Corrected complete fixture plus actual owned-agent Runtime-tab summary. Across the finite files, **47 distinct browser cases** have passing evidence; unchanged cases were not rerun after the narrow profile render. |
| Formatting/size | Scoped `biome check` — 22 files, no fixes; `rustfmt --edition 2021 --check crates/buzz-paired-owner/src/deployment.rs`; `cargo fmt --manifest-path desktop/src-tauri/Cargo.toml -- --check`; `CHECK_FILE_SIZES_BASE=0f7927f24bb79f9a3592dc80a5dbe56ed0f13717 node desktop/scripts/check-file-sizes.mjs`; `git diff --check` — **PASS** | No ceiling increase. The already-oversized instance dialog now passes one existing agent object, preserving its line count. |

The three hosted browser journeys use actual AgentDialog/AgentDefinitionDialog,
production submit handlers and shared PersonaModelField, with IPC as the boundary.
They cover one same-tick submission/definition/token; local Grok absent; two host
installed IDs sharing canonical Grok metadata; retained close/reopen; native login
and model availability as independent states; explicit refresh; exact opaque model
selection; current model disappearance and expiry; truthful offline-policy failure;
lost preparation response; channel failure/retry on the same identity without start;
cached registered label/cloud summary; and a delayed old result after closing and
reopening, followed by revoked-read refusal. Existing registration tests submit the
actual form despite its disabled button and observe zero create effects. Existing
jsdom owner/community switching test remains green. No local discovery is asserted
within hosted intervals; reopening an ordinary local-default form can legitimately
perform its existing discovery before a retained setup is selected.

Initial browser failures were selectors against the existing harness/picker roles.
Later boundary failures were the mock's ws-only membership scope convention, an
incomplete synthetic record, and attempting a toast action through the open modal.
The test closes the retained setup before using the existing membership retry.
The profile test exposed a real reachability issue: linked definitions use Edit to
open the definition editor. The same setup summary now also renders in the owned
registered agent's existing Runtime tab; Edit semantics and management provenance
remain unchanged. No new dashboard, directory query, login broker or local-start
fallback was added. Native host unit guidance is withheld after observed login
success, and registered labels come from the existing scoped query cache.

Synthetic rendered evidence: [hosted model selection](evidence/2722-hosted-setup-models.png).
It contains only fixture identities and was captured by the existing Playwright
journey. No real account, host or credential information is present.

## Measured size and reuse

Against dispatch baseline, final production-file diff is **+2,147 / -278** across
25 files. Excluding the 25-line embedded Rust metadata test gives **+2,122 / -278
production** (net +1,844). Tests/fixtures/configuration are **+1,756 / -18** across
8 files plus that embedded test. The test estimate was substantially low: the
finite real transport/native and real-form journeys carry explicit fixtures; no
new test subsystem was built. Five production siblings, 20 existing production
files. The revised conductor production range remains the current scope guidance.

| Responsibility | Production added / removed |
|---|---:|
| Existing owner journal/continuation | 404 / 61 |
| Native create bridge, projection and canonical metadata | 736 / 146 |
| Shared agent form, handlers, setup, label-cache read and summary | 829 / 71 |
| Existing profile Runtime-tab shared render | 5 / 0 |
| Typed hosted IPC projections | 148 / 0 |

New production siblings: owner `deployment/continuation.rs`, native
`commands/agents_hosted_commands.rs`, UI `lib/prepareHostedDefinition.ts`,
`ui/HostedAgentSetup.tsx`, and `shared/api/hostedAgents.ts`. The first two preserve
existing persistence/transport; the latter three share the two ordinary submit
callers and the two continuation entry points. No additional store, protocol or
coordinator was introduced.

Move accounting: only 10 consecutive exact-line reappearances (blocks of at least
four lines) are mechanically identifiable in added/removed diff lines. Therefore
new/rewritten additions are 2,112 and removed/rewritten lines 268 under that narrow
measure. Roughly 70 existing metadata-constructor body lines now serve both local
and hosted callers in place (read-derived reuse, not a claimed measured saving).
The original 100–200 moved-line estimate should not be treated as a measured result.
Main identity/attestation/save/profile machinery, definition/configuration form,
model picker, channel membership/retry and provenance predicates remain reused.

## Release and unrun acceptance

No owned fixture, WebSocket peer or browser server remains after verification.
The executor supplies evidence, not an independent gate. A fresh reviewer receives
this same committed worktree using the [review brief](../briefs/2722-windows-hosted-flow-review.md).
Native Windows command/build execution, full CI, human acceptance, actual host
native login and deployment are **NOT RUN**. No live feature flag was changed.
Accepted destination/checkpoint-2 host/relay evidence is carried from the spec's
linked receipts; hostd/relay/core wire/schema/ACP sources are unchanged and were
not broadly rerun. Online policy (#3719), native service/Grok acceptance (#3722),
rollout, installer/adoption and general lifecycle controls remain outside this packet.
