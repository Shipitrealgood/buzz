# #3721 netcup live activation

2026-11-10. **Native controller activation complete; Windows and provider pilot
acceptance remain pending.** The same state now runs under bryce; old system
controller is masked. No disposable agent was created.

Executor `/root/netcup_activation`, codecodex. Requested configuration:
`gpt-6-astra`, high reasoning; runtime identity is not independently exposed.
Applied execute and brief, with handoff content kept in this assigned receipt.

## Scope and pins

Workspace `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`. Starting HEAD
`8e291a45cf12cb6a586a334f199966ee095e31bd`; starting worktree clean.
Conductor subsequently committed the WSL handoff at `a27ec4fba`; this is an
expected documentation change. Executor owns only this receipt and private
activation evidence at
`/home/bryce/services/buzz-agents/releases/3721-netcup-live-activation-20261110/`.

Accepted [specification](../specs/3734-hetzner-netcup-rollout-preparation.md),
[preparation receipt](3734-hetzner-netcup-rollout-preparation.md) and
[independent review](../audits/post-audit/3734-netcup-rollout-preparation.md)
remain the implementation contract. Frozen package:
`/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/344baa091ad486653ee99aa8b1de64d4839b4ada/package/`.
Its OPERATOR.md supplies the exact cutover and prefix-aware rollback commands;
the entire rollback was read before any live change.

User approval, relayed by conductor, supersedes historical no-live-approval,
schema81 and Grok-only holds: relay is upgraded to schema82, real Windows
registration and disposable pair/revoke passed, deployment flag still off at
dispatch. Claude is preferred for ONE disposable agent, Goose an acceptable
fallback. This worker stops at native host readiness; Windows flow and provider
calls remain separately gated. No native login/logout, copied credentials,
API fallback, existing-agent changes, product edits, builds or new test framework.
The historical `3721-grok-pilot` workspace name is retained; neutral `runtimes:{}`
uses ordinary main discovery and does not restrict provider selection.

## Measured preflight

`sha256sum package/MANIFEST.sha256` equals
`ec1fb606e835ba238a39021a3ad17d90e1f8c3ea933b568f2fa5db78e5671441`;
`sha256sum -c MANIFEST.sha256`: all eight entries OK. Package unchanged.

Narrowly approved native read-only exec bypassed sandbox restrictions:
`systemctl --version` reports systemd257; system/user manager reads succeed;
`loginctl show-user bryce -p Linger` reports yes; `sudo -n id` reports root.
This proves the required access, not activation behavior. Normal account is
bryce1002:1003. Initial Hermit activation attempted its metadata self-update
and failed on the sandbox read-only cache; subsequent Git commands use
`HERMIT_SELF_UPDATE=false`.

System controller: enabled, active/running, PID43436, start
2026-11-09 16:19:35 UTC, NRestarts0, User/Group buzz-host-registration,
no drop-ins. User controller: not-found/inactive/MainPID0. Installed hashes:

- `/etc/systemd/system/buzz-host-registration.service`:
  `9cb7a6d7b9df3e18b39d46916d1d7b32140e0567e123305dfe8685a5e8a03bce`.
- `/usr/local/libexec/buzz-hostd`:
  `de05655071c5f6af3451cc70116969f53da0df0ff64448593a0fc913bf10d3ee`.

State metadata validated with root `python3 -I`, without opening database or key
contents: canonical `/var/lib/buzz-host-registration`,988:988/0700; only
host.sqlite and reconciler.lock, each988:988/0600, regular and single-linked.
User unit directory canonical. lstat established absence of both new user units,
all hosted instance drop-in directories, deployment.json, hosted directory,
pilot workspace and `/var/backups/buzz-host-registration/3734-trusted-344baa0`.
Permission errors were not treated as absence. Public binding was subsequently
measured during stopped-state cutover, below.

Protected baseline saved privately as `protected-agents-before.txt`,
`controller-before.txt` and `protected-unit-hashes-before.txt`:

| Unit | MainPID | Start UTC | NRestarts | State |
|---|---:|---|---:|---|
| buzz-agent@claude.service | 641476 | 2026-11-03 20:27:07 | 0 | active/running |
| buzz-agent@grok.service | 637752 | 2026-11-03 20:26:09 | 0 | active/running |
| buzz-agent@scout.service | 619136 | 2026-11-03 20:18:08 | 0 | active/running |

Shared protected template hash:
`bff20d3ced60c1d0880009142c12761f1eeae4fd85f7505ae9bacff6dd93c559`.
Each existing `90-buzz-main-candidate.conf` drop-in hash:
`4a551894bb51631ee88ee0effc0f5a19b01a61912e02b3c5b33b96b9cca6f230`.
No configuration contents, credential values, environment or service logs were
printed. Existing agents have not been restarted, stopped, adopted or prompted.

Executable lookup uses the package's explicit normal account PATH, without
executing a harness: Claude resolves to
`/home/bryce/.local/share/claude/versions/2.1.322`; Goose exists at
`/home/bryce/.local/bin/goose`; Grok resolves to
`/home/bryce/.grok/downloads/grok-1.0.70-linux-x86_64`.
`claude-agent-acp`, `claude-code-acp` and `claude-code` are absent from that PATH.
File availability is not provider/auth/model readiness; native main discovery
remains authoritative for Windows choices. No adapter installation was attempted.

## Relay checkpoint — attributed and locally measured

Conductor relayed the user-supplied WSL flag-on acceptance: candidate source
`929c2cd890fbc2c0993549c3cbc64a2714232832`, immutable image
`sha256:5bc530077c757a8b058c08ef6db9e17eba8f0551118d88d17388db972fceaa0f`,
schema82 and checksums unchanged; only deployment false→true. Registry=true,
auto-migrate=false, paired-hosts-v1=false, auth+membership=true, NIP-FI off.
Six nonprimary containers unchanged; health/auth checks passed; independent
Astra/high review returned zero findings. These are supplied WSL outcomes,
not independently rerun from netcup. WSL-local receipt
`/tmp/buzz-hosted-prep-929c-w39MsZNb/flag-enable-yfwl6ngy/final-flag-enable-receipt.txt`,
SHA256 `74ebe0d4b055f385e6b79b8af9da1be3e9f77e2979985762a8ae8c0f64640906`.
WSL recovery: `/root/buzz-hosted-929c2cd8-20261110/rehearsal-checkpoint`;
prefer same candidate deployment-off, preserving schema82/current data.

Measured immediately before netcup mutation using timeout-bounded `curl` with
`Accept: application/nostr+json`: public key
`9fd495a8df792128e1396c3766b12228076cad0ea7b791c3d5fbc446f74d62c1`,
registry `{versions:[2],kind:43103,admission:"host-scoped",reads:"req-v2"}` and
deployment `{versions:[1],kind:43104,admission:"host-scoped",reads:"req-v1"}`.
Saved `nip11-before.json`. Source diff since dispatch contains only conductor's
WSL handoff; package manifest and all eight entries pass again. Protected service
metadata and unit hashes match preflight exactly.

## Executed live sequence and results

All live commands used the reviewed OPERATOR sequence with fail-closed assertions
for its prose checkpoints. Root actions used narrowly approved native `sudo -n
/bin/bash`, `set -eu`, `umask 077`; user commands used bryce and its normal bus.
No persistent helper, alternate controller, state path or new framework was made.
All tool commands completed; only the intended managed user service remains.

1. Rechecked exact target absence, canonical unit directory, old unit/binary hashes,
   no drop-ins, PID43436 and relay capability. Created root-private
   `/var/backups/buzz-host-registration/3734-trusted-344baa0` using
   `install -d -m 0700 -o root -g root`.
2. Ran `systemctl disable --now buzz-host-registration.service`. Measured
   inactive/dead/MainPID0, old PID43436 gone and old cgroup empty/absent.
   Revalidated stopped state: only allowed regular single-linked files,988:988,
   directories0700/files0600. Saved metadata in the private backup.
3. Queried only public `identity.pubkey` and `grants.g,c,state,owner` using
   SQLite `mode=ro`. Host/active owner/generation matched exactly:

   - Host `66f5d7c51f4dbccc9746204f5860e57c98e30a56b777d05c6f4f209cd7f38d7a`.
   - Owner `f5c79eab2f44284ff278780c34aab621aafb149be4d7dba8f9625bfd99419733`.
   - Generation `6eaf423e2e2fffa23e142135bfb3583ba22dcbadf0daedc57c747fe54ec28b40`.
   - Community `https://buzz.interleaven.com`, state active.

4. Created `state-before.tar` via
   `tar --acls --xattrs --numeric-owner -cpf`, chmod0600, wrote its SHA256 and
   verified archive readability privately. Preserved `/usr/local/libexec/buzz-hostd`
   and `/etc/buzz-host-registration/controller.conf` via `cp -a`. Moved the exact
   regular system unit to backup, symlinked `/dev/null` in its place, ran
   `systemctl daemon-reload`, and required literal masked. No installed old
   binary or configuration was overwritten. Saved `stage1-completed.txt` privately.
5. Rechecked protected-agent metadata and absence of a user-controller PID.
   Revalidated stopped custody, then used the reviewed
   `chown -R --no-dereference bryce:bryce /var/lib/buzz-host-registration`.
   Installed exact package deployment.json and two user units at0600, and the
   ordinary pilot workspace at0700, all1002:1003. Saved stage2 marker privately.
6. As bryce, ran package `buzz-hostd --state-dir /var/lib/buzz-host-registration
   registration status`: same host/community/generation active. Ran
   `systemd-analyze --user verify` on installed controller: exit0, no diagnostics.
   Then `systemctl --user daemon-reload` and
   `systemctl --user enable --now buzz-host-registration.service`: exit0.
7. At22:13:04 UTC the new controller was active/running, PID2212700,
   NRestarts0. At22:14:04 UTC the same metadata still matched byte-for-byte:
   one minute stability, no restart. `/proc` confirmed UID1002, cwd `/home/bryce`,
   exact frozen executable and SHA256
   `3a31f56b453ec5c6c8d5e1140a5921bd2bfd392b8207c6071f0cd86b3b6f054d`.
   System controller remains masked/inactive/dead/PID0; old process absent.
8. Readonly public-column checks confirmed the same active binding after startup.
   State tree is1002:1003, directories0700/files0600, regular/single-linked;
   new hosted/ directory and deployment tables exist. Installed deployment and
   unit bytes equal the frozen package. Zero hosted instance drop-in directories.
   Public journal aggregate: one sent row, pending0; active inventory_seq0.
   These counts are local evidence, not fresh relay inventory or provider proof.
9. Protected Claude/Grok/Scout metadata compared byte-for-byte after each
   transition and at final stability; all original PIDs, starts, active/running
   states and zero restarts persist. Shared template/drop-in hashes match.
   Snapshot SHA256 recheck and old binary/config byte comparisons pass; saved
   original unit hash matches. No log dump, provider invocation, login or existing
   agent prompt was performed.

No cutover failure occurred. No rollback ran. Current-state snapshot and exact
old unit/binary/config remain root-private for prefix-aware recovery. Never
restore the snapshot over current journals. Reversal requires stopping the user
controller (and any subsequently recorded pilot), validating/snapshotting CURRENT
state, restoring988:988 custody, removing only matching owned files and restoring
the saved system unit using the reviewed masked branch. Windows creation flag
must be removed first. Manual containment does not establish confirmed Cleanup.

## Verification and handoff

Private evidence index: `protected-agents-before.txt`, `controller-before.txt`,
`protected-unit-hashes-before.txt`, `nip11-before.json`,
`protected-agents-pre-cutover.txt`, `protected-agents-stage1.txt`,
`registration-before-start.txt`, `controller-after-start.txt`,
`start-completed.txt`, `protected-agents-after.txt`, `native-after.json`,
`stability-time.txt`, `controller-stable.txt`, `protected-agents-final.txt`.
Root-private backup additionally contains original files, stopped archive/hash,
state metadata, public binding and stage markers. No raw secrets entered Git.

Native activation checks above passed. Documentation local-link and whitespace
checks ran before signed commit. Product/source/native fixture acceptance is
reused from the accepted packet; build, broad suites and just ci were not rerun
for this operational receipt. Executor verification is not an independent gate.

**Next gate:** conductor-owned independent receipt review, then user observes the
same netcup registration in installed Windows test.24 and proceeds through the
ordinary Add Agent flow with process-local deployment flag. Claude is preferred;
Goose fallback remains within the same ONE-disposable-agent scope. Current main
inventory/readiness and returned opaque models govern selection. Missing adapter,
NeedsLogin or failed discovery is a truthful blocker; no installation/login/API
fallback is authorized here. Retain the same prepared identity on retry.

**Not run:** Windows current-host observation, provider models/chat/authentication,
managed pilot credential delivery/notify/cgroup behavior, confirmed Delete/Cleanup,
rollback and reboot. No disposable agent was created. Do not infer end-to-end
readiness from active systemd state or public NIP-11. Any binding, custody, hash,
target or protected-agent discrepancy stops for assessment. No product/spec
redesign or frozen-package modification was required.
