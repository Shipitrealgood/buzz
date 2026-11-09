# #2722 / #3720 hosted destination step 1 implementation receipt

2026-11-09 · executor handoff to fresh independent Astra/high reviewer.
Status: **committed step-1 candidate; independent review pending; native gate
unrun and two broader gates have recorded failures. No deployment is enabled.**

## Source, scope and custody

Workspace: `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`. Measured starting HEAD:
`a2722aa9bdac7acefc8a01376151cdab6a249305`; starting worktree clean.
Requested configuration: `gpt-6-astra`, high reasoning. Runtime model identity is
not exposed in this worker's authoritative tool metadata; not independently verified.

Source commits:

- `df1910425`: inert core contract, validators/tests and storage decision.
- `cd8f825f0`: shared destination UI, create-boundary guards and regressions.
- This receipt commit adds only handoff evidence and clarifies the retained hostd
  unit ID in the storage decision. No executable behavior changed after the gates.

Accepted plan/brief live in conductor operations docs:
`/home/bryce/services/buzz-agents/docs/specs/2722-first-hosted-agent.md` and
`docs/briefs/2722-create-completion.md` there. Their planning-only banner was
superseded by explicit authorization for **slice 1 only**. This source receipt
carries review-brief content because the conductor owns the operations brief.

Measured final source is scoped and committed. No unrelated edits existed or
were staged. Cargo's native attempt generated two pre-existing manifest/lock
reconciliation lines; those were restored exactly to baseline and are excluded.
Migration 81, all migrations/schema, relay handlers, hostd/runtime/helper/service
code and historical proof stages remain unchanged. No taskctl, merge, push, PR,
canary, deployment, real account/provider or service operations were performed.

Measured change totals from baseline through this receipt: **26 files**.
Production: **16 files, +669 / -74 lines**; tests: **7 files, +748 / -0 lines**;
docs: **3 files, +296 / -5 lines** (includes this receipt and feature guidance).

## What landed and decisions

[Contract/storage decision](2722-hosted-destination-contract.md) is the slice-2
handoff. Core reuses registration Scope/Policy/identifiers, existing hostd catalog
IDs/hash, and the existing private identity/NIP-OA validator. No new runtime table,
provider catalog, provisioning service, transport codec or admitted wire operation.
Authority is explicitly unsigned data until slice 2 binds it in the verified
signed envelope; its validators do not substitute for authorization.

Windows Run on uses the existing private registration query keyed by relay and
owner, with a scope-filtered projection. It distinguishes registration states and
query failure without claiming offline/ready deployment. Same-name hosts include
key labels. Add Host remains visible when empty and opens the existing registration
form; portal submissions cannot bubble into agent creation. Host drafts stay
blocked through workspace changes and cannot become local by an async result.

Creation is unconditionally refused at dialog submission, persona-create action,
shared instance builder, shared API, and the native command before store/key or
publication effects. Persistent BackendKind is unchanged. The native destination
is create-only so slice 1 adds no partially functional host lifecycle variant.

Storage is additive **design only**: unique community+agent placement, immutable
operation commitment rows, existing protected owner journal intent containing exact
new-agent NIP-44 ciphertext. Main stays the key/config creation authority. Recovery
requires the original native owner signer; normal main key/record persistence is a
future replayable materialization. Stable hostd unit ID is the initial create
operation ID, retained across later deploy operations. No schema or storage writes
are implemented here.

## Measured verification

All commands ran from this worktree under `. ./bin/activate-hermit`; desktop
commands below ran from `desktop/`. Successful tool exit is recorded separately
from tests that could not execute.

| Gate | Actual result |
| --- | --- |
| `cargo test -p buzz-core --lib` | **305 passed**, including five new contract tests covering roundtrip, each scope binding, expiry/revision/hash, unsupported/empty/stale/revoked status, closed variants, bounded config, new-key/auth-tag binding, action/result correlation and overflow. |
| Focused node tests: `node --import ./test-loader.mjs --experimental-strip-types --test src/features/agents/ui/whereToRunIntent.test.mjs src/features/agents/lib/instanceInputForDefinition.test.mjs src/shared/api/hostDestination.test.mjs` | **31 passed**; no avatar-upload/native-IPC effects for refused hosts, complete provider fixture cannot enable host submission, local/provider mappings retained. |
| `node --import ./test-loader.mjs --experimental-strip-types --test 'src/features/agents/**/*.test.mjs' src/shared/api/hostDestination.test.mjs` | **1,696 passed / 1 failed**; exact failure below. Not an all-green suite. |
| `node --import ./test-jsdom-setup.mjs --import ./test-loader.mjs --experimental-strip-types --test-force-exit --test src/features/agents/ui/WhereToRunSection.jsdom-test.mjs src/features/agents/ui/PairedHostsSection.jsdom-test.mjs` | **5 passed**. Actual query/render seams: old-owner/community delayed result, even mis-scoped new projection, registration state table, unsupported/unavailable/empty, existing registration journey. All external boundaries mocked. |
| `pnpm build:e2e` then `pnpm exec playwright test --project=smoke paired-hosts.spec.ts where-to-run-config.spec.ts` | **12 passed** (8 registration/Add Agent, 4 existing provider Run on). Actual shared application UI in Chromium with mock bridge: duplicate labels, selection, forced form submission, empty Add Host, expired invitation without accidental agent creation; provider typing/probe/reset and local switch regressions. |
| `pnpm typecheck` | Passed. Also included in both build variants. |
| `pnpm build` | Passed, including protected feature artifact matrix; final dist is OSS variant. Existing large-chunk/dynamic-import warnings. |
| Scoped `pnpm exec biome check` for all 16 changed TS/TSX/MJS files | Passed, no fixes/warnings. |
| `cargo fmt --all --check`; `cargo fmt --manifest-path desktop/src-tauri/Cargo.toml --check` | Passed. |
| `CHECK_FILE_SIZES_BASE=a2722aa9bdac7acefc8a01376151cdab6a249305 node desktop/scripts/check-file-sizes.mjs`; `git diff --check` | Passed, including untracked candidate files before commit. |
| `cargo clippy -p buzz-core --all-targets -- -D warnings` | **Failed** on two unchanged `paired_message/body.rs` warnings: large enum variant (line 9), wrong-self-convention `from_owner` (line 133). Plain `cargo clippy -p buzz-core --all-targets` passed with those warnings. No scoped warning remains. |
| `cargo test --manifest-path desktop/src-tauri/Cargo.toml registered_host_create_is_rejected --lib` | **Could not execute test**: compilation stopped in glib-sys/gobject-sys; pkg-config cannot find `glib-2.0 >= 2.70` / `gobject-2.0`. No system installation attempted. Prior #3750 native evidence is hosted Windows; no reusable local project sysroot was found in its receipts. |

Broad JS failure: `src/features/agents/useKnownAgentPubkeys.test.mjs:22`,
“provenance context follows exact local inventory and rejects failed cached reads”:
`useCommunities must be used within a CommunitiesProvider`, through
`useRelayAgentsQuery` / `KnownAgentPubkeysProvider`. The test, provider and hook
are unchanged. **Appears pre-existing; baseline not independently rerun.** It is
reported for reviewer disposition, not silently excluded or repaired.

The native regression is authored at
`desktop/src-tauri/src/commands/agents_tests.rs:886`. It calls the actual native
create function with local/provider backends plus registered-host destination;
a poisoned first store lock prevents any key-generation/publication path if the
guard is removed. It asserts the specific refusal and no app-data directory,
malformed deserialization refusal, and legacy absent-destination validation.
This evidence is **read, not measured** until a supported native runner executes it.

Windows native compile/tests, actual Windows UI, human acceptance, real services,
relay/DB integration, provider auth/models and live pilot: **not run**. `just ci`
is not run: this dispatch explicitly limited verification and authorizes no PR.
Native test/build and strict lint gaps remain gates, not passing substitutes.

## Finite falsification evidence

Two controlled mutations were applied sequentially and restored byte-for-byte
in `try/finally`, then each exact test reran green:

1. Removed `createManagedAgent`'s destination guard in `shared/api/tauri.ts`.
   `hostDestination.test.mjs` failed when it reached mocked native IPC; restored
   test passed. Restored SHA-256:
   `9dbd1d56f31844b51547663a6319ae125fd1ee98c4436399804bd520b0f8d6c7`.
2. Removed `Authority::validate`'s scope equality guard.
   `cargo test -p buzz-core authority_roundtrip_and_each_binding_rejects_mismatch --lib`
   failed on foreign scope; restored test passed. Restored SHA-256:
   `4ad019ceccf817dda4702d89bb75d7eb33ed30181271f936bc80112730486c3b`.

Initial tests also exposed serde accepting unknown fields for internally tagged
unit variants. Capability/model/cleanup use empty struct variants instead;
explicit unexpected command/model-field regression cases now pass. This is an
implementation correction, not an upstream protocol change.

## Review watchpoints and deviations

1. **Authority is inert.** Probe `crates/buzz-core/src/host_deployment.rs:146` and
   `:259`: every signed field must eventually be authenticated; verify no reader
   or executor treats the current shape validator as permission. Registration
   v2 remains unchanged.
2. **No local fallback.** Probe `AgentDialog.tsx:186`, `whereToRunIntent.ts:65`,
   `instanceInputForDefinition.ts:121`, `shared/api/tauri.ts:771` and native
   `commands/agents.rs:405`. Consider forged destination plus local/provider
   backend, reserved host selection without metadata, stale owner/community.
3. **Query identity.** Probe `lib/hostDirectory.ts:34`,
   `WhereToRunSection.tsx:71` and `hostDestination.ts`. The shared cache key and
   scope filter must exclude late results; metadata stays registration-only.
4. **Nested registration form.** Probe `WhereToRunSection.tsx:207` and the final
   empty-directory browser test. Removing portal submit propagation isolation
   can accidentally invoke the surrounding Add Agent submit handler.
5. **Secret/config reuse.** Probe core `Command::validate` and
   `private_managed_agent.rs:529`: key must match bound agent and real NIP-OA
   owner attestation, no owner key/command/env/path transfer. Actual config
   adapter and model discovery are intentionally absent, not approximated.
6. **Persistence handoff.** Probe the companion storage decision: original
   create operation remains unit ID; changed bytes conflict; ciphertext recovery
   requires sender key; placement tombstones cannot be treated as deletion.
7. **Native verification gap.** Independently assess compile/test risk before
   approval. Browser platform emulation and core serde tests do not certify
   Windows IPC, native create side effects or protected journal permissions.

Adaptations within the approved contract: create-only destination instead of
widening persistent backend enums; supported-empty modeled in core but not
fabricated in the registration-only UI; existing registration form in a portal;
key recovery reuses exact NIP-44 content instead of introducing new DPAPI storage.
No upstream stage reopened and no foundational workaround implemented.

## Remaining scope and next gate

No step-1 product question remains. Conductor should dispatch fresh independent
Astra/high review against the committed range and this receipt, then disposition
native/strict-lint/broad-JS gaps. Do not proceed to slice 2 or mark whole #2722 or
#3720 complete from this scaffold. Slice 2 owns signed deploy authority and
placement/journal integration; later slices own service isolation/native login,
Windows/human journey, pilot approval and observed cleanup. Historical proof
stages, existing agents, provider sessions and production configuration are outside
this candidate. All worker test/build processes are stopped at handoff.
