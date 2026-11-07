# Preinstall review: registration-only controller (#3751)

Requested 2026-11-07 by the conductor; next reader: fresh independent codex-chuck.
Requested executor configuration: gpt-6-astra/high. Runtime identity is not independently observable.

## 1. Where the work lives

Worktree: `/home/bryce/services/buzz-wt-3720-host-registration`.
Branch: `proof/3720-host-registration-events` (not main).
Base: `f316fb46d89c3e16eedc6173ae06e85fe224a1a9`, initially clean.
Review the single signed-off implementation/evidence commit after that base.
No pre-existing dirty source files were present or altered.

Private release root: `/home/bryce/services/buzz-agents/releases/3751-netcup-registration`.
Its `deployment-receipt.json` is the site-specific index; host/process/config data stays
there rather than in this source repository. No invitation or production identity exists.

```sh
cd /home/bryce/services/buzz-wt-3720-host-registration
git log --oneline f316fb46d89c3e16eedc6173ae06e85fe224a1a9..HEAD
```

## 2. What landed

Prepared an optimized locked hostd binary from the accepted source pin and a restricted
remote-relay unit, then exercised the exact binary under actual systemd with synthetic
resources. The deployment probe found a real canonical unit defect: filesystem-policy
paths lacked `+`, so they did not constrain the RootDirectory view as intended. The
canonical unit and its hosted-fixture probe exception are corrected; product Rust
behavior remains unchanged. The new example is verification-only and creates no identity,
invitation, authentication or signed event.

Accepted inputs: [platform addendum](../audits/post-audit/3750-registration-integration-platform-addendum.md),
[prior re-gate](../audits/post-audit/3750-registration-integration-regate1.md), and the amended
private `buzz-agents/docs/specs/windows-netcup-registration-milestone.md` plus conductor dispatch.
The live relay source pin came from the accepted WSL receipt; public key/capability and
TLS were rechecked locally, without claiming an independent remote image inspection.

## 3. Your job

Review this finite deployment delta and its private package/evidence, especially whether
the effective namespace matches the documented claims. Return a preinstall verdict to
the conductor. The executor's measurements are evidence, not independent approval.
No permanent account/service activation or invitation is allowed before gate release.

Special instruction: the original noexec assertion failed in an actual namespace even
though unit text looked restrictive. Evaluate the corrected effective paths and canary
receipt, not merely the configuration spelling. Failed attempts remain preserved.

## 4. Specifically look for

1. **RootDirectory path semantics** — `crates/buzz-hostd/deploy/buzz-host-registration.service:45`.
   Probe: compare `NoExecPaths=+/ +/state`, root-relative executable/library allowances,
   and `ReadWritePaths=+/state` against installed systemd.exec(5). Private attempts 3/4
   show a copied state executable actually ran before correction; final rehearsal denies it.
   Attempt 5 passed confinement/TLS, then exposed a fixture startup-readiness race.
2. **Existing hosted fixture remains executable** —
   `crates/buzz-hostd/deploy/stage-registration-on-actions.sh:42`.
   Probe: its sole probe ExecPaths exception now also uses `+`; its bind remains unchanged.
   The complete hosted relay journey was not rerun. The actual local rehearsal exercised
   the same corrected canonical boundary with site overrides and a separate probe bind.
3. **Positive controls bind real denial claims** —
   `crates/buzz-hostd/examples/registration_deployment_probe.rs:25`.
   Probe: outside the sandbox, the same unprivileged UID reads all world-readable synthetic
   homes/state canaries, executes the harmless helper and connects to Unix/loopback listeners.
   Inside, all fail while `/state` writes work; copied executable and config writes are denied.
   Private `evidence/rehearse.py` owns all fixture paths and cleanup; no provider secrets used.
4. **Actual TLS path and bounded networking** —
   `crates/buzz-hostd/examples/registration_deployment_probe.rs:62` and private site unit.
   Probe: same `NostrWsConnection::connect`/native-root path as registration without explicit CA;
   connect/close only, no authentication. Fixed hosts mapping avoids DNS/loopback allowances.
   IP filtering is explicitly not a per-port firewall. No new public listener was observed.
5. **Lifecycle and source/artifact continuity** — private `evidence/rehearsal.json`,
   `evidence/agents-before.json`, `evidence/agents-after.json`, `package/SOURCE.json`.
   Probe: actual controller UID/capability/seccomp, singleton rejection, restart retaining
   identity, and all three existing agents' exact metadata/binary/config hash equality.
   Production-source changes since the accepted binary pin are still absent; this commit
   changes deployment/test/docs only. No production pairing or actual reboot is claimed.
6. **Finite installation/removal and privacy** — private `package/OPERATOR.md`.
   Probe: collision refusal, root ownership, disabled-login account and private state; rollback
   preserves identity and distinguishes stopping from revoking; human-only terminal invitation
   when ready. File hashes and private receipt pin the exact reviewable package.

## 5. Spec deviations and decisions

- Corrected the reusable unit and its existing probe exception after measured failure;
  conductor explicitly authorized this bounded canonical repair. No architecture reopened.
- Site config uses native TLS roots, an explicit remote address mapping and IP allowlist,
  the existing `--label netcup` flag, and resource caps. The stock loopback/fixture-CA unit
  is not a production install. Full effective diff is private `evidence/canonical-to-site.diff`.
- Initial fixture executable staging under `/run` hit the host's noexec mount; moved only
  disposable staging to the already executable filesystem. No mount policy changed.
- Fixture checks now wait for the actual executable before inspecting UID, because default
  Type=simple can return while systemd is still preparing its child. Final measured UID
  is unprivileged; the earlier transient root observation is retained, not concealed.

## 6. What NOT to audit / chase

Registration protocol/source/library and Windows automated reviews are accepted inputs.
Native Windows observations remain human-owned. Agent adoption, provisioning, subscription
changes, legacy Serve/HTTP retirement, broad installer, isolation campaign, SQL, Caddy,
Hetzner mutations, merge/push and taskctl writes are outside this assignment.
No existing Claude/Grok/Scout process, service, config, identity or subscription was changed.

## 7. Deliverables and preflight

Immutable content-addressed archive (read-only, privately held):
`netcup-registration-f316fb46-19d7ee7ab7cc.tar.gz`.
Archive SHA256: `19d7ee7ab7ccffabe481561aac25c4e4384a08b929630fd6400d327b1073fb45`.
Manifest SHA256: `7f145bb3c571d3f4ff7e372b34003be333465db1d3478c2a149c271b2990e373`.
Binary SHA256: `06ef06b83181db97338851b01a08627d7fd23c437bda7b6aac602cdb9124230a`.

Measured gates: optimized locked/offline two-job build; probe compilation; actual-systemd
boundary/TLS/singleton/restart gate; exact existing-agent continuity; scoped rustfmt,
existing staging-script `bash -n`, and `git diff --check`. Build limits were 4 GiB and
200% CPU at nice 10. See [sanitized index](../verification/3751/netcup-preinstall.json).
Not run: full repository CI, hosted relay journey rerun, permanent install, production
identity/invitation/pairing, native Windows human journey, actual reboot.
Standalone production-unit verification before installation reports the intentionally
absent executable; actual staged-unit `systemd-analyze verify` passed.

All foreground build/test commands completed, disposable service/processes stopped,
account/group and all owned fixture paths removed. Only release/evidence and ignored
build cache remain. Exclusive write/test ownership is released at the committed handoff.

Preflight for reviewer: confirm clean source and exact review range; verify private
archive/manifest hashes; read private receipt and failed/final actual-systemd evidence;
check the source-unit delta and finite operator procedure. Return findings and verdict
to the conductor without activating the service or generating an invitation.
