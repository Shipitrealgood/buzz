# #3722 — trusted-host execution and main readiness

**Status:** user-ratified trusted-host contract; S1 source implementation and independent review authorized on 2026-11-10. S2/S3 remain the next scoped increment. No live authorization.
**Date / source:** 2026-11-10; `a41272ffd673d0b7c09f076991e48c37333727eb`, `proof/3720-host-registration-events`.
**Preflight:** ran, bounded decision/source revalidation; earlier landscape work carried forward. Latest user direction resolves the trust model.
**Sources:**
- User direction, relayed in this assignment: netcup is the user's agent computer; shared normal account/HOME/native login; owner-managed provisioning; no live changes or existing-agent access.
- [Prior audit](../audits/pre-audit/3722-claude-host-reuse.md): reuse source findings H1/H2/M1; its isolation and auth-extraction recommendations are superseded.
- [Main agent trust](../../VISION_AGENT.md#design-principles), [remote provider contract](../../VISION_REMOTE_AGENTS.md#the-only-tether), [current registration](../../ARCHITECTURE.md#10-fork-extension-host-registration).
- [Archived user-service template](/home/bryce/services/buzz-agents/releases/pre-main-netcup/buzz-agent@.service:8), [provisioning pattern](/home/bryce/services/buzz-agents/docs/agent-provisioning-runbook.md:70); source references only, not live evidence or adoption instructions.

## Decision and boundary

**Ratification:** Bryce confirmed the complete walkthrough and requested documentation plus execution. The shared non-root account/HOME/native-login model below is settled, not a provisional security exception. Do not reopen private per-agent OS environments, root-helper requirements, or custom authentication without concrete new evidence and a user decision. S1 ends at reviewed source; live installation and existing-agent adoption require separate authorization.

This replaces the previous #3722 isolated-service/native-login-entry spec and the operative direction in [#3719](3719-host-online-readiness.md).
Per-agent OS users, DynamicUser/private roots, isolated provider credentials, exact-service interactive login entry, and root catalog/hash approval are **superseded, not passed**.
Run host management and new agents as the configured non-root host account, with its normal HOME, native harness logins, tool environment and networking.
Keep a distinct Buzz key, generated config, tracked workspace and deterministic service identity per agent. Same-account agents are trusted; file modes protect keys from other users, not from those agents or root.
Only the paired owner may manage host placements. Chat/respond permissions continue through main's existing policy; ordinary agent tools may spawn processes. Shared host-management delegation is later work.
Option A stays: Desktop generates the agent identity and encrypts its key to the host. Owner keys remain on Desktop; credentials and raw auth output stay out of logs/status.
Cleanup stops only the recorded new unit and removes only its generated Buzz artifacts. Never recursively delete shared HOME, provider login/config, or repositories, including tracked workspaces.
Existing agents/services/registration remain untouched. Main's provider API and the archived service are references; no automatic service adoption or migration.

## Main reuse and intentional fork behavior

Main's provider contract is substrate-neutral; user services are our VM-specific choice matching the existing setup, not an upstream mandate.
Main's [Kubernetes pod](../../crates/buzz-backend-kubernetes/src/pod.rs#L132) does isolate: non-root UID, dropped capabilities, no privilege escalation or service-account token, and its own emptyDir workspace; its writable root is intentional.
Reuse main's payload/harness/discovery/readiness semantics. This fork's persistent hosted execution and registered-host management intentionally differ from the [upstream bounded lifetime/no post-deploy substrate control](../../VISION_REMOTE_AGENTS.md#the-only-tether).
The existing fork cleanup acknowledgement also differs from the [provider API's lack of undeploy acknowledgement](../../desktop/src-tauri/src/commands/agents/provider_deploy.rs#L21). Do not rebuild that API or add Kubernetes to this patch.

## Read evidence and exact keep/adapt/retire map

| Paths / source seam | Disposition |
| --- | --- |
| [deployment.rs:26](../../crates/buzz-hostd/src/deployment.rs#L26), [refresh:38](../../crates/buzz-hostd/src/registration_refresh.rs#L38), [CLI:88](../../crates/buzz-hostd/src/registration_cli.rs#L88) | **Adapt:** replace the fixed root-socket `Helper` and installation checks with one local user-service executor. |
| [deployment.rs:182](../../crates/buzz-hostd/src/deployment.rs#L182), [:464](../../crates/buzz-hostd/src/deployment.rs#L464), [:628](../../crates/buzz-hostd/src/deployment.rs#L628) | **Keep:** durable journal, owner/scope/revision checks, exact result retries, stable unit IDs, cleanup tombstones and bounded retry scheduling. |
| [lib.rs:24](../../crates/buzz-hostd/src/lib.rs#L24), [:63](../../crates/buzz-hostd/src/lib.rs#L63), [context:120](../../crates/buzz-hostd/src/runtime_context.rs#L120) | **Adapt:** local runtime definitions and shared-account context; replace root-only hashes/paths and private HOME assumptions. |
| [activation.rs:13](../../crates/buzz-hostd/src/activation.rs#L13), [service_actions.rs:46](../../crates/buzz-hostd/src/service_actions.rs#L46), [unit](../../crates/buzz-hostd/deploy/buzz-paired-agent@.service#L8) | **Retire from normal deployment:** root installation, socket, private root, credential/login ceremony and exact isolated-unit shape. Reuse only lifecycle/idempotency mechanics that still apply. |
| [legacy unit.rs:13](../../crates/buzz-hostd/src/unit.rs#L13), [activate binary](../../crates/buzz-hostd/src/bin/buzz-host-activate.rs#L1) | Legacy provisioning still calls the helper. Keep only compatibility needed by that separate legacy entrypoint; no fallback or selectable second engine in new deployment. Do not uninstall live services. |
| [main catalog:50](../../desktop/src-tauri/src/managed_agents/discovery/catalog.rs#L50), [presets:107](../../desktop/src-tauri/src/managed_agents/discovery/presets.rs#L107), [discovery:939](../../desktop/src-tauri/src/managed_agents/discovery.rs#L939), [readiness:416](../../desktop/src-tauri/src/managed_agents/readiness.rs#L416) | **Reuse once:** main's harness facts, CLI/adapter resolution rules and readiness classifiers. No hostd provider matrix. |
| [hosted payload:37](../../desktop/src-tauri/src/commands/agents_hosted.rs#L37), [main payload:190](../../desktop/src-tauri/src/commands/agents_deploy.rs#L190), [models gate:118](../../desktop/src/shared/api/hostedAgents.ts#L118) | **Keep/adapt:** main creation/key/config/model flow; replace only hosted context and obsolete login-history gating. |

## S1 — first finite code change: local user-service executor

Flow: existing signed deployment request → existing host journal/reconciliation → local user service → main ACP harness.
The journal prevents duplicate effects on retry; it is not a new manager or protocol.

**Problem (read):** `Helper::installed` requires `/var/lib/buzz-hostd`, a root socket and root catalog; its six fixed verbs are local implementation, not a missing relay protocol (deployment.rs:31–104).
**Severity:** routing prerequisite; depends on approved shared-account direction only. Native login is not a blocker.
**Files proposed for this first patch:** `crates/buzz-hostd/src/{deployment,registration_cli,registration_refresh,lib,runtime_context}.rs`; `crates/buzz-hostd/src/user_service.rs` and `crates/buzz-hostd/deploy/buzz-hosted-agent@.service`; affected existing `service_actions_tests.rs` / `registration_refresh_tests.rs`.
Use the existing `stage/action/discover/remove_staged` call boundary. Replace socket RPC with direct local executor calls; keep reconciliation and encrypted command/result envelopes intact.
Use `systemctl --user` for a fixed template and the existing opaque `unit_id`; never accept a unit name, program, path, shell text or environment override from the remote payload.
For S1, adapt the existing local resolved runtime definitions/configuration to supply commands and tool PATH; preserve canonical harness mappings and add no provider switch. S2 performs the shared main extraction. Host approval means the local configured runtime set, not root ownership or executable digests.
Keep a revision of that local configuration/resolution snapshot for stale-command fencing. Check executable availability at use; updates may change the revision without a root approval ceremony.
Per-agent app state lives beneath the controller's account-owned state directory; directories are 0700 and key/config files 0600 with atomic writes. The service receives its own Buzz key through the existing credential-file pattern.
Use normal account HOME/XDG provider state and a locally assigned workspace; do not set `GROK_HOME` or HOME to per-agent storage. Configure host environment locally; reject remote arbitrary env/path/command fields as today.
The executor owns prepare, finite models/observe/cancel, start and cleanup for these recorded units only. Preserve mutual exclusion, nonce/deadline checks, bounded output, child termination, confirmed stop and retryable cleanup.
Preserve separate per-agent execution/observation artifacts; shared HOME does not make action receipts shared. Derive deletion paths from recorded IDs and refuse symlink/path escapes.
Remove root helper calls from the ordinary `--deployment` route and retire its custom login action there. Leave the separate legacy provisioning path explicit until its compatibility dependency is removed.
**Deliberate first-slice limit:** retain the hosted feature/activation gate off and conversation refusal until S2 supplies truthful readiness. Do not fake a native-login receipt, fall back to root, or enable a partially wired launch.
This gates new hosted creation only; it does not disable registered-host status or existing agents.
The slice ends when user-account lifecycle/context wiring compiles and passes the finite cases below with fake commands; it does not include full inventory, Desktop UI, service installation or native provider access.
**Verify in existing fixtures:**

- Prepare/replay maps one agent to one unit; restart/retry converges; mismatched scope/revision/unit refuses before effect.
- Local invocation is unprivileged and uses account HOME, locally resolved argv and the assigned workspace; per-agent Buzz keys remain distinct and out of output.
- Timeout/cancel terminates the finite process tree; competing finite/start actions cannot overlap; failed stop retains cleanup intent and artifacts for retry.
- Cleanup removes only generated artifacts after confirmed stop; shared HOME/provider files/repos and an unrelated unit survive. Unsafe path/symlink inputs refuse.
- Root helper is unreachable from the new deployment route; registration-only behavior remains unchanged; conversation remains unavailable before S2.

**Stop condition:** return the committed first slice and tests for independent review. Any need for a generic service manager, new relay action, interactive auth machinery or automatic migration returns to the conductor instead of expanding this patch.

## S2 — share main discovery/readiness and complete runtime launch

**Problem (read):** main has the definitions/probes; hostd has Grok-only login and requires a historical receipt before conversation ([context:205](../../crates/buzz-hostd/src/runtime_context.rs#L205), [:378](../../crates/buzz-hostd/src/runtime_context.rs#L378)). Audit H1/H2/M1 applies; depends on S1.
Extract only portable registry/discovery/readiness facts and classifiers into existing `buzz-core`; Desktop and hostd call that same source with their own explicit local resolver/context.
Keep Tauri, caches, installers and interactive auth orchestration in Desktop. Native provider login is performed externally on netcup under the shared account; Buzz supplies refresh/setup guidance, not a login UI or broker.
Resolve/probe on netcup using the host account's configured environment, never Windows PATH. Host-local approved definitions may live in user-owned config; discovered availability is not provider entitlement.
Reuse main checks for missing adapter/underlying CLI, versions and supported config. Preserve richer unsupported configurations as unsupported rather than silently accepting them.
Refresh performs bounded readiness then `buzz-acp models` in the same account/environment used by conversation. Main-owned provider-specific facts move once; hostd supplies execution and sanitized results only.
Readiness distinguishes logged-in, logged-out, not-applicable, unknown and config-invalid; timeout/execution failure remains unavailable. Missing evidence never means logged-in.
Claude/Codex use main's actual status probes. Grok's main `NotApplicable` means auth was not checked; it can permit an attempt when other gates pass, but must not display authenticated.
Model states remain Available / Empty / Unsupported / Unavailable. Launch needs available runtime, readiness permitting an attempt, fresh nonempty models and selected ID membership; recheck immediately before conversation.
Normal trusted-host networking replaces the offline-only conversation policy on this route; no root policy pairing, namespace or egress wrapper. Source configuration remains disabled for rollout until source review and explicit activation approval.
Delete the ordinary custom login command, `Mode::Login` usage and native-login receipt gate after replacement tests. Retain historical decode fields only where required; new observations use NotObserved/None for that history.
**Verify:** main callers retain existing semantics; native logged-in with no Buzz login receipt permits an attempt; old successful receipt plus logged-out/unknown refuses; NotApplicable never becomes authenticated; models never prove quota/subscription.

## S3 — project truthful host state through existing transport/UI

**Problem (read):** status lists catalog entries, observation has login history/models only, and UI gates on that history ([status:115](../../crates/buzz-core/src/host_deployment.rs#L115), [observation:62](../../crates/buzz-core/src/host_deployment/observation.rs#L62), [UI:118](../../desktop/src/shared/api/hostedAgents.ts#L118)). Depends on S2.
Adapt existing encrypted `Status` and `Observation` only for facts actually absent: bounded host inventory/availability and current readiness. Keep existing runtime mappings/config revision, scope, operation, placement revision, nonce and freshness gates.
Use at most 64 inventory entries, existing 8 KiB status and 2 KiB observation limits; overflow is explicitly unavailable, not silently truncated. Exclude local paths, provider output, credentials and account details.
Update `agents_hosted_commands.rs`, `hostedAgents.ts` and `HostedAgentSetup.tsx` to reuse main's metadata/model picker and show distinct setup, readiness and model states. Replace custom login directions with native setup on netcup and Refresh.
Keep Option A key creation/encryption and existing main respond permissions. No provider registry, new relay schema/kind/endpoint, or replacement deployment protocol.
**Compatibility:** these DTOs deny unknown fields. Defaulted optional fields let new readers treat old messages as unknown; old readers reject new fields. Use matched host/client rollout with hosted creation gated off, not a claim of transparent mixed-version compatibility.
**Verify:** stale/missing/wrong-agent observations refuse; refreshed host facts update selection independently of Desktop PATH; ordinary key/prepare/model/start/cleanup retries retain their identities and authority.

## Sequence, prerequisites and verification boundary

1. S1 scope was presented and approved on 2026-11-10; execute only S1 under its bounded dispatch, then obtain independent source review.
2. Implement/review S1 as a finite prerequisite; then S2 and S3 as the coherent readiness/launch/UI increment. Update obsolete hostd installation docs with those code changes.
3. Run affected existing hostd lifecycle/controller tests, core contract tests, main Rust discovery/readiness tests and existing hosted creation/model tests. Use the existing synthetic ACP fixture; no new test platform or provider calls.
4. Run applicable format/type/lint and repository-required pre-PR checks; independently review the pinned implementation range. Report skipped gates explicitly; source acceptance is not runtime acceptance.
5. Return an exact runtime rollout/rollback proposal for separate approval. Later native evidence must verify configured account/HOME/tool PATH, user manager persistence, native readiness and one disposable new agent's end-to-end chat with Windows closed, then confirmed cleanup.

**No active native-entry prerequisite.** Actual shared-account login/readiness is unmeasured here; later observation must not log out, reconfigure or probe through existing agents. Grok may be usage-exhausted; no provider contact is authorized by this plan.
**Remaining judgments:** no new product choice is required for S1. If mixed-version rollout, existing-placement migration or delegated host management is required, return that distinct decision before implementing it.
**Adjacent work:** main catch-up, provider installations, existing-agent migration and relay cleanup remain outside this unit.
**Self-review:** account trust matches the approved product direction; owner authorization, distinct Buzz identities and durable retries survive. Legacy helper retention is limited to its existing separate caller; no new dual-engine switch. No isolated-login workaround is needed.
**Planning verification:** source reads and bounded preflight/self-review only; docs links and diff checks recorded in the brief. Builds, source tests, full CI, native login/services/provider calls and human acceptance are NOT RUN.
**Configuration:** requested `gpt-6-astra` / `high`; actual served model/reasoning not independently exposed. No claim of runtime proof follows from this plan.
