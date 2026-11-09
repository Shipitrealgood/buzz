# Review scope: retained host registration refresh (#3750)

Date: 2026-11-09. Executor: codecodex; next owner: independent codex-chuck.
Requested model/reasoning: gpt-6-astra/high; runtime identity not independently exposed.

## 1. Where the work lives

Worktree: `/home/bryce/services/buzz-wt-3720-host-registration`.
Branch: `proof/3720-host-registration-events` (not main).
Review range begins at clean `a68c819029004e341fea43f2b527c454396c45b3`;
implementation commit `0bacd58f5369f61a4da1fd794c26d25503117a23`; the final receipt commit changes docs only.
The conductor handoff pins the terminal range. No unrelated starting changes.
[Evidence index](../verification/3750-refresh/run-index.json),
[execution ledger](../verification/3750-refresh/ledger.md).

## 2. What landed

Hostd retains the existing authenticated generation socket and protected live REQ,
verifies hints with the existing encrypted reply codec, and reconciles on changes,
reconnect and a 60-second backstop. Local journal checks run every second without
network reads. The existing single offered/active grant invariant normally means
one retained socket; old restrictive outboxes have independent bounded recovery.
No relay production code, protocol, schema, desktop, HTTP or agent execution changed.

Inputs: accepted #3751 architecture M1, the approved refresh amendment, the
[registration proof](../specs/3720-host-registration-event-proof.md), and accepted
[#3750 platform evidence](../audits/post-audit/3750-registration-integration-platform-addendum.md).
The original Windows/netcup milestone and #3751 pilot close remain in the ops checkout.

## 3. Your job

Independently review this delta and its evidence. Executor checks are not the merge
gate. In particular, probe scope/authority, bounded retries, lifecycle races and
exact outbox continuity. Do not treat this handoff as completion of all #3750.

## 4. Specifically look for

1. **No idle network polling.** `registration_refresh.rs:156` checks timers before
   accepting another frame; notifications coalesce to one reconciliation per five
   seconds, while missed notices recover through the 60-second backstop.
   Probe: sustained hints/Pings must not starve the local deadline or authority read.
2. **Subscribe before snapshot.** `registration/live.rs:19` waits for a verified
   page and EOSE; `registration.rs:314` observes interleaved live frames.
   Probe: clearing dirty state after a read would lose an in-flight invalidation.
3. **Exact writes before reads.** `registration_refresh.rs:126` flushes pending
   writes before subscribing because expired timely-ack admission cannot read.
   `registration.rs:373` completes dead local generations without ordinary reads.
   Probe: offline unpair and lost OK must retry the identical event, never revive.
4. **Generation fairness and capacity.** `registration_refresh.rs:80` caps the
   running set, including retired invitations still in bounded network awaits.
   `registration.rs:249` filters the eight-entry outbox window by generation and
   puts restrictive entries first. Probe: older tombstones must not starve newer
   work; completed dead/spent tasks release sockets. One offered/active grant
   remains enforced by the existing SQLite index.
5. **Hints are not authority.** `registration/live.rs:63` pins relay, query nonce,
   community, recipient and generation through `open_reply`, then checks hint
   operation, completeness and generation. CLOSED/invalid correlated replies fail
   the session. Probe: a hint can schedule a snapshot but cannot change a grant.
6. **Retry and compatibility.** `registration_refresh.rs:99` terminates the sixth
   failed attempt with capped jitter; a full healthy backstop resets the count.
   That terminal error stops the controller, preserving its former operator-retry
   contract; during retries another generation can progress. `:38` preserves
   `--once`; shared WS Ping/Pong and the controller lock remain reused.

All code paths above are under `crates/buzz-hostd/src/`.
The real executable journey is extended at
`crates/buzz-relay/src/handlers/host_registry/registration_executable_tests.rs:180`.

## 5. Decisions and deviations

The old loop used one transient socket at a time. Generation-scoped AUTH requires
separate bounded tasks to keep a new invitation responsive while an old tombstone
retries. At most 32 tasks/sockets may exist; normal retained live count is one.
The 32-work-item query alone was insufficient under rapidly replaced invitations,
so the final scheduler also caps the running set. No new transport was introduced.

`Host::page` keeps its existing shared-reference API; an internal RefCell stores
only ephemeral live-watch state, and no borrow spans an await. New facade helpers
reuse the existing key/journal. Dev dependencies reuse already-locked WS/futures
packages for synthetic socket tests; no dependency versions changed.

The relay still wakes its live DB read on all same-community registry mutations.
Its existing high-water query filters principal/generation, so unrelated changes
produce no host hint. This work does not claim to eliminate server-side fan-out
cost or benchmark a fleet. Five-second coalescing may cause one bounded extra
snapshot after the host's own write; idempotent exact replay prevents write loops.

## 6. Verification and remaining native checks

See the evidence index for exact commands, outcomes, source continuity and limits.
The synthetic WS fixture measures AUTH/pages/Pongs, burst coalescing, snapshot
interleaving, lost-OK replay, local CLI-store changes, scope rejection and failure
isolation. Backstop timing is shortened to two real seconds in its focused test;
the production constant is 60 seconds. No virtual-time claim is made.
The actual executable also runs through the isolated Postgres/Redis/TLS relay.
[Release candidate receipt](../verification/3750-refresh/release-candidate.json) pins
the optimized binary; it was built and help-tested, while the runtime journey used
the final debug binary. Independent review should decide any release runtime gate.

Native Windows and actual systemd checks were not run here. Prior successful real
pairing and ordinary restart observations remain accepted; do not re-pair.
Remaining #3750 observations: safe journal-loss/authorized-second-device recovery;
revocation with terminal persistence; invalid/expired invitation and cancel/retry;
identity/community changes during pending work; unavailable/retry; explicit human
confirmation. Use an isolated profile where a check could alter shared identity.
Actual reboot remains unrun. Automated owner tests do not substitute for these.

## 7. What not to chase; next action

No production access, installation, service changes, credentials, new canary,
Windows rebuild, HTTP retirement, migration, relay redesign or agent adoption.
Existing Claude/Grok/Scout remain outside this work. Full repository CI and strict
baseline lint remediation are separate; no all-project green is claimed.

After independent acceptance, the narrow deployment proposal is a reviewed hostd
binary replacement and restart of only the existing registration controller,
preserving its state, identity, generation, configuration and sandbox. Confirm the
same registration remains visible and the controller retains its connection.
Human approval precedes live changes. This executor performed none.
