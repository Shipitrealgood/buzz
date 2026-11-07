# #3751 — independent registration preinstall review

2026-11-07. **PREINSTALL BLOCK: one High finding; C=0 / H=1 / M=0 / L=0.**
The canonical sandbox correction and retained actual-service evidence are accepted
for this narrow gate. The exact package must not be installed until its collision
preflight is repaired and the replacement package independently re-gated.

Review round **0**, exclusive worktree
`/home/bryce/services/buzz-wt-3720-host-registration`, range
`f316fb46d89c3e16eedc6173ae06e85fe224a1a9..6d526b1a76d4156703c395cfa729d0595b31903f`.
Requested reviewer configuration: **gpt-6-astra/high**. Runtime model and reasoning
identity are **unknown**; requested configuration is not a runtime attestation.

Scope: the seven-line canonical service correction, hosted probe exception,
verification-only Rust example, deployment documentation, and exact private package.
The [execution brief](../../briefs/3751-netcup-registration-execution.md) and amended
private milestone define the seven finite obligations below. Registration protocol,
Windows source and relay deployment are accepted inputs, not reopened architecture.
No permanent install/account/state, invitation, signed event, agent/provider action,
Hetzner change, merge/push or board write was performed. Claude/Grok/Scout remain
outside the change. Deliverable is this sanitized signed-off report; private site
configuration and process details remain in the release directory.

## H-1 (High, high confidence) — collision preflight continues after failure — fix-in-lane

**Location:** private release `package/OPERATOR.md:31–42`, with destructive consumers
at `:58–65`. The release directory is
`/home/bryce/services/buzz-agents/releases/3751-netcup-registration`.

**Measured:** the path guard is `test ! -e "$p" && test ! -L "$p"` under `set -eu`.
For an existing regular file or directory, the first command fails in a non-final
position of an AND list; Bash's errexit does not stop execution. The following
`! getent` and `! pgrep` checks are also exempt from errexit. Their failure does not
enforce the documented refusal. An existing account may independently cause the
later useradd to fail, but that does not protect an existing path when the account
is absent.

Realistic consequence: an existing controller configuration, executable, or state
directory with no matching account passes the preflight. Installation creates the
account, changes directory ownership/modes, and can overwrite configuration/binary
files. That violates the explicit refusal to overwrite/adopt any existing identity
or controller and makes partial-failure cleanup ownership ambiguous.

Independent harmless reproduction used a temporary existing regular file and the
exact loop body; no install command ran and the file stayed unchanged:

```sh
set -eu
for p in "$1"; do
  test ! -e "$p" && test ! -L "$p"
done
printf '%s\n' INSTALL_REACHED
```

Called through `bash -c` with the existing file as `$1`: **exit 0,
`INSTALL_REACHED`**. Separate existing-account (`! getent passwd root`) and live
shell-process (`! pgrep -x bash`) controls also reached the marker with exit 0.
Temporary reproduction resources were removed. No permanent path was touched.

**Required repair:** make every collision check explicitly terminate before any
mutation, including existing/dangling-symlink paths, account/group/process matches,
and lookup failures that cannot establish absence. Verify a clean case proceeds and
collision/error cases stop before the first mutation using disposable resources or
bounded command stubs. Correct the class of guards, not just one path. Repackage
and update archive/manifest receipts; preserve the original failed package/evidence.

| Finding | Disposition | Owner and next action |
|---|---|---|
| H-1 | fix-in-lane | Conductor confirmed this ruling during review. Same executor repairs the finite operator procedure and republishes the immutable package; same reviewer checks this finding and the exact delta before installation. |

## Accepted evidence and continuity

The following distinguishes **measured** reviewer checks from **read** executor
measurements. Private evidence paths below are relative to the release directory.

| Obligation | Assessment and evidence |
|---|---|
| Effective root-relative executable policy | **Read/accepted.** Canonical [unit](../../../crates/buzz-hostd/deploy/buzz-host-registration.service):40–43 uses `+/`, explicit `+/state`, and root-relative executable/library exceptions. Installed systemd.exec(5) explicitly defines these `+` semantics. `evidence/rehearsal-attempt3.json` and `attempt4.json` show copied state executable execution and the resulting assertion failure; final `rehearsal.json` records two passing probes. This is a concrete before/after fault, not a text-only policy claim. No new mutation campaign was run. |
| Dedicated UID and filesystem/socket denial | **Read/accepted.** [Probe](../../../crates/buzz-hostd/examples/registration_deployment_probe.rs):13–60 requires nonroot, confirms outside world-readable canary/helper/Unix/loopback positive controls, then requires inside denial, state-write success, hosts-write denial and copied-state-executable denial. `evidence/rehearse.py:30–59` stages those same controls, and final receipt records nonroot UID, zero effective capabilities, NoNewPrivs and seccomp. Root-only production EnvironmentFile is consumed by systemd; probe fixture's public-value copy is less restrictive and adds no service access. |
| Remote transport and network scope | **Read/accepted.** Site unit/config use native CA roots, fixed hosts-only resolution and exactly one relay IP allowlist, with no DNS/loopback/Unix allowance. Probe `:62–70` uses the same default-root NostrWsConnection connector as `registration_cli.rs:109–115` and connects/closes without authentication. Final journal records success twice. **Measured:** current native CA/dependency hashes equal `evidence/platform.json`. Public TLS/key/capability preflight is supplied conductor/executor evidence, not a newly performed remote inspection. The guarantee is IP-only, not per-port; no port-443-only claim is accepted. |
| Exact artifacts, singleton, restart, resources | **Measured:** archive hash, all manifest entries, all eight archive files against package bytes, source/build-input hashes, and retained release binary against package all match the pins below. Exact deterministic fixture transformation of the site unit equals `evidence/rehearsal-effective.service`; only account/path substitution and probe bind/ExecStartPre are added. **Read/accepted:** `rehearse.py:60–84` checks actual executable readiness, no controller listener, singleton rejection before relay work and same public identity after service restart; receipt contains the bounded CPU/memory/task properties. The binary has no production Rust delta in this review. |
| Existing agents and cleanup | **Measured:** baseline, after, return and fresh reviewer metadata/hash snapshots have exactly equal `units` objects for all three agents: PID/start/restart, executable path/hash, and recorded unit/drop-in/environment-file hashes. No credential contents were printed. All named disposable fixture paths/account/group are absent; systemd reports fixture `LoadState=not-found`, `ActiveState=inactive`, `MainPID=0`. **Read:** executor cleanup and bounded build completion receipts. |
| Install/partial failure/rollback/invitation | **Blocked by H-1.** Remaining procedure is bounded and readable: private state/root-owned files; stop before backup; retain account/identity for recovery; restore same package without re-init; stop/remove is distinct from revoke. Invitation must use Bryce's real private interactive terminal, not captured tool output or fabricated TTY. CLI terminal check is confirmed in `registration_cli.rs:61–68`. These procedures were read, not executed. |
| Canonical hosted-fixture continuity | **Read/accepted.** [Hosted script](../../../crates/buzz-hostd/deploy/stage-registration-on-actions.sh):42 corrects the same root-relative probe exception. Its bind and actual probe entry point remain unchanged. Prior [Linux service review](3750-registration-integration-regate1.md) supplies unchanged protocol journey coverage; the new actual netcup unit exercise supplies this path correction's boundary evidence. Full hosted journey need not be repeated for this unit-only fix. |

Private artifact pins independently recomputed:

- Archive `netcup-registration-f316fb46-19d7ee7ab7cc.tar.gz`:
  `19d7ee7ab7ccffabe481561aac25c4e4384a08b929630fd6400d327b1073fb45`.
- Manifest: `7f145bb3c571d3f4ff7e372b34003be333465db1d3478c2a149c271b2990e373`.
- Binary: `06ef06b83181db97338851b01a08627d7fd23c437bda7b6aac602cdb9124230a`,
  source `f316fb46d89c3e16eedc6173ae06e85fe224a1a9`.
- Canonical unit: `ee30e40b472a05d5d7931464dbc692dddd652a9c36f65e4d98dc00d398155b3e`.
- Site unit: `9cb7a6d7b9df3e18b39d46916d1d7b32140e0567e123305dfe8685a5e8a03bce`.

## Verification and handoff

**Reviewer ran:** collision falsification; archive/manifest/file hash checks; source
and effective-fixture continuity checks; current dependency/CA hash checks; fresh
agent snapshot and cleanup/account/unit absence checks; scoped `bash -n`,
`rustfmt --edition 2021 --check`, and `git diff --check` (all scoped static gates
passed). An initial reviewer format invocation accidentally selected edition 2024
and reported formatting differences; repository Cargo.toml declares 2021, and the
corrected invocation passed. No source was modified. Initial sandboxed Hermit
activation could not update its cache; authorized escalation activated it normally.

**Read and retained, not rerun:** optimized locked/offline build, probe compilation,
actual-systemd boundary/TLS/singleton/restart rehearsal and original failed attempts,
prior protocol/Windows automated evidence. Continuity above supports reuse; it is
not a claim these runs were performed by this reviewer.

**Not run:** permanent install, production identity/invitation/pairing, native Windows
human journey, actual reboot, full repository CI, complete hosted journey rerun.
These remain explicit later-stage obligations/limitations; this is a preinstall gate,
not #3750/#3751 completion or PR-readiness attestation.

Minimalism and elegance: **9/10**. Correctness: **7/10** because H-1 defeats the
agreed collision contract; the named explicit-failure repair is required. No other
supported current-scope finding or architectural prerequisite was found.

Conductor routing: keep installation held; return H-1 to the same executor, then
re-gate the replacement pin/package. No broad CI, relay redeployment or new isolation
campaign is required by this finding. Reviewer launched no service/build/background
process; all review commands completed. Only this authorized report is committed.
Worktree write/test ownership is relinquished after its signed-off commit.

quality: model=unknown effort=unknown stage=post-audit verdict=merge-block verify=ran commits=f316fb46d89c3e16eedc6173ae06e85fe224a1a9..6d526b1a76d4156703c395cfa729d0595b31903f findings=C=0/H=1/M=0/L=0 rework_rounds=0
