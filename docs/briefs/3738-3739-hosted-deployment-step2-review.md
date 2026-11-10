# Independent review: hosted deployment slice 2 (#3738 / #3739)

Requested by conductor, 2026-11-10. Next gate: the same codex-chuck reviewer, Astra/high, re-gating the four accepted findings below.
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


## Bounded repair re-gate (after first review)

The historical first report is unchanged at
`docs/audits/post-audit/3738-3739-hosted-deployment-step2.md`, committed at
`42c88aa8fe6e739e4eb0c966a7fc8106208bbce7`. Review the repair delta from that
pin, not a new audit campaign. Tested repair source is
`4d8232383ee548bfa1d816f8e9b4a7b4faad0770`; the later handoff commit changes docs
only. The execution receipt records exact commands, actual deltas and resource
closure. Final affected regressions: 25 passed, zero failed/ignored.

- H-1: exclusive rotating pending pages, durable host cursors using existing
  work_cursor, per-item retained failure and bounded independent generation tasks.
  Challenge the forty-item WS/helper journey and forty-generation controller test,
  including restart and deliberately stalled helper response.
- H-2/M-2: first anchor plus one replaceable Cleanup row per placement; separate
  bounded allowance from ordinary Deploy history. Challenge exact current
  authority/revision, expired-before-receipt refusal, fresh replacement recovery,
  stable identity/unit/fence, and quota admission under both owner/community caps.
  The conductor approved lost historical retrieval for retired replacement rows;
  missing-or-superseded is explicit, semantic IDs cannot recreate fresh authority,
  and retained envelope bytes still compare exactly. The reserved `c1ea0001`
  prefix closes reuse as opaque Deploy or foreign first-Cleanup IDs (224-bit
  digest binding); explicit native Deploy input refuses it before effects. No
  forgotten-byte guarantee or unused future allocator.
- M-1: early REQ refusal now calls close::refuse. The same-ID regression covers
  disabled, malformed, multifilter, connection capacity and global capacity, with
  a real admitted prior holder/topic and rejected late delivery after CLOSED.

No new native acceptance is claimed, and previous native/environment gates remain.
No additional mutation experiment or broad baseline suite is requested by default.

## M3-only re-gate after the approved first-principles pause

Base `7db9df3b50cfd119e9dd2355be29fecdc3af2333`; tested repair source
`a59285d2d016e64484e53fa382177b7cf2d59132`; following handoff is documentation only.
Original four findings remain closed. Re-gate only M3 and defects introduced by
its delta. Review existing deployment.rs admission plus shared finish_deployment
and offline receipt path; no coordinator, schema, protocol or relay-store change.

The accepted first-principles rule is monotonic durable host and accepted relay
state with eventual terminal delivery. A previously selected pending frame can
arrive late and be rejected; do not reinstate the superseded zero-late-message
requirement. Replacement receipts bind their own request; no old signature is
relabeled. Exact tuple checks discard superseded completion writers.

The receipt records the deterministic red-before failure and final 24 passing
affected tests, including four held-response interleavings and restart/replay.
Test-only rusqlite edge reuses the existing package for disposable journal reads.
Production +105/-37, test code +370/-26, test config +3. All owned processes and
proof resources are closed before handoff; native/platform gaps remain unrun.
