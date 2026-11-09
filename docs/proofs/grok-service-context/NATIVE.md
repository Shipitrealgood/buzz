# Native offline proof: proposed command awaiting approval

**Actual native execution NOT RUN.** This is the concrete candidate for the
independent re-gate and subsequent user approval on the netcup terminal. The
current Codex environment has PID 1 `codex`; it cannot run this native gate.
No provider sign-in, native Grok, real relay or model request is authorized.

## Exact prepared inputs

- Archive: `/home/bryce/services/buzz-agents/releases/3719-3722-native-proof/native-offline-proof.tar`
- SHA-256: `d57e80a2ae3fef6dcc81c350daff05080b57676855cb50787911008a37f0fc64`
- Archive bytes: 236933120; unpacked file bytes: 236279806.
- Manifest entries: 336. All hashes checked after packaging.
- Bootstrap: `/home/bryce/services/buzz-agents/releases/3719-3722-native-proof/bootstrap-native.py`
- Bootstrap SHA-256: `66deaeb67d2897cb6107b5eae83644afb8845c7472219d3b9df1d4eab8172707`.
- M-1 repair source starts at `947378553964dfdb0f3f6a44e813defea9b7de14`; candidate
  commit is recorded in the operations receipt. Earlier offline acceptance was
  at `3e9f6dac0cc712d1871518dff6cd19c2247ab8b5`.

The release directory is mode 0700; archive/bootstrap are mode 0400. This is
checksum pinning, not filesystem immutability against the owning user. The root
command verifies the exact bytes before executing them. Changing source later
does not change this bundle; a changed candidate requires a new hash/review.

## Command for separate approval

Run only after the conductor's independent re-gate and explicit user approval:

```sh
sudo -- /usr/bin/python3 -I -S -c 'import hashlib,sys; data=open(sys.argv[1],"rb").read(); assert hashlib.sha256(data).hexdigest()=="66deaeb67d2897cb6107b5eae83644afb8845c7472219d3b9df1d4eab8172707", "bootstrap hash mismatch"; exec(compile(data, "<verified-native-bootstrap>", "exec"))' \
  /home/bryce/services/buzz-agents/releases/3719-3722-native-proof/bootstrap-native.py \
  /home/bryce/services/buzz-agents/releases/3719-3722-native-proof/native-offline-proof.tar \
  d57e80a2ae3fef6dcc81c350daff05080b57676855cb50787911008a37f0fc64
```

The literal root bootstrap reads and verifies its own supplied bytes before
execution, then reads the archive into memory and verifies the pinned hash before
extracting. It uses standard tar data filtering, rejects links/special files,
and extracts only into an exclusively created root-owned directory under `/run`.
Only those verified staged Python files are imported/executed as root. No root
`ldd`, runtime reads from an operator HOME, mutable-workspace imports, caller
command/descriptor or environment overrides are used. Grok is staged but never
launched. Runtime pins and the existing model-discovery interface remain those
of the offline receipt; this is still the older ACP artifact, not a current-source
build.

## Exact effects and sequence

The suffix is 16 random hex digits generated after verification. One invocation
owns only:

- `/run/buzz-service-proof-<suffix>`: verified sources/rootfs, four synthetic
  canary files, one filesystem socket; plus one abstract socket and IPv4/IPv6
  loopback listeners on kernel-assigned ports.
- `buzz-service-proof-<suffix>.service`: a transient systemd unit created with
  `systemd-run`. No installed unit, daemon-reload, enablement or existing agent
  service is involved.
- `/var/lib/buzz-service-proof-<suffix>` and systemd's corresponding
  `/var/lib/private/buzz-service-proof-<suffix>` mapping: one private writable
  StateDirectory. DynamicUser state is separate from the immutable rootfs;
  systemd cannot recursively chown the staged runtime.

Preflight requires root, PID 1 systemd, root-owned/non-writable staged paths,
valid staged manifest, no existing unit/state collision, free `/run` space of
archive size times two plus 32 MiB (507420672 bytes),
and 32 MiB free under `/var/lib`. The archive is under the 256 MiB input bound.
The bootstrap also holds the archive in memory; plan roughly 0.5 GiB transient
memory plus runtime processes. Native process limits remain 1 GiB, 128 tasks,
100% CPU, 20-second startup and five-second stop deadlines per unit.

The first start runs the same boundary probes, writes synthetic persistent
state, invokes real ACP against the unchanged synthetic adapter, checks fixed
failure handling, and re-reads state. `systemctl restart` repeats read/models;
per-start UID and file ownership are recorded. DynamicUser may change its numeric
UID: the invariant is the same logical enrolled service/home, private state
ownership/access, and preserved bytes. The namespace test still requires identical
numeric UID; it is not silently weakened. The native service then deliberately
exits 70 on a fixed synthetic flag, and the controller checks the unit failure.
All provider modes stay rejected and `PrivateNetwork=yes` stays enforced.

## Cleanup and limits

Normal/failure unwinding closes only owned listeners, verifies the unit's unique
Description before stopping it, checks its expected cgroup is absent or unpopulated,
resets only that unit's failed state, and removes only its known StateDirectory
and exact private-directory mapping. A successful bootstrap removes its own
staging. SIGINT/SIGTERM permit the controller cleanup to run. Forced process kill,
machine loss, or uncertain ownership can leave resources; the bootstrap prints
the exact preserved staging path and fails rather than deleting unknown paths.
A failed run is not a passing gate. Use that printed suffix and inspect the
matching unit/state before any manual cleanup; do not run a wildcard cleanup.

Sanitized JSON output contains generated unit/resource names, UIDs, synthetic
model payload and cleanup status. Provider credentials, invitation codes and
private endpoints never enter this fixture. A native pass would establish this
specific offline candidate only, not selective online egress, canonical service
credentials/readiness, authenticated Grok or production launch safety.

## Verification performed here

- Shared namespace regression and both falsification controls: PASS after refactor.
- Default dry-run plan and required systemd property assertions: PASS.
- Wrong confirmation, conflicting actions and unprivileged native execution:
  rejected before manager access.
- Python AST syntax; archive SHA, all manifest hashes and staged/source equality:
  PASS.
- `systemd-analyze verify` on the uninstalled candidate: PASS.
- Native manager/unit/root bootstrap execution: **NOT RUN**; pending approval.

For review, scrutinize `native-proof.py` staging ownership, unit collision checks,
per-start state mapping, and cleanup's exact path bounds; `bootstrap-native.py`
verified-byte handoff; `context.py`'s separate native ownership policy; and the
shared builder's unchanged namespace regression. No online-policy implementation
or application changes are included. The next step is independent review of
this delta, then approval of the command above.

## M-1 repair and superseded candidate

The archive `29f3007ab42a8cc19e610a8c071967a00dbe2e3f5c9a2314f1bda502145612e2` and its prior approval command are superseded.
The unchanged bootstrap's archive-hash predicate rejects that old hash against
the replacement bytes before native execution. The command above pins only the
new candidate; native execution remains **NOT RUN**.

The actual native descriptor writer now chmods the trusted generated
`/proof/probes.json` to 0444 before startup. The no-root regression invokes that
same helper under umask077, verifies JSON content and exact read-only mode, and
shows that suppressing chmod makes its permission assertion fail. Targeted test,
AST, unchanged dry-run plan, archive/source comparisons and all manifest hashes
pass. Shared namespace/context code is unchanged by this repair, so its accepted
regression was not repeated.
