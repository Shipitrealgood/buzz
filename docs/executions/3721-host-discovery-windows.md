# #3721 shared main discovery and Windows integration — execution

**Status:** S2/S3 source handoff for independent review, 2026-11-10. No rollout.
Worktree `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`. Clean baseline
`4149810598334eb8bff18f3c65d014cba3092a59`; no unrelated edits existed or were changed.
Requested configuration: `gpt-6-astra` / `high`; actual served model/reasoning
is not independently exposed. Executor `/root/host_discovery_windows_exec`.

Governing [ratified S2/S3](../specs/3722-main-discovery-native-login.md),
[prior audit](../audits/pre-audit/3722-claude-host-reuse.md), accepted
[S1 receipt](3721-trusted-host-executor.md) and
[S1 re-gate](../audits/post-audit/3721-trusted-host-executor.md) were inputs.
The shared normal account/HOME/tool environment/native login/network ruling is
preserved. No live host, existing agent, native provider command, credential
inspection, real systemctl, installation, relay mutation, push, merge or board
write occurred. Native probe tests exclusively executed disposable fake binaries.

## Commits and source accounting

| Commit | Logical change | Exact rename-aware git delta |
| --- | --- | --- |
| `cbe4bef846affba2a0cc34546af2c8ebe3f2aee5` | Main facts/classifiers extracted to existing core; Desktop delegates; pure metadata tests move with source | 11 files, +754 / -493, net +261 |
| `7720b41a853775eae23ce103944de904f626865b` | Host resolution/readiness/launch, bounded optional DTOs, owner and existing Desktop consumers, fixtures, README | 21 files, +1052 / -89, net +963 |

The source range is baseline through `7720b41a853775eae23ce103944de904f626865b`.
Documentation/evidence follows separately. Combined source commits: **+1806 / -582**
(net +1224), including README and tests. These are diff counts, not “new platform” LOC.

**Relocation inventory (measured against baseline):** 872 original lines were
relocated/adapted: builtin catalog 156; runtime metadata with its original test
221; Windows installer-string macro and original tests 225; preset struct/table
140; avatar constants 5; availability classifier/version floor 38; builtin default
args 9; auth outcome/signals/classifier 52; availability enum 12; version parser
11; exact builtin lookup 3. Destination `buzz-core/src/harness` totals 1108 lines;
its net 236 additional lines include portable candidate/readiness/inventory types,
shared tests, exports and documentation. The old provider tables are removed,
not copied. Desktop keeps its resolver/cache/process/install/interactive-auth code.

The only new host production module is `discovery.rs` (**195 lines**), supplying
account-local resolution and bounded native probes. Remaining host production
changes adapt existing catalog, finite runner, service transaction and deployment
journal seams. The large host test delta is +346/-4 in the existing fixture.
No dependency, relay kind/schema/endpoint, plugin framework or parallel form was added.

## Implemented behavior and evidence boundary

- **Read + measured:** optional local `buzz_acp` automatically projects all main
  candidates with `runtimes: {}`. Two fake account PATHs yield different installed
  results. Explicit local rows retain IDs/restrictions and resolve main command
  candidates. Catalog revision binds local configuration plus resolved snapshot;
  legacy catalog hashing remains unchanged. Main CLI/adapter/version classifiers
  are shared; missing inputs never become available.
- **Read + measured:** finite readiness uses the same account context as models
  and conversation. Claude/Codex use main probe argv/output interpretation; failure,
  timeout and oversized output stay unknown/unavailable. Config-parse output is
  sanitized to an enum. Required richer configuration and custom definitions are
  unsupported. Grok NotApplicable permits an attempt under other gates and never
  claims authentication. Models remain independent of login history/entitlement.
- **Read + measured:** actual `buzz-host-runtime --user-service` executes the
  production Models and Conversation branches. The fixture uses real `buzz-acp
  models` with a synthetic Claude ACP peer, followed by an instrumented local
  conversation harness that checks exact key, auth tag, owner, policy, HOME/PATH,
  workspace, selected model, strict-selection flag and ordinary networking URL.
  Logged-out native state and absent selected model refuse. Internal recheck
  emits no early READY and leaves no untracked action artifact. Exact start retry
  retains execution identity; cleanup preserves fixture HOME/provider/repository.
- **Limit:** the conversation harness is synthetic. This proves production exec
  wiring and guards, not a real main ACP conversation/session/prompt against a
  relay/provider, systemd credential delivery/cgroups, or native Windows interaction.
  Main's strict ACP model selection is selected by its existing launch environment;
  no change to its implementation or independent new provider proof is claimed.
- **Read + measured:** optional Status inventory and Observation readiness/availability
  stay inside existing encrypted transport and bounds (64 entries, 8 KiB / 2 KiB).
  Old missing facts remain unknown; old readers reject new fields. Invalid facts,
  stale/scope/revision/model mismatches refuse. Native owner continuation ignores
  login history as authority. Existing Windows metadata and model/setup consumers
  use remote facts; unknown/unsupported metadata is not selectable. Canonical
  labels show a version only when main supplies a parsed check. Native Create
  requires current supported host inventory. Main key/config/profile/channel and
  shared model picker/cloud indicator remain the existing route.

## Verification

All Cargo commands used Hermit, `HERMIT_SELF_UPDATE=false`, `--offline` and serial
behavior tests. Local socket tests required narrowly approved sandbox escalation.
No installation or provider fallback was used.

| Command/gate | Measured outcome / durable evidence |
| --- | --- |
| `cargo check --offline -p buzz-hostd -p buzz-paired-owner --all-targets` | PASS before behavioral gates |
| `cargo test --offline -p buzz-core --lib harness -- --test-threads=1` | 9 PASS, including moved original metadata/Windows installer-string tests; [log](../verification/3721-host-discovery/shared.txt) |
| `cargo test --offline -p buzz-core --lib host_deployment -- --test-threads=1` | 13 PASS; [log](../verification/3721-host-discovery/wire.txt) |
| `cargo test --offline -p buzz-hostd --all-targets -- --test-threads=1` | Library 26 PASS / 2 ignored, activation 1 PASS, controller 13 PASS; other targets have zero cases; [final log](../verification/3721-host-discovery/hostd.txt) |
| `cargo build --offline -p buzz-hostd --bin buzz-host-runtime -p buzz-acp --bin buzz-acp` | PASS |
| `BUZZ_TEST_RUNTIME="$PWD/target/debug/buzz-host-runtime" BUZZ_TEST_ACP="$PWD/target/debug/buzz-acp" cargo test --offline -p buzz-hostd --lib actual_ -- --ignored --test-threads=1` | Both ignored cases explicitly PASS (legacy six synthetic model calls and new actual runtime fixture); two expected negative launches print sanitized errors; [log](../verification/3721-host-discovery/actual-runtime.txt) |
| `cargo test --offline -p buzz-paired-owner --lib deployment -- --test-threads=1` | 2 PASS, existing large continuation/recovery fixtures; [log](../verification/3721-host-discovery/owner.txt) |
| Desktop `node --import ./test-loader.mjs --experimental-strip-types --test --test-isolation=none --test-reporter=tap src/features/agents/ui/whereToRunIntent.test.mjs src/features/agents/lib/instanceInputForDefinition.test.mjs` | 33 PASS; [log](../verification/3721-host-discovery/node.txt) |
| Desktop `pnpm exec tsc --noEmit`; scoped `pnpm exec biome check` for the three changed frontend files | PASS |
| Workspace affected-package fmt; Desktop `cargo fmt --manifest-path desktop/src-tauri/Cargo.toml -- --check`; `git diff --check` | PASS |
| Desktop `CHECK_FILE_SIZES_BASE=4149810598334eb8bff18f3c65d014cba3092a59 node scripts/check-file-sizes.mjs` | PASS after read-only git subprocess sandbox escalation |
| `cargo clippy --offline -p buzz-core -p buzz-hostd -p buzz-paired-owner --all-targets` | PASS with existing warnings outside changed logic; [log](../verification/3721-host-discovery/clippy.txt). Strict `-D warnings` not run; known core body warnings remain. |
| Desktop Rust `cargo check --offline --lib` from `desktop/src-tauri` | BLOCKED before app compilation: missing `gobject-2.0 >= 2.70`; [failure](../verification/3721-host-discovery/desktop-build-blocker.txt). Desktop Rust integration tests therefore NOT RUN. |

Initial host test attempt had expected socket EPERM and an obsolete S1 refusal
message assertion; the corrected contract assertion and same fixture passed.
An owner test initially attempted to replace an immutable terminal receipt and
correctly hit that guard; it was corrected to use the existing initial/refresh
operations. A test-only `Debug` bound compile error was corrected before the final
owner run. No production guard was weakened for these tests.

**NOT RUN:** full `just ci`, native Desktop/Windows build and UI journey, real
systemd lifecycle/credentials/cgroup/user-manager/reboot behavior, provider login,
refresh entitlement/quota/chat, live relay/database and human acceptance. Earlier
S1/Windows canary evidence is not promoted to proof of this changed path.

## Decisions, rollout boundary and handoff

No product/architecture deviation: optional local ACP path is the concrete
configured auto-discovery seam; explicit rows remain a local restriction mode.
Unknown/richer configuration is intentionally unsupported. Version absence means
not checked, not unknown success. Historical login fields remain decode-compatible
only; the ordinary route neither runs custom login nor synthesizes success.

Per conductor clarification, `Restart=no` preserves ordinary explicit agent
shutdown and no idle-stop policy was added. Automatic reboot startup and durable
explicit-stop lifecycle state remain later scope, not tested guarantees.

Rollout proposal for a separately authorized owner: first finish independent source
review and native Windows build/testing; deploy matched host/client versions with
creation still gated off; verify the chosen normal account and local configured
paths in a disposable new placement; only then explicitly enable hosted creation
and observe native readiness, models, actual chat and cleanup. Existing agents and
registration are not migrated. Rollback must disable new hosted creation, stop and
clean only the recorded disposable placement through its protected journal, then
restore matching binaries without deleting shared HOME, credentials, projects or
registration state. Exact native path/install approval remains outside this dispatch.

No unresolved source product question is returned. The remaining gate is the
conductor's fresh independent reviewer on the pinned source range, explicitly
carrying the Desktop GObject/native acceptance gap. [Review brief](../briefs/3721-host-discovery-windows-review.md).
Final process inspection found no checkout-owned build/test/runtime process.
Owned source is committed; after the documentation commit the checkout is clean
and exclusive write/test ownership is released. Independent review is not supplied
by this executor.
