# Same-reviewer re-gate: #3721 M1–M3

Return to the reviewer of [the initial report](../audits/post-audit/3721-host-service-discovery.md).
This is bounded rework of that assignment, not a new architecture audit.

Workspace `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`. Clean repair start
`85e20fc2df0b51817fc0cfe5cc4de7a255de9e5e`; signed source/test repair
`7517ebf3e7906fc51abe1dfbf468ff3069c944f6`. A following documentation-only
commit carries this brief and the updated [receipt](../executions/3721-host-service-discovery.md).
Original source candidate: `93aa5b2a4ca655957eafff7f99c2ca541514188a`.
Requested executor configuration Astra/high; served runtime metadata unknown.

## Re-gate focus

- **M1:** `service_actions::retire` deletes the previous correlated files before
  replacing its descriptor, after caller-confirmed inactivity. `restore` removes/
  reloads the finite drop-in before conversation mode. `shape` handles the stopped
  unlink-before-reload prefix using the protected finite descriptor. Check foreign
  and active cases still refuse; directory sync precedes descriptor replacement.
- **M2:** retirement saves a validated terminal login outcome before models or
  refresh supersedes it; missing output resolves InProgress to Failed. Bad output
  refuses replacement. Failed start records Failed after confirmed abort. Check
  the existing protected summary remains a current login fact, not entitlement.
- **M3:** new `discovery_login_refresh_and_production_offline_refusal` in the
  existing relay fixture proves prepare → synthetic fixed login → unpolled pending
  refresh → selected refusal. It asserts identity/unit/HOME persistence, current
  login, network-disabled models and bounded outputs. The production offline
  guard is shared with installed `run()`; bootstrap/systemd are not executed.

Host fixture additions cover both supersession paths before local observation,
repeated refresh/cleanup, interrupted unlink/reload, and failed/missing login output.
The older simulated-active relay journey remains only lifecycle/journal evidence.
The original V4 completion claim was overstated; the receipt explicitly corrects it.

## Evidence and limits

Measured: host lib18 + isolated relay5 + registration-refresh9 = **32 passed**.
Serialized replacement build passed after one test-only borrow correction. Formatting,
three scoped size checks and diff checks passed. Host clippy completed with existing
warnings only. Exact commands/timings and delta accounting are in the receipt.
Production repair: +90/−20 lines in three files; tests +427/−47 in two existing
files, including relocation of the manager closure. No new source files/subsystem.

Core/owner/ACP/desktop source is unchanged; carry the initial review's independent
checks. The explicit ACP-models gate and the full72 were not rerun. Native systemd,
rootfs dependency closure, confinement/network, provider, online activation, Windows,
Tauri native, full CI and human acceptance remain NOT RUN. No new online policy,
test bypass, store/history/sweeper, UI, installer/adoption or live-agent work.

All owned fixture containers and test/build processes closed; no unrelated edits.
Use Hermit with `HERMIT_SELF_UPDATE=false` and the same existing proof runner if
needed. Review the committed delta and these findings only, expanding on new
material evidence. Return an independent verdict/evidence to the conductor;
no push, merge, deployment, provider contact or taskctl authority is delegated.
