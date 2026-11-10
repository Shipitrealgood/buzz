# Hosted deployment slice 2 — independent post-audit

2026-11-10. **Merge-block: four source defects require fix-in-lane; native/platform acceptance remains incomplete independently.** Counts: C=0 / H=2 / M=2 / L=0. First review, rework round 0.

## Pin, authority and evidence

Reviewed `bded3a337c4b6defdfddc6815c8c1d4c88a97f1c..3b2acc356a324336bce31d1b66610dce2fe53132` in `/home/bryce/services/buzz-wt-3720-host-registration`, branch `proof/3720-host-registration-events`. Final implementation is `b5fde93d0ecc26ceaee42688b8fefc727a4fb8f4`; the final reviewed commit is documentation only. Requested reviewer configuration: `gpt-6-astra`, high. Served model/effort metadata is unavailable; requested configuration is not runtime identity evidence.

Inputs: project AGENTS/VISION/VISION_REMOTE_AGENTS, testing/database/schema guidance, portable conductor contract and post-audit skill; [review dispatch brief](../../briefs/3738-3739-hosted-deployment-step2-review.md), [executor receipt](../../executions/3738-3739-hosted-deployment-step2.md), and conductor-owned `buzz-agents/docs/specs/2722-first-hosted-agent.md` slice 2 / `docs/briefs/3739-hostd-provider-adapter.md`. Prior slice 1 acceptance was an input, not a new audit assignment.

Scope: private deployment wire/admission/read/fanout, additive persistence and revocation permits, owner journal/main materialization seam, existing host journal/helper reconciliation, affected regressions. Approved third status table, owner-self snapshot, explicit backend, helper extraction, permit linearization and installation deferral were preserved. No lifecycle UI, provider authentication, installer/service isolation redesign, migration-history repair, upstream catchup, or live changes.

Evidence vocabulary: **measured** means executed here; **read** means inspected source; **inferred** means the deterministic failure trace below, not a new executed reproducer. Executor receipts were checked against source and independently rerun where stated. No source mutation or new test framework was introduced. Findings below are read/inferred, high confidence; current passing suites do not cover their failing combinations.

## Findings and dispositions

### H-1 (High) — failing cleanup blocks unrelated recovery — fix-in-lane

**Confidence: high.** The relay always selects the same ordered prefix, including unexpired failed work and cleanup-pending regardless of expiry (`crates/buzz-db/src/store/paired_hosts/deployment.rs:188–191`). The host always takes its first eight (`crates/buzz-hostd/src/deployment.rs:246–251`), with no cursor/rotation. Eight earlier cleanup operations whose helper persistently fails remain cleanup-pending and permanently exclude a ninth valid operation. Merely processing all 32 returned IDs still fails once the retained prefix exceeds 32.

There is also an earlier controller-wide instance of the same class. `cleanup_deployments` selects eight without rotation and returns immediately on the first helper failure (`deployment.rs:423–432`). `retry_deployment_cleanup` propagates that error (`deployment.rs:448–450`). Production `registration_refresh::run` invokes it with `?` before launching any generation, and again inside its central tick (`crates/buzz-hostd/src/registration_refresh.rs:40–45,89–93`). One retained failing cleanup therefore exits the controller before unrelated registration/deployment work can progress. Restart repeats the same failed prefix. Even a slow helper occupies the central discovery loop instead of remaining generation-local.

**Fix:** give existing bounded work enumeration fair progress across operations and generations; retain each failure and its retry state, isolate it from unrelated work, and keep finite per-pass/attempt limits. A bounded continuation/rotation and per-item error retention fit the existing controller. No generic scheduler prerequisite is warranted. Do not discard cleanup or turn revocation into cleanup.

**Regression obligation:** more than eight operations, and enough retained work to cross the relay page limit; first cleanup remains failed across passes/reopen while later authorized work progresses. A failed cleanup in one generation must not prevent another generation's registration/deployment or a later cleanup. Exercise the production scheduling seam, not only a manually repeated single-unit library call. Assert failed intents stay durable and attempts stay bounded.

### H-2 (High) — cleanup expiring before first receipt cannot be recovered — fix-in-lane

**Confidence: high.** Admission permanently fills `cleanup_operation_id`, while every later new operation requires that column to be absent, including another Cleanup (`crates/buzz-db/src/store/paired_hosts/deployment.rs:102–125`). Consider a running agent whose Cleanup is admitted while the host is offline longer than the command lifetime. On first fetch, the host has no persisted cleanup authorization: `expired=true`, `persisted_cleanup=false` (`crates/buzz-hostd/src/deployment.rs:274–277`). It retains `cleanup=0`, correctly refuses the expired effect, and emits Failed (`deployment.rs:317–323,375–384`). The relay then excludes that expired Failed operation from pending enumeration (`paired_hosts/deployment.rs:189`). Offline cleanup cannot select it either (`deployment.rs:423`). A fresh, currently authorized Cleanup at the new revision is rejected by the permanent placement check. The existing service can continue running indefinitely with no protocol recovery path.

**Fix:** allow a new currently authorized, monotonically revisioned Cleanup against an already-fenced placement while continuing to deny every Deploy/resurrection. Preserve immutable prior operation bytes, initial unit identity, placement fence and explicit-intent semantics. Do not extend the expired command's execution authority. Coordinate with M-2 so replacement cleanup remains bounded.

**Regression obligation:** admit cleanup for a running/prepared identity; expire it before first host receipt; prove no expired-command helper effect; then admit a distinct fresh cleanup, complete it under the same unit/key, and prove Deploy remains denied throughout. Assert the old operation remains immutable.

### M-1 (Medium) — early denied replacement leaves the old subscription alive — fix-in-lane

**Confidence: high.** `crates/buzz-relay/src/handlers/host_deployment.rs:77–99` sends `CLOSED` directly for early read denials: disabled capability, ineligible actor, malformed/multiple filters or invalid subscription shape, and per-connection/global semaphore exhaustion. It never retires an existing same-ID subscription. Example: establish a live subscription `x`, then send a multi-filter deployment REQ with ID `x`. The client receives CLOSED but the old holder, registry entry, retained topic and refresh task remain; it can receive another EVENT on `x` after CLOSED. The disabled feature path can similarly leave a previous generic subscription alive.

This violates the existing lifecycle contract at `crates/buzz-relay/src/handlers/close.rs:28–41`. The later asynchronous error branch already uses `close::refuse` (`host_deployment.rs:115–120`), so the early branch is a divergence from available machinery.

**Fix:** route every applicable early REQ refusal through the existing lifecycle-locked refusal/retirement path before CLOSED, preserving EVENT response and COUNT semantics. Handle failed terminal enqueue with the existing cancellation policy. No new transport abstraction is needed.

**Regression obligation:** a real same-ID live holder followed by denied deployment replacement, covering malformed/disabled/capacity paths; assert holder and topic removal and no subsequent EVENT. Existing registration lifecycle tests passing does not validate this new dispatcher branch.

### M-2 (Medium) — ordinary-operation quota permanently prevents first cleanup — fix-in-lane

**Confidence: high.** `crates/buzz-db/src/store/paired_hosts/deployment.rs:113–115` applies the permanent 1,024-per-owner and 4,096-per-community operation count to Cleanup as well as Deploy. No operation pruning exists in this slice. A history of normal deploy/redeploy operations can fill the allowance while an agent is still running. Its first cleanup is then rejected forever; exact replay is no substitute because no cleanup was admitted. The community ceiling can also let other owners consume the remaining recovery capacity.

**Fix:** preserve bounded restrictive capacity tied to existing placements, including bounded replacement after H-2. Reserve or reuse specifically governed recovery capacity without uncapping operations, weakening exact replay/conflict checks, or introducing a retention service. Keep active immutable operation evidence intact.

**Regression obligation:** fill ordinary-operation capacity, prove another Deploy is refused, and prove a first Cleanup for an existing placement is admitted and completes. Cover both owner and community limits, then bound cleanup replacements without recreating an unrecoverable expiry dead end.

| Finding | Conductor disposition | Proposed owner / next action |
| --- | --- | --- |
| H-1 | fix-in-lane | #3739 executor: fair bounded reconciliation and controller failure isolation, then same reviewer re-gate |
| H-2 | fix-in-lane | #3738/#3739 executor: monotonic restrictive replacement and expiry regression |
| M-1 | fix-in-lane | #3738 executor: existing refusal lifecycle plus same-ID negative controls |
| M-2 | fix-in-lane | #3738 executor: bounded cleanup admission capacity alongside H-2 |

Conductor explicitly accepted all four dispositions during this review. No board records were mutated by this reviewer.

## Coverage and retained strengths

**Privacy and admission (read + measured):** kind43104 interception precedes generic parsing (`connection.rs:1333`); generic ingestion rejects it (`handlers/ingest.rs:2563`); private rows live outside generic events. HTTP bridge filters reject private kinds/extensions (`api/bridge.rs:34`), common reader filtering excludes the kind (`handlers/req.rs:1912`), and fanout rejects it (`handlers/event.rs:145`). Traced HTTP query/count/search callers of the shared filter. The actual deployment journey independently exercised admitted owner/host exchange, wrong-owner private denial, explicit generic REQ/COUNT refusal, wildcard history/fanout exclusion and live revocation closure. M-1 is a lifecycle defect, not evidence of cross-owner ciphertext delivery.

**Authority and persistence (read + measured):** writer verifies signed bytes, uses `paired_tx` with community deletion and registration admission lock (`paired_hosts.rs:75`), and rechecks current binding/membership. Placement primary key is community/agent; updates leave first unit identity fixed. DB tests covered exact replay, conflicting bytes/revision, cross-host duplicate placement, removed membership, cleanup/deploy fencing and revoke races. Host checks permit authority/unit/revision and durably consumes nonce before helper (`buzz-hostd/src/deployment.rs:148–185`). Wire tests covered pinned relay/recipient/challenge/freshness, altered signature/content and bounded response framing. Accepted permit issuance linearization remains coherent: no later permit after revoke; revoke alone does not create cleanup.

**Owner/main (read; library behavior measured):** `buzz-paired-owner/src/deployment.rs:57–170` atomically journals exact host ciphertext plus owner-self encrypted materialization; restore binds signed authority and snapshot, and phase/result checks retain identity and terminal evidence. Read native `agents_hosted.rs` and main create/save/publication callers: main generates identity and NIP-OA, uses effective payload projection, persists the explicit backend, restores its original snapshot, and retains the UI gate. Start/provider/stop/delete guards refuse unsupported host lifecycle; no forced-delete fallback was found. Owner tests exercised representative snapshots, not actual native main execution. Native source acceptance is conditional on the platform gate below; these reads are not a Windows compilation claim.

**Host/helper (read + measured):** actual host journal, catalog and helper body were exercised through a Unix socket; systemctl and installation paths/UID were synthetic. PendingLogin remained prepare-only; update/reopen retained key/unit, terminal replay avoided another helper effect, and explicit cleanup fenced before a simulated failure then retried after revoke. Root executable retains root/service-peer checks, trusted template and fixed installed paths (`src/bin/buzz-host-activate.rs:26–47`); the moved body preserves foreign-instance/drop-in rejection. Production opt-in reconciles the same Host journal; `Helper::installed` rejects incompatible installation before execution. The unchanged installed registration service is still not a deployment-capable service installation.

**Schema (read + measured):** migration82 adds only three plain private tables/indexes; admitted writers and catalog/purge/bootstrap inventory are updated. Migration81 is unchanged, SHA256 `7ca03c85b81e1e26af19531fc831bb678a3a914c815999af424254c7cc9446d4`. Populated81→82 registration readers/writers, live catalog parity and migration-mode deletion fence/purge passed. This is not an old-binary boot. Accepted old exact deletion-catalog and auto-migration limitations remain.

Minimalism 9/10; elegance 8/10 because M-1 duplicates an existing refusal boundary and H-1 omits bounded continuation; correctness 7/10 because H-1/H-2/M-1/M-2 violate concrete lifecycle/recovery contracts. The fixed seams and reused journals are useful foundations; no additional prerequisite architecture is proposed.

## Independent verification receipt

All Git/build commands used `export HERMIT_SELF_UPDATE=false` and sourced `./bin/activate-hermit` from repository root. The first Docker attempt was denied by the sandbox; the authorized exact fixture operations then ran under escalation. No denial/zero-test result is counted green. One summary-test launch used the wrong activation directory and failed before execution; the corrected root-activated command below passed.

| Command / existing fixture invocation | Measured outcome |
| --- | --- |
| `cargo test -p buzz-core host_deployment --lib` | 10 passed, 0 ignored |
| `cargo test -p buzz-relay -p buzz-db -p buzz-hostd -p buzz-paired-owner --lib --no-run` | current source built |
| `cargo test -p buzz-hostd -p buzz-paired-owner --no-run` | current affected executable/library tests built |
| proof `run target/debug/deps/buzz_relay-89115045a711bc85 deployment_ --ignored` | 2 passed, 0 ignored, 5.41s |
| proof `run target/debug/deps/buzz_db-417e37ddf5050076 deployment_atomic --ignored` | 1 passed, 0 ignored, 0.26s |
| proof `run-migration …/buzz_db-417e37ddf5050076 migration_schema_deployment_82_preserves_registration_and_matches_desired --ignored` | 1 passed, 0 ignored, 0.96s |
| proof `run-migration …/buzz_db-417e37ddf5050076 store::deletion::postgres_tests::paired_host_tables_obey_fence_and_purge --exact --ignored` | 1 passed, 0 ignored, 0.79s |
| proof `run …/buzz_db-417e37ddf5050076 embedded_migrator_contains_consolidated_initial_schema` | 1 passed, 0 ignored |
| proof `run …/buzz_hostd-0d871f539b36364e` | host library 11 passed |
| proof `run …/buzz_host_activate-c705fc126c9d6ce8` | executable peer check 1 passed |
| proof `run …/buzz_hostd-c6ec734a86c6765b` | controller 12 passed |
| proof `run …/buzz_paired_owner-13b529a6a28bdf14` | owner 15 passed |
| proof `run …/buzz_relay-89115045a711bc85 round1_setup_serializes_close_replacement_and_connection_cleanup --ignored` | existing shared lifecycle regression 1 passed; does not cover M-1 dispatcher |
| `pnpm --dir desktop typecheck` | passed |
| `(cd desktop && node --import ./test-loader.mjs --experimental-strip-types --test src/features/agents/ui/runOnSummary.test.mjs)` | 1 test file passed |
| workspace and native `cargo fmt --check`; `git diff --check` | passed |
| `scripts/test-postgres-test-discovery.sh` | failed on unchanged registration executable inclusion; new deployment tests actually ran above |
| `cargo check --manifest-path desktop/src-tauri/Cargo.toml` | failed before native application compilation: missing glib-2.0/gobject-2.0/gio-2.0 >=2.70 metadata |

Here `proof` means `scripts/3720-registration-proof.sh`; abbreviated binary paths begin `target/debug/deps/`. This review independently executed 56 Rust tests plus the JS test file, with no ignored tests counted as executed. Fixture `up`/`down` succeeded; runner/Postgres/Redis used the existing network-none namespace, no public ports or production volumes.

The executor's finite DropInPaths mutant was **not rerun**. Its receipt records red then restored-green; this reviewer measured restored `activation.rs` SHA256 `8dcef07da2d6e2e0e55467585da02d595da816fd5956c46a3770bc347ffb13cb` and ran the unmutated rejecting test green. Source continuity supports retaining that executor evidence, not claiming an independent mutation kill.

Discovery failure remains `registration_executable_tests.rs:4` requiring discoverable PostgreSQL placement. Both implicated files are unchanged from the review baseline; measured hashes match the receipt: scanner `bcc7a6b5518b931aa4d74f0a6ecdcbfb4cb93e94c57a9fc0a818556afe2dc9e3`, include `32e3de94e0572da848dfd480892dff0dcc6468f494e323862265e651b9283873`.

**Not run:** Windows compilation/native main materialization-save-publication acceptance; human journey; actual systemd, reboot, provider login/models/conversation; full `just ci`/broad integration/JS suites. Strict Clippy was not rerun: receipt's unchanged baseline warnings remain a disclosed red, not reviewer-green. Scoped Biome/file-size results are executor evidence only. No dependency bootstrap or adjacent repair was attempted. Supported native CI and later authorized platform/human acceptance still gate merge/release even after these source fixes. Library recovery and a synthetic helper are not substitutes.

## Ownership return

Measured source range: 61 files, +4,621/−299. Executor's hunk classification reconciles to that total: production +2,785/−271 (net2,514, including 239 helper lines moved), test/proof +1,549/−28, docs +287. Classification is attributed to the executor; gross totals and the changed surface were independently checked. This audit adds only this report.

All review test/build sessions terminated normally, exact disposable resources were removed with the label-verifying `down`, and the source worktree was clean before adding this report. No source fixes, held mutants, live requests, credentials, Claude/Grok/Scout contact, service/account/mount installation, merge, push, PR, deployment or taskctl writes. Return to the same executor for the four bounded repairs, then the same independent reviewer for re-gate of those findings and affected obligations. Do not infer live activation authority from this report.

quality: model=unknown effort=unknown stage=post-audit verdict=merge-block verify=partial commits=bded3a337c4b6defdfddc6815c8c1d4c88a97f1c..3b2acc356a324336bce31d1b66610dce2fe53132 findings=C=0/H=2/M=2/L=0 rework_rounds=0
