# #2722 coordinated delivery preparation — Phase 1

**Status: focused actual-relay integration passed; ready for independent review.**
No release, native UI or live chat acceptance is claimed. No production code,
dependency, migration or repository runner changed.

## Ownership and accepted inputs

Workspace `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`; starting clean HEAD
`d7714be862f4514822c303b124a347f5e07dc8c5`. Exclusive source/test ownership
assigned to `/root/host_discovery_delivery_prepare`. Requested configuration
**gpt-6-astra/high**; actual runtime identity not independently observable.

Consumed the mission handoff/spec in `/home/bryce/services/buzz-agents`, portable
conductor contract, project AGENTS/VISION/VISION_AGENT/VISION_REMOTE_AGENTS/TESTING,
[prior execution](2722-host-discovery-replacement.md), its failure receipts, and
[final independent re-gate](../audits/post-audit/2722-host-discovery-replacement.md).
Settled production design and prior seam gates were not reopened.

## Scope and decisions

Added one standard Cargo integration target,
`crates/buzz-relay/tests/host_discovery.rs`, and its local fixture support file.
It links the production relay library without `cfg(test)`, using public
AppState/router and existing authenticated owner/host adapters. This avoids the
obsolete private-home/root-helper unit fixture without editing, deleting,
ignoring or certifying that fixture. No new framework or runtime API.

Estimate was 400–600 test lines; rustfmt produces **744 lines across two test
files** (507 journey + 237 fixture). Conductor explicitly approved that variance
before execution to preserve clean-env setup, actual runtime and cleanup checks.
No production changes or source publication were authorized or performed.

The cached `postgres:17-trixie` runner lacks Python. Conductor approved a task-only
namespace invocation instead of a repository runner change, image pull or
installation. It enters **only the network namespace** of the exact owned,
labeled runner after PID/label recheck, then drops to ordinary UID 1002/GID 1003
with empty environment and temporary HOME/cwd. **The filesystem is the existing
host filesystem under that UID, not container filesystem isolation.** Fixture
code uses temporary account state; no real HOME, login or key state is inspected.
The three existing containers provide isolated loopback PG17/Redis, no host ports,
no external route and no persistent volumes. Exact reproduction is in the
[evidence receipt](evidence/2722-coordinated-delivery-preparation.md).

## What the test proves

Measured one finite scenario through a running Axum relay with actual NIP-42,
signed/encrypted EVENTs, protected REQs, writer admission, owner validation and
host registration/deployment reconciliation:

1. New synthetic owner pairs a fresh host; relay recovery and deployment status
   yield the installed `main/claude` harness and current catalog revision.
2. Actual `Registry::models` publishes; host learns the request through actual
   `messages`/`evidence` REQs. The shared finite runner executes real `buzz-acp
   models --json` against a fake ACP provider and returns exact ID
   `opaque/model:fixture`. Owner checks the actual correlated response.
3. Users, ordinary events, agent operations and placements stay at their baseline;
   host deployment journal and hosted directory are empty, fake service-manager
   call log absent, conversation marker absent. Agent keys/NIP-OA are generated
   in the test only after these assertions.
4. Reopen host journal and republish the exact request through the relay: response
   ID remains identical, only two model mailbox rows exist and discovery ran
   once. A valid signed request with the wrong enrollment parent is refused.
5. Feed the returned ID to existing `Device::prepare_deployment` and
   `Management::dispatch`. Dispatch before main phases is refused; synthetic main
   materialization advances the existing phases, then duplicate exact dispatch
   produces one placement/operation and one start. Actual host runtime rechecks
   readiness/models through ACP and launches the fake conversation harness with
   the exact selected ID, normal fixture HOME/workspace and strict-model flag.
   Owner reads a current `applied` receipt bound to the exact retained authority.
6. Signed Cleanup through the relay returns `cleaned`; staged service instance
   and active marker disappear while synthetic home/login/work files remain.
   Unpair then denies exact query replay, retained model evidence read and local
   host model execution; host reconciliation reaches `dead`.

**Real:** production relay/router/DB/Redis, owner Registry/Device/Management,
host journal/reconciliation, runtime guard and ACP discovery/normalization.
**Fake:** provider login-status executable, provider ACP responder, user-service
manager, conversation harness. The fake manager runs the real runtime and checks
READY. It is not systemd, and the final fake harness does not conduct chat.
Main desktop record creation/publication is represented at its existing owner
phase boundary, not executed by this Rust test. Prior seven real-form outcomes
and native-source evidence remain separate; this is not a native Windows test.
Loopback `connect_to_addr` preserves canonical Host without TLS; TLS is not tested.

## Gates and limitations

All Cargo commands activated Hermit with exported `HERMIT_SELF_UPDATE=false`,
`--offline --locked -j2`; process/socket testing ran sequentially.

| Command / gate | Measured result |
|---|---|
| `cargo test --offline --locked -j2 -p buzz-relay --test host_discovery --no-run` | Final build passed, no warning; production relay library and standalone target compile. |
| Exact test binary through the documented namespace command, `--ignored --exact owner_relay_host_discovery_selected_deploy_and_revoke --nocapture` | **1 scenario passed**, 0 failed/ignored, inner 1.29s. Outer clean-env wrapper also prints 1 pass; these are not two scenarios. |
| `cargo fmt --all --check` | Passed. |
| `git diff --check` including staged new files | Passed. |

Failure receipts: first shell redirect failed because the authorized releases
directory did not exist; created that directory and reran. First compilation
reported a temporary signer closure lifetime in `tokio::join!`; retained a named
closure. Next compile reported an unused membership snapshot; explicitly discarded
it before any client connection. Final compile is clean. Docker inspection first
hit sandbox socket permission; narrow fixture escalation succeeded. Python absence
was measured, then the approved namespace route was used. The first actual journey
run passed; no production predicate was loosened or source mutated for the test.

**Not run / not green:** full `just ci`/`just test`, obsolete inline relay lib tests,
native Windows/non-Windows compilation, human app/provider/chat journey, strict
repository clippy. Prior unchanged lint and Linux GLib/WebKit gaps remain as
recorded upstream. Prior core325/owner17/host28/runtime2/DB1/UI hook3/frontend57
and form7 evidence is retained by source continuity; none of that production or
test source changed here. Runtime/ACP cached debug binaries were reused; hashes
are in the evidence receipt. These are verification tools, not release artifacts.

## Cleanup and handoff

Measured disposable DB count **0** before container removal. Removed exactly the
owned registration Redis/Postgres/runner containers; final label-filtered Docker
listing was empty. All build/test commands terminal; exact integration-binary
process search empty. Fixture child awaited fake manager/runtime completion;
relay server/subscriber tasks were aborted and joined, audit drained and pool
closed. Temporary account and child HOME directories are RAII-removed on the
successful path. No live provider or service calls, installed dependencies,
existing-agent access, board mutation, release/tag/updater, merge, push or deploy.
Starting unrelated worktree changes: none. Final owned files: the two tests,
this execution/evidence record and the focused review brief. No unrelated changes.
Commit IDs and clean terminal handoff are returned separately after signed commit.

Next gate: independent reviewer in this same workspace, using
[focused brief](../briefs/2722-review-coordinated-delivery-preparation.md).
Pause implementation/publication pending conductor continuation. Matching Windows,
relay and hostd artifacts and eventual user-approved rollout remain separate work.
