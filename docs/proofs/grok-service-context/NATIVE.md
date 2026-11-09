# Native offline proof: repaired command awaiting review and approval

**Repaired candidate native execution NOT RUN.** The previous approved attempt
failed at systemd EXEC before Python or any proof ran. No native isolation,
model-discovery or service-lifecycle pass is claimed. This candidate awaits the
same independent reviewer, then separate user approval before execution.

## Observed failure and repair

The conductor's exact-unit read-only journal inspection reported
`Unable to locate executable /usr/bin/env: Permission denied`, status `203/EXEC`,
for `buzz-service-proof-f9f6cc3173de5c84.service`. Parent directories/executable had
expected modes; native findmnt showed `/run` mounted `noexec`. `/var/lib` is on the
existing executable ext4 filesystem. This was a staging-path error, not a Grok
or provider failure. The original namespace test used `/tmp` and did not establish
native `/run` execution. The conductor confirmed that old unit/state/cgroup are
gone; old `/run/buzz-service-proof-f9f6cc3173de5c84` staging remains intentionally
untouched. This command neither cleans nor reuses it.

The new candidate stages at `/var/lib/buzz-service-proof-stage-<16hex>`, separate
from `/var/lib/buzz-service-proof-<16hex>` DynamicUser state. Both bootstrap and
native preflight reject `ST_NOEXEC`; host mount flags are never changed. Failed
commands retain escaped, bounded stdout/stderr, and failure handling saves only
the exact synthetic unit's bounded journal to private `failure.json` before cleanup.

## Exact prepared inputs

- Archive: `/home/bryce/services/buzz-agents/releases/3719-3722-native-proof/native-offline-proof.tar`
- SHA-256: `b5f965927381d36d967469bd27e488bd1c0e4e4bce76d2ff4ea8c5524981c23d`
- Archive bytes: 236933120; unpacked file bytes: 236281343.
- Manifest entries: 336; all hashes and packaged/source equality checked.
- Bootstrap: `/home/bryce/services/buzz-agents/releases/3719-3722-native-proof/bootstrap-native.py`
- Bootstrap SHA-256: `09746fb9420bc612d2e471d3b445e57443d8b9938114213b5379540e718c4437`.
- Source repair starts at `055006a30901ef0e4d9fea6404856ae04c996e2a`; final commit
  is recorded in the operations receipt. Release directory is mode0700,
  archive/bootstrap mode0400; root trust comes from verified bytes, not owner
  inability to chmod a file.

The old archive `d57e80a2ae3fef6dcc81c350daff05080b57676855cb50787911008a37f0fc64` and bootstrap `66deaeb67d2897cb6107b5eae83644afb8845c7472219d3b9df1d4eab8172707`
are superseded. The prior command rejects the replacement bootstrap before it
executes; the prior archive hash also rejects the replacement archive. A source
change does not update this bundle without repackaging and re-review.

## Exact command for subsequent approval

```sh
sudo -- /usr/bin/python3 -I -S -c 'import hashlib,sys; data=open(sys.argv[1],"rb").read(); assert hashlib.sha256(data).hexdigest()=="09746fb9420bc612d2e471d3b445e57443d8b9938114213b5379540e718c4437", "bootstrap hash mismatch"; exec(compile(data, "<verified-native-bootstrap>", "exec"))' \
  /home/bryce/services/buzz-agents/releases/3719-3722-native-proof/bootstrap-native.py \
  /home/bryce/services/buzz-agents/releases/3719-3722-native-proof/native-offline-proof.tar \
  b5f965927381d36d967469bd27e488bd1c0e4e4bce76d2ff4ea8c5524981c23d
```

The literal command hashes the supplied bootstrap bytes before executing them.
That verified bootstrap reads the archive once, verifies its approved hash,
rejects links/special files and uses standard tar data filtering to extract into
an exclusively created root-owned stage directory. Root imports/executes only
those staged files. No root ldd, runtime read from operator HOME or mutable-source
import is performed. Grok is bundled but never launched; provider modes remain
rejected. ACP is still the exact older canary artifact with the source-continuity
limit recorded in README.md, not a current-source deployment build.

## Resources and tests

One generated suffix owns only:

- `/var/lib/buzz-service-proof-stage-<suffix>`: immutable rootfs/scripts,
  four synthetic canary files, one filesystem socket, and a failure receipt if
  needed. One abstract Unix socket and two loopback listeners use unique/kernel
  assigned addresses.
- `buzz-service-proof-<suffix>.service`: one transient unit via systemd-run.
  No installed unit, daemon-reload, enablement or existing agent action.
- `/var/lib/buzz-service-proof-<suffix>` and its standard
  `/var/lib/private/buzz-service-proof-<suffix>` mapping: private writable state.
  Systemd's state ownership handling cannot recursively chown the distinct stage.

Preflight requires root, PID1 systemd, executable staging filesystem, root-owned
non-writable staged inputs, correct manifest and no unit/state collisions.
Bootstrap requires 507420672 free bytes under `/var/lib`
(archive size times two plus 32 MiB); the native state preflight requires 32 MiB.
The archive is below its 256 MiB bound; allow about 0.5 GiB transient bootstrap memory
plus service processes. Unit limits remain 1 GiB/128 tasks/100% CPU, 20-second start
and five-second stop deadlines. `PrivateNetwork=yes` remains unconditional.

First start exercises the same filesystem/socket/network probes, synthetic
state write/read, real generic ACP discovery against the unchanged test adapter
and fixed failure handling. Restart repeats read/models and records per-start
UID/file ownership. Numeric DynamicUser UID may change; private own-state access
and persistence are required. Namespace tests continue to require equal numeric
UID. A fixed flag then forces unit exit 70 and its failure status is checked.
No native Grok, provider request, authentication, real model or relay traffic runs.

## Diagnostics and cleanup

On unexpected failure, the command error contains exit code and at most 4 KiB each
of captured stdout/stderr. Before cleanup, only this unit's last 40 journal lines
are requested with a ten-second/64KiB capture bound; the receipt retains at most
16 KiB journal and 4 KiB diagnostic stderr. JSON escapes terminal controls.
`failure.json` is mode0600 in the owned staging directory; original errors remain
available even when journal capture itself fails. No broad journal query occurs.

Cleanup checks the unique unit Description, stops only that unit, requires its
expected cgroup to be absent/unpopulated, resets only its failed status, and
removes only the exact owned StateDirectory/mapping. Success removes the new
staging tree. Unknown ownership, cgroup state, forced kill or machine loss can
leave resources: failure preserves and prints the exact stage path rather than
deleting unknown/live paths. Inspect only the printed suffix for recovery;
no wildcard cleanup and no prior-run cleanup is authorized by this candidate.

## Local verification and next gate

- Focused no-manager tests: distinct staging/state and executable/noexec guards;
  failed-command diagnostics; bounded exact-unit journal receipt and capture
  failure; existing umask077 descriptor regression: PASS.
- AST, dry-run plan, all archive manifest hashes and packaged-source equality:
  PASS. Old bootstrap/archive hashes reject replacement bytes.
- Updated uninstalled unit `systemd-analyze verify`: PASS.
- Shared builder/context are unchanged; accepted namespace regression retained,
  not repeated.
- Repaired root bootstrap/native unit: **NOT RUN**. The previous actual attempt
  failed 203/EXEC before proof execution; no native pass is inferred.

Reviewer attention: stage versus StateDirectory naming, both noexec preflights,
exact RootDirectory argument, bounded diagnostics before owned cleanup, and the
new literal command pins. No application, online policy, provider or broader
lifecycle changes are included. Same independent reviewer checks this delta,
then the conductor may request approval for this exact revised command.
