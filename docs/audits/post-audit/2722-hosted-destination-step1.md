# #2722 / #3720 — hosted destination slice 1 independent review

2026-11-09. **Source-only acceptance recommended and accepted by the conductor;
merge-safe attestation withheld while native/platform and human gates remain pending.**
No supported new source defects: **C=0 / H=0 / M=0 / L=0**. Rework rounds: 0.
Pending verification gates are not counted as invented code findings.

## Scope and pin

Workspace `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`, clean on entry. Reviewed range:
`a2722aa9bdac7acefc8a01376151cdab6a249305..5eec25bf1b95c8be46cc1e78e89cfc335f4a477a`.
Implementation: `df191042562aa1f0027153a64c80d17897c5cea4` and
`cd8f825f009ef31e2c5673681cf18e628844e59f`; final pin is documentation only.

Scope: inert deployment types and ordinary Add Agent registered-host destination.
Read the conductor contract, applicable AGENTS/VISION/TESTING guidance, post-audit
skill, [executor receipt](../../executions/2722-hosted-destination-step1.md),
[storage decision](../../executions/2722-hosted-destination-contract.md), and
conductor-owned `buzz-agents/docs/specs/2722-first-hosted-agent.md` /
`docs/briefs/2722-create-completion.md`. The dispatch's report-only contract carries
the handoff and proposed routing here; no additional brief or board mutation.

Requested reviewer: **gpt-6-astra / high**; actual served model/effort **unknown**
(no authoritative runtime metadata). Independent reviewer, no subagents.
No deployment, provider/service actions, installation, live requests, native login,
staging cleanup, merge/push/PR or later-slice work occurred.

## Source assessment — read evidence

Full diff and relevant callers support the scoped acceptance; no new defect found.

| Watchpoint | Assessment and source |
| --- | --- |
| Inert authority | `crates/buzz-core/src/host_deployment.rs:146` binds scope, identity, action, operation, revision, expiry and ciphertext commitment. `:273` checks inner action/new-agent identity with existing NIP-OA validation. `:115` is a status shape/freshness predicate, not signature admission. Repository search finds no relay/hostd consumer. Closed nested schemas and empty variants reject unknown fields. |
| Unconditional refusal | `desktop/src/features/agents/ui/whereToRunIntent.ts:50`, `AgentDialog.tsx:186`, `usePersonaActions.ts:226`, `lib/instanceInputForDefinition.ts:120`, and `desktop/src/shared/api/tauri.ts:770` block before relevant create/upload/IPC effects. Native `desktop/src-tauri/src/commands/agents.rs:405` rejects decoded destinations before the first store lock and key generation. Missing host metadata never maps the draft to local. |
| Scoped truthful projection | `lib/hostDirectory.ts:34` preserves existing query identity/polling; `lib/hostDestination.ts:10` and `ui/WhereToRunSection.tsx:73` also filter owner/community. Labels distinguish same-name hosts; status copy reports registration, never online/catalog readiness. Supported-empty capability remains core data, not fabricated directory evidence. |
| Existing form | `WhereToRunSection.tsx:207` embeds existing MyHosts and stops portal submit propagation. Local/provider mapping and probe cancellation remain intact; other shared builder callers retain their prior behavior. |
| Secrets/config | Command and ProtectedCreateIntent omit Debug (`host_deployment.rs:259,357`); existing PrivateIdentity Debug redacts secrets (`private_managed_agent.rs:136`). No executable/env/path override or second runtime catalog was introduced. The future adapter must retain main's config/visible-text checks; current validators claim shape validation only. |
| Storage | [Decision](../../executions/2722-hosted-destination-contract.md#exact-persistence-decision-for-slice-2-no-sql-in-step-1) is additive design only: unique community+agent placement, immutable operation bytes, protected owner ciphertext intent, original signer recovery, stable initial-operation unit ID and cleanup tombstones. Main retains key/config authority. The absent deploy bridge is an explicitly deferred slice. |
| Native risk | `desktop/src-tauri/src/managed_agents/types/requests.rs:151` adds only optional create destination; persistent BackendKind/lifecycle remain unchanged. Dependency alias exists at Cargo.toml:110; request builders deserialize JSON. The actual-command poisoned-lock regression at `commands/agents_tests.rs:886` is credible read evidence, **uncompiled/unrun here**. |

The complete 26-file range leaves migration 81, schema, registration v2 protocol,
relay handlers and hostd/helper/runtime/service implementations unchanged. The
accepted host direction extends the older external-provider vision through the
relay; no existing provider path was rewritten.

## Independent measurements

Hermit activated with `HERMIT_SELF_UPDATE=false`; the default update attempted an
unwritable cache metadata file. No toolchain installation or alteration.

| Gate | Reviewer result |
| --- | --- |
| `cargo test -p buzz-core host_deployment --lib` | **5 passed**, 300 filtered. Scope bindings, expiry/hash/revision, closed shapes, identity/action/result correlation and status cases. |
| Focused JS: whereToRunIntent, instanceInputForDefinition, shared-api hostDestination | **31 passed**, 0 failed. Used `node --import ./test-loader.mjs --experimental-strip-types --test --test-isolation=none` with the three files named in the executor receipt. |
| Destination DOM / existing registration DOM | **3 + 2 passed**, separate processes. Added `--import ./test-jsdom-setup.mjs --test-force-exit` to that Node invocation for `WhereToRunSection.jsdom-test.mjs` and `PairedHostsSection.jsdom-test.mjs`. Late/mis-scoped results, registration states and reused form exercised. |
| `pnpm build:e2e` | Passed, including TypeScript. Existing dynamic-import / chunk-size warnings. |
| `pnpm exec playwright test --project=smoke paired-hosts.spec.ts --grep 'ordinary Add Agent'` | **2 passed**, 10.2s: duplicate labels, forced parent submit, local return, empty-directory Add Host, cancel/expired invitation without agent creation (`paired-hosts.spec.ts:317,367`). Mock boundaries; not native Windows. |
| Known failing JS test | Individual `useKnownAgentPubkeys.test.mjs`: **0 passed / 1 failed**, missing CommunitiesProvider reproduced. |
| Native environment | `pkg-config --modversion glib-2.0 gobject-2.0`: both missing. Native compile/test **not run by reviewer**; executor's attempt stopped before tests. |
| Integrity | `git diff --check` passed; source stayed byte-unchanged, clean before report creation. |

Verification setup failures: initial Node commands reported file-level successes,
so final case counts above came from isolation-none runs. Combining both DOM files
in one process failed two cases because their global mock bridges overwrite each
other; separate processes preserved intended isolation and passed all five.
Initial browser launch failed on sandbox loopback binding; the approved rerun
passed. No source fixes. Ignored `desktop/dist` now contains the E2E build.

## Continuity and dispositions

[Executor evidence](../../executions/2722-hosted-destination-step1.md#measured-verification)
retains core 305, browser 12 (including existing provider regressions), production
build, typecheck, formatting, Biome and size checks. These are **executor-run**,
not reviewer-run. Source continuity holds through the documentation-only final pin.

The two named mutation controls remain executor evidence: API guard removal reached
forbidden IPC; Authority scope-guard removal failed foreign-scope validation. Both
had restored green runs. Reviewer independently reran the unmutated scoped suites
and matched both restored SHA-256 hashes exactly:

- `desktop/src/shared/api/tauri.ts`: `9dbd1d56f31844b51547663a6319ae125fd1ee98c4436399804bd520b0f8d6c7`.
- `crates/buzz-core/src/host_deployment.rs`: `4ad019ceccf817dda4702d89bb75d7eb33ed30181271f936bc80112730486c3b`.

No reviewer mutations. The conductor explicitly accepted the following dispositions;
remaining DOM cases subsequently passed. No ≥Medium source finding needs routing.

| Item | Disposition / next owner |
| --- | --- |
| Slice 1 source | **Accepted separately** by conductor; no source rework proposed. |
| G1 — native/platform gate | **Pending; blocks merge-safe attestation.** Conductor retains supported-runner compile/test including `registered_host_create_is_rejected`. No bootstrap or new test framework requested. |
| G2 — human acceptance | **Pending**, per repository AGENTS.md. Conductor retains actual Windows/human exercise; browser emulation does not satisfy it. |
| Strict Clippy | Executor's `-D warnings` failed on unchanged `paired_message/body.rs` large-enum / `from_owner` warnings; plain Clippy passed. Conductor carries separately; strict gate remains non-green. |
| Broad JS | Reproduced failure at `useKnownAgentPubkeys.test.mjs:22` through provider and `hooks.ts:388`; test/provider/hook/CommunitiesProvider are byte-unchanged from baseline. Conductor accepts **baseline-related/outside slice** disposition. Baseline execution not independently rerun; full suite is not green. |
| Full CI / release | `just ci`, native Windows build/tests, human flow and live pilot **not run**. No merge-ready/release-ready attestation. |
| Later slices | Fresh authorization required. No broader task closure or slice 2 launch. |

Minimalism **9/10**: one directory query/form, create-only destination avoids
lifecycle widening. Elegance **9/10**: existing identity, policy and config seams
reused. Correctness **9/10 for inspected inert source** with green scoped core/UI
checks; native correctness remains unverified under G1. No concrete source defect
justifies a lower score; these scores do not waive pending gates.

All reviewer test/build sessions completed and the browser preview server exited.
Only this report is committed with DCO sign-off; source remains unchanged. Continue
any authorized native verification in this same workspace against the pinned source.

quality: model=unknown effort=unknown stage=post-audit verdict=merge-block verify=partial commits=a2722aa9bdac7acefc8a01376151cdab6a249305..5eec25bf1b95c8be46cc1e78e89cfc335f4a477a findings=C=0/H=0/M=0/L=0 rework_rounds=0
