# Execution receipt: #3721 host service and discovery

Status: bounded M1–M3 repair for the same independent reviewer’s re-gate. This is not
native service, provider, Windows, online-activation or pilot acceptance.

Workspace `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`. Clean start measured at
`db829357458e90ed434af1487039bedbe0a5ad5f`; product baseline `da66e8d1c`.
No unrelated changes existed at start. Requested configuration: Astra/high;
independent runtime model-routing metadata was not exposed.
Toolchain measured under Hermit (`HERMIT_SELF_UPDATE=false`):
`rustc 1.98.1 (48a229cea 2026-09-01)`.

Accepted inputs: [spec](../specs/3721-host-service-discovery.md),
[executor brief](../briefs/3721-host-service-discovery.md), accepted M-3 repair.
The executor performed no independent merge gate. Next role is the same Chuck re-gate.
The first review at `85e20fc2df0b51817fc0cfe5cc4de7a255de9e5e` required
[M1–M3](../audits/post-audit/3721-host-service-discovery.md); the repair below
supersedes the earlier V4 completion claim.

## Committed source

| Commit | Coherent responsibility |
| --- | --- |
| `9562118dbc72bd05ab1752d2256922bdec870c04` | Shared main model DTO/normalizer; encrypted observation body and legacy decode; approved runtime/harness mapping |
| `e73992dcbbef3cbf2371095b16935792efcfa310` | Same-journal opt-in service binding; fixed same-unit modes/context, bounded supervisor and root-local login; offline source defaults |
| `a2bd462cd415dc5694224f673b8f07f8a1c8a6ff` | Preparation observation polling/immutable journal receipt, current-permission owner accessor, source journeys and bounded-custody refinements |
| `39bbd9e1aab2f6b1e922c4eab567f1a51a8b9b52` | Default-off strict ACP selection through all startup paths and actual-session/no-prompt tests; host-only enablement |

The public Result header, 4 KiB encrypted result limit, registration identity,
placement, retry/cleanup journals and legacy result accessor remain. Discovery
adds no table, query, event kind, cache, daemon or registry. Desktop retains its
process orchestration and re-exports the shared pure model projection.

## Decisions and deviations

- Conductor clarified V4: the shipped policy accepts only `"disabled"`, with
  `PrivateNetwork=yes`; successful online selected-start is N1/native NOT RUN.
  The spec records this clarification. Real finite commands execute offline;
  successful local model enumeration still reports network-disabled. Conversation
  refuses that policy. The future enabling seam requires a reviewed online policy
  variant, unit network configuration and native evidence; no bypass is present.
- The policy credential is root-owned configuration, not an attestation of kernel
  isolation. Empty RootDirectory and explicit installed-artifact binds are source
  configuration; their dependency closure must be reviewed before installation.
- Raw login output is stored only in the unit's RuntimeDirectory while running.
  Terminal StateDirectory output is sanitized. Root uses anchored no-follow reads,
  regular-file/ownership/link/size checks and exact action correlation. This avoids
  adding a cleanup daemon or history store. Native RuntimeDirectory/cgroup behavior
  remains unproven here.
- One current execution descriptor and native-login outcome per existing unit;
  no action history. Login start/cancel replaces prior success with in-progress/
  cancelled. No credential-file heuristic or subscription-entitlement predicate.
- Canonical harness keys (`grok`) use a separate bounded key validator from
  namespaced installed runtime IDs. Initial tests exposed that distinction.
- Decoder boxing resolves a new enum-size warning without changing wire bytes.
  The final self-review also moved mode-artifact creation after capacity admission.

## Finite measured verification

Commands ran in this workspace. `proof` below means the existing
`scripts/3720-registration-proof.sh`, using its owned no-egress runner, disposable
Postgres/Redis and per-run databases. Synthetic manager responses are not service
startup or confinement evidence.

| Command / exact filter | Outcome |
| --- | --- |
| `cargo test -p buzz-core --lib host_deployment` | 12 passed, final rerun 0.06s; private binding, legacy decode, expiry, oversized and encrypted/frame bounds |
| `cargo test -p buzz-core --lib agent_models` | 2 passed; moved existing normalization cases, stable-current precedence |
| `cargo check -p buzz-hostd -p buzz-paired-owner --all-targets` | Passed during contract/integration |
| `cargo test -p buzz-hostd -p buzz-paired-owner -p buzz-relay --no-run` | Final serialized build passed, 3m37s |
| `proof run target/debug/deps/buzz_relay-961a502ccefb2125 external_infra_deployment_tests --ignored --test-threads=1` | 4 passed, 14.45s; typed observation through current private query, exact retry/restart, authorization and M-3 overlapping cleanup |
| `proof run target/debug/deps/buzz_hostd-c33faca283336bb7 registration_refresh::tests --test-threads=1` | 9 passed, 34.91s |
| `proof run target/debug/deps/buzz_paired_owner-36c0dde1cda55f2f --test-threads=1` | 15 passed, 7.95s |
| `target/debug/deps/buzz_hostd-0d871f539b36364e --test-threads=1` with approved local socket access | Final 16 passed, 1 explicitly ignored, 2.05s |
| `BUZZ_TEST_ACP=$PWD/target/debug/buzz-acp target/debug/deps/buzz_hostd-0d871f539b36364e actual_acp_models --ignored --test-threads=1` | Explicit ignored gate: 1 passed, 0.10s; actual unchanged ACP models entrypoint with synthetic peer, initialize/session-new only, private context, empty/error and no prompt |
| `cargo build -p buzz-acp --bin buzz-acp` and `cargo test -p buzz-acp --lib strict_model --no-run` | Passed |
| `env -u BUZZ_ACP_ALLOWED_RESPOND_TO -u BUZZ_ACP_LAZY_POOL -u BUZZ_ACP_IDLE_POOL_SLEEP -u BUZZ_ACP_STRICT_MODEL target/debug/deps/buzz_acp-8f720c4a08edc90e strict_model --test-threads=1` | 2 passed, 0.61s; actual session/prompt path rejects missing/rejected model; already-current and default-off controls |
| Same cleared environment/binary with `model_switch_tests --test-threads=1` | 11 passed, 2.75s |
| `cargo clippy -p buzz-core -p buzz-hostd -p buzz-paired-owner -p buzz-acp --all-targets --no-deps` | Completed; existing warnings. New decode enum warning corrected; core and hostd rerun separately afterward |
| `cargo fmt --all --check`; Tauri manifest fmt check; explicit rustfmt check of external relay fixture; `git diff --check` | Passed |
| `CHECK_FILE_SIZES_BASE=db829357458e90ed434af1487039bedbe0a5ad5f node {desktop,web,mobile}/scripts/check-file-sizes.mjs` (three commands) | Passed with permitted git subprocess access |

Total enumerated behavior tests: **72 passed**. The one ignored host test was
explicitly executed separately; this is not a claim that every package/full CI ran.
The final narrow host tests reran after the capacity/login-state corrections; core
binding tests reran after decoder boxing. Relay evidence predates those final
changes: its successful capacity admission/model-mode path and wire bytes are
unchanged; neither cancelled login nor capacity refusal was its claimed coverage.
Independent review should assess this continuity, not treat the old binary as an
exact-final-build claim.

The original gates covered V1 source artifacts, V2 finite subprocess/context,
V3 synthetic ACP enumeration/strict sessions and V5 regressions. The original
V4 evidence covered private preparation/replay and an injected active lifecycle;
it did **not** cover login → pending refresh → production offline refusal.
The earlier claim of full V4 source completion was incorrect (review M3).
The bounded repair adds that named sequence below. Native V1/V2/V4 portions
remain **NOT RUN**. No external mutation campaign was added.

Observed development failures were corrected: namespaced-vs-canonical harness key
validation; explicit 0700 creation instead of inherited umask; strict CLI fixture
missing its required synthetic key; one stale overlapping compile's old Context
metadata. Final builds were serialized. Sandbox Unix socket and Node git-spawn
EPERM were environment restrictions; the same bounded checks passed with approved
access. Raw logs are not needed to reproduce the table; local build logs remain
under `target/3721-*.log`, and this committed receipt is the durable evidence index.

## Size and resource closure

Measured diff against the clean dispatch pin, excluding tests/docs and accounting
for inline test regions: approximately **1,726 production lines added, 298 removed,
net +1,428 across 24 production Rust/unit/config files**. Raw additions include
112 moved normalizer/DTO lines (the new shared production block is 122 lines with
imports/docs/derive changes), adaptation of the 58 removed old launcher lines, and
73 pool lines reindented around the strict branch. Those are reuse, not independent
new implementations. After these moves/reindentation, newly authored production
is approximately 1.5k lines. The file count exceeds the original 15–22 estimate
because strict startup has three production construction paths and unit/drop-in/
policy artifacts are separate files. The conductor accepted the bounded revised
breakdown before the service group. No subsystem expansion occurred.

All owned build/test processes finished. `proof down` removed the label-verified
runner, Postgres and Redis; subsequent `docker ps -a --filter label=buzz.proof=3720`
returned no containers. No TLS proxy was started. TempDir/process fixtures closed
with their tests. No host manager, live relay, provider, existing-agent credential,
install/deploy/push/merge/PR or taskctl mutation occurred.

NOT RUN: native intended systemd startup/mount/cgroup/file/socket denial, network
policy/DNS/IPv4/IPv6/redirect/host/sibling denial and positive online traffic, reboot,
real Grok login/credential persistence/refresh/entitlement/chat, Windows shared form
and native build, Tauri native tests (known GObject environment gap), full `just ci`,
and human acceptance. N1 remains #3719; provider evidence #3722; Windows #2722.

Next gate: [same-reviewer re-gate brief](../briefs/3721-host-service-discovery-review.md).


## Bounded review repair: M1–M3

Clean repair start: `85e20fc2df0b51817fc0cfe5cc4de7a255de9e5e` (report-only
atop candidate `93aa5b2a4ca655957eafff7f99c2ca541514188a`). Signed source/test
commit: `7517ebf3e7906fc51abe1dfbf468ff3069c944f6`. Requested Astra/high;
served runtime metadata remains unknown. No unrelated changes were present.

- M1: while the previous descriptor is retained and the unit is confirmed inactive,
  reconcile then unlink its exact state/runtime outputs with anchored custody and
  directory sync. Remove/reload the owned finite drop-in before saving conversation
  mode. An interrupted unlink/reload accepts only the exact missing owned path with
  the old finite descriptor and an inactive unit; active or foreign shapes fail.
  Conversation start now explicitly checks inactivity before this handover.
- M2: a stopped login's bounded correlated terminal result updates the existing
  root summary before models or pending refresh supersedes it. Missing terminal
  output turns InProgress into Failed; untrusted/nonterminal output refuses
  replacement. Confirmed failed start (exit or transport error) records Failed.
  No login result is promoted to model availability or subscription entitlement.
- M3: extended the existing fixture with `discovery_login_refresh_and_production_offline_refusal`.
  It drives private preparation, root-local fixed synthetic login, a new pending
  revision before local polling, and selected-start failure using the production
  `NetworkPolicy::require_conversation` guard also called by installed `run()`.
  It checks same secret/OA/agent/unit/HOME, persistent synthetic native state,
  current Succeeded login plus NetworkDisabled models, and one current output
  after refresh/zero after selected handover. The unit fixture repeats refresh
  and cleanup and exercises the interrupted drop-in prefix. No new platform/store.

This V4 source gate uses the production finite supervisor and offline predicate;
it does not execute the installed `run()` environment/credential bootstrap or
systemd. The older simulated-active journey is still a journal/lifecycle regression,
not online execution evidence. Successful online selected-start remains NOT RUN.

| Repair invocation (Hermit, `HERMIT_SELF_UPDATE=false`) | Measured result |
| --- | --- |
| `cargo test -p buzz-hostd -p buzz-relay --no-run` | PASS, 2m52s after correcting a test-only missing `&Path` borrow |
| `target/debug/deps/buzz_hostd-bd1729a8745cad5b --test-threads=1` | PASS, 18; 1 unchanged explicit ignore; 2.26s |
| `proof run target/debug/deps/buzz_relay-961a502ccefb2125 external_infra_deployment_tests --ignored --test-threads=1` | PASS, 5; 14.99s, includes new V4 and old replay/overlapping cleanup |
| `proof run target/debug/deps/buzz_hostd-c33faca283336bb7 registration_refresh::tests --test-threads=1` | PASS, 9; 34.30s |
| `cargo clippy -p buzz-hostd --all-targets --no-deps` | Completed, 4.60s; existing warnings, none in changed production files |
| `cargo fmt --all --check`; explicit relay-fixture `rustfmt --check --edition 2021`; `git diff --check` | PASS |
| Three `{desktop,web,mobile}/scripts/check-file-sizes.mjs`, `CHECK_FILE_SIZES_BASE=85e20fc2df0b51817fc0cfe5cc4de7a255de9e5e` | PASS |

**32 repair behavior tests passed; no behavioral failures.** Initial compile failed
on the new fixture's missing path borrow and was corrected before tests. No source
changed during the successful replacement build or subsequent behavior gates.
Core/wire/normalizer, ACP, owner accessor and desktop have zero source delta from
the reviewed pin; their independently rerun evidence in the review report carries.
The unchanged explicit ACP-models test was not rerun: the finite supervisor,
normalizer and ACP entrypoint bodies are unchanged; the only runtime-context delta
extracts the conversation policy guard. Full 72-test repetition was not performed.

Measured repair delta: **90 added / 20 removed production lines, net +70 in three
files**; **427 added / 47 removed test lines in two existing fixture files**,
including moving the existing injected manager closure for reuse by root-local
login. The one-line production offline guard moved into a documented shared
method. No new source files, schema, policy variants or adjacent subsystem changes.

All repair build/test processes exited. `proof down` removed the label-verified
runner/Postgres/Redis; the following `docker ps -a --filter label=buzz.proof=3720
--format '{{.Names}}'` returned empty. No proxy was started. Fixture TempDirs,
synthetic subprocesses and helper tasks closed. Native/network/provider/Windows/
Tauri/full-CI/human gates above remain NOT RUN. No live changes or board actions.
The same independent reviewer owns the remaining acceptance decision.
