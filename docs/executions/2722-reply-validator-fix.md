# 2722 — reply validator correction execution receipt

Status: implemented and source-tested; independent review and native Windows validation pending. No live failure is claimed fixed.

## Pins and scope

- Workspace: `/home/bryce/services/buzz-wt-3720-host-registration`.
- Branch: `proof/3720-host-registration-events`; starting HEAD `b910a0e03150afa968d14ea93875ad6bc97e7cb1`, measured clean.
- Approved input: `/home/bryce/services/buzz-agents/docs/audits/pre-audit/2722-prepare-refresh-read.md`; context `/home/bryce/services/buzz-agents/docs/briefs/2722-old-canary-jeff-resume.md` and conductor dispatch.
- Requested executor configuration: Astra/high; runtime identity is not exposed in this worker's metadata.
- Only reply lifetime validation, fixed WS categories, and safe setup error projection changed. No service, provider, test1/Jeff, installation/update, remote CI, push, board or release action.

## Changes and decisions

Read: `open_reply` authenticated the event before applying a reader-relative maximum reply lifetime. Changed to checked `Event.created_at + PERMIT_SECONDS`; retained exclusive expiry and the existing 30-second future-event bound, with checked arithmetic for that bound too. Overflow rejects. DB-ahead-signer rejection is explicitly retained, not repaired by extending expiry.

The existing WS `UnexpectedMessage` variant now carries only fixed `reply-invalid`, `reply-duplicate`, `reply-missing`, or `read-closed` for the four existing branches. A 17-line owner-module helper downcasts existing `anyhow` errors and returns static allowlisted categories, timeout/closed categories, or `operation-read-failed`. The desktop setup Err boundary formats only that category and preserves prior-error precedence. This helper is shared only to test the actual projection with the real owner/WS journey without introducing an anyhow dependency into WS or a new diagnostic framework.

Measured code diff (Git added/deleted lines): production **37/11**, tests **115/37**. Four production files: core `host_deployment/wire.rs`, WS `host_deployment.rs`, owner `deployment.rs`, desktop `agents_hosted_commands.rs`. Two existing test files extended: core `wire_tests.rs`, owner `deployment_tests.rs`. Documentation **84/0** lines across two files: this receipt and the [review brief](../briefs/2722-reply-validator-review.md). No unrelated baseline changes existed or were absorbed.

## Verification

All Cargo commands used the existing workspace `target/` cache, `CARGO_BUILD_JOBS=2`, `CARGO_NET_OFFLINE=true`, and `HERMIT_SELF_UPDATE=false` followed by `. ./bin/activate-hermit`. Cached cargo reports 1.98.1. No dependency or tool installation/update.

| Command / check | Measured result |
|---|---|
| `cargo test --offline --locked -p buzz-core host_deployment --lib` | 13 passed. |
| `cargo test --offline --locked -p buzz-core --lib` | 324 passed after mutation restoration. |
| `cargo test --offline --locked -p buzz-paired-owner -p buzz-ws-client --lib` | 16 owner + 5 WS passed with disposable loopback fixtures. Initial sandbox run: 13 owner passed, 3 socket-bind failures (`EPERM`), WS not reached; approved escalation resolved this environment restriction. |
| Temporarily restore reader-relative upper comparison; run core `result_binding_and_permit_challenge_are_not_interchangeable` | Expected failure at status/signed110/expiry120/reader109; restored source in `finally`, full core suite then green. |
| `cargo fmt -p buzz-core -p buzz-ws-client -p buzz-paired-owner -- --check` and `rustfmt --check --edition 2021 desktop/src-tauri/src/commands/agents_hosted_commands.rs` | Passed. |
| `cargo clippy --offline --locked -p buzz-core -p buzz-ws-client -p buzz-paired-owner --all-targets --all-features -- -D warnings` | **Failed**, pre-existing core large_enum_variant and wrong_self_convention in `paired_message/body.rs`, items_after_test_module in `harness/mod.rs`; unchanged code, not repaired. |
| Same clippy command without `-- -D warnings` | Completed exit 0, seven existing warnings total: above three plus owner retain_deployment too_many_arguments, cleanup collapsible_if, and two example explicit_auto_deref warnings. No diagnostic on new code. This is advisory, not a passing strict lint gate. |
| Desktop setup source projection inspection/assertions | Passed: actual Err arm calls helper and interpolates only category, no error string. Static evidence only, not native execution. |
| `pkg-config --exists 'glib-2.0 >= 2.70' 'webkit2gtk-4.1'` | Exit 1. Desktop native tests/check/clippy **not run**: missing Linux prerequisites, no installs. |
| `git diff --check` | Passed. |

Signed wire matrix: status/operation/permit, issuance110/expiry120 accepted at reader109 and110, rejected at120; expiry121 rejected; late signing111 accepted; signer109/expiry120 rejected; future bound140 accepted at reader110 and141 rejected; both arithmetic overflow paths rejected. Wrong nonce/recipient/signer/signature rejected. Existing historical-observation and repeat-read/refresh/exact-retry coverage remains green.

Owner journey uses actual `Management::operation` → `deployment_query` → safe category projection. It exercises invalid, duplicate, missing and CLOSED responses; CLOSED contains `SECRET_SENTINEL`. Arbitrary local, UnexpectedMessage and transport errors carry the same sentinel, including anyhow context, and project only the generic static category. Timeout/connection-closed categories are covered by typed errors, not a five-second timeout simulation.

Local logs, optional and not sole evidence: `/tmp/2722-reply-{core,core-full,owner-ws,owner-ws-loopback,mutation,clippy,clippy-advisory}.log`. All runs are terminal. Full repository `just ci`, database/hostd/service suites, native Windows and live test1 validation **not run** under the bounded dispatch.

## Handoff

Independent reviewer owns the next gate; see the review brief for probes. Executor releases source/test ownership after the signed commit containing this receipt; no detached processes or held mutations remain. Commit succeeded with DCO signoff; pre-commit and commit-msg hook files are absent (measured), so hooks did not run and were not bypassed. Final SHA and clean-status measurement are returned to the conductor. Existing clocks on Windows/DB/signer and the cause of the reported live failure remain unknown. Native Windows validation and any later test1 interaction require the conductor's separately authorized next step.
