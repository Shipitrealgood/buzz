# #3734 / #3721 — local netcup rollout preparation

2026-11-10. **Local P1 preparation complete for independent review; live activation
NOT authorized. WSL relay artifact/compatibility proof remains pending.**
Executor `/root/netcup_rollout_prepare`, codecodex; requested gpt-6-astra/high,
actual served configuration not exposed. Applied execute and brief. This is
executor evidence, not the independent review gate.

## Scope and ownership

Workspace `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`; starting and actual build HEAD
`344baa091ad486653ee99aa8b1de64d4839b4ada`. Starting worktree clean, no unrelated changes.
Only this receipt and [review brief](../briefs/3734-netcup-rollout-review.md) are
changed in Git. New artifacts are at [/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/344baa091ad486653ee99aa8b1de64d4839b4ada/package](/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/344baa091ad486653ee99aa8b1de64d4839b4ada/package).
No product edits, push/workflow, taskctl mutation, SSH, controller/agent operation,
real state/key access, provider command/login, signed traffic or live metadata
refresh occurred. Claude, Grok and Scout remain outside every executed command.
Only build, staged-file checks and documentation work ran; all owned processes
are terminal when this handoff is returned. Ownership releases at signed commit.

Accepted inputs: [ratified spec](../specs/3734-hetzner-netcup-rollout-preparation.md),
[dispatch brief](../briefs/3734-hetzner-netcup-rollout-spec.md),
[old execution/runbook](3721-trusted-host-pilot-preparation.md),
[M-1 independent re-gate](../audits/post-audit/3721-trusted-host-pilot-preparation.md),
[Cleanup/native final review](../audits/post-audit/3725-confirmed-hosted-cleanup.md:394),
portable conductor contract, applicable AGENTS/VISION/agent/remote-agent vision,
and TESTING including ACP guidance. Accepted architecture/source/native stages
were reused, not restarted.

## Source continuity and package provenance

Measured Hermit-activated `git diff --name-only 929c2cd890fbc2c0993549c3cbc64a2714232832..344baa091ad486653ee99aa8b1de64d4839b4ada`
lists exactly six Markdown files: Cleanup execution/brief/audit and3734 spec,
executor brief, WSL brief. No production/test/build inputs differ. The actual
build head is344baa0, never relabeled as929c. From old package sourcec972, the
scoped host dependencies (`crates/buzz-hostd`, `crates/buzz-acp`,
`crates/buzz-ws-client`, `Cargo.lock`, `deploy`) differ only in the accepted typed
sender at [connection.rs:179](../../crates/buzz-ws-client/src/connection.rs:179).
[production-continuity.txt](/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/344baa091ad486653ee99aa8b1de64d4839b4ada/production-continuity.txt) and
[host-dependency-delta.patch](/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/344baa091ad486653ee99aa8b1de64d4839b4ada/host-dependency-delta.patch) preserve measurements.

Read/accepted: Windows test.24 source `929c2cd890fbc2c0993549c3cbc64a2714232832`,111 native
cases plus ACL accepted at0851; installer SHA256
`b5c800b489cc7f85f654ec8a768ed96a7ceec408e79c11754fbfdc34f75a050e`.
Those suites were not rerun. Human test.24 observation remains pending.

Measured build: `HERMIT_SELF_UPDATE=false`; `. ./bin/activate-hermit`; then
`cargo build --offline --locked --release -p buzz-hostd --bin buzz-hostd --bin buzz-host-runtime -p buzz-acp --bin buzz-acp`.
Exit0, optimized release,19.93s; [build.log](/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/344baa091ad486653ee99aa8b1de64d4839b4ada/build.log).
Toolchain: rustc 1.98.1 (48a229cea 2026-09-01), cargo 1.98.1 (797e8a9bc 2026-08-05); target x86_64-unknown-linux-gnu.
Cargo.lock SHA256 `060b312fd66fd7cea5c1d8415143dad0bd710691e8fe024c6519fb09fb50d14f`.
Outputs remain on services disk (66GiB free at start); no tool install/update.
Matched binaries were copied together, hash compared to target/release outputs.

| Package file | Bytes | SHA256 |
|---|---:|---|
| OPERATOR.md | 30,043 | `34653012322ebb0ba0c998d7b8fca065aae04d5741e9310d6c6c66e752aaff69` |
| SOURCE.json | 6,789 | `98cc5bc669c6d4b1155d07bbcded82c340dae2a987c939118c45a988d52f8f9b` |
| buzz-acp | 18,545,880 | `b7fe18f1604db51dfeffc8d392676d8755f872662be710b87e84c11fc1e83f33` |
| buzz-host-registration.service | 952 | `0c5fb6ec7122a38d19d8ca30a0f9d89fdb22e9164072372f872a3e9664702ae0` |
| buzz-host-runtime | 3,818,728 | `98404e872a266d94457083bde657e179f3ab19fcd563771ac4e41f994c7bc16a` |
| buzz-hostd | 18,203,680 | `3a31f56b453ec5c6c8d5e1140a5921bd2bfd392b8207c6071f0cd86b3b6f054d` |
| buzz-hosted-agent@.service | 431 | `6c7759314a7da08cd018f29714edffa1b5f078e4d191f1d5d53993094e686394` |
| deployment.json | 917 | `f019bb6dc7b256cfb931f950fa45d53e95a677f6d138e7b3b7321f90c64e376b` |

[MANIFEST.sha256](/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/344baa091ad486653ee99aa8b1de64d4839b4ada/package/MANIFEST.sha256) SHA256:
`ec1fb606e835ba238a39021a3ad17d90e1f8c3ea933b568f2fa5db78e5671441`.
[SOURCE.json](/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/344baa091ad486653ee99aa8b1de64d4839b4ada/package/SOURCE.json) records full provenance/checks/limits.
The accepted old c972 package is preserved: its manifest remains
`f8a53553ca83eb4fe56481d9a257d9226840d1908ccbb888c9d4e8829cb94898`,
all8 entries verified. This new full-head directory has no moving/latest alias;
freeze its manifest for review. No installed binary/config was replaced.

## Exact bounded delta from accepted packet

- deployment.json and controller unit differ solely by new release path; capacity1,
  `runtimes:{}`, matched ACP/runtime, normal bryce HOME/tools/projects/native login,
  user-bus context and workspace remain. Hosted template is byte-identical to
  [source](../../crates/buzz-hostd/deploy/buzz-hosted-agent@.service) and mode0600.
- All9 operator shell fences are byte-identical to accepted M-1 after substituting
  release pin and unique backup name `3734-trusted-344baa0`. Both Python bodies
  are unchanged. This preserves original/moved/masked unit recovery, stop-before-
  custody, partial install/chown handling, CURRENT private state snapshot and
 988:988 restoration. No stale archive extraction, second controller or re-pair.
- Narrative updates carry test.24,111/ACL source acceptance, existing confirmed
  Delete/Check/Retry, fresh inventory/model selection and explicit pending Cleanup.
  Emergency stop remains containment. Previous Grok path/user-manager observations
  are historical inputs; they must be rechecked after approval.
- Relay need is settled: code+additive82+literal `BUZZ_HOST_DEPLOYMENT_V1=true`.
  Live f316/schema81/no deployment tables is supplied WSL evidence, not a fresh
  local measurement. Missing exact f316-on82 proof is a held gate; relay commands
  stay in WSL. This packet does not falsely supply a relay artifact.

No spec deviation or product defect was found. The only environment exception was
read-only syntax verification outside the sandbox after SO_PASSCRED EPERM; no
service mutation or broader permission was used.

## Finite verification actually run

[preparation-checks.json](/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/344baa091ad486653ee99aa8b1de64d4839b4ada/preparation-checks.json) and
[syntax-verification.json](/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/344baa091ad486653ee99aa8b1de64d4839b4ada/syntax-verification.json) retain results.

| Command/check | Measured outcome |
|---|---|
| Offline locked release build above | PASS exit0,19.93s |
| Staged `buzz-hostd --help`; `buzz-hostd registration run --help`; `buzz-acp models --help` | PASS each exit0; no provider/state/relay call |
| Staged `buzz-host-runtime --help` | PASS expected exit1 sanitized refusal; no help mode and no runtime launched |
| JSON parse/structure, all configured package binary paths, template bytes/mode | PASS; structural check, not production Catalog::read against live paths |
| All9 OPERATOR shell fences `bash -n`,2 embedded Python bodies `ast.parse` | PASS; parse only, zero operator/state-validation commands executed |
| Normalized command-block comparison to accepted M-1 | PASS9/9 byte-identical; no new control flow |
| `systemd-analyze --user verify` staged controller and synthetic64-hex hosted instance | PASS exit0 both, empty diagnostics via approved read-only retry; syntax only |
| Initial sandbox systemd attempts | Both exit1 `SO_PASSCRED failed: Operation not permitted`; retained [failed receipt](/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/344baa091ad486653ee99aa8b1de64d4839b4ada/syntax-verification-sandbox.json), not hidden |
| `sha256sum -c MANIFEST.sha256` new and old package; binary-vs-build hashes | PASS8/8 each; old package preserved |
| Exact staged/committed text, local links, `git diff --check` | PASS before commit; exact OPERATOR/config/unit/template text below |

**Not run:** native service install/start/credential delivery/notify/cgroups,
current controller/DB/public binding read, actual provider readiness/login/models/
subscription/chat, pilot create/confirmed Cleanup, live rollback/reboot, broad
source/Windows suites and full just ci. No PR-readiness attestation. Syntax is
not native activation proof; source/native fixture acceptance is not human/live
workflow evidence. No current-agent metadata was newly inspected; unchanged
operation is a scope guarantee, not a freshly measured PID comparison.

## WSL hold, approval boundary and failure thresholds

Conductor supplied read-only local `docker image inspect
sha256:1da22b2856a998de760a12b0767b30f0daf30e65b580ff193054fa17a4c85044`
result `No such image`; I did not repeat that daemon query. No duplicate relay
build or synthetic relay fixture was created here. The self-contained
[WSL preparation prompt](../briefs/3734-wsl-relay-preparation.md) owns candidate
image/archive and exact old-image/schema82 proof; conductor hands it to the user.
Required finite proof uses one synthetic database across schema81→82, exact f316
on82, candidate off/on/off, exact old rollback, candidate off restoration,
with signed ordinary chat/history/registration plus ledger/data preservation.
Missing/failing exact image proof holds live expansion; no rebuilt-old substitute.

After that receipt and independent review, explicit approval still precedes live
read-only checkpoint/load/migration/restarts/flags/custody/provider/pilot work.
Recheck source/image/policy, no legacy deployment history, public binding,
controller metadata and three old agents. One system→user transition preserves
real host/owner/generation and `/var/lib/buzz-host-registration`. Proposed order:
relay expand/candidate-off checkpoint, deployment-on checkpoint, netcup cutover,
human test.24 smoke, one explicitly approved Grok, controlled reply, confirmed
Delete/Cleanup, flag removal. Native login needs separate human direction.

Immediately stop for integrity/authority/identity/custody/protected-agent drift.
Persistent relay readiness/ordinary-chat failure90s triggers primary rollback;
missing human observation alone does not. Prefer SAME candidate deployment=false,
contain any admitted pilot as necessary; retain schema82/current DB and host/owner
journals. Only if needed use rehearsed exact f316, auto-migrate=false and its
community-deletion/deployment limits. Roll back host via observed original/moved/
masked branch after both controllers/pilot quiescent; restore CURRENT custody988:988,
never stale data. Uncertain quiescence/custody stays stopped. Manual containment
cannot certify Cleanup. Full command text is frozen below; none ran here.

## Exact deployment.json

```json
{
  "label": "netcup",
  "capacity": 1,
  "community": "https://buzz.interleaven.com",
  "relay_key": "9fd495a8df792128e1396c3766b12228076cad0ea7b791c3d5fbc446f74d62c1",
  "runtimes": {},
  "local": {
    "buzz_acp": "/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/344baa091ad486653ee99aa8b1de64d4839b4ada/package/buzz-acp",
    "home": "/home/bryce",
    "workspace": "/home/bryce/services/buzz-agents/workspaces/3721-grok-pilot",
    "tool_path": "/home/bryce/.local/bin:/home/bryce/.cargo/bin:/home/bryce/.bun/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
    "environment": {
      "XDG_RUNTIME_DIR": "/run/user/1002"
    },
    "unit_dir": "/home/bryce/.config/systemd/user",
    "launcher": "/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/344baa091ad486653ee99aa8b1de64d4839b4ada/package/buzz-host-runtime",
    "systemctl": "/usr/bin/systemctl"
  }
}
```

## Exact buzz-host-registration.service

```ini
[Unit]
Description=Buzz netcup trusted host controller
StartLimitIntervalSec=30min
StartLimitBurst=3

[Service]
Type=simple
ExecStart=/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/344baa091ad486653ee99aa8b1de64d4839b4ada/package/buzz-hostd --state-dir /var/lib/buzz-host-registration registration run --deployment --community https://buzz.interleaven.com --relay-key 9fd495a8df792128e1396c3766b12228076cad0ea7b791c3d5fbc446f74d62c1 --label netcup
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

## Exact buzz-hosted-agent@.service

```ini
[Unit]
Description=Buzz hosted agent %i

[Service]
Type=notify
NotifyAccess=main
TimeoutStartSec=75s
TimeoutStopSec=5s
KillMode=control-group
SendSIGKILL=yes
Restart=no
UMask=0077
StandardOutput=null
StandardError=null
LimitCORE=0
MemoryMax=1G
TasksMax=128
CPUQuota=100%
# The locally generated instance drop-in supplies the fixed runtime entrypoint
# and credential files. HOME, tools and workspace come from local configuration.
```

## Exact OPERATOR.md

<!-- BEGIN EXACT OPERATOR -->
# #3734 / #3721 netcup trusted pilot — prepared, NOT approved to execute

This is a bounded operator runbook, not an installer. All commands below target
live systems and require explicit approval AFTER independent review. Current
activation holds: the WSL relay artifact and exact deployed f316-image/schema82
compatibility receipt are pending, followed by independent packet review and
explicit live approval. Confirmed Delete/Check/Retry is now accepted in test.24;
actual provider/service/pilot Cleanup still requires post-approval observation.
No command below was run in preparation.

## Reviewed contents and purpose

Package build source 344baa091ad486653ee99aa8b1de64d4839b4ada is production/build-equivalent
to Windows test.24 source929c2cd890fbc2c0993549c3cbc64a2714232832 (six Markdown
files differ). hostd, buzz-host-runtime and buzz-acp are matched Linux release
builds including the accepted typed event sender. test.24 installer SHA256:
b5c800b489cc7f85f654ec8a768ed96a7ceec408e79c11754fbfdc34f75a050e.
111 native cases plus ACL are accepted evidence, not a live pilot or human smoke.
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
proposed for the pilot, subject to explicit approval. Normal PATH entries are
explicit, not inherited from Hermit. The prior accepted receipt resolved /home/bryce/.local/bin/grok to the
installed1.0.70 executable; revalidate the path after approval. Its current native
subscription/auth status is NOT measured.
An empty ordinary workspace is assigned at
/home/bryce/services/buzz-agents/workspaces/3721-grok-pilot; it is not a sandbox.
No XDG/provider store override is used except normal XDG_RUNTIME_DIR=/run/user/1002.

## Prerequisites and exact authorization boundary

1. Independent review accepts these files and manifest. Freeze them afterward.
2. WSL relay operator completes the reviewed relay packet: current serving source
   f316fb46d89c3e16eedc6173ae06e85fe224a1a9 / exact image
   sha256:1da22b2856a998de760a12b0767b30f0daf30e65b580ff193054fa17a4c85044;
   successful schema1–81,82 and deployment tables absent per supplied receipt.
   This requires candidate929c code + additive82 + BUZZ_HOST_DEPLOYMENT_V1=true,
   not a flag-only change. Exact old image on82 and same-candidate flag-off/on/
   rollback proof must pass in WSL before live approval. Keep schema82/current
   data on rollback, auto-migrate=false; old-image community deletion fails closed.
   After approved relay deployment, verify ordinary signed chat/registration,
   registry v2 and host_deployment={versions:[1],kind:43104,
   admission:"host-scoped",reads:"req-v1"}, with pinned public key matching
   deployment.json. Advertisement alone is not host/schema readiness.
   Relay load/migration/configuration/cutover commands belong to the separately
   reviewed WSL packet. Do not run netcup cutover before its relay checkpoint.
3. Use test.24 with accepted confirmed Delete/Cleanup and retained Check/Retry.
   Human test.24 registration/ordinary-chat observation remains pending. Delete
   retains identity and durable intent until current authenticated Cleaned and
   local completion; Close only dismisses. No package CLI supplies cleanup.
   At the approved live read-only checkpoint, recheck fresh-pilot scope:
   schema81/no deployment tables and registration-only host are expected. Unexpected existing deployment history
   stops rollout for compatibility assessment; never clear journals or relax
   exact-envelope comparison to bypass an older sender's bytes.
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
pkg=/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/344baa091ad486653ee99aa8b1de64d4839b4ada/package
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
pkg=/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/344baa091ad486653ee99aa8b1de64d4839b4ada/package
backup=/var/backups/buzz-host-registration/3734-trusted-344baa0
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
pkg=/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/344baa091ad486653ee99aa8b1de64d4839b4ada/package
"$pkg/buzz-hostd" --state-dir /var/lib/buzz-host-registration registration status
# Require same public host/community/generation; verify owner via readonly public-column query.
systemd-analyze --user verify ~/.config/systemd/user/buzz-host-registration.service
systemctl --user daemon-reload
systemctl --user enable --now buzz-host-registration.service
systemctl --user show buzz-host-registration.service -p MainPID -p ActiveState -p SubState -p NRestarts
```

Require active/running, expected executable hash and UID1002, stable registration
in the installed test.24 Windows client, no old system controller process, unchanged
existing-agent baseline. MainPID/active alone is NOT relay or provider acceptance.
New private hosted/ directory and additive deployment journal tables are expected.
No registration refresh through an independent controller. If startup fails,
restore as below; do not loosen permissions or invoke the legacy root helper.

## ONE disposable Grok — Windows owner, only after native host checks

Fully quit Buzz, then launch the installed test.24 executable with process-local
`BUZZ_NATIVE_HOST_DEPLOYMENT_V1=1` (PowerShell `$env:BUZZ_NATIVE_HOST_DEPLOYMENT_V1='1'`;
Start-Process the actual installed executable). Do not set a global persistent flag.
Keep the existing profile/identity. Add Agent → Advanced → Run on → netcup → Grok,
using ordinary name, channel and instruction-policy inputs. Use a clearly disposable
name such as Grok pilot3721; retain the new agent pubkey, operation, exact opaque
unit ID and placement revision from sanitized current operation evidence. Do not
infer unit names from the display name or touch buzz-agent@grok.service.

Observe fresh same-host main inventory/readiness, prepare, Refresh models, choose
a returned opaque model ID, then Launch. Expired/offline/revoked/unsupported or
NeedsLogin remains truthful and blocks the required action; retain the same setup. Grok NotApplicable is not proof of authentication;
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

## Confirmed Delete/Cleanup — same disposable agent

After the controlled reply, use the ordinary confirmed Delete action for this
hosted agent. Retain its identity, placement and Cleanup intent while pending;
Check/Retry reconciles/retries that SAME operation. Require authenticated Cleaned
and local completion before calling teardown successful. Observe the exact pilot
unit stopped/disabled/removed and no identity resurrection; retain journals and
sanitized outcome. Close is not cleanup. If cleanup is pending/failed/unknown,
report that status and contain the exact pilot below; do not remove its identity
or manufacture success. Recheck existing-agent and host-registration continuity.
Fully quit Windows, remove the process-local flag and restart normally when done.

## Failure thresholds and relay coordination

Any integrity, authority, identity/custody or protected-agent discrepancy stops
immediately. Persistent relay readiness/ordinary-chat failure90s triggers the
WSL primary rollback; lack of human observation alone is not a server failure.
Prefer deployment=false on the SAME candidate, retaining schema82/current data;
it blocks new admissions but an already admitted bounded host attempt may finish.
Quiesce exact controller/pilot when containment requires it. If necessary, WSL
returns only primary to the rehearsed exact f316 image, auto-migrate=false, with
schema82 retained and documented deployment/community-deletion limits. Never
restore stale DB/owner/host journals. Missing WSL proof blocks live expansion.

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
pkg=/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/344baa091ad486653ee99aa8b1de64d4839b4ada/package
backup=/var/backups/buzz-host-registration/3734-trusted-344baa0
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
retained. Emergency stop is NOT confirmed teardown. Preserve the actual pending Cleanup
status and current host/owner journals; accepted Delete/Check/Retry does not turn
manual containment into success. #3767 is unrelated legacy cleanup. Existing
agents stay up.

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

Unknown state stays stopped for assessment. These branches do not close the
pending relay proof/review gates or authorize live action during preparation/review. Re-enablement requires this current-source
package, fresh capability and exact retained-history reconciliation.
<!-- END EXACT OPERATOR -->
