# #3750 refresh execution ledger

Starting source: clean `a68c819029004e341fea43f2b527c454396c45b3`, branch
`proof/3720-host-registration-events`, pinned worktree
`/home/bryce/services/buzz-wt-3720-host-registration`.
Executor: native codecodex; requested Astra/high; runtime identity unavailable.
No live/board/systemd changes authorized. Sole source writer/test owner.

Plan: reuse generation AUTH, existing encrypted live REQs, protected reconciliation
and exact journal. <=32 retained sockets, <=one reconciliation/5 seconds per
changing generation, 60-second authoritative backstop, one-second local-only work
inspection, six capped jittered failed attempts. Exact pending writes precede
read-only admission; live subscription precedes snapshot. Preserve frames that
arrive during finite reads. Relay high-water already scopes principal/generation;
its community-wide DB invalidation wake remains unchanged.

Resource baseline: 111 GiB free source filesystem, target 17 GiB, /tmp 4.1 GiB free.
One Cargo lane, CARGO_BUILD_JOBS=2. Verification/evidence pending.

Negative control held: registration_cli.rs restored from a68c81902 for old scheduler build; saved implementation target/3750-refresh-cli.saved. Restore before green tests/commit.

Negative control terminal exit=101; implementation restored from saved source.

Final source gates: 43 library/binary tests,25 isolated registration cases, final executable recheck, formatter and size checks passed. Clippy completed with9 existing warnings; no new refresh diagnostics. Fixture removed; no task containers/private TLS keys remain. No live changes. Release build receipt follows after source commit.
