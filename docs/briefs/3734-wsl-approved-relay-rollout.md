# #3734 — approved relay-only rollout and live registration acceptance

Bryce approved this scope in the Buzz conversation after accepting the missing
isolated registration compatibility proof: “yea i am good with it”. Copy the
handoff below into the same WSL session. No taskctl access is needed there.

---

Proceed with the approved Hetzner PRIMARY RELAY upgrade and registration test,
using Astra/high. Own this bounded deployment through a sanitized result or a
specific stop condition. Read your existing local instructions and receipts.
This supersedes preparation-only restrictions solely for the actions below.
No product edits, builds, merge/push, new framework or further isolated protocol
campaign. No netcup access/controller changes, existing-agent/provider actions,
agent creation, deployment enablement, membership expansion or new owner account.

## Grounding and accepted gap

Use the retained evidence at `/tmp/buzz-hosted-prep-929c-w39MsZNb`:
`sanitized-receipt.txt`, `operator-plan/commands-NONEXECUTED.txt`, its reviewed
Compose comparator/overrides, and `registration-addendum-mpjtKOfC/addendum.txt`.
Verify receipt SHA256
`5900307f2796b37a72288f402e19b601d3cce9e9216157ad5d451532167f4ff0`
and addendum SHA256
`c7a3216a4003ca985c3ec97139136561155d1f4f6a00ad07793645337d60c505`.
Preserve originals. The original53 artifacts were reported unchanged.

The isolated registration test reached zero protocol stages: old/schema81
readiness failed and diagnostics were not retained. Do not call it passed or
infer a product regression. Bryce explicitly accepts testing registration live
without that proof. Database/catalog, signed chat and exact-old-on82 rollback
evidence remain credited. Do not silently reinstate the missing isolated proof
as a prerequisite or waive actual recovery/configuration/identity gates.

Pins, reverify before mutation:

- Source `929c2cd890fbc2c0993549c3cbc64a2714232832`.
- Candidate amd64 image
  `sha256:5bc530077c757a8b058c08ef6db9e17eba8f0551118d88d17388db972fceaa0f`.
- WSL `candidate-929c2cd8-linux-amd64.tar.gz` SHA256
  `dd7a82ce65b83db6fb92f9dd573cd6b0ac57c41b54f1cd5d2ccb23b2ae9abd39`.
- Exact current/rollback image
  `sha256:1da22b2856a998de760a12b0767b30f0daf30e65b580ff193054fa17a4c85044`.
- Disposable registration hostd SHA256
  `cd81ef66643abcb1b83e2f001d25db28df10500e09a86bdec4cbfe9c425ad6e8`.
  Existing owner example SHA256
  `6173ae7fd04af3242f6ac0aa71c38e4b86e2a2e20f6636e58405065bb3e1bb9b`;
  this tool generates its own owner, so do NOT use it to import Bryce's key or
  bypass membership. Windows will authorize the disposable host test instead.

Use established SSH only:

```sh
ssh -o BatchMode=yes -o ConnectTimeout=8 -o StrictHostKeyChecking=yes -o UpdateHostKeys=no -o IdentitiesOnly=yes -i ~/.ssh/id_ed25519_hetzner root@2.31.66.65
```

Expected hostname `ubuntu-4gb-fsn1-3`; HTTPS origin
`https://buzz.interleaven.com`; WSS `wss://buzz.interleaven.com`; relay identity
`9fd495a8df792128e1396c3766b12228076cad0ea7b791c3d5fbc446f74d62c1`.
Stop on access/trust/identity failure, no alternate route or firewall changes.

## Execute this scope

1. Recheck live image, schema81/all checksums, absence of deployment tables and
   effective policies. Record all service IDs/start times/images/config hashes
   as available. Preserve the actual Compose project/ordered inputs below.
   Run the reviewed clean-environment comparator: only primary image and its
   four explicitly reviewed flags may differ. Stop on unexpected drift; do not
   overwrite it. Keep registry=true, deployment=false, auto-migrate=false,
   paired-hosts-v1=false, existing auth-token/membership=true, NIP-FI off, keys,
   volumes, listeners, TLS and routing. Never print full config/environment.

2. Verify working recovery access and retained exact old image/configuration.
   Take a fresh encrypted coordinated recovery checkpoint using the previously
   verified method and validate its isolated restore. This approved recovery
   preparation may briefly stop/restart only the primary where required by that
   method; record interruption. Preserve permissions/ownership, especially Caddy.
   Do not recreate dependencies, replace keys, or revive the Caddy-hardening plan.
   If the established checkpoint method cannot run within this scope, stop before
   migration with the concrete reason. No new backup framework.

3. Hash-check the archive, stream it through the reviewed pipefail SSH load,
   require both pipeline exits successful and remote image ID/platform exact.
   Stage verified operator files under `/root/buzz-hosted-929c2cd8-20261110` (if
   already present, verify rather than overwrite). **Use candidate-off.yml only;
   candidate-on.yml is NOT authorized in this increment.** Keep the existing
   five Compose files in order and append the off override last:
   - `/mnt/HC_Volume_107262239/buzz/deploy/compose/compose.yml`
   - `/mnt/HC_Volume_107262239/buzz/deploy/compose/compose.caddy.yml`
   - `/mnt/HC_Volume_107262239/buzz/deploy/compose/compose.pairing.yml`
   - `/root/buzz-live-upgrade-723c7e8-cutover-e8c2348b84/relay-upgrade.yml`
   - `/root/buzz-registration-f316fb46-20261107d/registration-on.yml`
   Preserve the reviewed clean environment, explicit project directory/.env and
   project name `buzz-prod`; do not use the broad lifecycle wrapper.

4. Apply migration82 once through the prepared candidate `buzz-admin migrate`
   Compose `run --rm --no-deps --pull never -T` command. Verify every original81
   ledger checksum unchanged and82/catalog exact before serving the candidate.
   Migration82 SHA384:
   `fabf94ca81749037efd558745fa19886ffae033dbf4bebf97f76202f76745cfda64348a51d89826a4ea7a5b35d90d5d0`.
   No manual DDL, ledger edits, auto-migration or dependency recreation.

5. Recreate only `relay` using reviewed candidate-off command:
   `up -d --no-deps --no-build --pull never relay` with the full reviewed prefix.
   Verify readiness, HTTPS, root and /pair WebSocket/auth behavior, relay identity,
   registry-v2 present and host-deployment capability ABSENT. Check all nonprimary
   services remain unchanged. Preserve bounded failure diagnostics before any
   cleanup; never report an unexplained timeout without its available logs.

6. Ask Bryce to reopen the existing Windows canary in the usual community, check
   history/netcup registration and send an ordinary message without mentioning
   an agent. No provider calls are required. Use supported authenticated reads
   and narrow server observations to distinguish fresh host-list recovery/ongoing
   host reconciliation from a cached Registered label. Do not read/export account
   keys or take over the real host journal. Report any unavailable observation
   honestly; do not infer reconciliation solely from an unchanged SQL row.

7. After ordinary traffic is healthy, use the already-built hostd in a private
   temporary WSL directory as ONE clearly labeled disposable registration-only
   host. Generate its own identity/invitation, run bounded registration reconciliation
   against the real relay with the pinned public identity, without `--deployment`
   or installed services. Show the invitation only in Bryce's private terminal;
   ask him to enter it in Windows Add Host, observe Registered, then revoke that
   exact disposable host. Supply its public-key suffix and explicit warning to
   leave real netcup alone. Observe authenticated pairing acknowledgement and
   revocation on the disposable host. Never paste invitation/private key in chat,
   add a synthetic community member, SQL-seed a host, or weaken authentication.
   Human interaction may pause this stage; do not time it out as server failure.

8. Verify final primary image/schema/flags/identity/ordinary health, unchanged
   nonprimary services and real host binding, and disposable revocation. Stop
   disposable processes and remove owned private fixture material only after
   revocation is confirmed; if unresolved, stop the process and retain protected
   state for cleanup instead of calling it completed. Durable revoked registration
   history is expected; no SQL deletion. Keep recovery assets private. Obtain
   a bounded independent Astra/high review of the deployment receipt and exact
   scope/results, not a new source or architecture audit. Stop and return.

## Failure handling and endpoint

Halt on integrity/auth/identity failures. Persistent ordinary service/readiness
failure90seconds triggers the prepared primary rollback, retaining diagnostics.
Stop the disposable host client first if active. Because deployment is already
off throughout this increment, flag-off alone is not a repair: restore the exact
old image with the reviewed old-recovery override and auto-migrate=false, keeping
schema82/current messages/registration records. Verify health/auth/registration
afterward; registration rollback remains unproven until observed. Old-image
community deletion fails closed on82 and deployment is unavailable. No normal
database restore, downgrade, history deletion or blind repeated rollout retries.
If rollback fails, preserve evidence and report the exact recovery state.

Return PASS, PARTIAL (including human checks pending), ROLLED BACK, or BLOCKED
with source/image/archive pins, migration checksums/catalog result, effective
flags, actual interruption, ordinary traffic and registration outcomes, unchanged
service evidence, private recovery path and independent review. Never mark human
observations done yourself. Successful endpoint is candidate/schema82 serving
with deployment=false and actual registration acceptance recorded. Enabling
deployment, modifying netcup or creating any agent requires the next approval.
