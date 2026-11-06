# #3720 registration proof execution

Status: implementation in progress; no runtime acceptance.

Lane: `/home/bryce/services/buzz-wt-3720-host-registration`, branch `proof/3720-host-registration-events`. Baseline `723c7e849a65af766a7db8ae8c2b62821f7550b4`; initial worktree clean. Comparison upstream `88697f1c597cda5790d4c702520d0651c48496c0`.

Contract: [snapshot](../specs/3720-host-registration-event-proof.md), supplied by conductor from buzz-agents on 2026-11-06. Registration, two independent native journals, recovery and revocation only. No deployment, credentials, provisioning or board mutation. Existing v1 remains supported separately.

Requested configuration: gpt-6.1-sol/high. Runtime model and reasoning are not independently exposed in tool metadata.

## Ledger

- Grounding: read scoped dispatch, accepted #3737 decisions, applicable repository/testing/database instructions and portable worker guidance. Clean baseline measured.
- Tooling: Hermit activation required sandbox escalation for shared tool-cache metadata; pinned rustc/cargo1.98.1 confirmed. Docker29.9.2 available through approved tool escalation. No services started yet.
- Adaptation accepted by conductor: expose the real hostd durable store through a narrow library seam so tests and v2 callers share production state.

## Verification

P1–P12: not run. Negative controls: not run. Independent review: pending fresh reviewer after committed handoff and stopped processes. Native Windows, full CI and human testing: not run.

## Compile and fixture checkpoint

Measured: `cargo check -p buzz-core --locked --offline` passed. `cargo check -p buzz-relay -p buzz-hostd -p buzz-paired-owner --locked --offline` passed after adding the real durable-store library seam. New-field documentation and library dead-code warnings remain to address before clippy. Root-WS tests are now compiling; no runtime proof claimed yet.

The first dependency build exhausted the shared `/tmp` tmpfs. Its failed log is retained at `docs/verification/3720/relay-check-disk-full.txt`. Only this task's build output moved to lane `target/`; subsequent builds use lane `target/tmp`, debug0, incrementalfalse and two jobs. Measured free space after moving: `/tmp`791MiB, lane disk128GiB.

Fixture owner: this executor. Task-labelled containers `buzz-3720-registration-runner`, `buzz-3720-registration-postgres`, `buzz-3720-registration-redis`; IDs respectively `5a4add99d258`, `5f4404a66b77`, `e20e2ac42227`. Runner has Docker `--network none`; both services share that namespace and bind only127.0.0.1:5432/6379. No host ports, external egress, inherited URLs or persistent volumes. PostgreSQL storage is disposable tmpfs. Each test process gets a newly created `buzz_3720_fixture_*` database bootstrapped from `schema/schema.sql`; wrapper drops it on exit. Exact task-name/label checks precede mutation/cleanup. Script: `scripts/3720-registration-proof.sh`.

Owned compile process is currently building the relay test binary. Cleanup obligation: stop/reap owned cargo sessions and `scripts/3720-registration-proof.sh down` before handoff. No live systems or real credentials were accessed.

## First runtime receipts

Measured root WS journey: **1 passed /0 failed**,0.72s (`ws-journey-first.txt`). Real host.sqlite invitations/outboxes, two native Device journals, two same-label hosts, fresh-vs-stale status, exact ack retry, FENCED→DEAD and no generic event rows.

Expanded first matrix: **5 passed /1 failed**,1.68s (`ws-matrix-first.txt`). Passed scope/bootstrap, wrong parties/tenant/member policy, signed parent/generation/idempotency, journey and unique live-owner race. The second-node live assertion consumed a registry-change after owner removal before establishing an ack high-water barrier; whether that frame was already authorized is not proven by this failed run. Correction adds explicit committed high-water barriers on both nodes and a distinct newly committed host tombstone after removal. No post-removal delivery exception is accepted.

## Full WS matrix checkpoint

Measured: **13 passed /0 failed**,6.62s (`ws-matrix-third.txt`). Added both-node live barriers followed by a distinct post-removal commit; actual child-process kill before/after binding commit; expiry/strict AUTH/minimal removed-owner recovery; actual TLS through ordinary Nginx on two locally mapped DNS tenants; hostd reconciliation; cursor race and saturation controls. These remain executor evidence, not independent acceptance.

Conductor accepted one self-review correction after this run: replace exposed global numeric DB high-water with an opaque tenant/principal/view/generation-bound HMAC from the existing relay key. Numeric sequencing remains internal to the same DB and encrypted cursors. This prevents an authorized reader inferring unrelated tenant traffic from gaps in the shared identity sequence. No second authority/store/schema is added. Affected codec/WS gates will rerun.

Additional fixture: `buzz-3720-registration-tls-proxy`, ID `54f732ec7403`, ordinary Nginx image digest `sha256:552e7481ca93ffccd046aa658dbbed22caefbc09c66fa7cd247cbb90b8a5c609`. It shares the same no-egress namespace and listens only127.0.0.1:34443, forwarding to test relay127.0.0.1:32000. Certificates/keys are freshly generated synthetic fixtures under ignored `target/3720-tls`; no deployed keys were read. Cleanup script now owns all four containers.

## Committed implementation checkpoint

Measured final incremental baseline: **14 passed /0 failed**,6.95s (`ws-incremental-baseline.txt`). This includes the opaque high-water change, bounded native refresh/mailbox handling, minimal recovery under ordinary saturation, actual signed-host EVENT denial, owner/admin/direct-member policy, NIP-98 HTTP privacy, and24h retained-ack boundary. Compiler remains Hermit Rust1.98.1; only supported incremental compilation changed from0→1 on the task disk to shorten the five finite controls. No protocol/toolchain substitution.

Measured package/writer/conformance batch:857 actual passing tests in12 nonempty suites (`unit-gates.json`). Host runtime binary has0 tests and is not credited as runtime validation.678 infrastructure-gated DB tests were ignored in that unit run; focused existing paired/admission/deletion cases remain to run separately.

Strict clippy is baseline-blocked in unchanged v1 code. Complete supplemental inventory first found3 new diagnostics, all corrected (helper placement and two single-pattern test matches). Fresh inventory remains pending. The out-of-line external-infra discovery limitation is preserved in `postgres-discovery-out-of-line-failed.txt`; conductor accepted the canonical inline wrapper and will route the checker defect separately. No checker or v1 cleanup was absorbed.

Next: exactly the5 specified negative controls, byte restoration and one terminal green runtime suite, supplemental lint/static receipts, measurement and stopped-resource handoff. Independent review remains pending.

## Finite controls and bounded correctness repairs

All five complete controls were detected by exactly one failing production-entry test each; source/schema bytes restored exactly (`controls/index.json`). Two partial scope removals survived because membership and empty ordinary scopes independently deny the selected host; those survivor receipts are retained and are **not** credited as proof. The complete scope control removes dispatch/handler ceilings plus clearing ordinary NIP-42 scopes, using the independently-member host probe. Conductor approved that exact class-level control. No other mutation classes were added.

Conductor authorized two concrete corrections after restoration: a fresh v2 unpair operation on FENCED now returns a negative correlated OK (exact retained retries remain idempotent), and native owner recovery fetches protected binding/peer evidence for known pending generations absent from the current list. A host-signed DEAD tombstone settles the durable owner outbox; absence alone marks a cached card stale, without inventing revocation. V1 behavior is preserved.

Measured restored final runtime: **14 passed /0 failed**,7.41s (`ws-final-restored.txt`), including exact assertions for both corrections. The five targeted control mechanisms are unchanged by this delta: ordinary scope ceiling, owner membership, status parent matching, pair-ack unique-live protection, and live read authorization. Their receipts remain pinned to949063e57; the final green suite covers the subsequent unpair/native recovery delta. Independent reviewer must assess that continuity, not assume executor security acceptance.
