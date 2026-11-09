# #3738 / #3739 — hosted deployment slice 2 execution

Work in progress, source only. Requested configuration: Astra/high; runtime
identity/effort metadata unavailable. Exclusive writer/test workspace:
`/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`. Clean baseline
`bded3a337c4b6defdfddc6815c8c1d4c88a97f1c`. No unrelated baseline changes.

Inputs: accepted slice 1 contract in `2722-hosted-destination-contract.md`,
independent review `../audits/post-audit/2722-hosted-destination-step1.md`, and
conductor-owned `buzz-agents/docs/specs/2722-first-hosted-agent.md`. Current
slice 2 dispatch supersedes the spec's stale authorization header.

## Approved checkpoint refinements

- Kind 43104 is free in this pinned fork (repository kind search, read evidence).
  Registration kind 43103 and migration 81 remain unchanged. Deployment is an
  independent opt-in capability, default off with `BUZZ_HOST_DEPLOYMENT_V1`.
- Migration 82 adds private placements, immutable operations, and one expiring
  host capability record per generation. The third table is necessary before
  any agent placement exists; it confers no registration authority. All writers
  use community admission and the existing paired admission/restriction lock.
  No foreign keys, triggers, DB functions or backfills. This is fork expansion,
  not a solution to the known upstream migration-number collision.
- Existing schema81 registration queries/writers continue against the expansion
  without running DDL. Old auto-migrators cannot roll back over unknown migration
  82. The old exact deletion catalog fails closed during expand→deploy; community
  deletion must wait for code with the new catalog. Neither is claimed repaired.
- Execution permit issuance under the registration admission lock is the
  revocation linearization point. A permit issued first may authorize one
  immediately consumed bounded helper attempt; revocation denies subsequent
  permits, including retries. Each permit binds the operation, placement revision,
  initial-operation unit identity, ciphertext commitment and fresh challenge,
  with a ten-second lifetime. No retroactive remote cancellation is claimed.
- Cleanup fences terminally before helper work and retains user data. Restrictive
  local cleanup remains possible after authority is revoked or unavailable.
- Recovery needs main record/publication inputs absent from Deploy. One atomic
  owner intent also stores an owner-self NIP44 snapshot, bound to the exact
  command authority. It omits the agent secret; recovery opens that only from
  the original host command ciphertext. Owner-only inputs never reach the host.
- Main will use an explicit RegisteredHost backend. Existing lifecycle routes
  must refuse unsupported actions; no Local/Provider disguise. UI create remains
  blocked. Lower-level native materialization is independently default disabled.
- The shipped registration service is deliberately incompatible with execution:
  UID `buzz-host-registration`, isolated `/state`, no helper socket/catalog mount,
  and no AF_UNIX. The helper admits UID `buzz-hostd` and fixed staging paths.
  Slice 2's actual opt-in reconciliation must refuse incompatible installation;
  slice 3 owns service/account/mount integration. No key/journal copy or sandbox
  widening. The same production reconciliation is tested in a disposable layout.
- The fixed helper body may move into a callable production module so tests use
  actual helper logic with synthetic systemctl, not fabricated helper replies.
  The executable retains root/peer authentication and fixed production paths.

Initial estimate: production +1,600–2,300, tests +900–1,300, docs +150–250.
Conductor approved revised production gross +2,600–2,900, net +2,300–2,600,
including approximately 250 moved helper lines. Actual counts follow at handoff.

## Milestone 1 — private wire and relay writer

Implemented draft source: strict signed/NIP44 wire; private admission/read/live
adapter; additive schema/catalogs; exact-byte operation retry and unique placement;
current-authority execution permits; terminal cleanup placement fence. Owner and
host/helper/native materialization and full relay integration remain incomplete.

Measured on this workspace under Hermit (`HERMIT_SELF_UPDATE=false`):

- `cargo check -p buzz-relay`: passed (early compile included subsequently fixed
  missing-doc and unused-import warnings; clean repeat remains required).
- `cargo test -p buzz-core host_deployment --lib`: 9 passed, 0 ignored.
- `cargo test -p buzz-db --lib --no-run`: built the actual test binary.
- Existing `scripts/3720-registration-proof.sh` isolated runner, network none,
  tmpfs PostgreSQL/Redis, no host ports/persistent volumes:
  - `deployment_atomic_replay_placement_permissions_cleanup_and_revocation`:
    1 passed, 0 ignored.
  - `migration_schema_deployment_82_preserves_registration_and_matches_desired`:
    1 passed, 0 ignored. Full migrations through81, populated registration,
    expansion82, unchanged registration reader/writer calls, and live catalog
    column/index parity for the three new desired-state tables. This is not an
    old-binary boot test or full upstream history compatibility claim.
  - `store::deletion::postgres_tests::paired_host_tables_obey_fence_and_purge`:
    1 passed, 0 ignored; expanded to new tables and actual deployment writer fence.

First Hermit attempt used a non-exported setting and failed on read-only cache
metadata; corrected to exported `HERMIT_SELF_UPDATE=false`. Ordinary Docker call
was sandbox-denied; approved escalation started only task-owned proof resources.
No live service/provider/registration requests, credential reads, SSH, installs,
account/mount changes, merge, push or taskctl mutations.

Remaining gates: complete source, native-owner/host/helper/real-WS adversarial
journey, migration deletion mode, relevant lint/format/discovery checks and
independent Astra/high review. Native Tauri build/test remains blocked by the
already measured missing GLib; Windows/human and installed-service/pilot gates
are not run. This partial milestone is not slice 2 acceptance.

Additional milestone checks: clean repeat `cargo check -p buzz-relay` passed
without new warnings; embedded migration inventory test passed (1). Discovery
script is **not green**. Exact terminal error:

```text
PostgreSQL test discovery validation failed:
  - /home/bryce/services/buzz-wt-3720-host-registration/crates/buzz-relay/src/handlers/host_registry/registration_executable_tests.rs:4: executable_registration_restart_recovery_revoke_and_native_scope_fence requires PostgreSQL but is not discoverable; place it under postgres_tests or in a postgres_* integration binary
```

Read/measured continuity against baseline: both implicated file and discovery
script are byte-identical (`git show baseline:path` compared with working bytes).
SHA-256: discovery script
`bcc7a6b5518b931aa4d74f0a6ecdcbfb4cb93e94c57a9fc0a818556afe2dc9e3`;
registration executable include
`32e3de94e0572da848dfd480892dff0dcc6468f494e323862265e651b9283873`.
Migration81 likewise byte-identical:
`7ca03c85b81e1e26af19531fc831bb678a3a914c815999af424254c7cc9446d4`.
The conductor explicitly retained this discovery failure outside the slice;
manual execution is evidence for the named new tests, not a repaired CI scanner.

Milestone1 staged source/test classification (mixed files by changed hunk):
production **+1,061 / −12**, tests/proof runner **+523 / −5**. Documentation
counts are reported with the committed artifact below; no moved helper code is
in this first group. Pending owner/helper drafts remain uncommitted until their
own meaningful gates pass. They are owned work, not unrelated baseline edits.
