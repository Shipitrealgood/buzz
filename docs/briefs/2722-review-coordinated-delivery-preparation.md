# Independent review: actual-relay discovery proof (#2722)

Requested 2026-11-14 by conductor `/root`; next owner: fresh independent reviewer.

## 1. Where the work lives

- Worktree: `/home/bryce/services/buzz-wt-3720-host-registration`.
- Branch: `proof/3720-host-registration-events`, not main.
- Review range: `d7714be862f4514822c303b124a347f5e07dc8c5..HEAD`; executor returns
  exact signed commit separately. Only two test files and three documents.
- Starting unrelated changes: none; production source remains identical to d771.

```bash
cd /home/bryce/services/buzz-wt-3720-host-registration
export HERMIT_SELF_UPDATE=false
. ./bin/activate-hermit
git diff --stat d7714be862f4514822c303b124a347f5e07dc8c5..HEAD
```

## 2. What landed

One standalone Cargo integration test proves actual authenticated owner discovery
through the running relay into host reconciliation, exact returned model into
existing owner Deploy, and host runtime guard/cleanup with synthetic services.
It avoids compiling the obsolete relay unit fixture without changing it.
No production/dependency/migration/repository runner changes. Read the
[execution record](../executions/2722-coordinated-delivery-preparation.md) and
[exact evidence/reproduction](../executions/evidence/2722-coordinated-delivery-preparation.md).
Prior design and source review remain in
[accepted re-gate](../audits/post-audit/2722-host-discovery-replacement.md).

## 3. Your job

Assess whether the test binds the actual missing transport seam and whether the
receipt overstates its evidence. Exercise this one finite target independently;
no broad audit restart. Do not accept the executor's pass as your independent gate.

**Special attention:** native main materialization and real conversation are
explicit fixture boundaries. A current applied result here means host deployment
with a fake manager/harness; it cannot certify native Windows Create, actual
systemd, provider login or real chat. The test does execute the existing owner
phase/dispatch API and real host runtime/ACP guard after model discovery.

## 4. Specifically look for

1. **Actual transport rather than injected query.**
   Probe `crates/buzz-relay/tests/host_discovery.rs:158`: owner `Registry::models`
   publishes independently; bounded DB polling only schedules the host pass.
   Host obtains request through production messages/evidence REQs. Probe
   `tests/host_discovery/fixture.rs:12` and `:55` relative to `crates/buzz-relay`:
   production AppState and router, real sockets, real DB/Redis. No fake relay.
2. **No pre-Create effects and exact correlation.**
   Probe `host_discovery.rs:154` and `:184`: baseline users/events/operations/
   placements, empty host deployment journal/hosted directory and absent service
   calls/conversation marker. Agent key generation begins at `:305`. Runtime,
   revision, readiness and exact normalized opaque model ID are asserted.
3. **Replay and refusal are falsifiable.**
   Probe `host_discovery.rs:252`: reopen the journal, republish the same signed
   request, reconcile via relay, compare response ID, discovery-call marker and
   mailbox row count. At `:283`, a separately signed wrong-parent request must
   fail actual relay admission; no copied local admission predicate.
4. **Selected model reaches actual existing deployment.**
   Probe `host_discovery.rs:344`: Device intent, refused early dispatch, explicit
   synthetic main phases, duplicate dispatch, current applied receipt at `:376`,
   one start and exact prepared instance. Probe `fixture.rs:146`, `:161`, `:187`:
   fake provider speaks ACP to real buzz-acp; fake manager executes real runtime,
   checks READY, and final fake harness asserts the selected model/environment.
5. **Cleanup/revocation and resource boundaries.**
   Probe `host_discovery.rs:419` and `:469`: actual cleanup result, removed service
   instance, preserved synthetic account files, denied replay/evidence/local probe.
   Probe `fixture.rs:97`: abort/join server/subscriber, audit drain, pool close.
   Test wrapper `host_discovery.rs:41` refuses non-fixture DB URLs and reexecutes
   with empty environment; the task-only namespace command has outer timeout,
   ordinary UID/GID, label/PID guards and DB-drop trap. Only network isolation
   is claimed; host filesystem is available to the ordinary fixture UID.

## 5. Deviations and decisions to assess

The 400–600-line estimate became **744 formatted test lines**, explicitly approved
by conductor; no new framework or scenario matrix. Cached postgres runner lacks
Python, so conductor authorized a task-only network-namespace invocation, not
package installation or a repository runner feature. Reproduction is committed
as evidence; local invocation helper is under services/releases. No upstream
product assumption was invalidated and no production defect appeared.

Cached runtime/ACP binaries remain from the accepted implementation, with hashes
in evidence; their production source is unchanged. The test itself and linked
relay/owner/host libraries were rebuilt. Loopback transport uses the existing
canonical-Host `connect_to_addr` idiom; TLS remains outside this proof.

## 6. What not to chase

Review the new integration/evidence only. The 1500-line obsolete root-helper
fixture, full legacy integration migration, upstream-main catchup, Buzz App port,
installer design, lint baseline and Linux native dependencies remain separate.
No Jeff/test1/Claude/Grok/Scout state, provider login or live relay/host access.
No source publication/build staging/live rollout is implied by this handoff.
Conductor owns those actions and the next artifact-worker ownership transfer.

## 7. Verification and next gate

Executor measured final standalone offline/locked `-j2` compilation, one scenario
passed in1.29s, `cargo fmt --all --check`, and diff whitespace checks. Wrapper and
child each print1 pass; count only one journey. First build's closure lifetime and
unused membership snapshot were fixture-only corrections; first journey run
passed. Exact commands/raw-output snapshot and failure receipts are linked above.

Full CI, old relay lib test binary, native builds/human flow and strict lint were
not run or claimed green. Prior accepted seam evidence is retained by unchanged
source, not rerun as a new matrix. DB count was0; owned containers removed and
exact test process search empty. Executor processes terminal at handoff.

Return independent acceptance or concrete rework to conductor. After acceptance,
release preparation must still match Windows/relay/hostd production pins and wait
for user approval before live activation or existing-agent testing.
