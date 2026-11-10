# Execute scope: checkpoint-2 host service and discovery (#3721)

**Date requested:** 2026-11-10 UTC. **Requester:** conductor for Bryce.
**Next role:** fresh `codecodex` executor; independent reviewer follows.
**Related tasks:** #3719 isolation, #3722 native Grok evidence, #2722 Windows journey.

## Where the work lives

- Worktree: `/home/bryce/services/buzz-wt-3720-host-registration`.
- Branch: `proof/3720-host-registration-events`.
- Product baseline: `da66e8d1c23fa58e203035dbcbae23fb4591cac0`.
- [Executable contract](../specs/3721-host-service-discovery.md).
- [Operative pilot](/home/bryce/services/buzz-agents/docs/specs/2722-disposable-grok-pilot.md).
- [Accepted M-3 repair](../audits/post-audit/3738-3739-hosted-deployment-step2.md:172).

## Current state

Clean baseline was measured at spec start. This handoff commit adds only the spec
and this brief. No product changes, builds, tests, systemd operations, provider
calls, existing-agent inspection or taskctl mutations were performed. The spec
author has no running subprocess to hand over. Reuse this worktree sequentially;
the conductor supplies the documentation commit as the executor's exact start.

Hermit activation initially tried a forbidden cache self-update. The known lane
setting fixes that without bootstrap or a new toolchain:

```sh
export HERMIT_SELF_UPDATE=false
. ./bin/activate-hermit
git status --short
git rev-parse HEAD
```

## Sequence

1. Move only the actual shared pure model normalizer/DTOs from desktop to core;
   add the bounded encrypted result wrapper and approved runtime→harness mapping.
   Preserve legacy decoding and public result header. This contract is the first
   commit boundary because every runtime consumer depends on it.
2. Bind the existing registration journal into both controller/helper namespaces
   at `/var/lib/buzz-hostd`, using fixed `buzz-hostd` UID and the same physical
   `buzz-host-registration` StateDirectory. Keep a single controller/identity.
   Add same-unit fixed modes and the real runtime supervisor/context. Commit
   unit/helper/runtime changes together so their paths and credentials agree.
3. Integrate prepare/refresh discovery into existing pending Deploy operations,
   add verified owner observation access, and exercise the synthetic production
   journey. Refresh only inactive/nonlaunchable preparation; never stop a live
   conversation implicitly. No rotating host-status cache or new relay table.
4. Add default-off strict configured-model ACP startup and host-only enablement.
   Fresh selected-model discovery plus actual session rejection must prevent
   prompts on an unintended model; ordinary local/provider behavior stays intact.
5. Run finite V1–V5, record exact evidence and native limits, commit with `-s`,
   stop owned processes and return the candidate for independent review.

## Pre-flight

- Read the spec completely; F1–F4 describe one source packet, not full pilot acceptance.
- Starting source and prior M-3 acceptance must match the conductor's handoff.
- Protect existing Claude/Grok/Scout credentials and services; none are inputs.
- No live installation, relay traffic, native login, provider models or deployment
  is authorized. Synthetic disposable verification is authorized.
- PID 1 is `codex`, not systemd (conductor measurement). Native unit/reboot gates
  are unavailable here; neither installed manager tools nor unit syntax checks
  establish runtime isolation.

## Things to flag back before powering through

1. **N1 is open:** online namespace/DNS/IPv4/IPv6/redirect/destination policy is
   #3719's activation prerequisite. Implement `PrivateNetwork=yes` default and
   truthful unavailable state; do not provision a network/proxy platform.
2. **Result bound:** keep `wire.rs:245–247`'s 4 KiB encrypted result ceiling.
   Complete model catalogs that cannot fit return explicit oversized/unavailable.
   Return measured need before widening; never silently truncate IDs/catalogs.
3. **Mode restart:** login/model execution cannot inherit automatic restart.
   Spec defines one fixed root-owned `Restart=no` drop-in and exact unit-shape
   verification. If systemd cannot support this without a broader control plane,
   report the exact constraint; do not improvise arbitrary overrides.
4. **No fake authentication:** Grok preset at `presets.rs:156` and models command
   at `buzz-acp/src/lib.rs:6314` do not certify subscription readiness. Keep native
   login outcome separate from model enumeration, expiry and real chat evidence.
5. **Model fallback:** `buzz-acp/src/pool.rs:1733–1774` continues on rejection/miss.
   The approved strict flag must cover the actual session before prompt, including
   a catalog change after preflight. A preflight-only test misses this race.
6. **Helper output custody:** runtime-controlled output paths must not let root
   follow symlinks into operator/sibling files. Bind readback to the exact unit,
   action/revision and bounded regular file; raw login output is operator-local.

## What NOT to do

- Keep accepted journal/cleanup/ownership protocol foundations; change result
  assembly only as necessary and rerun affected receipt/regression cases.
- Use `buzz-acp models --json`; leave Tauri process/env orchestration desktop-only.
- Keep shared harness facts in existing metadata; no new registry, preset-table
  extraction, desktop native bridge or Windows UI wiring in this packet.
- Keep deployment flags default off. No copied rootfs, old proof-loop revival,
  broad host-read mount, inherited provider environment or credential copying.
- Keep full installer, existing-agent adoption, general lifecycle/fleet refresh,
  upstream catch-up and HTTP deletion outside this assignment.

## Done criteria and evidence

- F1–F4 implemented through production seams with V1–V5 outcomes and three
  falsification controls; failures and unavailable gates remain explicit.
- Documentation explains exact opt-in layout, offline defaults, fixed actions,
  private result semantics and later N1/#3722/#2722 gates.
- Finite source checks pass on the committed revision; no detached processes or
  unrecorded source mutations remain. Full CI/native/human requirements are
  reported honestly and no merge/release-ready attestation is implied.
- Return new/moved/deleted production size separately. Planning range is uncertain:
  800–1,500 net new lines, 150–300 moved lines, about 15–22 production files.
- Source spec checks: applicable documents/source read; clean initial HEAD;
  five reused main files compared with no diff. Documentation whitespace/link
  checks run before the handoff commit. All production/native verification is
  NOT RUN by the spec author.
- Conductor owns board routing and fresh independent review; N1 remains under
  #3719, real subscription/model/chat acceptance under #3722, Windows under #2722.
