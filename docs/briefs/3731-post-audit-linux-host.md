# Linux host runtime review for #3731

## Source

Review `1440c72b0024689ade7c2b455e8f97c41992ad18..01e0d3f1f62b9973567baa3d262cbe58281777b8`
on `rehearsal/3731-backend-catchup`. The checkout started clean.
The source for host behavior is `d7714be862f4514822c303b124a347f5e07dc8c5`.
The three release scripts come from `575b92aa96912937e400187609f87245e1180fd1`.

## What landed

The Linux host can reopen its existing journal, authenticate a generation on
the relay, discover harnesses through its trusted account and run a selected
model. The fork's release recipe builds the CLI, ACP, host runtime and daemon
from one clean revision. The rehearsal retained upstream CLI source.

## Review target

Check the WebSocket integration and the boundary between the account's
catalog, existing registration and new managed agents. The daemon, its state
format, its service assets and the shared harness tree compare byte-for-byte
with the fork. Runtime observations below used synthetic accounts and keys.

## Where to inspect

1. In `crates/buzz-hostd/src/store.rs:57` and
   `crates/buzz-hostd/src/registration.rs:64`, follow the state directory
   through key loading and the generation-scoped NIP-42 tag. The daemon's
   retry tests cover lost acknowledgments, generation rotation and offline
   revocation.
2. In `crates/buzz-ws-client/src/connection.rs:113`, follow the explicit
   connector through the same upgrade request builder as the ordinary client.
   At line 170 check the asynchronous upstream signer, the synchronous guarded
   signer, and the challenge and reply deadlines. At line 219 compare the
   published event bytes to a directly serialized signed event.
3. In `crates/buzz-ws-client/src/host_deployment.rs:9`, follow the
   subscription ID, generation, nonce and relay signer into the opened reply.
   Other frames still reach the registration subscriber.
4. In `crates/buzz-hostd/src/discovery.rs:30`, follow a candidate from the
   shared presets to resolution under the account's configured tool path.
   The source of the private user unit is `crates/buzz-hostd/src/user_service.rs:105`.
   The package includes only the four named binaries in
   `scripts/paired-hosts/package-release.py:36`.
5. In `crates/buzz-acp/src/relay.rs:1359`, check which EOSE messages can
   satisfy the Linux readiness socket after a reconnect. In
   `crates/buzz-acp/src/pool.rs:1706`, check the selected model against the
   fresh session, including an adapter that drops a previously offered model.
6. In `scripts/paired-hosts/stage-release-catalog.py:36`, examine the new
   catalog path, preserved launcher, and selection of ACP and CLI from one
   release. Candidate creation and checksum failure have fixture coverage.

## Implementation decisions

The baseline shared only wire-level harness facts. Commit `e165e5d81`
restores the fork's full harness tree, including catalog metadata referenced
by the portable definitions. The conductor approved this prerequisite.
`d2b2c72d0` adds the fork's ACP changes to the current upstream.
`361f4613d` ports hostd and adapts WebSocket signing and publication to
the upstream timeout handling. `01e0d3f1f` restores release selection.

## Verification

Native Linux Rust/Cargo 1.98.1, offline locked, two build jobs, unoptimized
profile with debug information and incremental compilation disabled.

- `cargo test -p buzz-core --lib harness::` passed 9 tests.
- `cargo test -p buzz-hostd -p buzz-ws-client --lib --bins -- --test-threads=1`
  passed 42 host and 9 WebSocket tests. Host counts include 13 daemon tests
  and one helper test. The runtime binary target itself has zero unit tests.
- The ACP lib test binary passed 21 tests with filters `strict_model`,
  `paired_startup`, `pool::model_switch_tests` and `pool::startup_effort_tests`.
- The built host runtime and ACP passed three explicitly enabled `actual_`
  host library tests with `BUZZ_TEST_RUNTIME` and `BUZZ_TEST_ACP` pinned
  to these binaries. They cover read-only discovery, selected launch, refusal
  paths, cleanup and preservation of synthetic account files.
- `python3 scripts/paired-hosts/test-release-catalog.py` passed three tests.
  Scoped Rust formatting, source-pin comparisons and whitespace checks passed.
- Initial sandboxed Unix socket creation returned EPERM. The socket-dependent
  tests above passed on the approved fixture run.
- Strict Clippy stops at two baseline findings in
  `crates/buzz-core/src/paired_message/body.rs:9` and line 133
  (large enum variant and method naming). This file compares identically to
  the integration baseline. Scoped Clippy at its default warning level
  completed, reporting the same two shared-core warnings and nine distinct
  warnings in the byte-identical fork hostd (argument count and borrows).

## Remaining gates

The independent reviewer owns the source gate. The conductor builds and
checks the matched release packages from a clean final pin. The optional
offline systemd parser test, full repository CI, real provider logins,
live services and migration rehearsals were outside these host checks.
