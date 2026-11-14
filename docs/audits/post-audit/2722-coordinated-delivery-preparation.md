# #2722 actual-relay delivery preparation — independent review

**Merge-safe for this test/evidence delta; C0/H0/M0/L0.** No merge blockers or
optional findings. This closes the assigned actual-relay integration gate, not
native UI, release readiness, provider login or real conversation acceptance.
Minimalism/elegance/correctness: **9/10 each** within that scope.

Reviewed `d7714be862f4514822c303b124a347f5e07dc8c5..f216c0434ba118a50698b21eaf4a6833cb459367`
in `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`, initially clean. Independent first pass,
rework rounds 0, 2026-11-14. Requested configuration: **gpt-6-astra/high**;
actual runtime model/effort unknown. Read portable conductor, applicable project
AGENTS/product/testing/architecture guidance, available local post-audit skill,
[dispatch brief](../../briefs/2722-review-coordinated-delivery-preparation.md),
[execution](../../executions/2722-coordinated-delivery-preparation.md),
[reproduction](../../executions/evidence/2722-coordinated-delivery-preparation.md)
and [accepted prior source review](2722-host-discovery-replacement.md).

## Findings and acceptance

No supported correctness, security or agreed-contract defect found. **Read and
measured:** all 744 test lines and three documents were reviewed; the exact
scenario passed independently. The assertions precede their printed witnesses.

- **Actual transport:** [fixture:12](../../../crates/buzz-relay/tests/host_discovery/fixture.rs#L12)
  constructs production AppState/router, PostgreSQL/Redis and authenticated
  sockets. Only synthetic owner membership is seeded. At
  [journey:158](../../../crates/buzz-relay/tests/host_discovery.rs#L158), DB polling
  schedules host reconciliation; it does not inject/admit the query. Production
  [Registry::models](../../../crates/buzz-paired-owner/src/registry.rs#L858) sends
  EVENT/REQ and checks signed correlation; production
  [reconcile_catalog](../../../crates/buzz-hostd/src/registration.rs#L436) fetches
  messages/evidence and invokes discovery. No fake relay or copied admission gate.
- **Before Create:** [journey:184](../../../crates/buzz-relay/tests/host_discovery.rs#L184)
  asserts runtime/revision/readiness and exact `opaque/model:fixture`, unchanged
  users/events/operations/placements, empty deployment journal/hosted directory,
  no manager/conversation effects and one probe. Agent identity is generated only
  at [line 305](../../../crates/buzz-relay/tests/host_discovery.rs#L305).
- **Replay/refusal:** [journey:252](../../../crates/buzz-relay/tests/host_discovery.rs#L252)
  reopens the host journal and republishes the same signed request; retained reply
  ID, two mailbox rows and unchanged probe count are checked. The separately
  signed wrong-parent query reaches production publish/admission; the writer
  checks its parent against the enrollment ack at
  [writer:299](../../../crates/buzz-db/src/store/paired_hosts.rs#L299).
- **Selected deployment:** [journey:344](../../../crates/buzz-relay/tests/host_discovery.rs#L344)
  uses Device/Management, rejects dispatch before main phases, then asserts exact
  current applied authority, prepared model/agent, one operation/placement/start.
  Main phases are explicitly synthetic. The
  [fake manager](../../../crates/buzz-relay/tests/host_discovery/fixture.rs#L187)
  executes the actual cached runtime and requires successful exit plus READY;
  [run_user:593](../../../crates/buzz-hostd/src/runtime_context.rs#L593) performs
  the fresh ACP/readiness/model check before launch. The
  [fake conversation harness](../../../crates/buzz-relay/tests/host_discovery/fixture.rs#L161)
  asserts exact model, strict selection, HOME/workspace and identity environment.
  This positive scenario executes that guard; it is not a new negative guard
  matrix or mutation-kill claim. Prior accepted guard coverage remains separate.
- **Cleanup/revocation:** [journey:419](../../../crates/buzz-relay/tests/host_discovery.rs#L419)
  checks authenticated Cleanup outcome, removed instance/active marker and
  preserved synthetic account files. Unpair rejects exact query replay, evidence
  read and local probe, and reconciles the host to dead. Those assertions call
  production methods, including active-binding checks before duplicate acceptance
  ([writer:151](../../../crates/buzz-db/src/store/paired_hosts.rs#L151)).

## Independent verification and boundaries

Measured commands used Hermit, exported `HERMIT_SELF_UPDATE=false`, offline locked
Cargo and `-j2`. Local raw logs: `/home/bryce/services/releases/2722-review-build.log`
and `2722-review-run.log`. Durable outcomes:

| Run | Result |
|---|---|
| `cargo test --offline --locked -j2 -p buzz-relay --test host_discovery --no-run` | Exit 0; rebuilt target in 3.45s, no warnings. |
| Inspected task helper, exact `--ignored --exact owner_relay_host_discovery_selected_deploy_and_revoke --nocapture` | Exit 0; **1 passed, 0 failed, 0 ignored**, inner 1.32s. Wrapper also reports 1 pass in 1.33s; only one scenario. All four assertion groups passed. |
| `cargo fmt --all --check`; pinned `git diff --check` | Passed. |

**Measured continuity:** pinned diff is exactly two test files and three documents,
1086 insertions; production/dependency/schema/runner inputs are identical to d771.
Cached `target/debug/buzz-host-runtime` SHA-256 is
`c01d619c6523446e35b9f36b37d417eff11b1cdf0b89ee12875d3373fe52a56f`;
`target/debug/buzz-acp` is
`50393875f45e339fdbe1971fd4b81cfee8590538e07ecbf00440796d36c67b0a`.
Both match the prior receipt. They were reused verification binaries, not newly
built release artifacts. Existing accepted seam suites were not repeated.

**Isolation measured/read:** initial sandbox Docker access was denied; narrow
fixture escalation succeeded. No pre-existing task containers; cached PG17/Redis
images only. The inspected helper verifies exact names/labels/PID and network-none,
then enters only that network namespace under a 150s timeout/5s kill grace, drops
to UID1002/GID1003, clears environment and uses temporary HOME/cwd. Host filesystem
remains accessible to that UID. No host ports, credentials or real account state
were used. The clean-env child has a 120s bound; fake manager/runtime completion
is awaited; relay/subscriber tasks are aborted/joined and audit/pool drained
([wrapper](../../../crates/buzz-relay/tests/host_discovery.rs#L41),
[shutdown](../../../crates/buzz-relay/tests/host_discovery/fixture.rs#L97)).
After the run: disposable DB count **0**, all three owned labeled containers
removed, label-filtered listing and exact test/runtime/fixture-process search
empty. All reviewer processes terminal; unrelated containers untouched.

**Not run/not claimed:** full CI/full relay unit fixture (previous compile block
unchanged), old matrices, strict clippy, native Windows/non-Windows builds, native
main Create, human flow, real login/systemd/chat, TLS or live rollout. Remote
Windows run `43619280528` was neither rerun nor assessed. Provider/login responder,
service manager and final conversation are intentional fakes. Applied/READY here
cannot establish conversational availability with a real provider.

**Routing:** accept this finite gate and transfer workspace/cache ownership back
to conductor for matching Windows/relay/hostd artifact preparation. No executor
repair requested. Remaining native and user-approved live acceptance stays with
the conductor. No source edits, board mutation, publication, build staging, merge
or live/provider/service calls. Only this report is committed; signed report SHA
and final clean status are returned separately.

quality: model=unknown effort=unknown stage=post-audit verdict=merge-safe verify=ran commits=d7714be862f4514822c303b124a347f5e07dc8c5..f216c0434ba118a50698b21eaf4a6833cb459367 findings=C=0/H=0/M=0/L=0 rework_rounds=0
