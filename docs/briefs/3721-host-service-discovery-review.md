# Independent review: checkpoint-2 service and discovery (#3721)

Requested 2026-11-10 by the conductor. Next role: fresh `codex-chuck`.
Related: #3719 online isolation, #3722 native Grok evidence, #2722 Windows journey.

## Where the work lives

Worktree `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events` (not main). Source range:
`db829357458e90ed434af1487039bedbe0a5ad5f..39bbd9e1aab2f6b1e922c4eab567f1a51a8b9b52`,
four signed commits. A subsequent documentation-only commit carries this brief
and the final receipt. No pre-existing unrelated changes were present.

```sh
cd /home/bryce/services/buzz-wt-3720-host-registration
export HERMIT_SELF_UPDATE=false
. ./bin/activate-hermit
git log --oneline db8293574..HEAD
```

## What landed

The existing registration controller/journal now has a checked-in deployment
binding to the fixed helper. Fixed login/models/conversation modes share the
persistent per-unit context; finite actions use bounded supervision and private
output custody. Preparation returns immutable encrypted observations through the
existing journal/private query, and ACP has a default-off strict model gate.
The shipped execution policy remains offline and conversation refuses it.

Read the [spec](../specs/3721-host-service-discovery.md),
[executor input](3721-host-service-discovery.md), and
[execution receipt](../executions/3721-host-service-discovery.md).
The receipt lists 72 passing focused tests and exact source/native limits.

## Your job

Independently evaluate the pinned source, especially the new authority and custody
edges. The executor's tests are evidence, not an independent gate. Do not infer
systemd, network, provider or online-start correctness from injected manager output.

**Special instruction:** the source currently accepts only the disabled network
policy. Real finite commands execute under that source configuration, but no native
service was run. Distinguish a real production-path implementation from the
remaining installation/dependency-closure/N1 proof. Successful online selected-start
is explicitly not claimed, by conductor ruling in the spec.

## Specifically look for

1. **Can root output reads escape the intended unit?**
   Probe `crates/buzz-hostd/src/service_actions.rs:165`: anchored directory/file
   opens, symlink/hardlink/regular-file/owner/cap checks, exact descriptor equality,
   sanitized terminal output and RuntimeDirectory-only live login output. Probe
   `:241` cleanup ordering after observed stop/disable; include missing outputs.
2. **Does finite selection preserve one unit and refuse live conversations?**
   Probe `service_actions.rs:47`, `:78`, `:263`: exact owned Restart=no bytes,
   foreign overrides, main/control/task emptiness, short root transactions and
   start/observe/cancel correlation. In `activation.rs`, pending revision refresh
   must refuse before the existing abort/replace path can stop a conversation.
3. **Does context and supervision stay bounded?**
   Probe `runtime_context.rs:182` and `:342`: clear environment, fixed login argv,
   unchanged ACP models command, shared HOME/GROK_HOME/XDG/cwd, combined 64 KiB
   capture and deadlines. Distinguish process tests from unproven native cgroup
   behavior. Read the empty-root unit bind list against the explicit closure limit.
4. **Can result assembly weaken accepted cleanup or immutable replay?**
   Probe `crates/buzz-hostd/src/deployment.rs:103` and `:284`: observation polling,
   post-discovery permit, terminal byte replay, superseded completion, oversize
   fallback and M-3 monotonic cleanup. Existing SQL/journal authority stays in place.
5. **Are observations private, current, and facts rather than entitlement?**
   Probe `crates/buzz-core/src/host_deployment/wire.rs:311`, `:337`,
   `observation.rs:110`, `:137`, and `crates/buzz-paired-owner/src/deployment.rs:334`.
   Check unchanged public header, 4 KiB ciphertext/2 KiB observation budgets,
   legacy decode, exact saved config binding, private current-permission query and
   expiry. Native login state must remain distinct from available models/entitlement.
6. **Can the actual session still prompt after strict rejection?**
   Probe `crates/buzz-acp/src/pool.rs:1691` through the strict missing/rejected
   branches, plus config/runtime/isolated_execution and both lib startup paths.
   Tests in `pool_strict_model_tests.rs` drive `run_isolated_prompt` and inspect
   actual requests; already-current needs no switch and default-off must continue.

## Deviations and uncertainty

- V4 ends at offline selected-start refusal; online success remains N1/native.
- RuntimeDirectory live bytes plus sanitized StateDirectory terminal result avoid
  a sweeper/history mechanism. Judge the exact custody/lifetime reasoning.
- One current root login outcome is overwritten by new login start/cancel, keeping
  a prior success from masquerading as the latest invocation's outcome.
- Main normalizer/DTOs moved; Tauri native tests were not run. Their existing cases
  ran through the same shared core function, and Tauri formatting passed.
- Final narrow reruns cover decode boxing and capacity/login-state corrections.
  Relay regression predates those deltas; the receipt spells out continuity.
- Source size is net approximately +1,428 production lines across 24 files;
  raw added/deleted/moved accounting and accepted estimate revision are in receipt.

## What not to chase

N1 online policy/namespace provisioning is #3719. Real Grok entitlement/login/chat
is #3722. Windows UI/build is #2722. Existing Claude/Grok/Scout services and
credentials, installer/adoption, legacy HTTP retirement, generic lifecycle/fleet
refresh, preset extraction, rootfs builders and architecture cleanup are outside
this review. Existing unrelated clippy warnings remain explicitly recorded.

## Deliverable and preflight

Return severity-ranked concrete findings and exact run/not-run evidence to the
conductor; no merge, push, deployment, live contact or board authority is delegated.
Read the full spec/receipt, confirm the source pin and clean tree, and keep one
exclusive writer/test owner. The owned fixture containers have been removed; the
existing proof script is the documented isolated regression runner if needed.
Requested executor settings were Astra/high; hidden runtime routing was unavailable.
