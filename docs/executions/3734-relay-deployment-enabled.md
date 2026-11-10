# #3734 — relay deployment flag enabled

User-supplied WSL final receipt; recorded by conductor, not independently
remeasured from netcup. The approved flag-only transition completed successfully.
Independent WSL Astra/high review reports zero open findings. No rollback or
duplicate live action after the WSL session interruption was needed.

## Reported measured state

- Source: `929c2cd890fbc2c0993549c3cbc64a2714232832`.
- linux/amd64 image:
  `sha256:5bc530077c757a8b058c08ef6db9e17eba8f0551118d88d17388db972fceaa0f`.
- Relay identity:
  `9fd495a8df792128e1396c3766b12228076cad0ea7b791c3d5fbc446f74d62c1`.
- Schema82; all applied migration checksums unchanged. Migration82 SHA384:
  `fabf94ca81749037efd558745fa19886ffae033dbf4bebf97f76202f76745cfda64348a51d89826a4ea7a5b35d90d5d0`.
- Only effective change: `BUZZ_HOST_DEPLOYMENT_V1=false` to `true`.
- Registry=true; auto-migrate=false; paired-hosts-v1=false; auth-token=true;
  relay-membership=true; NIP-FI unset/defaultOff.
- Registry capability: versions[2], kind43103, admission host-scoped, reads req-v2.
- Deployment capability: versions[1], kind43104, admission host-scoped, reads req-v1.
- TLS/identity, root and /pair WebSocket upgrades, root auth challenge and
  unsigned-query denial passed. Six nonprimary services retained IDs/images/
  start times/configuration hashes. Failed HTTP sample span7.506seconds is
  sampled availability, not an exact downtime measurement.

Earlier successful live Windows and disposable-host acceptance is reused. No
new human/signed-traffic/provider/pilot checks ran in this increment. Historical
isolated registration matrix remains0/6; it is not retroactively marked passed.
No WSL netcup access/activation, agent creation, build, migration or taskctl action.

## Recovery and provenance

Existing encrypted checkpoint:
`/root/buzz-hosted-929c2cd8-20261110/rehearsal-checkpoint` on Hetzner.
Retained same-candidate off command uses the reviewed clean-environment Compose
prefix (original five files), then candidate-off, candidate-on, candidate-off
overrides in that order and `up -d --no-deps --no-build --pull never relay`.
The last off override wins; no rollback command was run. Preserve exact actual
Compose ordering from the WSL receipt rather than shortening it on assumption.
Off gates new admission; it is not cancellation of admitted host work.

WSL receipt:
`/tmp/buzz-hosted-prep-929c-w39MsZNb/flag-enable-yfwl6ngy/final-flag-enable-receipt.txt`.
Supplied SHA256:
`74ebe0d4b055f385e6b79b8af9da1be3e9f77e2979985762a8ae8c0f64640906`.
Review: sibling `astra-final-flag-review.json`. Those WSL-local files have not
been opened or hashed independently by this conductor; preserve attribution.

## Next authorized action

Relay-on prerequisite for the existing netcup activation runbook is satisfied.
Executor `/root/netcup_activation` continues its same assignment: recheck native
drift/public capability, then exact same-identity/current-journal controller
cutover, preserving Claude/Grok/Scout. No new package or product implementation.
After native host readiness and bounded independent receipt review, Windows
tests ONE disposable Claude (Goose fallback) through main discovery/models/chat
and confirmed Cleanup. No provider login/logout or API-billing fallback implied.
