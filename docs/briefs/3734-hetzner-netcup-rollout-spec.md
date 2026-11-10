# Execute scope: concrete relay and trusted-host rollout preparation (#3734 / #3721)

**Date:** 2026-11-10. **Requester:** conductor for Bryce. **Next role:** fresh codecodex preparation executor, requested Astra/high. **Related:** #3738, #2722, #3725. Served model/effort unknown.

## Where the work lives

Use `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`; no new worktree. Starting spec baseline
`0851d1e4164ea5ae6ab06ed43deced84b870398a`; conductor supplies final docs commit.
Activate Hermit with `HERMIT_SELF_UPDATE=false` before Git/builds. Read
[spec](../specs/3734-hetzner-netcup-rollout-preparation.md) end-to-end.
Local source/artifact ownership is exclusive; WSL relay preparation uses its own
filesystem and does not modify this checkout.

## Working tree and accepted inputs

Baseline was clean. New work consists only of this brief, the specification and
[standalone WSL prompt](3734-wsl-relay-preparation.md), committed together.
Production candidate remains `929c2cd890fbc2c0993549c3cbc64a2714232832`, matching
Windows test.24. Measured929c→0851 is three Markdown files; no product delta.
Prior [host package/runbook](../executions/3721-trusted-host-pilot-preparation.md:112)
and [M-1 review](../audits/post-audit/3721-trusted-host-pilot-preparation.md:3)
are the implementation template. Current Cleanup/native acceptance is
[review round3](../audits/post-audit/3725-confirmed-hosted-cleanup.md:394).
Historical source/Windows gates are accepted inputs, not a request to rerun them.

## Sequence and commit boundaries

1. Reconfirm clean HEAD and production/build equivalence to929c; record actual
   build HEAD truthfully. Host delta from old c972 package is shared typed sender
   ([connection.rs:179](../../crates/buzz-ws-client/src/connection.rs:179)).
2. Execute P1 only locally: same offline locked three-binary release build;
   new immutable release directory, refreshed deployment/controller path pins,
   unchanged hosted template, preserved original/moved/masked rollback branches.
   Replace stale test.21/no-Cleanup blocks with test.24 and supported confirmed
   Delete/Check/Retry. Update manifest and exact committed copy of staged text.
3. Complete P1 finite help/hash/JSON/template/shell/Python/systemd-syntax checks.
   Do not execute any staged operator command against real state or manager.
   Store outputs under services disk, not `/tmp` (1.7GiB tmpfs at inspection).
4. Commit host preparation receipt and review brief together: they bind the exact
   package, shell commands and continuity evidence. Preserve c972 artifacts.
5. Conductor hands the standalone prompt to the user for WSL preparation. It is
   independently runnable without netcup/taskctl/new local docs. WSL builds the
   relay candidate and closes exact-old-image-on82 gap, returning image/archive
   provenance, compatibility evidence and relay-only operator plan. This worker
   neither sends to another task nor performs SSH. Host work need not wait on WSL.
6. Fresh independent Astra/high preparation reviewer checks the concrete host
   packet plus WSL relay receipt when available. Conductor seeks live approval
   only after exact-image compatibility and packet acceptance. A missing WSL
   result is an explicit held relay gate, not a reason to invent a substitute.

## Pre-flight

- Read project AGENTS/VISION/TESTING, portable conductor contract and spec.
- Prior worker/process ownership released; clean source pin matches dispatch.
- Accepted old package exists at `/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/c97256e20893b2688c1cc1c6bc2e23361aab707e/package/`.
- No live commands/provider login/model discovery or existing-agent operations
  in preparation; basic offline help probes use no real state.
- Exact relay old image is absent from local daemon per conductor's approved
  `docker image inspect` result; local ebac archive is unrelated. No redundant
  relay build here without a concrete artifact-transfer need.
- Source candidate929c is already public on approved fork canary branch; no push
  or workflow run is needed. test.24 human observation is still pending.

## Flag back before powering through

1. Source differs beyond accepted docs: identify actual affected inputs and get
   bounded routing; do not call it equivalent or silently broaden product work.
2. Old deployment history appears on inspected live state: raw-envelope exact
   equality may reject old sender bytes. Stop dependent rollout; retain journals.
3. Missing build/syntax tools: report precise prerequisite, no installation or
   copied-rootfs/native-fixture repair campaign.
4. Runbook edit alters custody/prefix recovery or touches any existing agent:
   return the delta for assessment before calling the package ready.
5. WSL cannot obtain exact old image through verified route, or exact f316 fails
   synthetic schema82 compatibility: live relay expansion remains blocked. Never
   build old source and label it the deployed image.

## What not to do

Source implementation, taskctl, pushes/CI, remote dispatch, live state/keys,
controller mutations, provider commands/login and existing Claude/Grok/Scout are
outside local preparation. Keep trusted normal account/main discovery/ACP/UI.
No per-agent private HOME, root helper, network framework, migration-history
rewrite, re-pair, registration redesign, new installer or generic lifecycle work.
Normal rollback retains schema82/current DB and host/owner journals; stale backup
restoration is not a shortcut. Read-only image save is not container export.

## Done criteria and return

Return signed commit(s), exact source/build/package pins and hashes, gate
outcomes, no live actions, clean tree and terminal-process receipt. Include
actual run/not-run split: syntax is not native systemd, models are not entitlement,
source proof is not human/live workflow, emergency stop is not confirmed cleanup.
The next reviewer receives original accepted packet, exact new delta and these
case-specific checks; the executor does not supply its own independent gate.

Specification verification: clean0851/branch confirmed; production equivalence
and host-dependency delta measured; source/runbook citations read. Documentation
checks PASS:32 local link/line targets,3 shell blocks through `bash -n`, and
`git diff --check` (rechecked staged content before commit).
No build/test/service/provider/native/human gate ran in this spec assignment.
