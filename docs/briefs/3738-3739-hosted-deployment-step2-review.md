# Independent review: hosted deployment slice 2 (#3738 / #3739)

Requested by conductor, 2026-11-10. Next agent: fresh codex-chuck, Astra/high.
Executor requested Astra/high; actual runtime identity metadata unavailable.

## Where the work lives

Worktree `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events` (not main). Baseline
`bded3a337c4b6defdfddc6815c8c1d4c88a97f1c`; first milestone
`0cfa9f892db471bfc55d9990f307e4e4c18061af`. Final tested source pin
`b5fde93d0ecc26ceaee42688b8fefc727a4fb8f4`; review baseline through the final
documentation handoff commit. No merge or push.

## What landed

Private kind43104, additive migration82, current-authority permits, exact owner
intents and main-record recovery hooks, and host reconciliation through the
existing identity/journal/catalog/fixed helper. All management gates default off.
The actual relay/DB/owner/host/helper journey runs in the existing isolated proof
fixture. Native main integration is source-only here: Tauri compilation stopped
at missing gobject-2.0 and Windows execution was not run.

Source specification: conductor-owned `buzz-agents/docs/specs/2722-first-hosted-agent.md`.
Rulings and measured commands: [execution receipt](../executions/3738-3739-hosted-deployment-step2.md).

## Your job

Perform independent post-audit. Challenge authority, persistence and effect
ordering; do not infer native acceptance from passing library tests. No live
service, provider, real registration, installation, merge, push or taskctl writes.

**Special attention:** the Windows main-record recovery hook could not be compiled
on this host. It must be independently assessed and run on supported CI before
merge. The journey's snapshot is a representative main-shaped value, not an
executed Tauri main save/publication. Systemctl in the journey is synthetic.

## Specifically look for

1. `host_deployment/wire.rs` and `paired_hosts/deployment.rs`: immutable exact
   bytes, cross-host placement uniqueness, bounded replies, expiry and current
   membership/revocation. Probe the admission lock shared with registration and
   the permit nonce/operation/revision/ciphertext binding. Revocation does not
   retroactively cancel an already-issued bounded attempt.
2. `handlers/host_deployment.rs`: default-off interception before generic parsing,
   ordinary/host AUTH ceilings, private reads and coalesced live reauthorization.
   Probe wildcard/explicit-kind/COUNT and HTTP bypasses, CLOSE replacement and
   cleanup of retained subscriptions. Ordinary generic refusal may be CLOSED;
   the positive private-query control is separately exercised.
3. `buzz-paired-owner/src/deployment.rs`: owner-self snapshot and original host
   ciphertext are atomic; phase changes are monotonic and terminal results cannot
   regress. Probe recovery after expiry and whether native callers always retain
   the original identity rather than entering main key generation again.
4. `commands/agents_hosted.rs` and `agents.rs`: main's own create phases remain
   authority. Probe scope checks around save/publication, identity/snapshot
   binding, unsupported environment/argument/provider handling, and retries when
   main save happened but its receipt did not. RegisteredHost must never enter
   Local/Provider execution or forced deletion. UI guard remains unconditional.
5. `buzz-hostd/src/deployment.rs`: fresh permits are consumed before helper calls;
   same initial operation chooses the unit across revisions. Probe invalid or
   expired work producing durable failure without starving later work, lost
   result replay, and cleanup retry after registration retirement. Revocation
   alone must not manufacture a cleanup intent.
6. `activation.rs` and `buzz-host-activate.rs`: extraction retains root/peer/fixed
   path boundary. Probe conflicting bytes, update identity stability, PendingLogin
   nonlaunchability, closed tombstone before cleanup, observed stop/disable, and
   helper failures. Unknown/unprepared launch material can remain cleanup-pending;
   no success is invented. No data-directory deletion occurs.
7. Migration82/desired schema/deletion manifests: preserve81 byte-for-byte and
   inspect all catalog sites. Current unchanged registration readers/writers were
   tested after expansion; no actual old-binary boot or upstream history bridge
   is claimed. Old exact deletion catalogs intentionally fail closed.

## Deviations and approved refinements

Third private status table; encrypted owner recovery snapshot; explicit backend;
helper extraction; permit issuance as linearization; compatible-installation
precondition deferred to slice3. The receipt records these conductor rulings.
No generic broker/coordinator, second daemon, caller executable/env/path channel,
provider credential reuse, UI create activation or service sandbox change.

## Do not chase

Known migration81 upstream collision, baseline discovery scanner error, baseline
Clippy warnings, missing GLib installation, broad JS provider failure, and live
Claude/Grok/Scout operations. These are not repair authority for this review.
No previous stopped proof directory may be altered.

## Evidence and next gate

Read the receipt's exact commands, counts and negative control. The helper
DropInPaths guard mutation caused the expected failure and was restored byte for
byte. All owned test containers and child processes are stopped before ownership
transfer. Reuse this worktree exclusively; executor work is not an independent
review gate. Return source findings and unsupported-native acceptance gaps to
conductor; no live activation follows from source review.
