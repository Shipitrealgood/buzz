# Independent review scope: Windows/hostd registration integration (#3750)

Requested 2026-11-07 by conductor `/root`; next owner is independently dispatched
Astra/high Chuck. Executor requested configuration: gpt-6-astra/high. Runtime
model/reasoning identity is not independently exposed by this worker's tools.

## 1. Where the work lives

Worktree: `/home/bryce/services/buzz-wt-3720-host-registration`.
Branch: `proof/3720-host-registration-events` (not main).
Code range: `621c777b886dad8aed7eeee4b428d57c153829e5..0026180c1ee00ca3f3eb583d30ca14302a415072`.
Two signed-off code commits: `8404ea2c3070622eeffc7341a24ccb4b89f08d4a`
(owner/native/UI), `0026180c1ee00ca3f3eb583d30ca14302a415072`
(host executable/service/rehearsal). The subsequent documentation commit adds
this brief and sanitized receipts only. Starting worktree was clean; no unrelated
changes were present or reverted. Code/tests are stopped before ownership transfer.

```bash
cd /home/bryce/services/buzz-wt-3720-host-registration
git diff --stat 621c777b886dad8aed7eeee4b428d57c153829e5..0026180c1ee00ca3f3eb583d30ca14302a415072
```

## 2. What landed

Windows registration commands now use the reviewed v2 registry, native signer
and protected Device journal. My hosts exposes Add Host for an empty supported
list and registration states without old provisioning/runtime controls. The
existing hostd binary has a separate `registration` command family, bounded
reconciliation and a shared controller lock. A dedicated restricted-root service
unit and the existing hosted Ubuntu workflow prepare the actual service gate.

Approved inputs remain in `/home/bryce/services/buzz-agents/docs/`:
`specs/windows-netcup-registration-milestone.md` section #3750 and
`plans/3750-acceptance-checklist.md`. Reuse the accepted baseline review
[3720 regate1](../audits/post-audit/3720-registration-proof-regate1.md).
This implementation does not complete #3750's platform/artifact/human gates.

## 3. Your job

Adversarially review this delta and the finite acceptance checklist. Assess the
source and measured evidence independently; the executor's self-review is not
the merge gate. Return findings to the conductor without board writes or merge.

**Special instruction:** the systemd unit has not run locally. Its restrictive
filesystem must still work with the real loader, TLS roots and boot clock on the
hosted runner. `ProcSubset=pid` was deliberately omitted because the existing
clock reads `/proc/sys/kernel/random/boot_id`. Do not upgrade a compiled probe
or a successfully registered ordinary process into service-isolation evidence.

## 4. Specifically look for

1. **Scope fencing before signing and persistence.**
   Probe `crates/buzz-paired-owner/src/registry.rs:492` and `:729`,
   `crates/buzz-ws-client/src/connection.rs:155`, and
   `desktop/src-tauri/src/commands/paired_hosts.rs:54`.
   AUTH obtains its signer after the challenge. Recovery checks native scope
   after network waits before journal effects. Existing static-key consumers
   delegate to the same methods. Windows session tests at
   `desktop/src-tauri/src/commands/paired_hosts/create.rs:317` cover mismatched
   owner/community, in-progress switch, changed key, lock, reset and generation;
   these Windows-only tests are prepared, not executed on Debian.
2. **Terminal recovery before retrying a rejected intent.**
   Probe `crates/buzz-paired-owner/src/registry.rs:705` and native directory
   `desktop/src-tauri/src/commands/paired_hosts.rs:129`. Self-review caught and
   fixed retry-before-recovery: a host-local revoke can retire an owner intent
   the relay never accepted. The actual executable journey now exercises that
   ordering using the same synchronization method the native directory calls.
3. **Honest UI and session-local feedback.**
   Probe `desktop/src/features/agents/ui/PairedHostsSection.tsx:42`, `:146`,
   `:258`, `:334` and its jsdom journey. No registration card exposes installed
   runtimes, freshness as online status, Add agent, or legacy history actions.
   Errors do not turn a cached/empty directory into permission to add/provision.
   Submitted invitations cannot be falsely “cancelled”; pre-submit Cancel clears
   the input, and session remounts fence late UI feedback.
4. **Actual controller boundaries and durable retry.**
   Probe `crates/buzz-hostd/src/main.rs:68`,
   `crates/buzz-hostd/src/registration_cli.rs:50`, `:144`, and
   `crates/buzz-hostd/src/registration.rs:45`, `:365`, `:433`.
   Registration exits the main dispatch before Store/HTTP provisioning setup.
   One socket operates at a time, each pass has a 45-second deadline, consecutive
   failed passes terminate at six with capped backoff; systemd also rate-limits
   restarts. Pending events are filtered to the AUTH generation before sending.
5. **Service proof is real but still pending execution.**
   Probe `crates/buzz-hostd/deploy/buzz-host-registration.service:1`,
   `stage-registration-on-actions.sh:1`, the isolation probe example, and
   `scripts/paired-hosts/registration-systemd-smoke.py:138`.
   The real unit binds its state, executable, runtime libraries, public CA and
   resolver files into an otherwise empty root. World-readable credential/home
   and executable helper canaries are accessible to its UID outside that root;
   ExecStartPre must reject them inside the actual unit. Only the probe binary
   is an extra test bind. Loopback IP policy is shipped; remote relay addresses
   require a separately reviewed installation drop-in. No per-port network
   isolation measurement is claimed by the file/helper probe.
6. **Artifact and human evidence boundaries.**
   Probe `.github/workflows/paired-hosts-staging.yml:1` and the two operator docs
   in `docs/verification/3750/`. Existing fork/ref restrictions remain; the job
   builds matching debug rehearsal artifacts and checksums, never publishes a
   release. The Windows canary shares `xyz.block.buzz.app`; it is not an isolated
   profile. A separate Windows OS test account or a proven fixture-only journal
   is required before journal-loss manipulation. The WSL handoff gives an agent
   the setup work; Bryce should receive a ready fixture and app steps.

## 5. Decisions and deviations

No protocol, relay SQL, migration or management-plane redesign. Narrow adapter
additions preserve the old public `connect`, `recover` and `authenticate` APIs.
Windows WS now enables native certificate roots so an explicitly trusted local
fixture CA works without disabling TLS verification. Hostd uses an operator-pinned
public relay key and optional explicit CA; invitations cannot change that pin.
The service uses a separate registration UID/state/root and exposes only the
registration command. The old v1 provisioning implementation remains for its
separate retirement task; this UI no longer mounts its forms.

Local program/router evidence substitutes for none of the hosted service or
native Windows requirements. The old hosted provisioning/message script remains
historical and is not called by the adapted workflow. Full just ci was not run;
scoped strict lint is blocked by unchanged baseline source. See the run index.

## 6. What not to chase

#3751 live transition, #3730/#3731 migration compatibility, #3732 external-test
source discovery limitations, #3739 HTTP retirement, provider login/placement,
agent lifecycle/adoption and the full installer are separate work. Ordinary
remote-agent cloud/provenance and local/provider creation code is unchanged.
Do not expand the finite acceptance into a new mutation campaign or fixture
framework. No live service, credential or user message was touched.

## 7. Verification, deliverables and next gate

Read [run-index.json](../verification/3750/run-index.json) and
[source-audit.json](../verification/3750/source-audit.json). Measured: registration
25/25; ordinary WS/REQ 104 passed +2 existing ignored; hostd library 8/8; owner
14/14; WS 5/5; UI journeys 2/2; desktop TypeScript; formatting; configured size
policy; Python/shell syntax and workflow YAML; candidate debug binaries built.
Strict lint emitted two unchanged core and seven unchanged legacy-host findings;
no strict-lint/full-CI pass is claimed. Static systemd verification locally stops
at the deliberately uninstalled `/usr/local/libexec/buzz-hostd`; actual unit
execution is not run here.

After independent review, conductor may push the exact candidate to the already
allowed staging/canary refs and trigger their workflows. Hosted systemd service
proof, Windows native checks/installer checksum, native UI → isolated Linux
journey and human confirmation remain open. The conductor owns those actions
and the board. No new permission request or live rollout is implied.

Cleanup: zero fixture databases before teardown; all four task-labelled Docker
containers and their anonymous volumes removed; synthetic TLS private keys
removed. No fixture subprocess remains. Retain the worktree and ignored build
cache for review; see artifact hashes and cleanup receipt in the run index.
