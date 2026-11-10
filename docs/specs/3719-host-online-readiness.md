# #3719 / #3721 host online readiness — decision packet

> **SUPERSEDED — 2026-11-10 trusted-host correction.** The operative plan is
> [#3722 shared-account execution](3722-main-discovery-native-login.md). Bryce approved normal non-root
> host account/HOME/native logins and networking, with distinct per-agent Buzz
> identities/config/workspaces/service tracking. Earlier per-agent OS isolation,
> private-root/provider-credential separation, root helper/catalog approval and
> exact-DynamicUser native-login-entry requirements are superseded, **not passed**.
> Retain owner authorization and journal/retry/cleanup; the root helper leaves the
> ordinary new-deployment path. The text below is historical only. No live action
> or existing-agent change is authorized.

**Status:** USER SELECTED T — normal host networking, 2026-11-10; source implementation remains held for the Claude reuse assessment and conductor review.
**Date:** 2026-11-10 UTC.
**Source pin:** `f4b24f42c340dc55f081a8cfd95893fdf1ff098a`, branch `proof/3720-host-registration-events`, worktree `/home/bryce/services/buzz-wt-3720-host-registration`.
**Preflight:** ran; bounded audit and design comparison included here.
**Requested configuration:** `gpt-6-astra`, high. Actual served model/reasoning metadata was not exposed to this worker.

**Sources:**

- [Accepted service/discovery spec](3721-host-service-discovery.md), especially N1; [independent source acceptance](../audits/post-audit/3721-host-service-discovery.md:3) at `fb31164e9`; [execution receipt](../executions/3721-host-service-discovery.md).
- [Pilot](/home/bryce/services/buzz-agents/docs/specs/2722-disposable-grok-pilot.md) and newer [delivery map](/home/bryce/services/buzz-agents/docs/plans/2026-11-06-host-completion.md). The pilot's Windows-pending text is superseded by the map and [native canary acceptance](../audits/post-audit/2722-windows-native-canary.md:104): 101 native tests, ACL and build evidence; not the full human journey.
- Complete supplied task exports `/tmp/buzz-{3719,3721,3722}-current.json`, including histories/amendments/links. Governing records: #3719 acceptance history 36602 and amendment 778; #3721 acceptance 36611, amendments 779/793 and source acceptance history 36824; #3722 acceptance 36620 and amendment 780. These preserve private-state/co-resident isolation while stopping the copied-rootfs fixture.
- [Grok mechanism receipt](/home/bryce/services/buzz-agents/docs/audits/3722-grok-subscription-proof.md): native 1.0.70 help and binary pin; full runtime closure/authentication unproven.
- `AGENTS.md`, `VISION.md`, [agent trust principle](../../VISION_AGENT.md:57), [remote substrate trust](../../VISION_REMOTE_AGENTS.md:31), `TESTING.md`; portable conductor contract at `/home/bryce/projects/taskctl/guidance/workflow/conductor.md`.
- Personal [relay operations note](/home/bryce/personal/projects/interleaven/reference/buzz-relay-ops.md:3): relay moved to Hetzner, agents on netcup; its old GCP IP is historical, never an allowlist input. Bounded Buzz/host search of `projects/personal-os/idea-threads.md` found no governing host-egress ruling.
- This dispatch's conductor-supplied read-only host observations, transcribed below with their limits. The conductor halted further design expansion pending Bryce's trust-model discussion. The subsequent explicit user choice is recorded below.

## Outcome and trust model

The immediate objective remains one disposable new Grok agent using main's creation/ACP path and existing hostd registration, controller, catalog, privileged helper, DynamicUser, StateDirectory and LoadCredential. Login, models and conversation use the same unit and persistent HOME. Root remains trusted. Existing agents and live registration remain untouched in this assignment. Latest user steering: the host is verified relay placement plus easy provisioning, and the same existing agents will eventually migrate. Migration itself remains outside this packet.

**Control-plane authorization and runtime isolation are separate choices.** Verified host registration, owner-authorized placement and narrow helper operations do not logically require a new network-confinement layer. Stronger isolation was carried from earlier task acceptance; it is not automatically required by the user's provisioning goal or future migration. Option T below explicitly separates them.

**Per-agent OS isolation is this fork's deployment contract, not a prerequisite for relay registration or a protection automatically supplied by main.** Main's agent vision explicitly permits shell execution at operator trust level (`VISION_AGENT.md:57`); its remote vision makes provider/substrate selection a trust decision (`VISION_REMOTE_AGENTS.md:31,49–51`). Neither statement supplies provider-only network filtering. Accepted host tasks deliberately add denial of operator/sibling private files, credentials, sockets and co-resident network paths. The selected T decision below explicitly supersedes only the network/abstract-socket denial part of that prior acceptance; private filesystem/identity controls remain.

Conductor-supplied read-only observations of current live agent services: all use `bryce` UID 1002; `DynamicUser=no`, `PrivateNetwork=no`, empty `NetworkNamespacePath`, `ProtectHome=no`, `ProtectSystem=no`, and no unit IP allow/deny entries. These are **reported measurements, not independently rerun by this worker**. They establish absence of those unit protections; they do not establish absence of a system-wide firewall, effective root privileges, actual secret access, or every effective vendor/tool sandbox. Their ordinary shared-operator trust model must not be confused with the new canonical unit below.

## Bounded audit and preflight

Verdict: sound accepted offline foundation; online execution remains unimplemented; trust choice resolved to T below. Counts **C=0 / H=2 / M=1 / L=0** are remaining activation/verification gaps, not newly discovered regressions in the accepted offline code.

| ID | Severity; grounding | Routing consequence |
| --- | --- | --- |
| N1 | High, security/dependency, read: [runtime policy](../../crates/buzz-hostd/src/runtime_context.rs:90) has only Disabled; [unit](../../crates/buzz-hostd/deploy/buzz-paired-agent@.service:17) sets PrivateNetwork; [conversation](../../crates/buzz-hostd/src/runtime_context.rs:377) refuses it. | T is now selected below. Implement and independently review its exact unit/policy/helper configuration; current offline default remains intentional. No namespace lifecycle is required. |
| R1 | High, dependency/security, read: [unit binds](../../crates/buzz-hostd/deploy/buzz-paired-agent@.service:14) expose named runtime/OS paths; [catalog verifier](../../crates/buzz-hostd/src/lib.rs:24) pins two executables, not a complete tool/library/data closure. Grok receipt explicitly leaves conversation/tool closure unmeasured. | Pin the approved installed runtime closure and prove the actual mount view before activation. Missing tools never justify mounting operator HOME, all host root, or private machine configuration. This is installation evidence work, not a new rootfs builder. |
| V1 | Medium, testing, read: [accepted review](../audits/post-audit/3721-host-service-discovery.md:51) retains native startup/mount/cgroup/socket/network/reboot as NOT RUN. | Synthetic manager success cannot close native gates. Select an existing authorized native runner or prepare a concrete host test proposal. No new test platform is implied. |

Resolved: main reuse, existing controller/journal/helper authority, immutable discovery receipts, same-unit finite action custody and strict selected-model behavior. Carry accepted source evidence rather than re-audit it. Resolved decision: T, normal host networking (see below). Unresolved evidence: installed closure, actual native confinement and service/reboot recovery. Actual Grok entitlement is #3722 pilot evidence, never inferred from model metadata.

## Selected decision: T — normal host networking

Bryce selected **T** in the conductor dispatch for the Claude reuse audit at
`a59b3db1dd4e6c7977a915e9e9e127925eb83e48`: normal host networking,
existing protections, smallest disposable Grok changes, and first assess Claude.
This supersedes the earlier requirement to deny co-resident network paths and
host abstract Unix sockets. Those denials are **waived by the selected trust
contract, not tested or passed**. Normal host networking may reach host/sibling
services and abstract sockets subject to their own access controls. It adds no
provider-only egress promise. Filesystem Unix-socket isolation remains a mount
boundary to verify.

Retain owner authorization, the existing narrow root-helper operations,
root-approved catalog/runtime pins, per-agent DynamicUser identity, private
StateDirectory/HOME and credentials, reviewed filesystem exposure, finite-action
custody, strict model selection, retry/fence/cleanup and registration rollback.
No namespace/veth/nft implementation, network oneshot, proxy or firewall framework
is in scope. The former conditional D design and its 450–740-line estimate are
superseded; historical detail remains in commit `a59b3db1dd4e6c7977a915e9e9e127925eb83e48`.

The smallest prospective source change is an explicit root-approved trusted-host
online policy, an exact opt-in unit configuration and matching helper validation.
Preserve the shipped offline default until that source change is independently
reviewed and live activation explicitly approved. No source change is authorized
by this documentation update. A numerical T estimate is not yet measured.
Runtime/library/CA/tool closure and resolver access still need bounded installed
artifact evidence; normal networking does not populate an empty RootDirectory.

## Finite verification and authority boundaries

Carry accepted checkpoint-2/3 evidence for unchanged source; do not rerun Windows
or build a copied-rootfs test platform. All native/provider gates below remain
**NOT RUN by this assignment**.

| Gate | Evidence still required |
| --- | --- |
| Source policy/helper | Offline default still refuses; exact root-approved T configuration accepts; unknown policy or arbitrary override refuses; controller cannot initiate login or choose commands; finite modes retain bounded supervision. |
| Action/retry/cleanup | Same identity/unit/HOME through prepare/login/refresh/selection; failure retains retry/fence; cleanup cannot resurrect; retain accepted lifecycle evidence and test affected seams only. |
| Native runtime/filesystem | Intended service executes pinned runtime/loader/CA/tools with private HOME/credentials; operator/sibling private files and filesystem sockets denied; missing dependencies fail without broad mount fallback. |
| Native selected networking | Verify effective normal host networking and resolver/relay/provider reachability under explicit approval. Co-resident network and abstract-socket denial is no longer an acceptance criterion; disclose shared networking. |
| Native restart/teardown | Actual service startup/failure/cgroup cleanup and retained state; finite actions do not restart on boot; isolated-host reboot remains unrun without a named approved runner. |
| Provider pilot | Separately approved disposable agent: native login in its own context, credential destination/modes/persistence, cancellation/re-auth, truthful models, selected conversation while Windows is closed, recovery and teardown. Metadata is not entitlement proof. |

Before live approval, prepare exact hashes, unit/drop-in/helper delta, approved
runtime mounts/resolver files, commands, synthetic sentinels, rollback and redacted
evidence plan. Registration opt-in separately preserves the controller key/database
and single-controller ownership, with registration-only rollback. No SSH, service,
network, provider, existing-agent or credential actions are authorized here.

## Next stage and verification performed

1. Conductor records selected T and superseded network-denial acceptance in the roadmap/tasks.
2. Complete the [Claude reuse audit](../audits/pre-audit/3722-claude-host-reuse.md) and [forward brief](../briefs/3722-claude-host-reuse.md); conductor reviews the actual generic/provider-specific boundary before normal-networking implementation.
3. Shape the small existing policy/unit/helper change, then fresh execution and independent source review. Prepare a concrete native/live proposal only afterward.

This update records a user decision, not an implementation or native acceptance.
Earlier bounded audit/preflight evidence above remains historical; N1 no longer
requires namespace enforcement. R1 installed closure and V1 native evidence remain
open. Documentation checks and commit are returned with the Claude audit. No
product code or live state changed for this decision update.
