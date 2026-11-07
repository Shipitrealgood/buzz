# #3720 registration repair round1 — focused re-gate

## 1. Source and next owner

Same lane `/home/bryce/services/buzz-wt-3720-host-registration`, branch `proof/3720-host-registration-events`. Clean start `b7576dc4ae1400d9688ddcc8b36b44d350703b96`; repaired product commit **9005460668bbd3eb0a6cc60095c786bcfee86b67**. Subsequent commit is docs/tooling/receipts. Requested gpt-6.1-sol/high; runtime identity unavailable. Executor used execute/brief; same independent reviewer owns the focused re-gate. No independent acceptance is claimed here.

## 2. Findings → repairs → measured proofs

All tests below ran within the passing **24-case root WS suite**,18.39s. Names are suffixes under `handlers::host_registry::external_infra_registration_tests::external_infra_tests`. [Exact test map/counts](../verification/3720/round1/run-index.json), [raw terminal result](../verification/3720/round1/ws-final.txt).

| Obligation | Repair / concrete assertion |
|---|---|
| H-1 | Null cursor starts retained snapshot; stale continuation still resets. `round1_pruning_resets_fresh_mailbox_and_rejects_stale_expired_cursors` runs real pruning, rejects pruned/expired cursor, reconciles Host to fresh nonce, retains consent. |
| H-2 | Existing clock refresh after awaits/before durable effects. `round1_reconcile_refreshes_clock_and_keeps_timely_saved_ack_retry`: delayed expiry, rollback, boot change, saved exact ack, expired readiness skipped. |
| M-1 | Existing lifecycle lock covers map/index/topic setup outside timeout cancellation; post-claim failure closes only its owner. `round1_old_errors_and_timeouts_preserve_new_subscription_owner` and `round1_setup_serializes_close_replacement_and_connection_cleanup`: old error/5s timeout, CLOSE/replacement/cancel, no late frames, zero leaked ownership. |
| G0 | `round1_wire_exchange_is_complete_and_correlated`: actual synthetic AUTH/OK/EVENT/REQ/EVENT/EOSE/CLOSE/CLOSED; [complete sanitized trace](../verification/3720/round1/synthetic-wire-exchange.json). |
| G1 | `round1_legitimate_signer_contradictions_have_no_consumer_effect`: legitimate re-encryption/signature for all duplicated authority fields, operation/body, ack/parent/nonce, wrong H/recipient/tenant; consumer rejects before durable/freshness effect; valid follow-up. |
| G2 | `round1_valid_oa_and_channel_guest_cannot_gain_registration`: valid supported OA ordinary profile baseline, OA-only registration denied, direct member still eligible; actual channel guest read/registration denial, mixed scope and enforcing-FI ingress refusal. |
| G3 | `round1_offline_tombstone_restart_retry_and_fresh_generation`: real host learns fence offline, persists DEAD/tombstone before send, reopens SQLite/retries exact pending tombstone after reconnect, fresh G with one live owner; DEAD survives old ack replay, altered legitimate ack/new old-G work denied. |
| G4 | `round1_database_failure_retains_cache_and_lost_notice_recovers_on_timer`: real CLOSED/no late success/EOSE, failed snapshot retains cache, DB recovery; selected committed root notification dropped, bounded native timed resync restores label. H-1 covers cursor reset; crash test now boots replacement child relay before exact ack replay. |
| G5 | `round1_held_work_slots_and_encrypted_boundaries_are_bounded` + existing TLS test: hold3 ordinary slots/configured global queue; reserved binding recovery on direct socket/shared TLS proxy; serialized16KiB edges, exact8KiB payload/24KiB page, malformed/decryption failure, valid follow-up. |
| P9 supplement | Matching-author valid NIP98 `/events` rejects private kind400; [source census](../verification/3720/round1/privacy-source-census.json) plus inherited nonstorage/privacy tests supports downstream exclusion. |

## 3. Specifically inspect the repairs

1. DB `crates/buzz-db/src/store/paired_hosts/registry.rs:225`: fresh retained snapshots may omit pruned transients but retain immutable consent/revocation. No silent continuation clamp.
2. Hostd `crates/buzz-hostd/src/registration.rs:339`,`:357`,`:365`,`:391`: fresh clock at consumption; saved timely ack remains exact/idempotent; restrictive revoke retains rollback semantics.
3. Relay `crates/buzz-relay/src/handlers/host_registry.rs:504`,`:539`,`:583`: setup cannot be cancelled halfway through index/topic effects; owner-fenced timeout/error cannot retire replacement. Existing5s read bound remains.
4. Test-only notification fault at handler`:375`,`:698`; actual crash replacement at test file`:1284`. These are finite fixture seams, not production fault controls.
5. All ten `round1_` probes are in the existing external-infra test module. Prior five mutation controls retain identical removed guard text; [continuity receipt](../verification/3720/round1/control-continuity.json). No controls rerun because their protected mechanisms did not change.

## 4. Decisions / deviations

Conductor accepted real channel guest ingress: no distinct guest bearer/token root route exists. Valid OA requires the supported allow_nip_oa_auth configuration; OA-derived admission is distinct from direct membership. Exact historical ack may return idempotent OK while DEAD remains terminal; this grants no renewed authority. Immediate same-socket tombstone retries can hit its occupied recovery slot, so the durable lost-OK proof reconnects with exact bytes. No host key export, schema, permission service or subscription framework was added.

## 5. Gates and historical failures

Measured: root **24/24**, ordinary connection/REQ **104 pass/2 ignored** (2.24s), hostd library **8/8** (1.37s); supplemental clippy **40 baseline/0 new/error/0 unexplained**. Formatting, whitespace, discovery **685 files**, exact-baseline configured size gate **10 Node self-tests + policies** passed. Strict clippy remains inherited **baseline-blocked**, not green. Earlier package suites remain accepted inherited evidence, not newly rerun executions.

2026-11-06/07 failed receipts remain: initial5-case run3/5, next10-case7/10, full24-case21/24 then23/24. Probe corrections covered DB3s lock timing versus request5s timeout, valid OA configuration, total versus momentarily available permits, valid-base64 edge isolation, matching HTTP author/status and historical ack semantics. Hostd first7/8 hit unchanged SUN_LEN under long build TMPDIR; actual short-/tmp runner retry8/8 passed. Final status replaces those historical failures. No required H/G case is deferred.

## 6. Cleanup and measured cost

[Cleanup](../verification/3720/round1/cleanup.json):0 fixture DBs before teardown; all4 task-labelled no-egress containers/owned volumes removed; synthetic private TLS files deleted; all tracked processes terminal and no cargo/rustc/cargo-clippy remains. Worktree/ignored cache retained for same-lane review. No live service, credentials, migration history, board, push, merge or deployment changes.

[Exact original-baseline and round1-only accounting](../verification/3720/diff-measurement.json) includes all commits/new files, separated production-only/mixed/tests/docs/tooling/probe sources/receipts. Physical lines, not executable SLOC. No paths retired or old HTTP savings claimed. Large receipt volume is separately counted.

## 7. Endpoint and exclusions

Re-gate only existing H-1/H-2/M-1 and G0–G5 against900546066 and terminal receipts; preserve the original [independent report](../audits/post-audit/3720-registration-proof.md). Executor has completed these finite repairs/probes; independent acceptance remains pending. No further executor checks planned.

Not run/no claim: full CI/full integration, nextest wrapper (unavailable), Windows/native desktop UI, deployed schema/migration parity, real provider/lifecycle, human testing or rollout. Wider #3720 remains separate. Exclusive write/test/process ownership is relinquished with the returned final clean HEAD.
