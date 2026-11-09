# #3750 registration refresh — independent post-audit

Date: 2026-11-09. Reviewer: codex-chuck. Requested configuration: **gpt-6-astra / high**;
served model and runtime effort are not independently exposed. Rework round: **0**.

## Verdict and scope

**Merge-safe for the scoped registration refresh candidate. C=0 / H=0 / M=0 / L=0.**
No supported merge blocker or required repair. This accepts the reviewed increment
for a later human-approved controller update; it does not close #3750, attest human
Windows acceptance, or authorize installation.

Reviewed range: `a68c819029004e341fea43f2b527c454396c45b3` →
`da4e8c3acab16af005b15f03bebc650856fc4343`. Implementation:
`0bacd58f5369f61a4da1fd794c26d25503117a23`; the later commit contains receipts/docs.
Workspace: `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`. The initial read-only stage ended at the
conductor's explicit exclusive test/report ownership handoff. No other writer or
build lane overlapped this review.

Contract: [executor scope and acceptance](../../briefs/3750-registration-refresh.md),
accepted #3751 architecture M1 and the November 9 amendment in the ops checkout's
`docs/specs/windows-netcup-registration-milestone.md:7–33`. The existing private
event architecture and successful pilot pairing/ordinary restart are accepted
inputs. No protocol, relay production, database schema, desktop, shared-client
production, provisioning, or agent lifecycle redesign is in this review.
No SSH, production access, taskctl mutation, deployment, or agent changes occurred.

**Evidence labels:** measured = executed independently here; read = inspected
source or existing receipt; inferred = conclusion from that evidence. Detailed
commands/results are in the [review run index](../../verification/3750-refresh-review/run-index.json).

## Contract coverage

| Obligation | Independent assessment and evidence |
|---|---|
| Idle AUTH/read amortization | **Measured:** 12 real seconds with AUTH=1, live REQ=1, finite pages=2, Pongs=12, open socket=1; this invokes production `run`, with production timing constants. The exact optimized executable retained the same relay connection UUID across six idle seconds and the status flow. [Libraries receipt](../../verification/3750-refresh-review/libraries.txt), [release journey](../../verification/3750-refresh-review/registration-release.txt); [scheduler](../../../crates/buzz-hostd/src/registration_refresh.rs#L156), [executable assertions](../../../crates/buzz-relay/src/handlers/host_registry/registration_executable_tests.rs#L180). |
| Subscribe before snapshot; interleaved hints; bounded recovery | **Read:** initial protected live page/EOSE precede `connected`; dirty state is consumed before reconciliation, and finite pages observe other live frames. Timers are checked before another frame and an outer deadline bounds the shared reader's Ping loop. **Measured:** burst coalescing, hint-during-snapshot, silent-revocation backstop, plus continuous traffic control: 473 Pongs, one AUTH, terminal local dead in 3003ms with a two-second injected backstop. This is real time, not virtual time or a 60-second measurement. [Live state](../../../crates/buzz-hostd/src/registration/live.rs#L19), [page receiver](../../../crates/buzz-hostd/src/registration.rs#L311), [probe source](../../verification/3750-refresh-review/heartbeat-control.rs.txt), [probe result](../../verification/3750-refresh-review/heartbeat-control.txt). |
| Exact writes before protected reads; restrictive progress | **Read:** reconnect flush precedes live subscription; `reconcile` flushes before active binding reads and returns after a dead-generation flush. Per-generation selection occurs before the eight-row limit, with restrictive entries first. **Measured:** lost-OK sends the same tombstone twice before any read; generation-window test selects newest restrictive work despite ten older generations; existing real-relay cases cover timely expired ack admission, offline tombstone retry, and terminal non-resurrection. [Flush ordering](../../../crates/buzz-hostd/src/registration_refresh.rs#L121), [outbox](../../../crates/buzz-hostd/src/registration.rs#L249), [libraries](../../verification/3750-refresh-review/libraries.txt), [registration suite](../../verification/3750-refresh-review/registration-release.txt). |
| Scope and authority | **Read:** existing AUTH remains generation-scoped. `open_reply` checks signer, signature, ciphertext, recipient, community, exact query including nonce/generation, and outer tags; live handling requires a complete correlated page/change and matching changed generation. A hint changes only ephemeral dirty/high-water state. **Measured:** untrusted signer, other community/generation, malformed hint rejection; existing real-relay scope/party/tenant/privacy cases passed. [Verifier](../../../crates/buzz-core/src/host_registry.rs#L525), [hint handling](../../../crates/buzz-hostd/src/registration/live.rs#L63), [AUTH](../../../crates/buzz-hostd/src/registration.rs#L65), [receipts](../../verification/3750-refresh-review/run-index.json). |
| Local changes, fairness, resource cleanup | **Read:** individual local tasks own SQLite handles and generation-scoped connections. The actual running set, including retired tasks awaiting the network, is capped at 32; this is a source proof, not a measured 32-socket stress claim. One offered/active grant remains protected by the unchanged SQLite index. Local eligibility/pending checks are network-free, and completed dead/spent generations leave the running set. **Measured:** local revoke/new offer while connected, retired socket closure, and failed older write not blocking new offer subscription. [Running set](../../../crates/buzz-hostd/src/registration_refresh.rs#L66), [local checks](../../../crates/buzz-hostd/src/registration_refresh.rs#L171), [one-grant index](../../../crates/buzz-hostd/src/store.rs#L26), [libraries](../../verification/3750-refresh-review/libraries.txt). |
| Retry/compatibility and self-notices | **Read:** jitter is bounded, the sixth failed attempt exits the controller, and a session lasting a backstop interval resets the failure count; global terminal exit is the expressly retained operator-retry policy. `--once` retains a bounded transient pass, existing controller locking and shared Ping/Pong handling. Exact replay does not create new status rows, and relay generation high-water suppresses unchanged hints. **Measured:** jitter bounds, actual `--once`/singleton/restart journey, one status response without additional status rows over six seconds, and notification-driven revoke before its ten-second assertion bound. [Retry/once](../../../crates/buzz-hostd/src/registration_refresh.rs#L38), [lock](../../../crates/buzz-hostd/src/registration_cli.rs#L105), [relay high-water](../../../crates/buzz-relay/src/handlers/host_registry.rs#L629), [release journey](../../verification/3750-refresh-review/registration-release.txt). |
| Source/artifact continuity and cleanup | **Measured:** all nine source-manifest hashes matched before testing and after restoring the temporary probe. Release SHA256 `de05655071c5f6af3451cc70116969f53da0df0ff64448593a0fc913bf10d3ee` matched the candidate and executable path used by the fixture. Debug executable restored to its original hash. Fixture containers and private synthetic TLS material removed. [Substitution hashes](../../verification/3750-refresh-review/executable-substitution.txt), [restoration/cleanup](../../verification/3750-refresh-review/source-restoration.json). |

## Verification and falsification

**Measured:** 43 host/owner/shared-WS library/binary tests; 25 scoped isolated
registration cases on the final source; one additional continuous-heartbeat
control. All passed. The 25-case run includes the optimized candidate's actual
command journey through the isolated TLS relay, Postgres and Redis. The fixture
hardcodes `target/debug/buzz-hostd`; the reviewer temporarily substituted the exact
candidate there, then restored the original debug bytes in an EXIT trap. No source
was changed for that substitution. The test-runner binary was prepared against
the pinned source with locked offline dependencies before execution.

The heartbeat probe temporarily appended one test to the existing synthetic
fixture, called production `connected`, and was removed immediately after its
targeted run. Its source and result are preserved; all original source bytes were
rechecked. No production mutation or new verification framework was introduced.
The fixture/control processes are terminal and no review-owned daemon remains.

**Read and accepted:** the executor's old-scheduler control built successfully,
then failed the real executable's retained-connection assertion (observed 0,
expected 1). Its recorded restoration and final debug green agree with the
unchanged manifest and independent release green. This is a valid discriminating
negative control; it was not unnecessarily repeated. [Build receipt](../../verification/3750-refresh/old-scheduler-build.txt),
[expected red](../../verification/3750-refresh/old-scheduler-red.txt),
[executor final green](../../verification/3750-refresh/final-executable.txt).

**Measured:** workspace formatting, explicitly included executable-test formatting,
and whitespace checks passed. **Read and reused:** executor differential size
check and non-strict clippy receipts on identical source; clippy reports nine
existing diagnostics, not a warning-free gate. [Size receipt](../../verification/3750-refresh/size.txt),
[clippy receipt](../../verification/3750-refresh/clippy.txt).

**Not run:** full `just ci`, strict baseline clippy remediation, 104 unchanged
ordinary relay cases, native Windows UI/recovery/revoke/negative/scope observations,
actual systemd/reboot, human acceptance, live installation, and a fleet benchmark.
These remain disclosed limits, not green gates. Shortened timer tests do not
measure the production 60-second interval; its value and use were read directly.
No served-model or production availability claim follows from these results.

## Quality, compatibility, and routing

**Inferred:** minimalism **9/10**, elegance **9/10**, correctness **9/10** for the
scoped increment. The change reuses authentication, reply codec, protected reads,
journal, exact replay and transport heartbeat handling. The new scheduler is
justified by independent generation recovery; the small live-state module keeps
notification validation separate from authoritative reconciliation. No missing
foundation or structural workaround was found. The remaining server-side
same-community database wakes are disclosed accepted cost, not claimed eliminated.

**Read/inferred:** the conductor's proposed rollout in
`/home/bryce/services/buzz-agents/docs/briefs/3750-controller-refresh-rollout.md`
is compatible with this change: no schema/wire/key-storage change, executable-only
rollback retaining the current journal, and no stale backup restoration or new
pairing. Live binary/unit/configuration compatibility must still be inspected after
approval; the historical installed hash is not a fresh measurement. This review
did not rehearse operational steps or inspect the live host.

**Measured:** the conductor's durable copy at
`/home/bryce/services/buzz-agents/releases/3750-refresh-de05655071c5f6af/buzz-hostd`
has the same verified release SHA256. Its `SOURCE.json` identifies the reviewed
implementation and records that installation and approval remain pending.

**Routing:** accept this increment and retain the exact candidate for a separate
human-approved controller update. Required repairs: none. Optional new followups:
none. Continue the already-open native Windows and human observations under #3750;
do not relitigate successful #3751 pairing or ordinary restart, or mark the full
milestone complete. No board disposition or merge was performed by this reviewer.

quality: model=unknown effort=unknown stage=post-audit verdict=merge-safe verify=ran commits=a68c819029004e341fea43f2b527c454396c45b3..da4e8c3acab16af005b15f03bebc650856fc4343 findings=C=0/H=0/M=0/L=0 rework_rounds=0
