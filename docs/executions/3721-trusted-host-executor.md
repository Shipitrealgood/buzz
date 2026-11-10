# #3721 — S1 trusted-host executor receipt

Date: 2026-11-10. Executor: `/root/trusted_host_executor` (`codecodex`). Requested configuration: `gpt-6-astra` / `high`; actual served model/reasoning is not independently exposed. Status: implementation and finite source checks complete; independent review remains the merge gate. No native acceptance or deployment claim.

## Source and scope

Worktree `/home/bryce/services/buzz-wt-3720-host-registration`, branch `proof/3720-host-registration-events`, clean starting revision `b8a9ccd97e62465d804bbc85125ce858a873fe8d`. No pre-existing dirty files. Implementation/test commit: `b21fe0ff8` (DCO). Governing input: [ratified spec S1](../specs/3722-main-discovery-native-login.md#s1--first-finite-code-change-local-user-service-executor), [planning brief](../briefs/3722-claude-host-reuse.md), accepted prior audit, and conductor dispatch. S2/S3 are excluded.

**Read:** existing signed authorization, journal, stable opaque unit ID, encrypted payload/result, retry and reconciliation logic remain in `deployment.rs`. The root Unix-socket implementation was removed from that ordinary deployment adapter. Its `Helper` public name is an alias to the one local `UserService`, not an engine selector. The separate legacy root helper and caller remain; there is no new-route fallback.

**Read and measured:** `--deployment=false` does not construct the executor or read deployment configuration. Registration identity, pair/auth/owner/generation/revocation checks, transport and store code are unchanged. The registration CLI changes only its conditional deployment preflight/help. Refresh passes the actual state directory into that conditional path instead of `/var/lib/buzz-hostd`. Installed registration unit/sandbox files were untouched. Existing agents, registrations, provider credentials, and all live systems were untouched.

## Implementation

- `user_service.rs`: fixed prepare/models/observe/cancel/start/cleanup adapter; per-ID nonblocking flock; account-owned atomic 0600 files and 0700 generated directories; monotonic identity/revision checks; closed markers; confirmed stop before generated artifact deletion; bounded manager output/deadline. Unit names are only `buzz-hosted-agent@<existing opaque ID>.service`.
- `lib.rs`: optional `Catalog.local` configuration for this route, mandatory at its entry. The complete local configuration participates in the existing catalog SHA revision. User runtime checks require available executable files, not root ownership/digests. Legacy verification remains unchanged.
- `runtime_context.rs` and the existing `buzz-host-runtime`: explicit `--user-service <state> <unit>` entry, reusing `Execution`, `ActionResult`, finite output normalization and runner. It executes only Models in S1, under configured HOME/tool PATH/workspace; XDG/Grok overrides are explicitly local. No provider credential file is read/copied. The finite child receives no Buzz key or auth tag. Per-agent credential copies remain available to the fixed runtime through the existing systemd credential-file pattern.
- Finite supervision now additionally kills the child process group, including descendants after direct-child exit. The user template uses `KillMode=control-group`, bounded startup/stop, null output, existing memory/task/CPU limits and `Restart=no`. Observation requires inactive service/task state before exposing a terminal result.
- New route login actions refuse. New route conversation/start refuses until S2 current readiness exists; there is no fabricated successful login receipt. Existing feature flags and UI are unchanged.
- Cleanup retains a fencing marker and retry context on failed stop. It removes only recorded instance/execution/result and exact drop-in files. It preserves the shared template, per-ID lock/tombstone, account HOME/provider fixtures, assigned repository, and unrelated units. Unknown files or symlink escapes refuse cleanup rather than being recursively removed.

The local file is `<controller-state>/deployment.json`, using the existing Catalog/runtime mapping plus `local` (`home`, `workspace`, `tool_path`, optional XDG/Grok `environment`, `unit_dir`, `launcher`, `systemctl`). No remote wire fields or new provider registry were added. No installer or actual service installation ran. Local service location/launcher changes for an existing prepared unit refuse; cleanup uses the saved context, independent of current catalog revisions. Operator argv/tool/runtime changes within that placement use the existing revision fence.

## Verification — measured

All commands ran with Hermit and `HERMIT_SELF_UPDATE=false`, offline Cargo, without installing toolchains. Socket tests used narrowly authorized sandbox escalation solely for disposable Unix/TCP fixtures and fake commands.

| Gate / exact command | Result |
| --- | --- |
| `cargo check --offline -p buzz-hostd --all-targets` | PASS |
| `cargo test --offline -p buzz-hostd --lib user_service -- --test-threads=1` | Initial 4 pass / 1 sandbox notification-socket EPERM; subsequent source suite covers all five green |
| `cargo test --offline -p buzz-hostd --lib trusted_finite_runner -- --test-threads=1` | PASS 1 after correcting the fixture's normalized JSON to actual raw ACP output shape |
| `cargo test --offline -p buzz-hostd --all-targets -- --test-threads=1` | Final production source: library **23 pass / 1 ignored**, activation binary **1 pass**, runtime binary 0 tests; controller **12 pass / 1 fixture timing failure**. The controller fixture observed manager arrival before the durable issue write; correction waits for the unchanged >=39 durable-failure assertion. No product change followed this run, only fixture timing and comments. |
| `cargo test --offline -p buzz-hostd --bin buzz-hostd -- --test-threads=1` | Final controller rerun **13 PASS**, including registration-only auth/heartbeat/revoke/reconnect and slow-cleanup scheduling |
| `cargo test --offline -p buzz-core --lib host_deployment -- --test-threads=1` | **12 PASS**; 302 unrelated cases filtered |
| `cargo build --offline -p buzz-acp --bin buzz-acp` | PASS, existing main harness |
| `BUZZ_TEST_ACP="$PWD/target/debug/buzz-acp" cargo test --offline -p buzz-hostd --lib actual_acp_models -- --ignored --test-threads=1` | **1 PASS**, existing synthetic ACP fixture: six Grok/Claude model invocations, no real adapter/provider |
| `cargo clippy --offline -p buzz-hostd --all-targets -- -D warnings` | BLOCKED in unchanged `buzz-core/src/paired_message/body.rs`: `large_enum_variant` and `wrong_self_convention` |
| `cargo clippy --offline --no-deps -p buzz-hostd --all-targets` | PASS with existing warnings only: too-many-arguments in request/transport, needless-borrow in onboard/transport, formatting borrows in registration_cli/main. No warning in new/changed implementation lines. Strict warning-free lint is not claimed. |
| `cargo fmt -p buzz-hostd -- --check`; `git diff --check`; local document links | PASS; 27 local targets/line bounds validated. One stale upstream brief cleanup line was updated after the executor deletion shifted it. |
| `CHECK_FILE_SIZES_BASE=b8a9ccd97e62465d804bbc85125ce858a873fe8d node {desktop,web,mobile}/scripts/check-file-sizes.mjs` (three separate commands) | PASS, exit 0 each after Node/git sandbox escalation. Those existing policies cover their named surfaces, not backend Rust; no backend size exemption was added. Initial node wrapper failed without exported Hermit flag, then sandbox blocked git spawn; both failures were resolved without installs. |

Counts are test executions per suite, not a claim of unique tests summed across library/binary duplicate store modules. Baseline library contains 19 cases (18 default + 1 ignored); S1 adds five. The controller retains its 13 cases. No legacy tests were removed.

The five new production-bound cases cover prepare/replay/reopen stable identity and distinct Buzz keys; scope/config revision/unit refusal before effects; start/login refusal; finite exclusion/nonce correlation/cancel retry; failed-stop durable cleanup and exact preservation/symlink rejection; normal non-root HOME/PATH/argv/workspace with bounded output and timed-out descendant termination. A static route/template assertion supplements those behavior tests; it is not native systemd proof.

**NOT RUN:** full `just ci`, Desktop/Windows/native platform builds, real systemd/user-manager lifecycle, boot/login persistence, installed native providers/login/readiness/quota/chat, relay/database live integration, human acceptance. These need their own scoped environment/authorization. No actual `systemctl`, live service, provider, firewall, deployment, push or merge operation ran.

## Size and deviations

Measured against the clean input, excluding documents: production-bearing files **+708 / -114 (net +594)**; test-only files **+491 / -35 (net +456)**. The new executor is **540 lines**, user template **20**, new test file **396**. Existing `deployment.rs` is **+6/-97**; runtime context **+101/-8** including comments. These are literal diff/file line counts, not a claim that all additions are novel executable logic. No whole functions/files were mechanically moved; existing journal and finite normalization/runner are called in place. Test fixture reuse needed one visibility change and optional-field initializers in existing fixtures. Legacy code is retained solely for its separate caller, not reported as deleted.

Implementation choices within S1: use the existing runtime executable rather than a new binary; fixed generated per-instance drop-in supplies local paths and credential files; shared template persists after per-agent cleanup; finite actions are never enabled for reboot. Conversation launch/restart remains deliberately blocked, while prepare retry/reopen keeps the same key and unit. No generic service manager, auth broker, migration or new protocol was required.

## Remaining work and handoff

Smallest S2: shared main registry/readiness facts, same-account current readiness/model refresh and strict launch recheck, then enable the conversation path under that truthful guard. Smallest S3: existing private DTO/UI projection and matched gated rollout. Neither is hidden in this patch. Native rollout requires a separate exact approval and disposable new-agent test; existing agents stay excluded.

Next gate: fresh independent reviewer on the committed range using [review brief](../briefs/3721-trusted-host-executor-review.md). No open product/architecture question was introduced. Implementation commit is pinned above; the documentation commit/final clean status are returned by the executor. All test/build sessions completed; the targeted process check found no remaining disposable manager/model fixture children. Exclusive write/test ownership is released on return.
