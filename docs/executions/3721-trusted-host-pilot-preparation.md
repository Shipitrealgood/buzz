# #3721 trusted-host pilot preparation

2026-11-10. **PREPARED FOR REVIEW — NOT READY TO ACTIVATE.** No live mutation,
provider command, login, private-key read, authenticated relay request, service
reload/start/stop, board mutation, push or deployment was performed.

Executor `/root/netcup_trusted_pilot_prepare`, role codecodex. Requested
`gpt-6-astra/high`; actual served model/reasoning not exposed. Applied execute and
brief. Independent review remains the gate; this receipt is executor evidence.

## Scope, source and evidence continuity

Exclusive worktree `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`, clean baseline/build pin
`c97256e20893b2688c1cc1c6bc2e23361aab707e`. No pre-existing dirty files.
Only this execution document and the [review brief](../briefs/3721-trusted-host-pilot-review.md)
are changed. No new product code, architecture, deployment framework or cleanup
utility. Source/build processes finished before ownership release.

Inputs read: applicable AGENTS/VISION/TESTING, portable conductor guidance,
[ratified trusted-account spec](../specs/3722-main-discovery-native-login.md),
[S1 receipt](3721-trusted-host-executor.md), [S2/S3 receipt](3721-host-discovery-windows.md),
[accepted S1 review](../audits/post-audit/3721-trusted-host-executor.md),
[accepted S2/S3 review](../audits/post-audit/3721-host-discovery-windows.md),
[Windows test.21 review](../audits/post-audit/2722-shared-discovery-windows-canary.md),
current host-completion roadmap headers and supplied task snapshots. Historical
isolated/root-helper/private-login instructions are superseded by the trusted
normal-account ruling. Accepted 99 source tests and 110 Windows tests plus ACL
are carried forward, **not rerun**. Human test.21 success is registration/ordinary
chat smoke, not this hosted pilot.

Measured `git diff --name-only 23d2dc15c6c45554181938b7956b59c085980455..c97256e20893b2688c1cc1c6bc2e23361aab707e`
contains only three Windows execution/brief/review Markdown documents. Production
and build inputs therefore match the installed test.21 canary. Building ACP is
necessary to pin the harness used by the current runtime: existing agents use a
much older independently installed88697f1c5 harness. The package does not replace
that harness or any existing agent's executable/config.

## Installed context — measured metadata, read receipts

Read sanitized #3751 install/pair/deployment receipts and package, then the later
#3750 rollout receipt. Historical #3751 executable is NOT current. Read-only live
metadata confirms the SYSTEM unit `/etc/systemd/system/buzz-host-registration.service`
is enabled/active, no drop-ins, MainPID43436, NRestarts0, start November9 16:19:35UTC.
Current executable `/usr/local/libexec/buzz-hostd` SHA256
`de05655071c5f6af3451cc70116969f53da0df0ff64448593a0fc913bf10d3ee` matches #3750
source `0bacd58f5369f61a4da1fd794c26d25503117a23`. Unit SHA256 remains
`9cb7a6d7b9df3e18b39d46916d1d7b32140e0567e123305dfe8685a5e8a03bce`.

The old dedicated UID/GID988 controller has RootDirectory and fixed-IP sandbox;
actual state is `/var/lib/buzz-host-registration` (0700), containing `host.sqlite`
and `reconciler.lock` (0600), all988:988. Only names/metadata were inspected, no
live journal contents. Public host/owner/generation baseline is from the sanitized
post-pair receipt; the future operator must recheck it after stop without secrets.

Bryce is UID1002/GID1003, HOME `/home/bryce`; actual user manager responds
systemd257.13-1~deb13u1, `/run/user/1002/bus` exists, Linger=yes. Existing units
`buzz-agent@claude.service`, `buzz-agent@grok.service`, `buzz-agent@scout.service`
are active/running. Their exact metadata is in `existing-agents-before.txt` beneath
the evidence root below; final `existing-agents-after.txt` is byte-identical.
Final controller metadata still shows MainPID43436, NRestarts0, same start time
and active state, same fragment and no drop-ins. These are process-metadata
continuity checks, not fresh provider/configuration-content audits. Initial guessed unit names returned not-found and were
not used as continuity evidence. Grok path resolution only (no execution) finds
`/home/bryce/.local/bin/grok` → `/home/bryce/.grok/downloads/grok-1.0.70-linux-x86_64`.
The proposed user controller and hosted template paths are absent.

**Required transition:** stop/disable the old controller, preserve its stopped
state, move its regular system-unit file into root-only backup, mask that path,
transfer SAME state custody to bryce, then install/start ONE user controller.
Masking an existing regular `/etc` unit directly would fail; the runbook handles
that explicitly. Root helper drop-in `deploy/buzz-host-registration.service.d/50-deployment.conf`
is legacy and untouched. Host-key custody explicitly shifts to all trusted bryce
processes. No second state directory, identity initialization, competing controller
or account migration of existing agents is proposed.

## Concrete blockers and smallest prerequisite map

1. **Relay deployment capability absent (measured).** Public unauthenticated NIP-11
   GET from `https://buzz.interleaven.com` returned the pinned relay key and
   registry v2 kind43103, but no `host_deployment`. Raw public response is preserved
   as `public-nip11.json`. Read source [nip11.rs:423](../../crates/buzz-relay/src/nip11.rs#L423)
   requires `{versions:[1],kind:43104,admission:"host-scoped",reads:"req-v1"}`;
   [config.rs:678](../../crates/buzz-relay/src/config.rs#L678) also requires registry
   admission, and [config.rs:1471](../../crates/buzz-relay/src/config.rs#L1471)
   reads exact `BUZZ_HOST_DEPLOYMENT_V1=true`. Latest found historical relay receipt
   names registration-era f316fb46. Current deployed source/config cannot be inferred
   from absence. Next relay owner must inspect serving version/config/schema and
   determine flag-only versus reviewed compatible rollout. No SSH, relay build,
   flag change or relay deployment in this task.
2. **No shipped confirmed per-agent teardown (read).**
   [HostedAction:256](../../desktop/src-tauri/src/commands/agents_hosted_commands.rs#L256)
   exposes only Check/Retry/Refresh/Launch. [Close:197](../../desktop/src/features/agents/ui/HostedAgentSetup.tsx#L197)
   dismisses the view; its later `cancelled` variable only cancels a local async
   effect. [prepare_deployment:61](../../crates/buzz-paired-owner/src/deployment.rs#L61)
   requires absent placement and private retention refuses non-Deploy;
   [continuation:226](../../crates/buzz-paired-owner/src/deployment/continuation.rs#L226)
   builds Deploy only. [lifecycle:1037](../../desktop/src-tauri/src/commands/agents.rs#L1037)
   and [delete:1253](../../desktop/src-tauri/src/commands/agents.rs#L1253) explicitly
   refuse RegisteredHost actions. Underlying
   [host cleanup:453](../../crates/buzz-hostd/src/deployment.rs#L453) and relay
   `Action::Cleanup` already exist. Smallest proposed repair under **#3724/#3725**:
   owner-retained Cleanup intent after fresh exact placement/revision/permission
   reconciliation, dispatch/retry exact bytes via current management channel,
   narrowly expose setup cleanup, reconcile current authenticated acknowledgement,
   and only then allow local removal. Retain identity/record/intent until confirmed
   cleanup; preserve retries on absent/failed/unknown acknowledgement. This needs
   no new protocol, generic installer or full lifecycle UI redesign. It is NOT
   deferred legacy cleanup #3767. No repair was implemented here.

Both are pilot gates. Recommended order: resolve/read-verify relay compatibility
and implement/review the narrow cleanup affordance; refresh matched artifacts if
source changes; then seek precise live approval; host transition and native checks;
Windows flag; one Grok; models/chat; confirmed teardown. An emergency stopped unit
with retained journal is rollback containment, never successful cleanup.

## Artifacts and finite preparation checks

Evidence root:
`/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/c97256e20893b2688c1cc1c6bc2e23361aab707e/`.
Package is its `package/` directory; it is the exact proposed executable location,
not a generic latest symlink. No old release was overwritten. SOURCE.json records
Cargo.lock hash, toolchain, build/source pins, sizes, checks and limits.

| Built file | Bytes | SHA256 |
| --- | ---: | --- |
| buzz-hostd | 18,201,640 | `fa4a2961506f82a2c356aafd5413cd4e7b96b54f3f88b7ad976311b79cf264d3` |
| buzz-host-runtime | 3,818,744 | `18f4b89d96e56c0c976d144766d214f9b32994d6ed0bda779f73f76b6b1b4f17` |
| buzz-acp | 18,545,880 | `b7fe18f1604db51dfeffc8d392676d8755f872662be710b87e84c11fc1e83f33` |

Package MANIFEST.sha256 hash:
`f8a53553ca83eb4fe56481d9a257d9226840d1908ccbb888c9d4e8829cb94898`.
OPERATOR.md hash:
`aaae22077602a16228965671a9e0608a749d4f789eb1e8dc511d934497550ab5`.
Exact staged config/unit text and runbook are included below for durable review;
the hosted template is byte-identical to the pinned source template.

| Gate | Measured outcome |
| --- | --- |
| `HERMIT_SELF_UPDATE=false; . ./bin/activate-hermit; cargo build --offline --locked --release -p buzz-hostd --bin buzz-hostd --bin buzz-host-runtime -p buzz-acp --bin buzz-acp` | PASS exit0, optimized1m26s; build.log; rustc/cargo1.98.1; no installs/toolchain update |
| Staged `buzz-hostd --help`, `buzz-hostd registration run --help`, `buzz-acp models --help` | PASS exit0 each; saved text; no real state/relay/adapter used |
| Staged `buzz-host-runtime --help` | Expected sanitized refusal exit1 (binary has no help mode); PASS invalid invocation, no runtime started |
| JSON parse/structure and template byte comparison | PASS; structural check only, not production Catalog::read against future paths |
| `systemd-analyze --user verify` staged controller and synthetic generated hosted instance | PASS exit0 each, empty diagnostics; syntax-verification.json. Syntax only, no manager install/start/reload |
| Runbook nine shell fences via `bash -n`; two Python heredocs via `ast.parse` | PASS after M-1 repair; no runbook commands executed |
| `sha256sum -c MANIFEST.sha256`; exact staged-text/document-link checks; `git diff --check` | PASS, all8 package files; both docs and exact staged text verified |

Initial sandbox attempts could not read root-owned sanitized receipts/connect
systemd, failed DNS, and syntax verify returned SO_PASSCRED EPERM. Narrow approved
read-only retries succeeded. These were environment restrictions; no service or
provider actions were used to resolve them. Raw credential/provider stores were
never opened. Actual manager availability was measured by a readonly Version query,
not inferred from socket existence or syntax verification.

**NOT RUN:** real service credential delivery/notify/cgroups, new runtime start,
normal-account current provider readiness/login/models/subscription/chat, Windows
hosted create/cleanup, actual rollback, reboot and full just ci. Accepted source
and Windows tests stand; no new behavioral coverage is claimed. Native activation
and provider checks remain future explicit authorization. No PR/merge readiness
or `buzz-review-completed` attestation.

## Proposed live targets and approval action

Only proposed: system registration unit's stop/disable/mask; new root-private backup;
same `/var/lib/buzz-host-registration` ownership and new deployment.json/hosted state;
new bryce user controller and exact hosted template/generated opaque-ID drop-in;
new empty pilot workspace; one new agent's Buzz identity/placement/config through
ordinary Windows Add Agent; process-local Windows creation flag. Current installed
system binary/config remain for rollback; existing Claude/Grok/Scout unchanged.
No relay target is included; that needs its separate compatibility plan.

After both blockers and independent review are satisfied, ask approval for this
exact host cutover/custody transfer, native service/models/chat checks, ONE new
Grok and supported confirmed cleanup, with the documented emergency containment
and restoration limits. **Do not request activation against this currently blocked
packet.** Remaining decisions are owned by conductor; no new product choice was
silently implemented.

## M-1 repair — interruption-aware rollback, 2026-11-10

Continuation of this same preparation assignment from clean review pin
`c6c1ae869b7d41545f48fe851ca20c107a85223b`. Input: independent
[M-1](../audits/post-audit/3721-trusted-host-pilot-preparation.md#m-1-medium-high-confidence--rollback-only-covers-a-completed-mask),
conductor disposition fix-in-lane. No source/binary/config change or live action.

The runbook now selects observed `original`, `moved` or `masked` system-unit
state before recovery. It proves old-controller stop even when the forward stop
was uncertain, handles a missing user controller without unconditional disable,
and recognizes exact installation before daemon-reload or cached state after
file removal. Before ownership restoration it requires both controllers and any
recorded pilot quiescent. An early failed archive leaves the original unit and
unchanged988:988 tree recoverable without a nonexistent saved-unit/archive
requirement. Partial custody instead requires a fresh private CURRENT-state
snapshot, link/type/owner/mode validation, explicit numeric988:988 restoration,
0700 directories/0600 files and post-validation. Exact owned user files are removed;
instances keep their template. Only the matching system-unit branch restores a
verified original, then returns the recorded enabled/active state. Every failure
stops before start; current database/journal are never replaced by an archive.

The M-1 change is operator instructions only, not an installed recovery utility.
Python heredocs inspect metadata; they are isolated from Python environment options
and were syntax-parsed, never executed against state. Shell fences were checked
with `bash -n`, never run. A static inspection covered seven named prefixes,
including interruption during rollback unlink/move and before original restart.
`m1-repair-checks.json` records that inspection as read/inferred, not fault injection.

**Measured:** nine shell blocks and two Python heredocs parse; unchanged three
binary hashes/sizes and three config/template hashes match the original manifest;
new manifest8/8, exact staged-document text, local links and `git diff --check`
pass. No rebuild, help rerun, systemd call, provider/native check, live state read
or fault-fixture campaign. Previous runtime limitations and BOTH pilot blockers
remain. Original package manifest/runbook/SOURCE are retained in `history/pre-M1/`;
original manifest SHA256 is
`02e2ffe9adcb06a8c83b51b51921c0c83e93654de6ad0b222f2b4318b1655490`.
Only OPERATOR.md, SOURCE.json preparation-check annotations and MANIFEST.sha256
changed in the package. Independent report is preserved unchanged. Next gate:
same reviewer, M-1 and defects introduced by this repair only.

## Exact staged configuration (proposal only)

### deployment.json

```json
{
  "label": "netcup",
  "capacity": 1,
  "community": "https://buzz.interleaven.com",
  "relay_key": "9fd495a8df792128e1396c3766b12228076cad0ea7b791c3d5fbc446f74d62c1",
  "runtimes": {},
  "local": {
    "buzz_acp": "/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/c97256e20893b2688c1cc1c6bc2e23361aab707e/package/buzz-acp",
    "home": "/home/bryce",
    "workspace": "/home/bryce/services/buzz-agents/workspaces/3721-grok-pilot",
    "tool_path": "/home/bryce/.local/bin:/home/bryce/.cargo/bin:/home/bryce/.bun/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
    "environment": {
      "XDG_RUNTIME_DIR": "/run/user/1002"
    },
    "unit_dir": "/home/bryce/.config/systemd/user",
    "launcher": "/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/c97256e20893b2688c1cc1c6bc2e23361aab707e/package/buzz-host-runtime",
    "systemctl": "/usr/bin/systemctl"
  }
}
```

### buzz-host-registration.service

```ini
[Unit]
Description=Buzz netcup trusted host controller
StartLimitIntervalSec=30min
StartLimitBurst=3

[Service]
Type=simple
ExecStart=/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/c97256e20893b2688c1cc1c6bc2e23361aab707e/package/buzz-hostd --state-dir /var/lib/buzz-host-registration registration run --deployment --community https://buzz.interleaven.com --relay-key 9fd495a8df792128e1396c3766b12228076cad0ea7b791c3d5fbc446f74d62c1 --label netcup
WorkingDirectory=/home/bryce
Environment=HOME=/home/bryce
Environment=PATH=/home/bryce/.local/bin:/home/bryce/.cargo/bin:/home/bryce/.bun/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
Environment=XDG_RUNTIME_DIR=/run/user/1002
Environment=DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1002/bus
UMask=0077
Restart=on-failure
RestartSec=30s
KillMode=control-group
TimeoutStopSec=30s
LimitCORE=0
StandardOutput=null
StandardError=null

[Install]
WantedBy=default.target
```

## Exact operator runbook (proposal only)

# #3721 netcup trusted pilot — prepared, NOT approved to execute

This is a bounded operator runbook, not an installer. All commands below target
live systems and require explicit approval AFTER independent review. Current
activation holds: public relay NIP-11 has no host_deployment capability, and the
shipped owner/setup flow has no confirmed per-agent cleanup action. Resolve
both separately before touching netcup or attempting the disposable pilot. No command below was run in preparation.

## Reviewed contents and purpose

Package/source c97256e20893b2688c1cc1c6bc2e23361aab707e is behaviorally identical
to Windows test.21 source23d2dc15c6c45554181938b7956b59c085980455 (only three docs
differ). hostd, buzz-host-runtime and buzz-acp are matched Linux release builds.
The current installed registration binary is de05655071c5f6af3451cc70116969f53da0df0ff64448593a0fc913bf10d3ee,
from the later #3750 refresh; do not restore the older #3751 package binary.

One controller changes from system service UID/GID988 to bryce UID1002/GID1003.
Its state remains /var/lib/buzz-host-registration. Its real host key, owner binding,
journal, revocations and outbox are retained, never initialized/copied to a second
controller. This expressly transfers host-key custody to bryce, including programs
running as bryce. That is the ratified trusted-account boundary. Existing
buzz-agent@{claude,grok,scout}.service units, keys/config/provider stores stay intact.
Normal HOME, network, tools and native login are shared. No root helper, per-agent
account, private HOME, RootDirectory, network restriction or credential broker.

Automatic main discovery is configured by local.buzz_acp and runtimes:{}; capacity1
limits managed placements, not the existing three agents. Only ONE new Grok is
approved for the proposed pilot. Normal PATH entries are explicit, not inherited
from Hermit. Grok resolves through /home/bryce/.local/bin/grok to the already
installed1.0.70 executable. Its native subscription/auth status is NOT measured.
An empty ordinary workspace is assigned at
/home/bryce/services/buzz-agents/workspaces/3721-grok-pilot; it is not a sandbox.
No XDG/provider store override is used except normal XDG_RUNTIME_DIR=/run/user/1002.

## Prerequisites and exact authorization boundary

1. Independent review accepts these files and manifest. Freeze them afterward.
2. Separate relay operator establishes deployed source compatibility, required
   schema and BUZZ_HOST_DEPLOYMENT_V1=true with registry v2 still enabled. Public
   NIP-11 must advertise host_deployment={versions:[1],kind:43104,
   admission:"host-scoped",reads:"req-v1"}, and the pinned public key must match
   deployment.json. Current public absence cannot distinguish missing code from
   disabled flag. The historical relay receipt names f316fb46, predating deployment;
   no newer authoritative serving-binary receipt was found. A flag-only operation
   is NOT justified without that inspection. No relay update is part of this plan.
3. Separately complete the narrow owner-authorized Cleanup continuation and setup
   affordance under #3724/#3725, using the existing relay/host cleanup machinery;
   independently verify confirmed cleanup. Current client can only Check/Retry/
   Refresh/Launch; Close only dismisses the view. No package CLI supplies cleanup.
4. Obtain approval for the exact netcup cutover, provider models/chat attempts, ONE
   disposable Grok, and the bounded emergency stop described below. Keep the Windows
   creation flag off until host acceptance. Native login changes are separately
   human-directed if needed; do not copy/reset credentials or log out existing agents.
5. Recheck public NIP-11 just before cutover. Recheck current unit/hash, UID/GID,
   user-manager availability, Linger=yes, target absence and existing agent baseline.
   Any difference from the recorded context stops the operation for assessment.
6. The assigned operator owns the entire stop/backup/cutover sequence; do not leave
   a detached script or a second controller. Record completion of each step privately.

Run as bryce unless a block explicitly says root. Define in each shell as needed:

```sh
pkg=/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/c97256e20893b2688c1cc1c6bc2e23361aab707e/package
(cd "$pkg" && sha256sum -c MANIFEST.sha256) || exit 1
systemctl --user show -p Version
loginctl show-user bryce -p Linger
systemctl --user show buzz-agent@claude.service buzz-agent@grok.service buzz-agent@scout.service \
  -p Id -p MainPID -p NRestarts -p ExecMainStartTimestamp -p ActiveState -p SubState
```

Save the last output as the activation baseline; compare after every transition.
Check hashes of existing unit/config files without printing their contents if the
existing continuity receipt is being renewed. Do not use `show-environment` or logs.
Require these targets absent (lstat, including dangling symlinks; lookup errors
are NOT absence): ~/.config/systemd/user/buzz-host-registration.service,
~/.config/systemd/user/buzz-hosted-agent@.service, any buzz-hosted-agent@*.service.d,
/var/lib/buzz-host-registration/deployment.json, /var/lib/buzz-host-registration/hosted,
and the pilot workspace. All are new names, not adoption of a prior pilot.
Existing user unit directory must be canonical; never chmod/chown that shared dir.
Require the current system unit to have no drop-ins and hash
9cb7a6d7b9df3e18b39d46916d1d7b32140e0567e123305dfe8685a5e8a03bce.
Current state metadata is directory988:988/0700, host.sqlite and reconciler.lock
988:988/0600. SQLite sidecars may appear; validate all entries before custody changes.
The public binding baseline is host66f5d7c51f4dbccc9746204f5860e57c98e30a56b777d05c6f4f209cd7f38d7a,
ownerf5c79eab2f44284ff278780c34aab621aafb149be4d7dba8f9625bfd99419733,
generation6eaf423e2e2fffa23e142135bfb3583ba22dcbadf0daedc57c747fe54ec28b40,
communityhttps://buzz.interleaven.com. Recheck only public columns with a readonly
SQLite connection after stopping; never print SELECT*, secret, proof or payload.

## Cutover — root shell, after all prerequisites and approval

Use `set -eu`; stop on any failure and inspect the completed prefix. This block is
intentionally not installed as an executable. The backup directory must not exist.

```sh
set -eu
umask 077
pkg=/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/c97256e20893b2688c1cc1c6bc2e23361aab707e/package
backup=/var/backups/buzz-host-registration/3721-trusted-c97256e
# Before stopping, assert backup path absent and current baseline above matches.
install -d -m 0700 -o root -g root "$backup"
systemctl disable --now buzz-host-registration.service
systemctl show buzz-host-registration.service -p ActiveState -p SubState -p MainPID
# Require inactive/dead and MainPID=0; verify old cgroup empty before proceeding.
# No user controller may exist yet. Keep the old executable/config untouched.
tar --acls --xattrs --numeric-owner -cpf "$backup/state-before.tar" \
  -C /var/lib buzz-host-registration
chmod 0600 "$backup/state-before.tar"
sha256sum "$backup/state-before.tar" > "$backup/state-before.sha256"
# Verify archive readability privately; do not print state contents.
tar -tf "$backup/state-before.tar" >/dev/null
cp -a /usr/local/libexec/buzz-hostd "$backup/buzz-hostd"
cp -a /etc/buzz-host-registration/controller.conf "$backup/controller.conf"
# mask cannot replace a regular /etc unit: save it first, then place the mask.
mv /etc/systemd/system/buzz-host-registration.service "$backup/buzz-host-registration.service"
ln -s /dev/null /etc/systemd/system/buzz-host-registration.service
systemctl daemon-reload
mask_state=$(systemctl is-enabled buzz-host-registration.service 2>/dev/null || true)
[ "$mask_state" = masked ] || exit 1
```

The mask check accepts only literal `masked`; any failed lookup with no such
answer refuses. Before custody transfer validate the stopped tree: no symlinks, special files, hardlinked
files, foreign UID/GID or group/world-accessible entries; canonical state directory.
At initial cutover only host.sqlite, reconciler.lock and optional SQLite sidecars
are allowed. Record exact numeric ownership/modes of every path in the root-only
backup. If unexpected entries exist, stop; do not recursively adopt them.
Then change ownership of that validated tree, preserving modes:

```sh
chown -R --no-dereference bryce:bryce /var/lib/buzz-host-registration
install -o bryce -g bryce -m 0600 "$pkg/deployment.json" /var/lib/buzz-host-registration/deployment.json
install -d -o bryce -g bryce -m 0700 /home/bryce/services/buzz-agents/workspaces/3721-grok-pilot
install -o bryce -g bryce -m 0600 "$pkg/buzz-host-registration.service" /home/bryce/.config/systemd/user/buzz-host-registration.service
install -o bryce -g bryce -m 0600 "$pkg/buzz-hosted-agent@.service" /home/bryce/.config/systemd/user/buzz-hosted-agent@.service
```

No root StateDirectory-managed service can restart while the mask is present.
Keep the system mask until custody has been restored on rollback. Avoid `init`,
`offer`, pairing/revoke or copying the key into a new state directory.

## Start — bryce shell, after root block completed

```sh
export XDG_RUNTIME_DIR=/run/user/1002
export DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1002/bus
pkg=/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/c97256e20893b2688c1cc1c6bc2e23361aab707e/package
"$pkg/buzz-hostd" --state-dir /var/lib/buzz-host-registration registration status
# Require same public host/community/generation; verify owner via readonly public-column query.
systemd-analyze --user verify ~/.config/systemd/user/buzz-host-registration.service
systemctl --user daemon-reload
systemctl --user enable --now buzz-host-registration.service
systemctl --user show buzz-host-registration.service -p MainPID -p ActiveState -p SubState -p NRestarts
```

Require active/running, expected executable hash and UID1002, stable registration
in the installed test.21 Windows client, no old system controller process, unchanged
existing-agent baseline. MainPID/active alone is NOT relay or provider acceptance.
New private hosted/ directory and additive deployment journal tables are expected.
No registration refresh through an independent controller. If startup fails,
restore as below; do not loosen permissions or invoke the legacy root helper.

## ONE disposable Grok — Windows owner, only after native host checks

Fully quit Buzz, then launch the installed test.21 executable with process-local
`BUZZ_NATIVE_HOST_DEPLOYMENT_V1=1` (PowerShell `$env:BUZZ_NATIVE_HOST_DEPLOYMENT_V1='1'`;
Start-Process the actual installed executable). Do not set a global persistent flag.
Keep the existing profile/identity. Add Agent → Advanced → Run on → netcup → Grok,
using ordinary name, channel and instruction-policy inputs. Use a clearly disposable
name such as Grok pilot3721; retain the new agent pubkey, operation, exact opaque
unit ID and placement revision from sanitized current operation evidence. Do not
infer unit names from the display name or touch buzz-agent@grok.service.

Observe current main inventory/readiness, prepare, Refresh models, choose a returned
opaque model ID, then Launch. Grok NotApplicable is not proof of authentication;
models success is not subscription/entitlement proof. If native login is needed,
Bryce uses Grok's own supported login directly as bryce with HOME=/home/bryce;
no private service HOME, copying credentials, agent-auth broker or API fallback.
Do not conduct a fresh login unless separately directed; shared credentials affect
all bryce agents. Refresh the SAME prepared agent afterward, not create another.

Measure actual user-service credential delivery, Type=notify readiness, cgroup
containment, normal workspace/HOME/PATH, strict selected-model launch, and one
controlled ordinary chat reply through main ACP. Preserve only sanitized outcomes;
no credential/environment dumps or full conversations. Recheck three old units.
On any uncertainty stop this pilot; no repeated provider loop, automatic installs,
model fallback or toolchain update. Reboot/agent autostart is not part of this pilot;
Linger=yes exists but actual reboot was not run. Agent template has Restart=no.

## Rollback — select the observed prefix, never replay cutover

These are human-operated recovery blocks, not an installed script. All remain
subject to the live approval gate. Fully quit Windows and remove its process-local
creation flag first. Use one root shell with `set -eu`. A failed lookup, failed
stop, unexpected file/hash/owner/mode or unavailable manager stops recovery before
custody changes or old-service start. Never restore a stale database snapshot,
run init, revoke real netcup or discard a placement/journal.

### 1. Identify the system-unit prefix and establish quiescence

This also covers interruption before old stop or an uncertain stop result. Inspect
actual state; an originally active process is not assumed dead because a command
was issued. The prior recorded state to restore is **enabled and active**, UID/GID988.

```sh
set -eu
umask 077
pkg=/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/c97256e20893b2688c1cc1c6bc2e23361aab707e/package
backup=/var/backups/buzz-host-registration/3721-trusted-c97256e
state=/var/lib/buzz-host-registration
system_unit=/etc/systemd/system/buzz-host-registration.service
saved_unit=$backup/buzz-host-registration.service
user_unit=/home/bryce/.config/systemd/user/buzz-host-registration.service
template=/home/bryce/.config/systemd/user/buzz-hosted-agent@.service
unit_hash=9cb7a6d7b9df3e18b39d46916d1d7b32140e0567e123305dfe8685a5e8a03bce
# Convenience for this one account only; never use root's user manager.
userctl() { runuser -u bryce -- env XDG_RUNTIME_DIR=/run/user/1002 DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1002/bus /usr/bin/systemctl --user "$@"; }
if [ -L "$system_unit" ]; then
  [ "$(readlink "$system_unit")" = /dev/null ] || exit 1
  prefix=masked
elif [ -f "$system_unit" ]; then
  prefix=original
else
  [ ! -e "$system_unit" ] && [ ! -L "$system_unit" ] || exit 1
  prefix=moved
fi
if [ "$prefix" = original ]; then
  original=$system_unit
else
  # Required only once the original unit was moved; never unlink without it.
  original=$saved_unit
fi
[ -f "$original" ] && [ ! -L "$original" ] || exit 1
[ "$(stat -c '%u:%g:%a' "$original")" = 0:0:644 ] || exit 1
[ "$(sha256sum "$original" | cut -d ' ' -f 1)" = "$unit_hash" ] || exit 1
old_load=$(systemctl show buzz-host-registration.service -p LoadState --value)
case "$old_load" in
  loaded|masked) systemctl stop buzz-host-registration.service ;;
  not-found) ;; # acceptable only if quiescent checks below also succeed
  *) exit 1 ;;
esac
[ "$(systemctl show buzz-host-registration.service -p MainPID --value)" = 0 ] || exit 1
old_state=$(systemctl show buzz-host-registration.service -p ActiveState --value)
case "$old_state" in inactive|failed) ;; *) exit 1 ;; esac
old_cgroup=$(systemctl show buzz-host-registration.service -p ControlGroup --value)
if [ -n "$old_cgroup" ] && [ -d "/sys/fs/cgroup$old_cgroup" ]; then
  old_tasks=$(find "/sys/fs/cgroup$old_cgroup" -name cgroup.procs -exec cat {} +)
  [ -z "$old_tasks" ] || exit 1
fi
if [ "$prefix" = original ]; then
  systemctl disable buzz-host-registration.service
fi
```

Check the recorded old PID/cgroup is gone as well if manager metadata lost it.
If an old process survives, do not continue. The original regular-unit branch
covers failed/incomplete state backup: it does not require a saved unit or archive.
The moved branch covers the gap between `mv` and mask creation. The masked branch
keeps its mask throughout custody restoration.

### 2. Stop the new USER controller, or prove it was never installed

```sh
new_load=$(userctl show buzz-host-registration.service -p LoadState --value)
# Interrupted install before daemon-reload: inspect exact file before loading it.
if [ "$new_load" = not-found ] && { [ -e "$user_unit" ] || [ -L "$user_unit" ]; }; then
  [ -f "$user_unit" ] && [ ! -L "$user_unit" ] || exit 1
  [ "$(stat -c '%u:%g:%a' "$user_unit")" = 1002:1003:600 ] || exit 1
  cmp "$pkg/buzz-host-registration.service" "$user_unit"
  userctl daemon-reload
  new_load=$(userctl show buzz-host-registration.service -p LoadState --value)
fi
case "$new_load" in
  not-found)
    [ ! -e "$user_unit" ] && [ ! -L "$user_unit" ] || exit 1
    ;;
  loaded)
    [ "$(userctl show buzz-host-registration.service -p FragmentPath --value)" = "$user_unit" ] || exit 1
    userctl stop buzz-host-registration.service
    if [ -e "$user_unit" ] || [ -L "$user_unit" ]; then
      [ -f "$user_unit" ] && [ ! -L "$user_unit" ] || exit 1
      [ "$(stat -c '%u:%g:%a' "$user_unit")" = 1002:1003:600 ] || exit 1
      cmp "$pkg/buzz-host-registration.service" "$user_unit"
      userctl disable buzz-host-registration.service
    else
      # Rollback interrupted after unlink, before daemon-reload: no file to disable.
      # Its enablement must already be gone; never silently skip a dangling link.
      wants=/home/bryce/.config/systemd/user/default.target.wants/buzz-host-registration.service
      [ ! -e "$wants" ] && [ ! -L "$wants" ] || exit 1
    fi
    ;;
  *) exit 1 ;;
esac
[ "$(userctl show buzz-host-registration.service -p MainPID --value)" = 0 ] || exit 1
new_state=$(userctl show buzz-host-registration.service -p ActiveState --value)
case "$new_state" in inactive|failed) ;; *) exit 1 ;; esac
new_cgroup=$(userctl show buzz-host-registration.service -p ControlGroup --value)
if [ -n "$new_cgroup" ] && [ -d "/sys/fs/cgroup$new_cgroup" ]; then
  new_tasks=$(find "/sys/fs/cgroup$new_cgroup" -name cgroup.procs -exec cat {} +)
  [ -z "$new_tasks" ] || exit 1
fi
```

The pre-branch exact-file check handles installation before daemon-reload. A
cached loaded unit whose file was already removed is still stopped and checked;
missing file alone never proves process absence. An incomplete/mismatched file or
unknown cached unit must be assessed while stopped; never overwrite it or proceed
into old start.

If a pilot exists, stop only its recorded opaque unit AFTER controller quiescence:

```sh
# Execute this block only when the recorded pilot placement exists.
# Set pilot_unit from its retained sanitized operation, never from its name.
printf '%s\n' "$pilot_unit" | /usr/bin/grep -Eq '^[0-9a-f]{64}$' || exit 1
userctl stop "buzz-hosted-agent@$pilot_unit.service"
[ "$(userctl show "buzz-hosted-agent@$pilot_unit.service" -p MainPID --value)" = 0 ] || exit 1
pilot_state=$(userctl show "buzz-hosted-agent@$pilot_unit.service" -p ActiveState --value)
case "$pilot_state" in inactive|failed) ;; *) exit 1 ;; esac
pilot_cgroup=$(userctl show "buzz-hosted-agent@$pilot_unit.service" -p ControlGroup --value)
if [ -n "$pilot_cgroup" ] && [ -d "/sys/fs/cgroup$pilot_cgroup" ]; then
  pilot_tasks=$(find "/sys/fs/cgroup$pilot_cgroup" -name cgroup.procs -exec cat {} +)
  [ -z "$pilot_tasks" ] || exit 1
fi
```

Require no surviving provider descendants. An unrecorded instance or uncertain
stop blocks restoration. Leave that instance's drop-in/keys/placement/journal
retained. Emergency stop is NOT confirmed teardown; current owner/UI Cleanup is
still a #3724/#3725 pilot blocker. #3767 is unrelated legacy cleanup. Do not proceed
with a pilot merely because emergency containment exists. Existing agents stay up.

### 3. Validate the current tree; snapshot and restore partial custody

Run only after both controllers and any pilot are proven quiescent. The tree may
mix988:988 and1002:1003 after an interrupted recursive chown. Validate metadata
without reading keys/database contents. Reject links/special files/foreign owners
and unexpected modes; do not normalize unexplained state. All original entries
were988:988, directory0700 and files0600. Known new private managed files use the
same modes; keep them and the current database/sidecars/journal.

```sh
custody=$(python3 -I - <<'PY_CUSTODY'
import os, re, stat
from pathlib import Path
root = Path('/var/lib/buzz-host-registration')
assert root.resolve(strict=True) == root
allowed = {'host.sqlite','host.sqlite-wal','host.sqlite-shm','host.sqlite-journal','reconciler.lock','deployment.json','hosted'}
entries = [root, *root.rglob('*')]
changed = False
for p in entries:
    m = p.lstat()
    directory = stat.S_ISDIR(m.st_mode)
    assert directory or (stat.S_ISREG(m.st_mode) and m.st_nlink == 1), p
    assert (m.st_uid, m.st_gid) in {(988,988),(1002,1003)}, p
    assert stat.S_IMODE(m.st_mode) == (0o700 if directory else 0o600), p
    if p.parent == root:
        assert p.name in allowed or re.fullmatch(r'[0-9a-f]{64}\.cred', p.name), p
        if p.name in {'deployment.json','hosted'} or p.name.endswith('.cred'):
            changed = True
    changed |= (m.st_uid, m.st_gid) != (988,988)
assert (root/'host.sqlite').is_file() and (root/'reconciler.lock').is_file()
print('changed' if changed else 'unchanged')
PY_CUSTODY
)
case "$custody" in
  unchanged)
    # Early backup failure: original tree still exact988:988/private; no archive
    # is required to restart it. Keep failed backup evidence; never restore it.
    ;;
  changed)
    [ -d "$backup" ] && [ ! -L "$backup" ] || exit 1
    [ "$(stat -c '%u:%g:%a' "$backup")" = 0:0:700 ] || exit 1
    # A fresh private snapshot of CURRENT stopped state, not an old DB restore.
    snapshot=$(mktemp "$backup/state-latest.XXXXXX.tar")
    tar --acls --xattrs --numeric-owner -cpf "$snapshot" -C /var/lib buzz-host-registration
    chmod 0600 "$snapshot"
    tar -tf "$snapshot" >/dev/null
    sha256sum "$snapshot" > "$snapshot.sha256"
    find "$state" -printf '%P %U:%G %m\n' > "$snapshot.metadata"
    chmod 0600 "$snapshot.sha256" "$snapshot.metadata"
    chown -R --no-dereference 988:988 "$state"
    # Input validation established exact allowed modes. Restore and verify them.
    find "$state" -type d -exec chmod 0700 {} +
    find "$state" -type f -exec chmod 0600 {} +
    ;;
  *) exit 1 ;;
esac
python3 -I - <<'PY_VERIFY'
import stat
from pathlib import Path
root = Path('/var/lib/buzz-host-registration')
assert root.resolve(strict=True) == root
for p in [root, *root.rglob('*')]:
    m = p.lstat()
    directory = stat.S_ISDIR(m.st_mode)
    assert directory or (stat.S_ISREG(m.st_mode) and m.st_nlink == 1), p
    assert (m.st_uid, m.st_gid) == (988,988), p
    assert stat.S_IMODE(m.st_mode) == (0o700 if directory else 0o600), p
PY_VERIFY
```

If snapshot creation fails after custody changed, keep controllers stopped and
current state intact for assessment; do not bypass the failure. If interrupted
during custody restoration, repeat quiescence/validation, snapshot the current
mixed tree and complete numeric restoration. Never extract an archive over state.

### 4. Remove exact owned user files; restore the matching system-unit branch

```sh
# Both absent and partially installed sets are allowed, but no unexpected files.
if [ -e "$user_unit" ] || [ -L "$user_unit" ]; then
  [ -f "$user_unit" ] && [ ! -L "$user_unit" ] || exit 1
  [ "$(stat -c '%u:%g:%a' "$user_unit")" = 1002:1003:600 ] || exit 1
  cmp "$pkg/buzz-host-registration.service" "$user_unit"
  rm -- "$user_unit"
fi
# Preserve template whenever any recorded instance drop-in remains.
instances=$(find /home/bryce/.config/systemd/user -maxdepth 1 -name 'buzz-hosted-agent@*.service.d' -print)
if [ -z "$instances" ] && { [ -e "$template" ] || [ -L "$template" ]; }; then
  [ -f "$template" ] && [ ! -L "$template" ] || exit 1
  [ "$(stat -c '%u:%g:%a' "$template")" = 1002:1003:600 ] || exit 1
  cmp "$pkg/buzz-hosted-agent@.service" "$template"
  rm -- "$template"
fi
userctl daemon-reload
# Reassert no user or old system controller restarted before opening old custody.
[ "$(userctl show buzz-host-registration.service -p MainPID --value)" = 0 ] || exit 1
case "$(userctl show buzz-host-registration.service -p ActiveState --value)" in inactive|failed) ;; *) exit 1 ;; esac
[ "$(systemctl show buzz-host-registration.service -p MainPID --value)" = 0 ] || exit 1
case "$(systemctl show buzz-host-registration.service -p ActiveState --value)" in inactive|failed) ;; *) exit 1 ;; esac
# Recheck source hash immediately before any removal/move.
[ -f "$original" ] && [ ! -L "$original" ] || exit 1
[ "$(stat -c '%u:%g:%a' "$original")" = 0:0:644 ] || exit 1
[ "$(sha256sum "$original" | cut -d ' ' -f 1)" = "$unit_hash" ] || exit 1
case "$prefix" in
  original)
    # Original /etc regular file never moved: leave it in place. No backup needed.
    ;;
  moved)
    [ ! -e "$system_unit" ] && [ ! -L "$system_unit" ] || exit 1
    mv -- "$saved_unit" "$system_unit"
    ;;
  masked)
    [ -L "$system_unit" ] && [ "$(readlink "$system_unit")" = /dev/null ] || exit 1
    unlink "$system_unit"
    mv -- "$saved_unit" "$system_unit"
    ;;
  *) exit 1 ;;
esac
# A failed unlink/move terminates this set-e shell; never continue into start.
[ -f "$system_unit" ] && [ ! -L "$system_unit" ] || exit 1
[ "$(stat -c '%u:%g:%a' "$system_unit")" = 0:0:644 ] || exit 1
[ "$(sha256sum "$system_unit" | cut -d ' ' -f 1)" = "$unit_hash" ] || exit 1
[ "$(sha256sum /usr/local/libexec/buzz-hostd | cut -d ' ' -f 1)" = de05655071c5f6af3451cc70116969f53da0df0ff64448593a0fc913bf10d3ee ] || exit 1
systemctl daemon-reload
systemctl enable --now buzz-host-registration.service
[ "$(systemctl is-enabled buzz-host-registration.service)" = enabled ] || exit 1
[ "$(systemctl show buzz-host-registration.service -p ActiveState --value)" = active ] || exit 1
[ "$(systemctl show buzz-host-registration.service -p User --value)" = buzz-host-registration ] || exit 1
```

Recheck public binding and existing-agent continuity using the baseline procedure.
Keep current DB, retained hosted data and private snapshots. A DB-open failure
stops recovery for review; no reset/init. Workspace removal is optional `rmdir`
only if empty. Existing HOME/provider stores/repos and real host pairing remain.

| Observed interruption | Recovery route |
| --- | --- |
| Before old stop / result uncertain | Section1 proves actual stop/cgroup state; no custody assumption. |
| Stop/disable succeeded; archive/copy failed | `original` + unchanged custody; no saved-unit/archive dependency. |
| Original unit moved; mask not made | `moved`; verify saved file, restore into truly absent destination. |
| Mask made; chown partly/fully done | `masked`; sections2–3 stop new work, snapshot current mixed tree, restore988:988/0700/0600. |
| Only some user files installed | Absent units must be manager-quiescent; installed exact files are stopped/disabled/removed. No reload yet: reload only after exact-file inspection, then repeat section2. |
| Rollback interrupted between unlink and mv | Re-enter from section1; actual `moved` branch uses the still-saved original. |
| Original restored but start not reached | Re-enter `original` branch; current journal/custody verified before restoring enabled/active. |

Unknown state stays stopped for assessment. These branches do not close either
activation blocker or authorize any live action during preparation/review.
