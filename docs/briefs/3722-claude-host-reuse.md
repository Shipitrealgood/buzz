# #3722 — trusted-host implementation handoff

**Date:** 2026-11-10. **Next owner:** fresh executor for user-approved S1 source implementation, then independent review. The shared-account decision is ratified; live actions remain unauthorized. Requested Astra/high;
actual served model/reasoning not independently exposed.

## Where the work lives

Worktree `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`; clean input `a41272ffd673d0b7c09f076991e48c37333727eb`.
Activate Hermit with `HERMIT_SELF_UPDATE=false`. Reuse this worktree sequentially.
Conductor owns roadmap/taskctl. This worker owns only the two 3722 documents and
superseded notices on the two 3719 documents; preserve all unrelated changes.

## What is in the tree

[Replacement spec](../specs/3722-main-discovery-native-login.md) replaces the
isolated-service/native-entry plan, rather than extending it. The historical
[audit](../audits/pre-audit/3722-claude-host-reuse.md) still identifies real main
reuse/readiness gaps; its private-HOME/root-catalog and auth-extraction direction
is superseded. No product or test code changed in this planning assignment.
The two 3719 notices mark old isolation requirements superseded, not passed.

The approved boundary is the ordinary shared non-root host account, HOME, native
provider logins and networking. Each new agent keeps its own Buzz identity,
generated config, tracked workspace and service. Same-account agents are trusted;
key file permissions protect against other users, not against those agents.
Owner-only host management remains; chat permissions follow main. Login happens
externally on netcup. No exact-DynamicUser-login-entry prerequisite remains.

Main's provider interface is a reuse reference, not proof that main has no
isolation: its Kubernetes backend isolates pods. The user-service choice matches
this VM; persistent execution and registered-host lifecycle are intentional fork
behavior. The spec records these differences without broadening the patch.

## Sequence and first patch

1. **S1 scope presented and approved.** Existing signed deployment request → existing
   host journal/reconciliation → local user service → main ACP harness. The
   journal prevents duplicate effects on retries; it is already implemented.
2. **S1: replace the local executor boundary.** Modify hostd
   `deployment.rs`, `registration_cli.rs`, `registration_refresh.rs`, `lib.rs`,
   `runtime_context.rs`; add the focused `user_service.rs` and
   `deploy/buzz-hosted-agent@.service`; adapt existing hostd fixtures. The precise
   behavior and finite cases are in the spec. Use existing local runtime definitions
   for S1; shared main extraction belongs to S2. Stop after this backend patch and
   its independent source review. No line-count estimate is promised.
3. **S2–S3: complete readiness/launch/UI separately.** Share main discovery and
   readiness semantics once; update private observations and existing model/setup
   UI. Native login receipt history no longer gates readiness. This is the full
   Add Agent completion increment, not part of the first backend patch.
4. **Separate live gate.** After source review, present exact matched-build
   rollout/rollback and a disposable-agent journey for explicit user approval.
   Keep new hosted creation gated until ready. This never means disabling the
   registered host, registration status, or any existing live agent.

## Pre-flight

- Read the replacement spec and current dispatch; earlier isolation prose is history.
- Reuse the accepted audit; this assignment did not rerun a broad health audit.
- Confirm exclusive write/test ownership and pinned source before implementation.
- Read applicable repository/feature instructions and test guidance for code edits.
- Reference [archived user unit](/home/bryce/services/buzz-agents/releases/pre-main-netcup/buzz-agent@.service:8)
  for credential and supervision mechanics only; its hardcoded paths are not new defaults.
- Source inspection is not proof of current host login, user-manager persistence,
  runtime dependencies or provider quota. Those await an approved native gate.

## Things to flag back before powering through

1. **Root fallback or generic service manager.** [Helper:26](../../crates/buzz-hostd/src/deployment.rs#L26)
   is the replacement seam. If this grows into arbitrary unit/program management,
   stop and return scope. Retain only the separate [legacy caller:13](../../crates/buzz-hostd/src/unit.rs#L13)
   compatibility; ordinary new deployment must not select the root engine.
2. **Host-owned configuration.** [Current runtime pins:24](../../crates/buzz-hostd/src/lib.rs#L24)
   encode obsolete root trust. Replace them with local resolved definitions and
   revision fencing. Remote payload still cannot supply arbitrary argv/env/path.
3. **Absent readiness.** [Current login gate:378](../../crates/buzz-hostd/src/runtime_context.rs#L378)
   must not be bypassed by a fabricated receipt. First-slice conversation stays
   unavailable until the replacement readiness gate lands. Unknown remains unknown.
4. **Cleanup ownership.** [Cleanup retry:537](../../crates/buzz-hostd/src/deployment.rs#L537)
   survives the executor swap. Never delete shared HOME/provider credentials/repos;
   only confirmed stopped units and their generated Buzz artifacts are removable.
5. **Mixed versions or migration.** [Observation:62](../../crates/buzz-core/src/host_deployment/observation.rs#L62)
   denies unknown fields; use matched gated rollout. A requirement for rolling
   interoperability, old-placement migration or delegated management returns to
   the conductor as a distinct decision, not an invented compatibility platform.

## What NOT to do

Keep live agents, pairing, credentials and services untouched. No provider calls,
login/logout, rootfs/namespace test platform, custom auth UI/broker, new provider
matrix, auto-adoption, relay schema/protocol replacement or general service manager.
Normal agent tools may spawn processes; host-management authorization is separate.
Grok may be usage-exhausted; source tests cannot certify entitlement or chat.

## Done criteria and evidence

Planning endpoint: a bounded first patch with exact files, preserved retry/owner
semantics, retired root launch path, later readiness/UI increment and separate live
gate. No product implementation, source acceptance or native success is claimed.
Source inspection, bounded preflight and spec self-review completed. Measured:
45 local link targets/line bounds passed across the 3722 documents; both 3719
superseded notices passed; `git diff --check` passed. Only the four owned docs
changed. The worker return pins the DCO commit and final worktree status. Builds, product tests, full CI,
native login/service/provider checks and human acceptance are **NOT RUN**.
No background process was launched. Return docs-only commit and release ownership.

## S1 execution handoff — 2026-11-10

S1 source implementation now has an [execution receipt](../executions/3721-trusted-host-executor.md)
and [independent-review brief](3721-trusted-host-executor-review.md). Use that bounded
review packet for the committed implementation range; this planning document remains
its upstream contract. Registration-only behavior and installed registration units
remain unchanged. S2/S3, native login/readiness and live rollout remain unexecuted.
