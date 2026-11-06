# #3720 isolated registration proof — bounded execution result

**Status: implemented; bounded runtime tests pass; full proof acceptance remains OPEN.** The P1–P12 table below records covered outcomes and unrun subcases. Neither full #3720 completion, independent security acceptance, Windows compatibility nor deployment readiness is claimed.

Worktree `/home/bryce/services/buzz-wt-3720-host-registration`, branch `proof/3720-host-registration-events`. Initial HEAD `723c7e849a65af766a7db8ae8c2b62821f7550b4` and initial worktree were measured clean. Implementation commits: `949063e57`, then bounded correctness repair `496c4967d`. Later commits contain verification/handoff artifacts only. Comparison upstream is `88697f1c597cda5790d4c702520d0651c48496c0`, not today's main.

Contract: [committed snapshot](../specs/3720-host-registration-event-proof.md), supplied from buzz-agents on2026-11-06. Accepted #3737 responsibility/authority decisions were inputs, not reopened. Requested configuration: **gpt-6.1-sol/high**; actual runtime model/effort is not independently exposed by available metadata.

## What changed and what is reused

- A separate fork kind43103/version2, strict signed peer routing, encrypted payloads/pages, closed REQ extension and explicit NIP-11 capability. Feature defaults off and remains unavailable under enforcing NIP-FI. V1 kind43102/parser/HTTP behavior remains separate.
- Root WS host-scoped NIP-42 uses main's signature/challenge/time/tenant handling and connection lifecycle. Ordinary scopes are cleared and a separate generation ceiling excludes ordinary messages/reads/count/sync, even when the host independently becomes a member. Registration grants no membership, allowlist, agent-owner or profile rows.
- Existing paired writer/admission/restriction lock, unique-live binding, transcript tables and tombstones remain authoritative. V2 operation IDs use existing request_id only for registration records; no provisioning request rows, new registry store, schema or migration changes.
- Protected pages/live invalidations use that same DB policy, existing subscription lifecycle/guarded owner sends, Redis topics and cursor cryptography. Generic storage/result/fanout and HTTP bridge handling exclude the private kind. High-water tokens hide the underlying global sequence.
- A narrow hostd library facade uses the daemon's actual host.sqlite identity, invitation, attempts, transaction and exact outbox code. Native owner v2 uses actual Device journals and the shared WS client. Existing daemon CLI/HTTP provisioning and desktop UI activation remain outside this proof; no old HTTP files were retired.

## Measured verification

| Gate | Actual result | Durable receipt |
|---|---|---|
| Final root WS registration matrix | **14 passed,0 failed**,7.41s | [ws-final-restored.txt](../verification/3720/ws-final-restored.txt) |
| Ordinary relay protocol/subscription/event/read/connection | **188 passed**,16 infrastructure cases ignored | [ordinary-relay-gates.json](../verification/3720/ordinary-relay-gates.json) |
| Core/auth/native owner/hostd/DB writer/conformance/WS suites | **857 passing executions** across12 nonempty suites;678 DB cases ignored | [unit-gates.json](../verification/3720/unit-gates.json) |
| Focused existing admission, human invites, paired-v1 and tenant purge | **26 exact cases passed**, each with a fresh DB | [focused-postgres/index.json](../verification/3720/focused-postgres/index.json) |
| Five complete guard-removal controls | **5 detected**, exact restoration | [controls/index.json](../verification/3720/controls/index.json) |
| Supplemental changed-line clippy | **PASS:40 baseline diagnostics,0 new/error,0 unexplained** | [clippy-inventory.json](../verification/3720/clippy-inventory.json) |
| Strict clippy with -D warnings | **BASELINE-BLOCKED**, not green | [strict failure](../verification/3720/clippy-strict-baseline-failed.txt) |
| Formatting, whitespace and shell parse | Passed | [format](../verification/3720/format-final.txt), [diff](../verification/3720/diff-check-final.txt) |
| Canonical PostgreSQL discovery | Passed over **685 Rust source files** | [discovery](../verification/3720/postgres-discovery-final.txt) |
| Repository differential file-size/static gate | Passed;10 self-tests + all configured policies | [size gate](../verification/3720/file-size-static.txt) |
| Local kind collision at both exact source pins | No matches before allocation | [allocation](../verification/3720/kind-allocation.json) |

Counts are executions, not a claim of distinct independent tests: hostd's reused store tests compile in both library and daemon targets. The host-runtime test binary had0 tests and is **not** credited as validation. One focused infra-free case initially ran0 tests under --ignored; that receipt is retained and a corrected exact run passed. Unit receipts precede the final v2-only revocation repair; unchanged v1 paths retain that evidence, while final root WS and all-target clippy cover the repaired source.

Commands used pinned Hermit Rust/cargo1.98.1, two build jobs, debug0 and task-local target/TMPDIR. Incremental was initially0, then1 on the roomy task disk for the finite controls. No toolchain substitution. Standard scripts/postgres-test-run.sh was **not run**: cargo-nextest is absent ([receipt](../verification/3720/nextest-availability.txt)); focused cases instead ran one exact compiled test per fresh desired-state database through the confined fixture wrapper.

## P1–P12 acceptance map

All named root cases are under `handlers::host_registry::external_infra_registration_tests::external_infra_tests`; each final matrix invocation ran14 cases, excluding the internal child helper. “Partial” means the covered assertions passed while full family acceptance remains open.

| Case | Status and concrete evidence | Unrun or limited evidence |
|---|---|---|
| P1 valid transcript | Registration journey and TLS reconciliation commit exactly one binding; same/second-node live cases observe committed changes. Signed peer EVENT transcripts retained. | Full raw socket trace of AUTH/OK/REQ/EOSE/pages was not captured; peer mutation transcripts are complete. |
| P2 second desktop | Two empty independent native journals, two same-label/distinct-key hosts, exact selected consent, fresh nonce for one and stale other host. | Linux native library seam only; Windows runtime unrun. |
| P3 wrong parties/tenant | **Partial**: wrong owner/control, tenant separation, signed parent/generation/duplicate-tag and borrowed cursor/nonce denial. | Dedicated re-encrypted inner-body mismatch permutations and wrong-host ack packet matrix not separately run. |
| P4 member policy | **Partial**: direct member/owner/admin, open-relay nonmember denial; real invite mint/claim/expiry/replay/issuer-restriction regressions pass. | Dedicated guest/token and valid OA-only registration credential matrix unrun. |
| P5 bootstrap/ceiling | **Partial**: preclaim denial, wrong G/challenge/relay/version, duplicate/mixed scope tags, known generic message deny matrix, independently-member ceiling, no membership/allowlist/users side effects. | Scoped Nostr CLOSE ownership has no dedicated assertion; ordinary close/lifecycle regressions pass. |
| P6 invitation/retry | **Partial**: host unit proof-attempt/expiry/reboot/rollback/outbox tests; actual timely expired ack and retained24h admission boundary. | Dedicated lost/corrupt-local-offer permutations not separately run. Historical retained-claim boundary fixtures are explicitly seeded, followed by real WS admission/mutation. |
| P7 competing owners | **Partial**: simultaneous signed acks produce one live owner; host local proof consumption and same-label identity separation pass. | Full v2 dead-G reuse/re-enrollment journey not separately run; existing v1 reenrollment/tombstone regression passes over reused store logic. |
| P8 revocation | **Partial**: FENCED commit, new work denial, exact historical replay cannot reactivate, local DEAD/outbox, removed-owner minimal recovery, banned-host denial, owner tombstone/outbox settlement. | All status/read/ack interleavings and a dedicated disconnected-host v2 reenrollment sequence unrun. |
| P9 privacy | **Partial**: no private generic-event rows; WS kindless/ID/mixed/count/search, NIP-98 HTTP query/count, unsupported bridge and scoped cursor denial; owner removal live denial. | Dedicated export/feed/push/workflow spy/census run not performed; absence from generic storage is measured, downstream exclusion additionally read in code. |
| P10 live/pages | **Partial**: actual Redis same/second-node live delivery, committed high-water barrier, distinct post-removal tombstone, stale subscriptions denied;16+1 pagination and mutation resync. | Explicit DB/Redis loss injection during snapshots/delivery and every cursor-error permutation unrun. |
| P11 restarts | **Partial**: actual child relay killed before/after commit, exact durable host ack retry, host restart/tombstone, operation-ID conflict, second-device recovery. | Timed lost-notification periodic recovery and pruned/expired-cursor reset not separately exercised. Readiness challenges intentionally require a fresh process-local nonce after restart. |
| P12 bounds/compat | **Partial**: codec page/cursor/depth bounds, recovery under principal/source saturation, direct/TCP/TLS proxy, two DNS tenants, disabled/old/v1 capability rejection and ordinary relay/v1 regressions. | Large concurrent crypto/slot/fanout stress and every size edge unrun; Windows/full UI/platform CI unrun. |

These gaps remain required acceptance work, not automatic deferrals or independent reviewer PASS. The next reviewer should first distinguish inherited evidence from missing probes, then route only targeted work within this finite contract.

## Controls, corrections and deviations

Complete control receipts are pinned to949063e57. Scope removes message-dispatch/handler ceilings **and** clearing ordinary AuthContext scopes; the independently-member host probe then fails. Two partial removals survived due independent membership/empty-scope protections; both survivors are retained and never credited as proving scope. Other controls remove owner membership, accepted-ack parent correlation, both unique-live checks (writer quota + fixture index), and live read reauthorization. Every mutation compiled, ran exactly one failing production-entry test, and restored every source/schema byte before the next control.

Repair496c4967d changes only fresh unpair conflict and native pending-generation/tombstone recovery. It does not change those five control mechanisms. Final14-case green includes both regressions; no claim is made that all controls were independently re-gated on the later docs/repair pin. Reviewer must assess this explicit continuity.

Implementation adaptations: encrypted peer payload uses closed nested header/body objects; global numeric high-water became a tenant/principal/view/G-bound opaque HMAC using the existing relay key; v2 expired readiness records use a five-minute retention window under the existing mailbox quota, keeping latest status/parent and immutable consent/revocation evidence. V1 retains its prior retention/parser semantics. Native refresh is bounded1–60s and snapshots retry at most three times. No second permission service or durable registry was added.

Strict lint failures are in unchanged v1 files; the supplemental gate inventories all diagnostics and fails on every new file/changed-line primary span or error. No global warning allowances ship. Discovery required a canonical inline external_infra wrapper because the checker indexes out-of-line names but omits them in its external-module classification (`scripts/check-postgres-test-discovery.py:348`); failed receipts are retained for conductor-routed tooling follow-up. Root crate Rust files are outside the currently configured desktop/web/mobile size roots; no limit was raised.

## Reproduction and cleanup

Activate Hermit, build the scoped crates with the recorded profile, then:

```bash
scripts/3720-registration-proof.sh up
scripts/3720-registration-proof.sh tls
scripts/3720-registration-proof.sh run target/debug/deps/<relay-lib-test-artifact> \
  handlers::host_registry::external_infra_registration_tests \
  --skip child_root_relay --ignored --nocapture --test-threads=1
# On committed sources only, with the pinned activated cargo in PATH:
python3 scripts/3720-registration-controls.py
scripts/3720-registration-proof.sh down
```

All services were task-labelled, isolated in one Docker --network none namespace, loopback-only, with no host ports or existing networks/volumes. Actual TLS used an ordinary Nginx proxy and fresh synthetic CA. Tool/package fetches happened outside the confined test services. The first build exhausted shared /tmp; only this task's target moved to the task disk and the failed receipt was preserved.

**Measured cleanup:**0 fixture databases remained before service removal; all4 owned containers and their anonymous volumes removed; synthetic TLS private keys removed; runner had only sleep before removal; no cargo/rustc/cargo-clippy or test subprocess remained. [Resource receipt](../verification/3720/resources-before-cleanup.json), [cleanup](../verification/3720/cleanup.json). Worktree and ignored build cache remain for sequential independent review. No live endpoints, deployment .env/journals/keys, systemd units, provider processes, board state, push or merge were touched.

## Diff accounting and next gate

[Exact measured numstat/category/package index](../verification/3720/diff-measurement.json) includes new files and all commits. Rust physical-line delta: **+5,087/−36**, net+5,051: production-only Rust+1,115/−19; mixed production/inline-test files+2,185/−17; standalone tests/fixtures+1,787/−0. Mixed files are not all credited as production. Manifests, lockfile, docs/tooling and large receipt/log volume are separate in that index.

No paths were retired. Old HTTP routes/polling and provisioning code remain; no removal savings are claimed. Upstream88697 comparison is context only and includes pre-existing fork/catch-up work. Source report and next gate: [review brief](3720-registration-proof-review.md). Fresh independent review is required, with remaining finite acceptance probes/repairs still open. No full CI, native Windows, deployed-schema parity or broader provider/lifecycle acceptance is claimed.
