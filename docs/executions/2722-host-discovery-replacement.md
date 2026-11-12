# 2722 — Host discovery replacement execution

Implementation is ready for independent review. Scoped source tests passed;
full relay integration, Windows native compilation/UI and live activation are
**not accepted** by this execution. No live failure is claimed fixed.

## Source and authority

- Workspace `/home/bryce/services/buzz-wt-3720-host-registration`, branch
  `proof/3720-host-registration-events`; measured clean baseline
  `93ea3f2a29f663e9c53cd560df33e73a785dc37a`.
- Read-only upstream reference `4c60be6a1e8dc2c7839f7c0294391fb93941ed9f`.
- Approved spec `/home/bryce/services/buzz-agents/docs/specs/2722-host-discovery-replacement.md`;
  accepted upstream audit `/home/bryce/services/buzz-agents/docs/audits/pre-audit/2722-prepare-refresh-read.md`.
- Requested Astra/high; actual runtime configuration not independently observable.
  Execution/brief skills used. No subordinate agents or board writes.
- Exclusive source/test ownership; no pre-existing dirty files. No merge, push,
  install, remote CI, live relay/provider/host-service call or existing-agent access.

## Implemented contract

Read: Add Agent uses the existing host inventory and model picker. The query
selects one approved runtime and catalog revision on private 43103 via explicit
`models-query` / `models`. It creates no agent, profile, NIP-OA, placement or
service. Owner subscribes before publication, verifies the exact signed reply
and closes at completion/deadline. Unavailability never becomes an empty catalog.

Hostd checks authorized mailbox evidence and local grant/catalog before invoking
the extracted finite runner with the configured account HOME/environment. Output
and process lifetime remain bounded. The journal saves the exact reply before
publication; a completed replay does not rerun the provider. Revocation fences
new model writes and reads while preserving historical enrollment recovery.
`latest_status_id` remains registration-status-only. No schema change.

The shared picker scopes cache and late replies by owner/community/host/generation,
installed runtime and catalog revision. Only returned IDs enable Create. Blank
Default maps to a returned literal default ID; absent default requires selection.
Create uses main identity/configuration phases with `Selected`; the existing host
runtime still rechecks current readiness/model before conversation. Old unfinished
PendingLogin/Check/Retry/Refresh/Launch/Cleanup flows remain.

## Measured change size and adaptations

Against baseline, standard Git line diff with inline test sections separated:
**17 production files, +649 / -189 = +460 net**. Moves receive no discount.
Tests: +758 / -28 across seven test regions/files (larger than the spec estimate,
including real signed-response and current runtime fixtures). Contract docs before
these handoff artifacts: +38 / -11. No new production file, kind, table or service.

Conductor accepted an extra active-binding filter in existing registry
messages/evidence reads: old registration history remains readable after fencing,
new discovery does not. The Windows explicit re-export in `commands/agents.rs`
is also required. Conductor approved one test-only baseline repair in
`store/deletion.rs`: missing optional `Status.inventory` is now `None`.
The obsolete relay root-helper fixture is preserved and remains a known gate gap.

## Verification receipts

All Cargo commands used `export HERMIT_SELF_UPDATE=false; . ./bin/activate-hermit`,
`--offline --locked -j2`. No dependency installation. Local socket/process fixtures
needed narrow sandbox escalation; tests ran sequentially. See
[evidence](evidence/2722-host-discovery-replacement-checks.md) for failures and limitations.

| Command / scope | Measured outcome |
|---|---|
| `cargo check … -p buzz-core -p buzz-hostd -p buzz-paired-owner -p buzz-db` | passed |
| `cargo check … -p buzz-relay` | passed, production relay path |
| `cargo test … -p buzz-core -p buzz-paired-owner --lib -- --test-threads=1` | core 325 passed; unchanged owner 16 passed; new owner fixture initially failed, corrected and independently rerun below |
| `cargo test … -p buzz-paired-owner --lib registry::registration_adapter_tests::models_wire -- --test-threads=1` | 1 passed, seven signed response scenarios |
| `cargo test … -p buzz-hostd --lib -- --test-threads=1` | 28 passed, 3 ignored before adding new ignored journey |
| `cargo build … -p buzz-hostd -p buzz-acp --bins` | passed; used only fake provider programs |
| `BUZZ_TEST_RUNTIME=$PWD/target/debug/buzz-host-runtime BUZZ_TEST_ACP=$PWD/target/debug/buzz-acp cargo test … -p buzz-hostd --lib user_service::tests::actual_ -- --ignored --test-threads=1` | preserved old journey passed; new fixture corrected and rerun below |
| Same env, filter `user_service::tests::actual_host_first` | 1 passed; pre-Create zero service effects, real ACP model projection, replay, selected launch, later login/model refusal, cleanup |
| `cargo test … -p buzz-db --lib --no-run` | passed after the approved fixture correction |
| `scripts/3720-registration-proof.sh run target/debug/deps/buzz_db-8990db044ccf2a6b store::paired_hosts::postgres_tests::registry_models_admission_and_revoked_reads_have_no_agent_effects --ignored --exact --nocapture` | 1 passed on a fresh dropped-after-run DB; genuine writer/permission code |
| Desktop hook jsdom test, `node --import ./test-jsdom-setup.mjs --import ./test-loader.mjs --experimental-strip-types --test-isolation=none --test src/features/agents/ui/usePersonaModelDiscovery.jsdom-test.mjs` | 3 passed |
| Same non-jsdom loader; `usePersonaModelDiscovery.test.mjs`, `agentDefinitionSubmitPayload.test.mjs`, `instanceInputForDefinition.test.mjs`, `hostDestination.test.mjs` | 57 passed |
| `pnpm --dir desktop typecheck` | passed |
| `cargo fmt --all --check`; Tauri manifest fmt check; `git diff --check` | passed |
| `CHECK_FILE_SIZES_BASE=93ea3f2 node desktop/scripts/check-file-sizes.mjs` | passed with narrow Node/Git subprocess escalation |
| Strict clippy on five changed crates | failed on two unchanged core warnings; not green |
| Same clippy without `-D warnings` | completed; diagnostics on unchanged paths only |

Isolated PG/Redis used existing cached images and the existing network-none task
harness, with no host ports. Test DB dropped; all three task containers removed.
No background test/build processes retained. No unrelated worktree changes.

## Unrun gates and next action

Windows-only native `hosted_models` and updated native create fixture are not
compiled/run here; Linux lacks native GLib/WebKit and is not Windows evidence.
The real relay integration test binary fails to compile due the old removed
root-helper fixture, so no end-to-end owner → actual relay → hostd mailbox test
is claimed. DB writer, codec, owner parser, hook and actual runtime seams were
exercised separately. Full `just ci` / `just test` not run: these known native
and relay-test prerequisites prevent a green result; default dev DB is forbidden.

Fresh independent reviewer should assess these gaps and the request-to-Create
chain. Rollout requires coordinated relay, hostd and desktop upgrades, native
Windows verification and a separately approved human journey. All live activation
is held. Jeff/test1 and existing Claude/Grok/Scout state remain untouched.
