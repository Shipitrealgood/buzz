# #3751 — collision preflight re-gate

2026-11-07. **PREINSTALL ACCEPT for the replacement package below. H-1 is resolved;
current findings C=0 / H=0 / M=0 / L=0.** This releases the independent preinstall
hold to the conductor; it is not an installation, pairing, Windows human acceptance,
or declaration that #3751 is complete.

Same independent assignment, repair round **1**, exclusive worktree
`/home/bryce/services/buzz-wt-3720-host-registration`. Reviewed clean range
`d2e41477d40917b518ea85070f6c2912114974fb..a48b54674c6cab84d10730e74bef80061dd3cc8c`.
Requested configuration: **gpt-6-astra/high**. Runtime model/reasoning identity:
**unknown**, distinct from that requested configuration.

Scope is [H-1](3751-netcup-registration-preinstall.md), its exact repair and introduced
defects, under the [repair handoff](../../briefs/3751-netcup-registration-h1-repair.md).
All other original obligations remain accepted with their original evidence scope.
No implementation edits, installation, account/state/identity creation, invitation,
signed event, agent/provider action, relay deployment, merge/push or board write occurred.

## H-1 — closed (read and independently measured)

Private release root:
`/home/bryce/services/buzz-agents/releases/3751-netcup-registration`.
In its `package/OPERATOR.md:33–99`, package verification and every collision/lookup
failure now explicitly exit. Path checks use `os.lstat`, detect dangling links, and
accept only FileNotFoundError as absence. Account/group checks accept only getent
status 2; process checks accept only pgrep status 1. Matches and unexpected statuses
exit nonzero. The installed getent(1)/pgrep(1) documentation confirms those status
meanings. Lookup calls use five-second timeouts; timeout and command failure statuses
take the explicit refusal branches. The same-root-shell instruction at `:102–104`
retains the preflight's shell state for installation.

**Independent run: 45 cases passed**, using the inspected
`evidence/h1-preflight-check.py`. Only its receipt destination was redirected to a
reviewer temporary file to preserve executor evidence; its extracted operator text,
test cases and assertions were unchanged. No install block executes. The harness
substitutes temporary package/path locations, stubs account/process lookup outcomes,
and appends a marker standing for the first mutation. Filesystem collision checks
use real temporary files/directories/symlinks and real Python lstat.

| Case group | Measured result |
|---|---|
| Original package controls | 3 known failures reproduce: existing file/account/process reaches the mutation marker, exit 0. |
| Repaired clean control | 1 reaches the marker with exit 0, so blanket refusal cannot give a false green. |
| Repaired refusals | 41 exit nonzero without the marker: every target path as file/directory/dangling link; invalid-parent lookup error; account/group/process matches, error and unavailable-command status controls; path-probe errors; bad package checksum; timeouts. |
| Actual timeouts | 4 refuse at 5.008–5.035 seconds. The stub executes sleep under real GNU timeout; these are measured elapsed timeouts, not merely injected status 124. |
| Syntax and cleanup | Extracted repaired block passes `bash -n`; temporary test directory is absent after completion. No installation block or real account/process lookup ran. |

Preflight text SHA256, independently derived by the run:
original `a7033939eded2b6cfaac73899af99e1602cb5036b227419ecd81580ffc4f81f9`;
repaired `9ae94acc21f55d5d54293bb02db89f665150e42c97091f631085d63cf0e5a0c7`.
The [committed executor case inventory](../../verification/3751/netcup-preinstall-h1.json)
documents the same 45 cases; its authorship remains executor evidence, with the
independent outcomes above recorded here. No further collision repair is required.

## Exact installable package and evidence continuity

**Measured:** both immutable archives match their declared hashes; every manifest
entry matches; each archive's eight files equal its associated package directory.
The preserved `package-pre-H1` matches the original reviewed archive. Comparing its
complete file set against `package` finds exactly two changed files: `OPERATOR.md`
and `MANIFEST.sha256`. Binary, service unit, hosts, nsswitch, controller configuration
and SOURCE.json are byte-identical. Source commit delta contains only documentation
and verification receipts; no production source, probe or build input changed.

Accepted replacement:

- Archive: `netcup-registration-f316fb46-h1-419b315bb4cd.tar.gz`.
- Archive SHA256: `419b315bb4cdbd7320bba92437a9995e2227568cf1438c770d4a70b3e9906982`.
- Manifest SHA256: `795bde032271f7d923a08eda2f659f30544e9f625db4cea68d18b030b3696659`.
- Binary SHA256: `06ef06b83181db97338851b01a08627d7fd23c437bda7b6aac602cdb9124230a`;
  unchanged binary source `f316fb46d89c3e16eedc6173ae06e85fe224a1a9`.
- Site-unit SHA256: `9cb7a6d7b9df3e18b39d46916d1d7b32140e0567e123305dfe8685a5e8a03bce`.

The original `netcup-registration-f316fb46-19d7ee7ab7cc.tar.gz` remains historical
failed-package evidence and **must not be installed**. The execution brief explicitly
points to the replacement handoff. The private deployment receipt also names this
replacement. No ambiguity remains about which package this verdict accepts.

Prior build, actual-systemd sandbox/TLS/singleton/restart, existing-agent continuity
and cleanup evidence are retained under the exact byte-continuity argument above;
they were not repeated or relabeled as new measurements. The operator procedure
still requires current collision, public relay, library/CA and agent-continuity
preflight before actual installation. The review does not freeze those live facts.

## Routing, limits and ownership

Reviewer ran the 45-case targeted suite, package/archive/manifest and byte-continuity
checks, source delta inspection and `git diff --check`. All required re-gate checks
passed. Minimalism/elegance/correctness: **9/10 each**; no new supported finding.

Not run: actual installation, production identity/invitation/pairing, native Windows
human journey, reboot, full CI or a repeated systemd/hosted protocol campaign.
These remain later acceptance steps or previously disclosed scope limits.

Recommend conductor release of this exact replacement to the same executor for the
already authorized bounded live continuation. Existing agents remain outside scope;
invitation output still requires the real private human terminal. No new user approval
or wider verification campaign is introduced by this re-gate.

All test subprocesses completed and temporary fixtures were removed; no service,
background build, controller or account was created. Only this sanitized report is
committed. Exclusive write/test ownership is relinquished at its signed-off commit,
with a clean worktree.

quality: model=unknown effort=unknown stage=post-audit verdict=merge-safe verify=ran commits=d2e41477d40917b518ea85070f6c2912114974fb..a48b54674c6cab84d10730e74bef80061dd3cc8c findings=C=0/H=0/M=0/L=0 rework_rounds=1
