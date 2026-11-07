# #3720 isolated registration proof — execution result

**Finite repair/probe gates pass; independent re-gate pending.** This is an additive isolated registration prototype. Full #3720/provider/UI/Windows/deployment readiness is not claimed.

Lane `/home/bryce/services/buzz-wt-3720-host-registration`, branch `proof/3720-host-registration-events`; initial clean baseline `723c7e849a65af766a7db8ae8c2b62821f7550b4`. Product commits949063e57,496c4967d,**9005460668bbd3eb0a6cc60095c786bcfee86b67**. Review/repair artifacts are subsequent committed evidence. Requested gpt-6.1-sol/high; runtime identity unavailable. [Operative spec](../specs/3720-host-registration-event-proof.md), [round1 focused handoff](3720-registration-repair-round1.md), [independent findings](../audits/post-audit/3720-registration-proof.md).

The protocol uses the actual root WS, strict separate fork v2 kind43103, generation capability without ordinary host rights, existing paired transaction/admission/binding/transcript authority, protected encrypted pages/live hints and actual hostd/Device journals. Main signature/tenant/restriction/socket/client/clock/lifecycle helpers are reused. Feature defaults off; incompatible enforcing-FI/old/stock relays fail explicitly. Existing v1 HTTP/CLI/UI provisioning remains; no schema or key-custody changes and no second registry store.

| Measured gate | Result / evidence |
|---|---|
| Terminal root WS registration | **24/24**,18.39s; [raw receipt](../verification/3720/round1/ws-final.txt) |
| Affected ordinary connection/REQ | **104 pass/2 ignored**,2.24s; [receipt](../verification/3720/round1/ordinary-affected.txt) |
| Hostd library | **8/8**,1.37s; [short runtime receipt](../verification/3720/round1/hostd-library-short-runtime.txt) |
| Supplemental all-target lint | **40 baseline/0 new/error/0 unexplained**; [inventory](../verification/3720/round1/clippy-inventory.json) |
| Strict lint | Inherited **baseline-blocked**, not green; [raw strict failure](../verification/3720/clippy-strict-baseline-failed.txt) |
| Formatting/whitespace | Passed; [fmt](../verification/3720/round1/fmt-final.txt), [diff](../verification/3720/round1/diff-check.txt) |
| Canonical PostgreSQL discovery | **685 files**; [receipt](../verification/3720/round1/postgres-discovery.txt) |
| Exact-baseline differential size/static | **10 Node self-tests + configured policies**; [receipt](../verification/3720/round1/file-size-exact-base.txt). Root Rust is outside configured roots; no limit raised. |
| Original guard-removal controls | **5 detected**, exact restoration at949063e57; [original index](../verification/3720/controls/index.json), [unchanged-guard continuity](../verification/3720/round1/control-continuity.json) |

Inherited measured evidence, accepted by first review:857 package/source/conformance executions across12 nonempty suites;188 ordinary relay cases/16 ignored;26 exact fresh-DB admission/invite/v1/tenant cases. These were not rerun wholesale. Counts are executions, not distinct tests; reused hostd store cases duplicate targets. Zero-test host-runtime and initially empty focused selector are not credited. Final24 includes original14 plus10 repair probes. [Round1 exact index](../verification/3720/round1/run-index.json).

| Family | Concrete acceptance evidence after round1 |
|---|---|
| P1 | Actual root journey plus complete correlated synthetic wire exchange (G0). |
| P2 | Two independent Linux native journals/same-label distinct hosts, exact consent/nonce/stale host; fresh readiness survives pruning (H-1). |
| P3 | Outer routing controls plus legitimate-signer encrypted contradictions/wrong host/ack/parent/nonce/tenant/recipient (G1). |
| P4 | Direct member/owner/admin, retained invite regressions, valid OA-only and actual guest boundary (G2). |
| P5 | Generation ceiling before/after independent membership, mixed auth/feature-FI refusal and owned CLOSE/replacement lifecycle (M-1/G2). |
| P6 | Existing expiry/attempt/outbox proofs plus delayed async consumption, rollback/boot change, timely saved retry and expired readiness (H-2). |
| P7 | Competing signed acks/unique binding plus v2 DEAD→fresh-G reenrollment (G3). |
| P8 | FENCED/DEAD monotonicity, removed-owner/banned-host policy, durable tombstone/restart/exact retry and no old-G resurrection (G3). |
| P9 | Nonstorage/generic WS/HTTP privacy, matching-author signed HTTP event refusal and downstream source census. |
| P10 | Same/second-node current-policy send barriers, pagination races, lifecycle error/timeout/setup and DB/notification loss (M-1/G4). |
| P11 | Exact durable outboxes/crash windows, actual replacement child process, timer resync and pruning/expired reset (H-1/G3/G4). |
| P12 | Source/principal budgets, held ordinary/global slots/reserved recovery direct+TLS proxy, two DNS tenants and encrypted frame/payload/page edges (G5). |

Each family remains subject to independent re-gate; no whole-family waiver is inferred from one journey. [Exact H/G→test mapping and watchpoints](3720-registration-repair-round1.md). Conductor accepted supported guest-channel ingress equivalence, source census instead of six downstream spies, and idempotent historical ack OK with DEAD unchanged. No exhaustive schedules, throughput campaign or platform certification was requested/run.

Hermit Rust/cargo1.98.1,2 jobs,debug0,incremental1, task-local build target/TMPDIR; short runtime /tmp. Disposable fixtures used network none/shared namespace, loopback only, fresh desired-state DB per run and synthetic TLS. Reproduce with `scripts/3720-registration-proof.sh up`, `tls`, then `run target/debug/deps/<relay-test-artifact> handlers::host_registry::external_infra_registration_tests --skip child_root_relay --ignored --nocapture --test-threads=1`; finish with `down`. Never use inherited/default service URLs. Standard nextest wrapper was unavailable; no schema/history changes.

Historical failed receipts remain dated through original execution/review/round1 artifacts. Round1 last preterminal23/24 was solely the corrected matching-author HTTP status assertion; terminal24/24 follows. Hostd first7/8 hit legacy SUN_LEN under long build TMPDIR; corrected short runtime8/8 passed. Original five complete controls remain valid; two partial scope survivors retain zero kill credit.

**Cleanup measured complete:**0 fixture DBs, four owned services/volumes removed, synthetic private TLS files removed, no tracked build/test process. [Cleanup](../verification/3720/round1/cleanup.json). Worktree/cache retained for sequential review; no live endpoints, deployment files/keys, provider/systemd, board, push or merge touched.

[All-commit/new-file physical-line accounting](../verification/3720/diff-measurement.json) separates production-only Rust, mixed production/inline-test files, standalone tests, verification probe sources, manifests/lock, docs/tooling and receipts/logs, including round1-only delta from b7576dc4a. Mixed is not pure production. No paths retired or old HTTP removal savings claimed. Upstream88697 comparison is context only, excluding pre-existing fork work from the proof delta.

Remaining release-only/unrun gates: full CI/full integration, native Windows/desktop UI, deployed migration/schema parity, real provider/lifecycle, human testing and rollout. Next action is the same reviewer's focused re-gate of existing findings/G0–G5; no additional executor campaign.
