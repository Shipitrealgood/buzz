# Re-gate scope: collision preflight H-1 (#3751)

2026-11-07; same executor assignment, same independent reviewer; repair round 1.
Requested executor: gpt-6-astra/high; actual runtime identity independently unobservable.

## 1. Where the work lives

Worktree `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`, clean starting review commit
`d2e41477d40917b518ea85070f6c2912114974fb`.
Review only the signed-off repair commit after that base and the replacement private package.
No production source, service unit, probe or build input changed in this repair.
Private release root: `/home/bryce/services/buzz-agents/releases/3751-netcup-registration`.

## 2. What landed

Repaired the exact operator preflight identified in
[H-1](../audits/post-audit/3751-netcup-registration-preinstall.md).
Only private package `OPERATOR.md` and its manifest changed. The original package is
preserved as `package-pre-H1`; its immutable archive, original receipt, and measured
fail-open reproductions remain available. All other bounded obligations were accepted
by the first review and are continuity inputs, not repeated tests.

## 3. Your job

Re-gate H-1 and defects introduced by this operator-procedure delta. Check that every
collision and uncertain lookup stops before mutation and that the exact replacement
package matches these pins. Installation remains held until conductor release.

## 4. Specifically look for

1. **Paths** — private `package/OPERATOR.md`, first shell block.
   Probe: bounded Python `os.lstat` accepts only FileNotFoundError as absence;
   existing file/directory/dangling link and other OSError explicitly exit nonzero.
   The shell explicitly exits on probe failure/timeout rather than relying on errexit.
2. **Account/group** — the same block's two `getent` calls.
   Probe: match status 0 explicitly exits, only documented missing-key status 2 proceeds,
   and every other status (including timeout/command failures) explicitly exits.
3. **Controller process** — the same block's `pgrep` call.
   Probe: only no-match status 1 proceeds; match and all uncertain statuses stop.
4. **Evidence binds the operator text** — private `evidence/h1-preflight-check.py`.
   Probe: extracts the first shell block from old and repaired OPERATOR.md; substitutes
   only disposable package/path locations, supplies finite command stubs and appends
   the first-mutation marker. No installation block executes. Old collision controls
   reach the marker; repaired collision/error controls must exit nonzero without it.
5. **Immutable replacement and continuity** — private receipt and MANIFEST.sha256.
   Probe: only OPERATOR.md and MANIFEST.sha256 differ; binary, SOURCE.json, unit, hosts,
   nsswitch.conf and controller.conf retain their accepted byte hashes. Old artifacts
   are retained. Actual-systemd/agent-continuity evidence remains the accepted first-pass
   evidence; no new service/account/provider operation occurred during this repair.

## 5. Decisions and deviations

Used Python lstat rather than shell `test -e`/`test -L` so filesystem lookup errors
cannot silently masquerade as absence. Python and GNU timeout are existing host tools;
missing/failing commands are fatal. Each lookup is bounded to five seconds.
No new installer framework or deployment behavior was introduced. Package verification
also has an explicit failure exit. These changes implement the accepted H-1 ruling.

## 6. What NOT to chase

The seven original obligations and source/unit/probe behavior are already reviewed.
Do not rebuild or repeat actual systemd/relay/protocol/Windows suites for this docs-only
repair. No activation, identity/invitation, merge/push, board write or agent operation
belongs to this re-gate. Existing Claude/Grok/Scout remain outside the change.

## 7. Deliverables and verification

Replacement archive: `netcup-registration-f316fb46-h1-419b315bb4cd.tar.gz`.
SHA256: `419b315bb4cdbd7320bba92437a9995e2227568cf1438c770d4a70b3e9906982`.
Manifest SHA256: `795bde032271f7d923a08eda2f659f30544e9f625db4cea68d18b030b3696659`.
The prior archive/manifest pins in the original execution brief are superseded for
installation and retained as historical evidence.

Measured with `python3 <release>/evidence/h1-preflight-check.py`: **45 cases passed**:
3 old fail-open reproductions; 1 repaired clean continuation; 41 repaired refusals.
These cover every target path as regular file/directory/dangling symlink, invalid-parent
lookup error, account/group/process matches, unexpected statuses and missing commands,
4 actual five-second timeout controls, and package-checksum failure. `bash -n` of the
extracted repaired block passed. No mutation marker was reached in any repaired refusal.
Manifest/archive verification and byte-continuity checks passed; `git diff --check` passed.
[Sanitized receipt](../verification/3751/netcup-preinstall-h1.json).

Not rerun: accepted builds/systemd/agent-continuity gates and unrelated CI.
Not run: installation, production identity/invitation/pairing, native Windows or reboot.
All bounded test processes completed and the temporary directory was removed. Source
was initially clean; only repair brief/index evidence is committed. Return ownership
to the same reviewer at the clean committed handoff. No named open question remains.
