# #3734 / #3721 — concrete Hetzner and netcup rollout preparation

**Status:** executable preparation specification; live activation NOT authorized.
**Date:** 2026-11-10. **Requested model/effort:** gpt-6-astra / high; served identity not exposed.
**Workspace:** `/home/bryce/services/buzz-wt-3720-host-registration`, branch `proof/3720-host-registration-events`.
**Specification baseline:** clean `0851d1e4164ea5ae6ab06ed43deced84b870398a`.
**Candidate production source:** `929c2cd890fbc2c0993549c3cbc64a2714232832`.
**Preflight:** skipped-self-contained: bounded preparation of ratified, accepted implementation; no new architecture decision. Current task amendments and roadmap were read for drift. Reuse accepted audits; no repeat source audit.
**Sources:**

- [Current host roadmap](/home/bryce/services/buzz-agents/docs/plans/2026-11-06-host-completion.md:3), task snapshots `/tmp/buzz-rollout-{3738,3721,2722}.json`: #3738 comment37167 (WSL receipt), #3721 comment37222 (sender/package refresh), #2722 amendment807 (trusted account) and current source/native acceptance. The committed sources below preserve the load-bearing evidence independently of temporary snapshots.
- [Relay readiness](/home/bryce/services/buzz-agents/docs/audits/3738-relay-deployment-readiness.md:3) and [sanitized WSL receipt](/home/bryce/services/buzz-agents/releases/3738-relay-readonly-20261110/wsl-readonly-receipt.json).
- [Accepted host preparation](../executions/3721-trusted-host-pilot-preparation.md:112), [interruption-aware rollback review](../audits/post-audit/3721-trusted-host-pilot-preparation.md:3).
- [Existing migration/source proof](../executions/3738-3739-hosted-deployment-step2.md:60), [final review](../audits/post-audit/3738-3739-hosted-deployment-step2.md).
- [Cleanup and sender receipt](../executions/3725-confirmed-hosted-cleanup.md:266), [final native/artifact review](../audits/post-audit/3725-confirmed-hosted-cleanup.md:394).
- [Prior WSL image method](/home/bryce/services/buzz-agents/docs/briefs/3751-wsl-live-registration-pilot.md:13): reuse only build/transfer/primary-service mechanics; its historical live authorization does not apply.

## Outcome and scope

Prepare exact Linux host binaries locally, and a self-contained WSL assignment for the relay OCI artifact, minimal relay-only override and missing exact-old-image/schema82 compatibility proof. Freeze both artifact manifests and updated operator runbooks before live approval. Carry accepted source tests rather than repeat their campaign. A fresh independent reviewer assesses this finite preparation delta before the conductor asks for live approval.

Preparation may build/stage into new release directories, inspect local artifact/tool availability and use isolated synthetic fixtures under conductor dispatch. This specifier only writes docs. No live service/configuration/schema changes, remote dispatch, provider execution/login, real key access, signed live traffic, external push, tool installation or taskctl changes follow from this document. Claude, Grok and Scout services/configuration/identities/provider stores remain untouched. Existing system controller and pairing remain active during preparation.

This is the trusted ordinary `bryce` account model: shared HOME/tools/projects/network/native subscription login, per-agent Buzz identity/config/managed state, main discovery/readiness/ACP/UI. No root helper, per-agent OS account/private HOME, login broker, network framework, new deployment framework, migration-history bridge or existing-agent adoption. These choices are ratified, not open design questions.

## Grounded state and remaining gaps

| Class | Evidence and consequence |
|---|---|
| Measured in this specification | Hermit-activated `git status --short` empty at0851; `git diff --name-only 929c..0851` lists only three Cleanup Markdown files. Production/build inputs equal Windows source929c. `git diff c97256e..929c -- crates/buzz-hostd crates/buzz-acp crates/buzz-ws-client Cargo.lock deploy` changes only `crates/buzz-ws-client/src/connection.rs` (+5/-1). |
| Read, accepted Windows evidence | test.24 source929c; 111 distinct native cases plus ACL; installer SHA256 `b5c800b489cc7f85f654ec8a768ed96a7ceec408e79c11754fbfdc34f75a050e`. [Receipt](../executions/3725-confirmed-hosted-cleanup.md:397). Human test.24 observation remains pending; test.21 smoke is historical. |
| Read, user-supplied WSL measurement | Live source `f316fb46d89c3e16eedc6173ae06e85fe224a1a9`, image `sha256:1da22b2856a998de760a12b0767b30f0daf30e65b580ff193054fa17a4c85044`; matching successful migrations1–81;82 and three deployment tables absent. Code+82+flag required; no history repair. |
| Read, accepted host measurement | System unit uses `/usr/local/libexec/buzz-hostd`, hash `de05655071c5f6af3451cc70116969f53da0df0ff64448593a0fc913bf10d3ee`, source0bacd58f; state `/var/lib/buzz-host-registration`, UID/GID988. Target bryce1002:1003, HOME `/home/bryce`, user bus and linger existed. [Receipt](../executions/3721-trusted-host-pilot-preparation.md:35). Revalidate immediately before any approved live action. |
| Read, compatibility limit | Old Value sender reordered raw event envelopes; accepted typed sender preserves journal bytes ([connection.rs:179](../../crates/buzz-ws-client/src/connection.rs:179)). No transparent upgrade of old deployment history is claimed. Inspected relay has none; ensure this remains true before activation ([review](../audits/post-audit/3725-confirmed-hosted-cleanup.md:340)). |
| Read, conductor measurement during this assignment | `/usr/bin/docker` exists; approved read-only `docker image inspect sha256:1da22b2856a998de760a12b0767b30f0daf30e65b580ff193054fa17a4c85044` returned `No such image` (daemon accessible). Services storage66GiB free; `/tmp` tmpfs1.7GiB. Existing releases search found an ebac relay archive, not f316. Record this command/result in the executor receipt; this specifier did not query Docker. |
| Preparation gap, not new product defect | Final host package, WSL-built relay image/archive and exact-old-image-on82 proof remain. Historical WSL image-build method explains cache location; no local substitute old image is acceptable. |

No new source defect/foundation gap is established. Severity counts C=0/H=0/M=0/L=0 describe this bounded read, not a new independent source audit. Missing artifact/proof is an activation hold. If environment cannot supply it, name that precise prerequisite and stop the dependent lane; do not grow tooling.

## Preparation deliverables and files

| Output | Work | Requirement |
|---|---|---|
| New `releases/3721-trusted-host-pilot/<actual-build-head>/package/` under `/home/bryce/services/buzz-agents` | P1 | Three matched binaries, updated deployment.json/controller unit, unchanged hosted template, OPERATOR.md, SOURCE.json, MANIFEST.sha256. Preserve c972 package. |
| WSL private `3734-relay-deployment/929c2cd890fbc2c0993549c3cbc64a2714232832/` beneath a verified disk-backed evidence root | P2–P3 | OCI archive/hash/image provenance when available; ordered override proposals; isolated compatibility receipt; exact missing input otherwise. No unverified image called ready. |
| `docs/executions/3734-hetzner-netcup-rollout-preparation.md` and review brief | P1–P4 | Sanitized commands/results, artifact inventory, limits, handoff; no secret raw logs in git. |
| Self-contained WSL preparation/rollout handoff in that packet | P4 | Inline all remote-required facts/commands and approval boundary; no reliance on netcup filesystem/taskctl. |

## Execution order

1. P1 establishes final host/source provenance and refreshes accepted templates/runbook.
2. P2 prepares a WSL relay assignment; only when separately dispatched does WSL inventory its local build/image facilities and build/stage there. The host package may proceed independently.
3. P3 answers only the remaining exact-old-image/schema82 gap, or returns its exact environment blocker.
4. P4 freezes a concrete sequential rollout/recovery packet and transfer handoff; independent review follows. No live action during any P step.

### P1 — refresh the accepted host package

**Severity:** activation hold. **Depends on:** clean source continuity. **Problem:** c972 package predates sender fix d093d6; [receipt:362](../executions/3725-confirmed-hosted-cleanup.md:362).

Use the same worktree, activate Hermit with `HERMIT_SELF_UPDATE=false`, record actual build HEAD plus equivalence to929c; do not falsely label later docs HEAD as exact929c. Reuse the accepted command:

```sh
export HERMIT_SELF_UPDATE=false
. ./bin/activate-hermit
cargo build --offline --locked --release -p buzz-hostd --bin buzz-hostd --bin buzz-host-runtime -p buzz-acp --bin buzz-acp
```

Stage all three outputs together in a new immutable package directory. Record Cargo.lock hash, toolchain, full source/build pin, code-equivalence diff, binary hashes/sizes and template/config hashes. Existing ACP used by Claude/Grok/Scout is not an installation target.

Reuse accepted c972 `package/OPERATOR.md`, `deployment.json`, `buzz-host-registration.service` and hosted template. Replace only release path/pins, test.21→test.24 and resolved no-Cleanup/unknown-relay branches. Retain exact M-1 original/moved/masked prefix recovery; same state path, real host/owner/generation, 988→1002 custody, one-controller proof, regular-unit move then mask, private CURRENT-state snapshot and numeric owner/mode validation. No archive restore over current journals. Update Cleanup instructions to supported Delete/Check/Retry and confirmation; emergency unit stop remains containment. Do not mechanically remove all historical safety holds because Cleanup source is accepted.

**Verify:** manifest all entries; safe help/invalid-invocation checks from prior receipt (runtime has no help mode); config JSON parse and path references; hosted template byte comparison; staged user-controller and synthetic-instance `systemd-analyze --user verify` syntax only; runbook shell fences via `bash -n` and Python bodies via `ast.parse`; exact staged text/committed packet match. No real manager mutation, real model discovery/provider invocation or private-state open. Stop if offline dependencies or syntax facility are unavailable; report, do not install. Prior source/native tests stay credited unless source changes.

### P2 — pin the relay artifact using existing facilities

**Severity:** activation hold. **Depends on:** source pin. **Problem:** f316 lacks deployment code; [readiness:3](/home/bryce/services/buzz-agents/docs/audits/3738-relay-deployment-readiness.md:3).

Conductor measured exact old image absent locally. Select WSL for relay build/proof, retaining local executor ownership of host artifacts. First inventory WSL local Docker/build capacity and exact old ID, without full environment/config dumps. If absent there too, a separately authorized WSL preparation dispatch may stream `docker image save` of that immutable ID through the established strict SSH route into a WSL-local archive, verify hash and loaded image ID, and use it only locally. This reads the server image store and writes no server file/config/database. No daemon load/pull/build on Hetzner; no remote action by this specifier or local executor. A rebuilt f316 or ebac archive is not the exact old image.

Use existing root [Dockerfile:70](../../Dockerfile:70), `runtime` target, linux/amd64; it builds locked relay/admin/pair binaries, but only primary relay is a deployment target. Use exact929c from the approved public fork in a WSL source directory, verify full commit, then build the existing root Dockerfile. Record the actual WSL paths and disk capacity. Locally any source/archive staging must use services disk, not the1.7GiB `/tmp` tmpfs. Pass `BUZZ_SOURCE_SHA` truthfully, record base image IDs/digests and build args, final immutable image ID and source evidence; save OCI/docker archive plus SHA256. Use existing build caches/tooling. No registry publication/new CI workflow. If WSL dependencies/build engine are absent, report that precise prerequisite without installing tools. Public source is `https://github.com/Shipitrealgood/buzz.git` pin929c (`build/paired-hosts-canary` is discovery only). Do not build a duplicate netcup relay artifact without a verified transfer path. WSL keeps candidate archive local until separately approved live load/cutover.

**Verify:** archive hash and loaded immutable image identity in isolated preparation environment, linux/amd64, embedded source/build identity, migration inventory82. Dockerfile's built `buzz-admin migrate` is the existing migration entrypoint ([main.rs:197](../../crates/buzz-admin/src/main.rs:197)); it acquires schema/destruction lock through [migration.rs:33](../../crates/buzz-db/src/runtime/migration.rs:33). Do not apply raw82 SQL or edit `_sqlx_migrations` manually. Staged migration command must use candidate image as a one-shot with existing database access, no dependency recreation/service port publication; keep serving auto-migrate false.

### P3 — close only exact old-image compatibility

**Severity:** activation hold. **Depends on:** exact old image and candidate image plus existing isolated Postgres/Redis method. **Problem:** accepted81→82 proof used current unchanged registration code, not f316 executable ([source proof:65](../executions/3738-3739-hosted-deployment-step2.md:65)).

In the separately dispatched WSL preparation lane, with the exact old image available, reuse the existing isolated loopback/no-egress fixture approach ([script:1](../../scripts/3720-registration-proof.sh:1)); no production DB clone, credentials, user/provider state or host ports. Fixed fixture names require absence checks and exclusive ownership; use only recorded disposable resources. Reuse existing synthetic old-client/registration drivers; a small one-off orchestration receipt is enough. Do not create a reusable rehearsal framework or rerun all private deployment tests.

Finite obligations on one synthetic database: (a) seed schema81 plus synthetic ordinary chat/membership/registration using accepted mechanisms; (b) apply82 with candidate admin migration path; (c) boot exact f316 with auto-migrate=false and prove readiness, signed ordinary chat round-trip/history and unchanged registration binding; (d) switch to candidate with deployment false, then true, verify capability toggle; credit accepted private deployment/Cleanup source proof without repeating it; (e) disable deployment on SAME candidate image, preserve all82 rows/history and prove ordinary chat/registration; (f) return to exact f316 on SAME82 database, ordinary chat/registration preserved and deployment absent; (g) restore candidate off and verify data/ledger still intact. No database restore/drop between transitions. Record actual outcomes, image IDs, row/ledger invariants and interruption. Reuse prior private-event authorization/revocation proof; do not invent an all-green claim for unexercised drivers.

Migration inventory must match all1–81 hashes/success flags, then exact82 columns/order/types/nullability/defaults/keys/indexes. Migration81 SHA384 `0869914121c74f591a54450ef5ff3f58ddf9a2d21569fb76fe6aaa657bd6bedd6d39e1d4f12da348ee1ec25d0c5c9685`;82 SHA384 `fabf94ca81749037efd558745fa19886ffae033dbf4bebf97f76202f76745cfda64348a51d89826a4ea7a5b35d90d5d0`. [Migration82:1](../../migrations/0082_paired_agent_deployments.sql:1) is additive, no backfill. Carry the existing source parity proof explicitly.

Old-image limits: auto-migrate must remain false; old deletion catalog fails closed on82, so community deletion is unavailable throughout old-image-on82 operation. This is bounded temporary recovery, not full old-image feature parity. A failed exact-image compatibility check blocks live expansion/cutover; leave current production untouched and return the concrete failure. Missing exact image/runtime access is a named proof prerequisite; do not rebuild f316 and call it equivalent. Prepare WSL-local proof instructions if that environment owns the exact image; the WSL preparation dispatch must explicitly authorize read-only exact-image streaming over SSH; it still grants no live changes.

### P4 — freeze rollout packet and approval request

**Depends on:** P1/P2 completed; P3 green for full live-readiness claim, otherwise explicit blocked obligation. Include exact files/hashes and literal operator commands, not only prose. The following sequence is the contract for those commands, not authorization to execute them.

1. **Live read-only checkpoint after approval, before mutation.** Revalidate supplied WSL route/trust, primary image/source, complete ledger/schema, policy and exact ordered Compose inputs. Recheck public relay key `9fd495a8df792128e1396c3766b12228076cad0ea7b791c3d5fbc446f74d62c1`. Prove still no old deployment history: expected schema81/tables absent and registration-only host/controller unactivated suffice for the fresh-pilot scope. If82/history unexpectedly exists, stop and route compatibility assessment; never clear journals or relax exact-envelope guards. Record host/controller and three existing agents' unit fragment, MainPID/start time, restart count and active state without reading credential/config content. Recheck immediately after each live checkpoint.
2. **Recovery ready.** Confirm exact old primary image retained, approved encrypted checkpoint and existing restore-validation method available, prior effective config privately retained, and recovery access works. Checkpoint is disaster recovery; normal rollback keeps current database. Preserve Caddy data ownership, keys/volumes, pair-relay and dependencies. No ingress/TLS/firewall change.
3. **Expand then deploy primary only.** Use candidate admin one-shot existing locked migrator for82, verify ledger/catalog, then primary candidate with deployment=false. Maintain registry=true, auto-migrate=false, paired-hosts-v1=false, auth-token=true, relay-membership=true, NIP-FI off and existing secure relay URL. Exact config precedence matters. Switch/recreate only `relay` using existing Compose project and `--no-deps`; no `down`, broad `up`, dependency upgrades or pair-relay replacement. Stop on policy/schema drift.
4. **Candidate ordinary traffic checkpoint, then deployment enable.** Verify health/readiness, TLS/public identity, ordinary signed chat through approved synthetic/user test mechanism, existing registration and `/pair`. No mentions/provider prompts to existing agents. Then change ONLY `BUZZ_HOST_DEPLOYMENT_V1=true` (literal true) on the same candidate. Expected NIP-11 `host_deployment={versions:[1],kind:43104,admission:"host-scoped",reads:"req-v1"}` alongside registry v2. Advertisement alone is not schema/host readiness ([nip11.rs:423](../../crates/buzz-relay/src/nip11.rs:423)).
5. **Netcup same-state cutover.** Execute refreshed accepted runbook only after relay checkpoint, one system→user controller transition. Stop/disable old, snapshot stopped state, move regular unit/mask, validate custody, install exact new user files/start user controller. Preserve real host key/owner/generation/state path. No re-pair, init, second controller/state tree, service adoption or existing-agent restart. Verify same binding and fresh host capability, exact binary paths and user-service context. Native credential delivery/notify/cgroups remain actual post-approval checks, not syntax evidence.
6. **Human Windows and one disposable pilot.** Confirm test.24 binary/hash and retained existing profile. Human observes registration/ordinary chat before new agent. After explicit pilot/provider approval, fully quit Buzz and start actual installed executable with process-local PowerShell `$env:BUZZ_NATIVE_HOST_DEPLOYMENT_V1='1'`; no persistent flag. This differs from relay's literal true ([agents_hosted.rs:155](../../desktop/src-tauri/src/commands/agents_hosted.rs:155)). Ordinary Add Agent → Advanced / Run on → netcup; capacity1; one new clearly disposable Grok, normal identity/channels/instruction policy. Use fresh same-host inventory/readiness, retained setup, explicit Refresh models and returned opaque model ID. Expired/offline/revoked/unsupported/NeedsLogin remains truthful and blocks required action ([host_deployment.rs:146](../../crates/buzz-core/src/host_deployment.rs:146)). Native login occurs as bryce only if separately directed: shared credentials affect existing agents. No login during preparation, credential copying, API billing fallback or repeated provider loop. Resume SAME identity after refresh. Record one controlled chat reply, cloud icon/Hosted on netcup, then confirmed Delete/Cleanup; Check/Retry retains identity and intent until authenticated Cleaned plus local completion. Close is not cleanup. Record exact opaque unit and placement/operation IDs without secrets.
7. **Finish or stop.** Verify pilot unit stopped/disabled/removed, durable cleanup and no identity resurrection; current registration and old agents unchanged. Remove process-local Windows flag and fully restart normally. Retain rollout/checkpoint/journals; no broad cleanup. Reboot/two-host/Android/full release validation remain separate, not implied.

**Failure/rollback contract:** any integrity, authority, identity/custody or protected-agent mutation discrepancy halts immediately. Persistent readiness/ordinary-chat failure90s triggers primary rollback; missing human observation alone is not a server failure. Keep exact recorded units only. First contain any pilot using accepted runbook, retain current host/owner journals and actual pending status; emergency stop is not successful Cleanup. Prefer same candidate deployment=false to preserve schema-compatible code; it stops new admissions but is not a guarantee that an already admitted bounded host attempt cannot finish. Quiesce exact host controller/pilot if containment requires it. If necessary, switch only primary to rehearsed f316 with auto-migrate=false, keep82 and all current data/journals, accept the documented community-deletion/deployment limitation. Restore host system controller through observed original/moved/masked branch after proving both controllers/pilot quiescent, preserve CURRENT state and restore custody988:988; never restore stale archive, reinitialize/re-pair or erase operation history. If quiescence/custody is uncertain, remain stopped and report. Re-enablement needs current-source host package, current capability and exact history reconciliation.

## WSL transfer contract — mandatory self-contained content

Conductor root has no verified Hetzner route. The [standalone WSL preparation prompt](../briefs/3734-wsl-relay-preparation.md) is self-contained; no task is sent and no SSH performed now. Recommended artifact location is WSL where the historical relay build/access route already lives. Source is public fork exact929c; never use moving main or old23d2dc. Build and retain candidate/proof locally in WSL; return sanitized receipt text to the conductor/user. After separate live approval, the same WSL operator can stream the verified candidate archive over the existing strict SSH route to `docker load`, verify exact ID, and use only that image for primary cutover. Image load is a live mutation and is forbidden during preparation. No local-netcup→WSL artifact-transfer dependency is introduced. If choosing a local candidate instead, first establish a concrete archive-transfer route and recheck its hash/image identity; do not reuse another image's acceptance.

Inline current image/source, migration hashes, all flags, host/relay identity, gates, limits and exact ordered Compose paths:

1. `/mnt/HC_Volume_107262239/buzz/deploy/compose/compose.yml`
2. `/mnt/HC_Volume_107262239/buzz/deploy/compose/compose.caddy.yml`
3. `/mnt/HC_Volume_107262239/buzz/deploy/compose/compose.pairing.yml`
4. `/root/buzz-live-upgrade-723c7e8-cutover-e8c2348b84/relay-upgrade.yml`
5. `/root/buzz-registration-f316fb46-20261107d/registration-on.yml`

Append reviewed deployment override(s) last; do not replace earlier inputs. Effective overrides set only candidate image and deployment boolean while preserving enumerated policies. Keep separate off/on and rollback proposals so precedence is unambiguous. Privately render/compare allowlisted effective fields; do not print full compose config/env.

Existing WSL route (future approved use only):

```sh
ssh -o BatchMode=yes -o ConnectTimeout=8 -o StrictHostKeyChecking=yes -o UpdateHostKeys=no -o IdentitiesOnly=yes -i ~/.ssh/id_ed25519_hetzner root@2.31.66.65
```

Expected hostname `ubuntu-4gb-fsn1-3`. Missing identity/trust, changed key, timeout or unexpected host stops dependent work. No speculative route, host-key acceptance, private-key inspection/copy or firewall widening. The WSL worker owns relay only; it needs no netcup files/taskctl and must return sanitized artifact/proof/rollout result to the conductor/user. A preparation-only prompt explicitly forbids live load/migrate/restart/flag actions; read-only exact-old-image streaming is allowed only when explicitly included in its dispatch. Historical #3751 approval is not portable to this rollout.

## Finite gates, judgment calls and verification record

| Gate | Success criterion | Status at specification |
|---|---|---|
| Source continuity | clean pin;929c production/build equivalence; sender included | Measured PASS for0851; executor rechecks its build head |
| Host artifacts/runbook | P1 manifest/help/config/syntax/text gates, unchanged protected targets | Not run here; prior c972 evidence accepted, refresh required |
| Relay artifact | P2 immutable image/archive/provenance and minimal effective config | Not run; conductor found old image absent locally; WSL preparation required |
| Missing compatibility gap | P3 exact f316 on82 and same-image off/on/recovery with synthetic data | Not run; cannot mark ready for live rollout without result |
| Documentation | links/source citations/shell syntax/diff checks | PASS:32 local link/line targets and3 shell blocks; whitespace checked before commit |
| Independent preparation review | separate reviewer accepts artifact/runbook/proof delta | Not run; conductor dispatches after execution |
| Live/native/provider/human | actual approved checkpoint receipts including confirmed Cleanup | Not run; separate explicit approval required |

Judgment calls: reuse accepted mechanisms and restrict proof to missing deployed-image gap; a fresh-pilot-only serialization boundary is already accepted. No new product/architecture ruling is requested. If exact image or native build facilities are unavailable, the conductor chooses a separately dispatched WSL preparation lane using these exact inputs; that is an execution-location decision, not permission to skip proof. No broadened full CI, old Caddy campaign, copied-rootfs repairs, native isolation campaign or provider matrix. No new tests merely to mirror docs.

Self-review: additive schema is retained across rollback; both controller interruption prefixes and current journal custody survive; flags differ intentionally by surface; source/native proof is distinct from real service/provider behavior; capability freshness and raw-envelope history are checked before creation. Unchased questions (historical deployments elsewhere, reboot/two-host/release journeys) reopen only on actual drift or a separate user request. Conductor records routing; no board writes by this worker.
