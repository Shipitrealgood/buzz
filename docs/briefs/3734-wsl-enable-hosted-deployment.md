# #3734 — approved deployment flag enablement

Copy below the divider into the same WSL session. Bryce approved proceeding with
prepared netcup activation, relay deployment enablement and one disposable agent.
Claude is preferred because Grok is at its weekly limit; provider choice is made
from the normal host discovery/readiness flow, not by changing the relay.

---

Proceed using Astra/high with the approved flag-only primary-relay transition.
Netcup is handled by the conductor separately. Do not access netcup, create an
agent, run a provider, alter existing agents or add registration test identities.
No product changes, builds, migrations or repeated isolated compatibility tests.
Taskctl is local to netcup; report back rather than accessing it.

Read your retained final live receipt:
`/tmp/buzz-hosted-prep-929c-w39MsZNb/live-rollout-dwokhHfk/final-registration-acceptance.txt`.
It reports healthy candidate/schema82, successful Windows history/messaging/
registration refresh and disposable host pair/revoke/cleanup; deployment=false.
The conductor has only that summary, so include exact final pins, recovery path
and prior review disposition in your next sanitized receipt.

Reuse the reviewed operator-plan under
`/tmp/buzz-hosted-prep-929c-w39MsZNb/operator-plan/`, its comparator, and files
actually staged on Hetzner by your completed rollout. Do not guess a new active
Compose prefix. Expected candidate image:
`sha256:5bc530077c757a8b058c08ef6db9e17eba8f0551118d88d17388db972fceaa0f`;
source `929c2cd890fbc2c0993549c3cbc64a2714232832`.
Prepared candidate-on.yml SHA256:
`519918e197c3ca307d567b275e427e46b30140d9cde60467ce9d575983765e34`;
candidate-off.yml SHA256:
`e823580bab01320888ca36852e17be16ca78ac4f1fec1a4c22a559347c1511ba`.
Keep the original artifacts unchanged and preserve unrelated work.

Use the same verified strict SSH route to root@2.31.66.65 with
~/.ssh/id_ed25519_hetzner, IdentitiesOnly=yes, BatchMode=yes,
StrictHostKeyChecking=yes, UpdateHostKeys=no, ConnectTimeout=8.
Expected hostname ubuntu-4gb-fsn1-3 and public relay identity
`9fd495a8df792128e1396c3766b12228076cad0ea7b791c3d5fbc446f74d62c1`.
Stop on trust/access/identity drift; no access or firewall changes.

1. Recheck serving image, successful82 ledger/catalog, active ordered Compose
   configuration and expected off flags against the completed receipt. Verify
   retained recovery assets/previous off override and healthy normal endpoints.
   Use existing recovery checkpoint; do not repeat the full backup/rehearsal
   campaign for this flag-only change. Record nonprimary service IDs/start times.
2. Verify the reviewed on override and clean-environment Compose comparator.
   Its ONLY effective difference from current serving configuration must be
   BUZZ_HOST_DEPLOYMENT_V1 false -> literal "true". Same image, auth, registry,
   keys, volumes, listeners, networks and all other services. Stop on other drift.
3. Use the reviewed full Compose prefix with candidate-on override last and
   `up -d --no-deps --no-build --pull never relay`. No broad wrapper, down, pulls,
   migrations, dependencies or /pair-relay recreation. This flag change is
   authorized now; no second approval is needed for this exact step.
4. Verify healthy HTTPS/root and /pair WebSocket/auth behavior, unchanged relay
   identity, registry-v2 and host_deployment capability
   `{versions:[1],kind:43104,admission:"host-scoped",reads:"req-v1"}`.
   Keep auto-migrate=false, paired-hosts-v1=false, registry=true, auth-token and
   relay-membership=true, NIP-FI off. Check all nonprimary services unchanged.
   Reuse completed signed/Windows acceptance; report new human checks only if
   actually performed. Capability success permits host activation, not a claim
   that an agent has run.
5. On integrity/auth/identity failure stop; on persistent health failure90seconds
   restore the same candidate-off configuration, retaining schema82/history.
   Capture bounded diagnostics first. Do not retry blindly or restore a DB.
   At this stage netcup has not been activated and no pilot is authorized to run
   from this WSL session. Report rollback or pending state immediately.

Return exact image/source/schema/flags, measured interruption, capabilities,
identity and unchanged-service checks, retained recovery/off commands, and a
bounded independent Astra/high receipt review. Preserve private contents and
do not print full environment/configuration. Stop after flag checkpoint; no
netcup change, host re-pairing, provider operation or new test framework.
