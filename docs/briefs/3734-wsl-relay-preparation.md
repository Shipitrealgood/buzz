# Self-contained WSL handoff — relay rollout preparation only

The conductor may copy the text below to Bryce for the existing WSL session.
This file does not dispatch that session. The preparation/live boundary below
supersedes historical #3751 wording authorizing deployment.

---

Prepare the existing Hetzner Buzz relay update, using Astra/high. Own relay
artifact/proof/operator-plan preparation only. Local netcup executor separately
refreshes hostd/runtime/ACP; do not access netcup or Claude/Grok/Scout. No taskctl
or netcup files are needed. Read applicable WSL/project instructions and preserve
unrelated work. Return a sanitized receipt and stop before live changes.

Authorized in this preparation assignment: local source retrieval/build; read-only
Hetzner metadata inspection via the already verified strict SSH route; **docker
IMAGE save** of the exact currently deployed image streamed to WSL if needed;
WSL-local image load and isolated synthetic compatibility checks; local operator
plan files. Forbidden on Hetzner: staging files, pulls/loads/builds, migrations,
restarts/recreates/flags/config changes, container export, DB dumps, key/volume
copies, provider commands and signed live test traffic. No firewall/access or
host-key changes. Historical deployment approval grants no authority here.

Known state (user-supplied WSL read-only receipt, November10; recheck drift):

- Relay/community `https://buzz.interleaven.com`, secure relay URL matches
  `wss://buzz.interleaven.com`; public relay pubkey
  `9fd495a8df792128e1396c3766b12228076cad0ea7b791c3d5fbc446f74d62c1`.
- Exact old image `sha256:1da22b2856a998de760a12b0767b30f0daf30e65b580ff193054fa17a4c85044`;
  source `f316fb46d89c3e16eedc6173ae06e85fe224a1a9` via matching November7
  deployment receipt (old image lacks revision label).
- All applied migrations1–81 successful and match candidate;82 and tables
  `paired_agent_placements`, `paired_agent_operations`,
  `paired_host_deployment_status` absent. No historical deployment envelopes on
  this inspected relay. Source code update plus additive82 then flag required.
- Effective registry=true, auto-migrate=false, paired-hosts-v1=false,
  auth-token=true, relay-membership=true; deployment unset/source absent,
  NIP-FI off (unset/default). Preserve these policies and listener boundaries.
- Public registry `{versions:[2],kind:43103,admission:"host-scoped",reads:"req-v2"}`;
  deployment absent. Public metadata does not prove schema readiness.
- Current candidate/source is **929c2cd890fbc2c0993549c3cbc64a2714232832** at
  `https://github.com/Shipitrealgood/buzz.git`, already on
  `build/paired-hosts-canary`. Fetch exact commit, not moving branch/main.
  Windows test.24 builds that source;111 native Rust cases+ACL accepted; human
  test.24 and live/provider checks remain pending. No Windows action here.

Use only this established access, expected hostname `ubuntu-4gb-fsn1-3`:

```sh
ssh -o BatchMode=yes -o ConnectTimeout=8 -o StrictHostKeyChecking=yes -o UpdateHostKeys=no -o IdentitiesOnly=yes -i ~/.ssh/id_ed25519_hetzner root@2.31.66.65
```

Missing identity/trust, changed host key, timeout or unexpected host ends dependent
remote work. Do not inspect/copy private keys, guess another route, weaken SSH or
change hotspot/firewall allowlists. Never print full `.env`, Docker inspect or
Compose config; select metadata fields in memory. Read-only DB catalog access,
if needed for drift recheck, uses existing approved local PostgreSQL access and
an explicit read-only transaction; no application rows or secret connection URL.

Existing Compose inputs, EXACT order:

1. `/mnt/HC_Volume_107262239/buzz/deploy/compose/compose.yml`
2. `/mnt/HC_Volume_107262239/buzz/deploy/compose/compose.caddy.yml`
3. `/mnt/HC_Volume_107262239/buzz/deploy/compose/compose.pairing.yml`
4. `/root/buzz-live-upgrade-723c7e8-cutover-e8c2348b84/relay-upgrade.yml`
5. `/root/buzz-registration-f316fb46-20261107d/registration-on.yml`

Capture actual primary service/project labels and config order; preserve original
Caddy, pair-relay, PostgreSQL/Redis/other dependencies, keys/volumes/TLS/auth policy.
Any later deployment override must be appended last, primary `relay` only.

Execute this finite preparation:

1. Establish a disk-backed private WSL evidence/source directory with sufficient
   space. Record absolute paths. Check existing Docker/toolchain/caches and exact
   old image availability. The netcup daemon lacks that image; its old ebac relay
   archive is not a substitute. No tool installation/new build framework.
2. If old image is absent locally, inspect remote immutable ID/architecture first,
   then stream ONLY `docker image save sha256:1da22b2856a998de760a12b0767b30f0daf30e65b580ff193054fa17a4c85044`
   through the strict SSH command into a WSL-local archive. Check SSH/save exit
   status and archive hash, load in WSL only, require matching immutable ID and
   linux/amd64. Do not use container export, server temporary archive or volume
   snapshot. If this route fails, return that exact blocker; never rebuild f316
   and claim it is the deployed artifact.
3. Fetch candidate929c, activate its Hermit environment with
   `HERMIT_SELF_UPDATE=false` for repo tooling, read AGENTS, docs/database.md and
   accepted execution/review `docs/{executions,audits/post-audit}/3738-3739-hosted-deployment-step2.md`
   and `3725-confirmed-hosted-cleanup.md`. Reuse accepted source81→82 parity/private
   deployment/revocation/cleanup tests; do not rerun their broad suites.
4. Build one release linux/amd64 candidate using existing root Dockerfile target
   `runtime`, truthful `BUZZ_SOURCE_SHA=929c2cd890fbc2c0993549c3cbc64a2714232832`,
   locked dependencies and existing WSL build method. Record base images/digests,
   build args/tool versions, source, final image ID/architecture and exported
   archive SHA256/size. Root Dockerfile builds relay and buzz-admin; this is not
   authority to replace pair-relay. Keep artifact in WSL for later approved load.
5. Reuse existing isolated synthetic loopback/no-egress Postgres/Redis method;
   source `scripts/3720-registration-proof.sh` documents the existing pattern.
   No real credentials, production database copy, published host ports or provider
   traffic. Check fixture names absent; record ownership and clean only your own
   resources. Do not launch `just setup/reset` against default services.
6. The ONLY newly missing compatibility proof is exact deployed f316 on expanded82.
   On one synthetic DB with schema81 and synthetic ordinary chat/membership/host
   registration, run candidate `buzz-admin migrate` (existing schema/destruction
   lock path). Verify matching all81 ledger rows plus exact additive82 catalog.
   Boot exact old image with auto-migrate=false: readiness, signed ordinary chat
   round-trip/history, registration binding survive. Then candidate deployment
   false→true→false: expected capability toggles, ordinary traffic survives and
   schema/current synthetic rows remain. Return to exact f316 on SAME82 database,
   verify ordinary traffic/registration, then candidate-off if useful. No database
   restore/drop between transitions. Carry accepted private deployment/Cleanup
   source proof rather than recreating it. Reuse existing synthetic traffic driver;
   if unavailable, report that bounded missing check instead of inventing a new
   integration framework or reporting boot-only as full compatibility.
7. Expected82 checksum SHA384:
   `fabf94ca81749037efd558745fa19886ffae033dbf4bebf97f76202f76745cfda64348a51d89826a4ea7a5b35d90d5d0`;
   expected81:
   `0869914121c74f591a54450ef5ff3f58ddf9a2d21569fb76fe6aaa657bd6bedd6d39e1d4f12da348ee1ec25d0c5c9685`.
   Compare all applied versions/success/checksums to exact Git blobs, then82
   ordered columns/types/defaults/nullability/keys/usable indexes to
   `migrations/0082_paired_agent_deployments.sql`. Never rewrite81/history. No raw
   manual DDL or migration-ledger insertion in the planned live procedure.
8. Prepare literal, nonexecuted relay-only cutover/rollback commands and exact
   off/on override files using established Compose project/all five inputs plus
   new override last. Future live sequence: private recovery checkpoint and
   retained old image/config; candidate admin one-shot migration; verify82;
   candidate primary deployment=false; ordinary traffic/registration checkpoint;
   same candidate `BUZZ_HOST_DEPLOYMENT_V1=true`; health/capability checkpoint.
   Serving auto-migrate=false and v1=false throughout. Use `--no-deps`, never
   `down` or recreate dependencies. Record how the verified WSL archive will be
   streamed through the existing SSH route to `docker load` AFTER live approval;
   no server load/staging now. Provide exact filenames/hashes in return.
9. Propose same-candidate deployment=false as first rollback, preserving82 and
   current DB/journals. Exact-old-image rollback is temporary degraded recovery:
   deployment unavailable and old community-deletion catalog fails closed on82;
   old auto-migrator would reject82, so auto-migrate=false is mandatory. No normal
   DB restore/history deletion. An admitted bounded host attempt may finish after
   relay flag-off; conductor separately handles exact host/pilot containment.
   Integrity/auth/identity failure requires immediate halt; persistent ordinary
   traffic/readiness failure90s triggers planned primary rollback. Human check
   absence alone is not server failure. No actual live rollback runs now.
10. Have an independent Astra/high reviewer inspect this bounded artifact/proof/
    operator-plan delta (not the entire source campaign); resolve concrete
    findings. Return all results with read/measured/inferred attribution and
    exact missing checks. Release local fixture/process ownership. Stop.

Return: full source/image/archive hashes and local paths; exact old-image identity
provenance; all81→82 comparison; synthetic proof matrix and real outcomes; source
proofs reused; interruption/recovery limits; concrete primary-only override and
commands; independent review verdict; preserved live image/config/services; unrun
native/Windows/provider/human/live gates. Sanitized receipt text suffices for
conductor handoff; it cannot read WSL-local files. No requirement to send messages
or access another session. If a prerequisite fails, finish independent safe
preparation and return the precise blocker. Wait for separate explicit approval
before any live image load, database migration, restart or flag change.
