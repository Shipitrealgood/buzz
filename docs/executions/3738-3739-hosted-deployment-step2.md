# #3738 / #3739 — hosted deployment slice 2 execution

2026-11-10, source candidate for independent review. Requested Astra/high;
actual runtime identity/effort metadata unavailable. Exclusive worktree:
`/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`. Clean baseline
`bded3a337c4b6defdfddc6815c8c1d4c88a97f1c`; milestone1
`0cfa9f892db471bfc55d9990f307e4e4c18061af`; final tested source: b5fde93d0ecc26ceaee42688b8fefc727a4fb8f4.
No unrelated baseline changes. No merge/push, deployment or board writes.

Inputs: [slice1 contract](2722-hosted-destination-contract.md),
[independent slice1 review](../audits/post-audit/2722-hosted-destination-step1.md),
and conductor-owned `buzz-agents/docs/specs/2722-first-hosted-agent.md`, which
explicitly authorizes slice2. Review brief:
[3738-3739-hosted-deployment-step2-review.md](../briefs/3738-3739-hosted-deployment-step2-review.md).

## Implemented paths and approved decisions

- Kind43104 is a free fork allocation (repository conflict search). Signed NIP44
  commands bind owner/community/host/generation/agent/action/op/revision/expiry
  and exact ciphertext hash. Command plus result fits a bounded encrypted reply:
  32KiB peer events, 4KiB result ciphertext, 60KiB private response ceiling.
- Relay `BUZZ_HOST_DEPLOYMENT_V1` defaults off, independently of registration.
  Current membership and binding gate admission, reads and every live send.
  Explicit-kind/extension interception precedes generic parsing; generic events,
  historical/fanout/search/HTTP paths cannot deliver this private data. Live
  refresh coalesces at five seconds; result/work batches and retention are bounded.
- Additive migration82 creates private placements, immutable operations and one
  expiring host-signed capability per generation. The third table is approved
  status storage, not grant authority. Unique community/agent placement retains
  its initial operation as unit identity. Cleanup permanently fences placement.
- Permit issuance shares registration's transaction/admission lock and is the
  revoke linearization point. One ten-second challenge binds exact authority and
  placement; host consumes it durably before each deploy helper call. Revoke
  denies later permits/retries; it does not kill an already-running agent.
- Existing protected owner.sqlite atomically stores exact command/event bytes,
  phase/result and owner-self NIP44 main materialization snapshot. Snapshot binds
  the command authority and omits the agent secret. Recovery obtains the same key
  from original Command ciphertext; main's existing record requires its key field.
- Windows native `commands/agents_hosted.rs::create` reuses main create phases,
  effective deploy projection, key/NIP-OA/save/publications. It is default off via
  `BUZZ_NATIVE_HOST_DEPLOYMENT_V1=1`, with current native session checks. New agent
  starts PendingLogin. Unsupported env/command/argument/provider/timeout/session
  overrides fail closed. RegisteredHost persists explicitly, summarizes pending,
  and refuses unsupported start/stop/forced-delete paths. UI create stays blocked.
- Host `registration run --deployment` opts into the same Host identity/journal,
  catalog and fixed helper. Capability advertises only approved, verified binaries
  with credential_profile none. No provider/session borrowing. Invalid/expired
  queued commands get durable failure with no helper, so they do not starve later
  work. Exact terminal receipts replay without effects. Expired failed operations
  leave the poll queue; recovery requires an explicit new operation, not a new key.
- Only a persisted explicit cleanup intent authorizes offline restrictive retry.
  Helper fences before stop/disable/removal, checks observed state, retains the
  tombstone and user data, and reports pending on failure. Unknown/unprepared
  launch material can remain cleanup-pending; success is never invented.
- Fixed helper body is callable for the actual integration test. Its executable
  retains root/peer checks and fixed paths. No caller path/env/executable channel,
  second daemon, generic broker, installer, service sandbox or runtime redesign.

## Storage and compatibility limits

Migration81 byte-identical SHA256:
`7ca03c85b81e1e26af19531fc831bb678a3a914c815999af424254c7cc9446d4`.
All desired schema, migration inventory, deletion catalog/purge sites updated.
No FK/trigger/function/backfill. Migration82 is compatible fork expansion, **not**
a solution to the known upstream81 collision. Current unchanged registration
readers/writers ran against populated81→82 with auto-migrate disabled after
expansion. This is not an actual old-binary boot. Old exact deletion catalogs
fail closed until expanded code is deployed; old auto-migrators cannot roll back
through unknown82. SQLite additions stay inside existing protected journals.

Shipped registration service is unchanged: UID buzz-host-registration, `/state`
sandbox, no helper/catalog mounts or AF_UNIX. Execution requires the same journal
at `/var/lib/buzz-hostd`, compatible buzz-hostd peer UID, fixed helper socket and
operator catalog. CLI refuses the current incompatible installation before
helper work. Slice3 owns that installation; no journal/key copy or live changes.

## Measured verification

Hermit used with exported `HERMIT_SELF_UPDATE=false`. Final production behavior
was rebuilt and exercised by the last relay run against the exact source committed as b5fde93d0ecc26ceaee42688b8fefc727a4fb8f4 (the following
commit only updates receipts). Commands, excluding the common Hermit activation:

| Command | Measured result |
| --- | --- |
| `cargo test -p buzz-core host_deployment --lib` | 10 passed, 0 ignored |
| `cargo test -p buzz-relay -p buzz-db --lib --no-run` | actual binaries built |
| `scripts/3720-registration-proof.sh run target/debug/deps/buzz_relay-89115045a711bc85 deployment_ --ignored` | final 2 passed, 0 ignored, 5.45s |
| `scripts/3720-registration-proof.sh run target/debug/deps/buzz_db-417e37ddf5050076 deployment_atomic --ignored` | 1 passed, 0 ignored, 0.30s |
| `scripts/3720-registration-proof.sh run-migration target/debug/deps/buzz_db-417e37ddf5050076 migration_schema_deployment_82_preserves_registration_and_matches_desired --ignored` | 1 passed, 0 ignored, 1.44s |
| `scripts/3720-registration-proof.sh run-migration target/debug/deps/buzz_db-417e37ddf5050076 store::deletion::postgres_tests::paired_host_tables_obey_fence_and_purge --exact --ignored` | 1 passed, 0 ignored, 1.17s |
| `cargo test -p buzz-hostd -p buzz-paired-owner --no-fail-fast` | library 11 passed; socket-dependent targets initially EPERM in sandbox |
| `scripts/3720-registration-proof.sh run target/debug/deps/buzz_host_activate-c705fc126c9d6ce8` | real peer-UID check 1 passed |
| `scripts/3720-registration-proof.sh run target/debug/deps/buzz_hostd-c6ec734a86c6765b` | controller regression 12 passed |
| `scripts/3720-registration-proof.sh run target/debug/deps/buzz_paired_owner-13b529a6a28bdf14` | owner regression 15 passed, including new recovery test |
| `pnpm --dir desktop typecheck` | passed |
| From desktop: `node --import ./test-loader.mjs --experimental-strip-types --test src/features/agents/ui/runOnSummary.test.mjs` | test file passed, including RegisteredHost summary |
| workspace/native `cargo fmt --check`; scoped Biome; `git diff --check` | passed |
| `CHECK_FILE_SIZES_BASE=bded3a337c4b6defdfddc6815c8c1d4c88a97f1c pnpm --dir desktop check:file-sizes` | passed after sandbox git-spawn EPERM; read-only escalation |

New relay tests live in canonical nested `external_infra_deployment_tests`:
`deployment_owner_host_helper_restart_cleanup_journey` and
`deployment_default_off_admission_and_capability_are_independent_of_registration`.
They use actual relay/PG/owner journal/dispatch/host journal/helper body over Unix
socket. Systemctl and installation UID/paths are synthetic; no actual unit runs.
Coverage: default-off advertisement/admission, positive private read, wrong-owner
read/generic REQ/COUNT refusal, no private events on generic view, live close after
revoke; original key/ciphertext/snapshot after crash-before-save and reply loss;
PendingLogin prepare-only, selected-model update with same unit/key, expired
queued work, current-authority loss without killing the unit, invalid inner
command failure, cleanup helper failure/fence and offline retry after revoke.

DB test `deployment_atomic_replay_placement_permissions_cleanup_and_revocation`
adds cross-host duplicate placement, conflicting bytes/revision, expired/signature
failure, removed-member permits, stale queued deploy after cleanup, revoke race
and denied subsequent reads/permits. Wire tests cover tamper/foreign scope,
nonce/expiry and bounded command+result reply. Owner test
`protected_main_snapshot_same_key_expiry_tamper_and_terminal_receipt` exercises
reopen after expiry, no plaintext duplicate secret, snapshot tamper/scope,
monotonic phases/results. Main-native UI guard test extended but **not run**.

Finite falsification: removed only helper DropInPaths rejection; focused
`activation::tests::rejects_overridden_or_preexisting_foreign_instance` failed
0/1 as required. Exact restore SHA256
`8dcef07da2d6e2e0e55467585da02d595da816fd5956c46a3770bc347ffb13cb`, then same test
passed1/0. No held mutation. Earlier regression tests remain relevant to unchanged
registration/helper boundaries; final WS run covers subsequent deployment edits.

## Explicit red / unrun gates

Native `cargo check --manifest-path desktop/src-tauri/Cargo.toml` stopped at
missing `gobject-2.0 >= 2.70`. No dependency install. Windows compilation, native
main save/publication journey, human UI, actual systemd/service and pilot are
**not run**. Owner-library snapshot recovery is not native Tauri acceptance.
Strict scoped Clippy `--all-targets -- -D warnings` stops on unchanged core
large_enum_variant/wrong_self_convention. Non-strict scoped Clippy completed;
remaining warning sites are unchanged core/host legacy/DB reads/owner examples/
relay phase tests (16 distinct sites), not new deployment code.

Discovery scanner still exits1 solely on unchanged
`registration_executable_tests.rs:4: executable_registration_restart_recovery_revoke_and_native_scope_fence requires PostgreSQL but is not discoverable; place it under postgres_tests or in a postgres_* integration binary`.
New tests are discoverable and manually executed. Implicated baseline SHA256:
script `bcc7a6b5518b931aa4d74f0a6ecdcbfb4cb93e94c57a9fc0a818556afe2dc9e3`,
include `32e3de94e0572da848dfd480892dff0dcc6468f494e323862265e651b9283873`.
No broad CI/JS/provider repair was attempted; independent review remains required.

## Size and ownership handoff

Measured against baseline, counting test hunks separately in mixed files:
wire+DB+relay **+1071/−12**; owner+WS **+424/−0**; host/helper **+917/−245**;
native+TS **+373/−14**. Production **+2785/−271**, net **2514**. Includes 239
production lines removed from helper executable into the adapted shared body;
23 test lines moved out too. Test/proof **+1549/−28**. Final docs: **+287/−0**.
Conductor accepted revised production ceiling3150/net2900 and actual test growth
for distinct existing acceptance cases; no framework expansion.

All owned test/build sessions finished. `scripts/3720-registration-proof.sh down`
completed successfully and removed the label-verified runner/Postgres/Redis
containers (no TLS proxy was started). No live ports/volumes were used.
No real relay/provider/production credential/SSH/unit/account/mount/volume changes.
Historical stopped proof directories remain untouched. Next gate: independent
Astra/high review on this clean committed worktree; supported native CI before
merge, installation and pilot only in their later authorized slices.
