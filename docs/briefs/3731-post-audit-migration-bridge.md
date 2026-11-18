# Migration bridge review for #3730 and #3731

## Source

Review branch `rehearsal/3731-backend-catchup` from canonical
`3876720c345bb6a9b709401707e35225e66bdb31`. Deployed fork source is
`d7714be862f4514822c303b124a347f5e07dc8c5`.

## Result

The public migration entry point reconciles the two historical paired
migrations before upstream prebuilds. SQLx subsequently applies the remaining
upstream and paired migrations to their respective ledgers. The desired-state
manifest includes all six paired tables. The two SQLx ledgers are excluded
from declarative pruning.

## Review target

Assess whether a legitimate interrupted upgrade can resume and whether an
ambiguous database can lose a receipt. The production entry points are
startup auto-migration and `buzz-admin migrate`, both via `Db::migrate`.
The rehearsal called the same `migration::run_migrations` function on
isolated PostgreSQL databases.

## Checks that distinguish the histories

1. Compare the SQL under `crates/buzz-db/extension-migrations/` with the
   identically named files under `migrations/` at the deployed fork revision.
   The two SHA-384 fingerprints derive from these bytes via pinned SQLx.
2. At `crates/buzz-db/src/runtime/migration/extension_history.rs:49`,
   test total prefixes across both ledgers. A fork 81 with a canonical 82,
   missing shared 80, or an extra fork receipt in the destination must fail.
   A transfer followed by a failed upstream 83 can leave canonical 1–82 and
   both fork receipts in the destination.
3. At `extension_history.rs:81`, both SQLx ledger creations, ledger locks,
   validation, and the copy/delete execute in one transaction on the backend
   that holds the outer schema lock. At line 132, compare each transferred
   timestamp and execution time alongside its version, description, checksum
   and success value. The six catalog checks establish table presence and
   kind. Index and row preservation are separate rehearsal measurements.
4. At `crates/buzz-db/src/runtime/migration.rs:62`, preserve the order of
   reconciliation, bounded upstream runs, final upstream run and extension
   run. The historical test helper at line 49 applies only the requested
   upstream prefix.
5. Inspect the six manifest includes at `schema/schema.sql:130` alongside
   the SQLx ledger exclusions at `.pgschemaignore:14`. The desired-state
   files follow the repository's unqualified table-name convention. The
   historical extension migration bytes must remain frozen.

## Implementation decisions

The extension source is crate-local. The bridge verifies version, description,
success and checksum against each embedded source before transferring
anything. The current schema must be public because the frozen SQL mixes
unqualified and public-qualified table references. Schema writers on this
path must obey the existing schema/destruction advisory lock. The rehearsal
assumes exclusive migration ownership while upgrading a historical database.

## Release boundary

Community deletion requires a separate integration. At
`crates/buzz-db/src/store/deletion.rs:3601`, canonical code compares the live
tenant table set with its catalog and rejects the six extra paired tables as
unknown. Paired runtime behavior, deletion integration and fleet rollout are
outside this isolated migration proof.

## Evidence ownership

The executor checked frozen files against both Git revisions and exercised
the Rust migration unit suite. The conductor owns the PostgreSQL fixtures,
historical runtime, dump/restore comparisons and final-head evidence.
The real desired-state parity test, which invokes pgschema, is PostgreSQL-gated.
The independent reviewer owns the acceptance verdict.
