# #3721 checkpoint-2 source post-audit

Date: 2026-11-10 UTC. **Verdict: rework — three required Medium findings.**
Source acceptance is held for the concrete defects and missing named source gate
below. This verdict does not require native/online success and grants no merge,
deployment, live-system, or whole-pilot acceptance.

Reviewed clean source pin
`db829357458e90ed434af1487039bedbe0a5ad5f..93aa5b2a4ca655957eafff7f99c2ca541514188a`
in `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`. Source commits are `9562118db`,
`e73992dcb`, `a2bd462cd`, `39bbd9e1a`; final pin is documentation only.
Rework rounds: 0. Requested reviewer configuration: `gpt-6-astra`, high.
Served runtime model/effort metadata: unknown; requested configuration is not
independent runtime identity evidence.

Read the project instructions, product/remote-agent vision, repository and ACP
TESTING, complete [spec](../../specs/3721-host-service-discovery.md),
[review brief](../../briefs/3721-host-service-discovery-review.md),
[execution receipt](../../executions/3721-host-service-discovery.md), and current
checkpoint-2 mission. Review scope is the new service/action/discovery/model
selection seams and their affected consumers, not another architecture audit.
N1, native confinement, provider login/chat, Windows, and existing-agent adoption
remain outside source acceptance. No implementation edits or source mutations ran.

## Required findings

### M1 (Medium) — previous action custody is lost on refresh — fix in lane

**High confidence; read-derived control/data-flow finding.** A completed finite
action leaves its sanitized `.buzz-action-<nonce>` in persistent HOME
([runtime_context.rs:333](../../../crates/buzz-hostd/src/runtime_context.rs#L333)).
A new pending operation reaches
[activation.rs:305](../../../crates/buzz-hostd/src/activation.rs#L305), confirms
stop, then calls `restore`. That calls `conversation` first, overwriting the
only protected execution descriptor with a new nonce
([service_actions.rs:111](../../../crates/buzz-hostd/src/service_actions.rs#L111),
[service_actions.rs:161](../../../crates/buzz-hostd/src/service_actions.rs#L161)).
The next models action deletes files using that *new* conversation descriptor
([service_actions.rs:309](../../../crates/buzz-hostd/src/service_actions.rs#L309));
it no longer knows the preceding finite action's filename. Cleanup likewise
knows only the current descriptor
([service_actions.rs:247](../../../crates/buzz-hostd/src/service_actions.rs#L247)).

**Concrete failure:** prepare/discover A, refresh/discover B, refresh/discover C
retains A and B in HOME. Cleanup removes only C. Each further refresh leaves
another result, contradicting the one-current/no-history custody contract. These
are sanitized result files, not demonstrated raw-login credential leakage.

The same ordering has an inconsistent interruption prefix: after the conversation
descriptor is saved but before the finite drop-in is removed, the exact owned
`Restart=no` artifact remains paired with `Mode::Conversation`; `shape` rejects it
([service_actions.rs:67](../../../crates/buzz-hostd/src/service_actions.rs#L67)),
including before cleanup dispatch. A crash in that prefix requires operator
reconciliation instead of normal retry. This is read-derived, not a crash test.

**Minimum repair:** retire the previous correlated output while its descriptor is
still available, after confirmed inactivity, and order existing descriptor/drop-in
updates so interrupted prefixes remain recoverable. Reuse the current helper
transaction and anchored removal; no history store or sweeper. Add an existing
fixture regression for repeated refresh followed by cleanup and the relevant
interrupted prefix.

### M2 (Medium) — completed login can become permanent InProgress — fix in lane

**High confidence; read-derived concrete interleaving, not a measured native race.**
Login start durably records InProgress
([service_actions.rs:318](../../../crates/buzz-hostd/src/service_actions.rs#L318)).
The runtime publishes a finished native outcome, but the root login summary is
updated only when a later `observe` sees it and the unit inactive
([service_actions.rs:371](../../../crates/buzz-hostd/src/service_actions.rs#L371)).
The frontend polls every 500 ms
([buzz-host-activate.rs:64](../../../crates/buzz-hostd/src/bin/buzz-host-activate.rs#L64)).

**Concrete failure:** login exits successfully and the unit becomes inactive;
before the next local poll, a controller models request wins the helper lock.
The request is allowed, deletes the old result and replaces its descriptor at
[service_actions.rs:309](../../../crates/buzz-hostd/src/service_actions.rs#L309).
The local poll now fails correlation. The root summary still says InProgress;
subsequent completed model results copy that stale state at
[service_actions.rs:386](../../../crates/buzz-hostd/src/service_actions.rs#L386).
Pending refresh has the same loss through M1. There is no remaining correlated
result from which ordinary observation can recover the successful outcome.
This causes a false continuing-login state and requires another login invocation.
A start failure similarly leaves InProgress after confirmed abort
([service_actions.rs:331](../../../crates/buzz-hostd/src/service_actions.rs#L331)).

**Minimum repair:** before superseding a stopped login action, validate and commit
its terminal outcome using the existing protected summary (or refuse replacement
until that reconciliation completes). Confirmed start failure must become a
terminal failure. Do not treat missing/untrusted output as success. Cover completed
login → models and completed login → pending refresh before the local observer,
plus failed-start status, in the existing finite-action fixture. This is state
custody repair, not a provider authentication redesign.

### M3 (Medium) — the named V4 journey has not run — fix in lane

**High confidence; read and measured test coverage.** The required
[spec V4](../../specs/3721-host-service-discovery.md#implementation-sequence-and-finite-verification)
is preparation → synthetic fixed login → new pending refresh in the same context
→ truthful selected-start refusal under the shipped offline policy. Existing
relay coverage adds a first preparation observation, but then switches directly
to Selected. Its manager implements `enable` by setting active
([external_infra_deployment_tests.rs:262](../../../crates/buzz-relay/src/handlers/host_registry/external_infra_deployment_tests.rs#L262))
and the journey asserts active
([external_infra_deployment_tests.rs:936](../../../crates/buzz-relay/src/handlers/host_registry/external_infra_deployment_tests.rs#L936)).
It neither executes the runtime's offline conversation guard nor performs the
intervening login/refresh sequence. The separate finite supervisor test executes
login but does not bind this transition to root summary/refresh.

The 72 tests pass; that does not establish this omitted acceptance sequence.
[Execution receipt:90](../../executions/3721-host-service-discovery.md#L90)
overstated V4 completion. Prior selected-start manager simulation remains useful
for older journal tests but cannot supply the new claim.

**Minimum repair:** extend the existing fixture with the named synthetic sequence,
assert identity/unit/HOME continuity, current login outcome and bounded result
retention, and bind the selected-start failure to production offline policy
validation. Keep native systemd enforcement explicitly unrun. Correct the receipt
and review brief to distinguish old injected lifecycle evidence from this new
source gate. No online policy or successful online-start test is requested.

## Dispositions and routing

The conductor explicitly ruled all three **fix in lane** during this review.

| Finding | Required/optional | Owner and next action |
| --- | --- | --- |
| M1 — fix in lane | Required source correctness | Existing executor: retire old action before replacement; preserve recoverable update ordering; bounded regression |
| M2 — fix in lane | Required source correctness | Same executor: preserve observed terminal login across supersession and failed start; bounded regression |
| M3 — fix in lane | Required named source verification | Same executor: complete existing-fixture V4 and correct evidence claims |

Repair these together in the existing action lifecycle; no new manager, history,
protocol, schema, test platform, or native campaign. Return the committed delta
for this reviewer's re-gate of these findings and affected finite obligations.
Counts: C=0 / H=0 / M=3 / L=0. No separate architecture prerequisite was identified.

## Verification and limits

All commands below were independently run at the final source pin with
`HERMIT_SELF_UPDATE=false` and the repository Hermit environment. Test counts
are measured. Local raw logs are `target/3721-review-*.log`; this committed table
is the durable outcome index. The existing proof script supplied isolated
no-egress disposable Postgres/Redis with per-run databases. No live relay/provider
or existing agent credentials were accessed.

| Gate / exact invocation | Independent result |
| --- | --- |
| `cargo test -p buzz-hostd -p buzz-paired-owner -p buzz-relay --no-run` | PASS; final snapshot rebuilt, 1m35s |
| `cargo test -p buzz-core --lib host_deployment` | PASS, 12 |
| `cargo test -p buzz-core --lib agent_models` | PASS, 2 |
| `scripts/3720-registration-proof.sh run target/debug/deps/buzz_relay-961a502ccefb2125 external_infra_deployment_tests --ignored --test-threads=1` | PASS, 4, 14.82s; coverage limit M3 |
| Same runner, `target/debug/deps/buzz_hostd-c33faca283336bb7 registration_refresh::tests --test-threads=1` | PASS, 9, 36.10s |
| Same runner, `target/debug/deps/buzz_paired_owner-36c0dde1cda55f2f --test-threads=1` | PASS, 15, 7.80s |
| `target/debug/deps/buzz_hostd-bd1729a8745cad5b --test-threads=1` | PASS, 16; 1 explicit ignored; 2.18s |
| `cargo build -p buzz-acp --bin buzz-acp`; `cargo test -p buzz-acp --lib strict_model --no-run` | PASS, current ACP binary/tests rebuilt |
| `BUZZ_TEST_ACP=$PWD/target/debug/buzz-acp target/debug/deps/buzz_hostd-bd1729a8745cad5b actual_acp_models --ignored --test-threads=1` | PASS, explicit ignored test 1, 0.10s |
| `target/debug/deps/buzz_acp-8f720c4a08edc90e strict_model --test-threads=1` with four ACP environment variables cleared | PASS, 2, 0.61s |
| Same cleared environment and binary, `model_switch_tests --test-threads=1` | PASS, 11, 2.96s |
| `cargo clippy -p buzz-core -p buzz-hostd -p buzz-paired-owner -p buzz-acp --all-targets --no-deps` | Completed, existing warnings; not warning-free |
| `cargo fmt --all --check`; Tauri manifest fmt; `rustfmt --check --edition 2021 crates/buzz-relay/src/handlers/host_registry/external_infra_deployment_tests.rs` | PASS |
| Three `{desktop,web,mobile}/scripts/check-file-sizes.mjs`, `CHECK_FILE_SIZES_BASE=db829357458e90ed434af1487039bedbe0a5ad5f` | PASS after approved git subprocess access |
| `git diff --check` | PASS |

Total **72 behavior tests passed**. The cleared variables were
`BUZZ_ACP_ALLOWED_RESPOND_TO`, `BUZZ_ACP_LAZY_POOL`, `BUZZ_ACP_IDLE_POOL_SLEEP`,
`BUZZ_ACP_STRICT_MODEL`. Initial sandbox attempts could not access Docker or bind
Unix sockets; the host suite initially reported 15 passed/1 failed/1 ignored
because notification-socket bind returned EPERM. Node file-size checks likewise
hit git-spawn EPERM. The same bounded commands passed with approved access; these
were environment restrictions, not discarded product failures. No mutations ran.

The executor's prior-to-final relay continuity claim is superseded by this fresh
full-pin relay rebuild/run. That run confirms the reported regression tests, not
M3's missing journey. Read checks additionally traced anchored NOFOLLOW/NONBLOCK
reads, regular-file/UID/link/size and exact descriptor validation, root-local login
versus controller models authority, busy refusal before refresh stop, combined
capture bounds, environment clearing, private typed/header-bound legacy-compatible
results, expiry/saved-config/current-permission query, terminal replay/M-3 handling,
and strict selection through the three startup constructors. The shared normalizer
is the moved desktop algorithm, preserving its identifiers and serialization;
strict default-off and actual-session no-prompt controls passed. These strengths
do not remove the supersession defects.

NOT RUN: complete agreed V4 (M3); native intended service startup/mount/cgroup,
file/socket denials, DNS/IP/redirect/network-positive evidence, reboot, actual Grok
login/credentials/refresh/entitlement/chat, Windows shared UI/build, Tauri native
tests, full `just ci`, human acceptance. These native/product deferrals are the
settled #3719/#3722/#2722 boundary, not additional source findings.

Review-owned build/test processes exited. The label-verified proof runner,
Postgres and Redis were removed with `proof down`; no TLS proxy was started.
No implementation files changed; only this report is committed. Workspace returns
to the conductor for the bounded repair at this reviewed source pin plus the
report commit.

quality: model=unknown effort=unknown stage=post-audit verdict=rework verify=partial commits=db829357458e90ed434af1487039bedbe0a5ad5f..93aa5b2a4ca655957eafff7f99c2ca541514188a findings=C=0/H=0/M=3/L=0 rework_rounds=0
