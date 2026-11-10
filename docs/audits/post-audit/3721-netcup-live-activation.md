# #3721 — independent netcup controller activation review

2026-11-10. **Merge-safe for controller activation only.** C=0 / H=0 / M=0 /
L=0; no merge blockers or required repairs. This does not accept provider readiness,
the disposable-agent pilot, confirmed Cleanup, reboot or overall release readiness.

## Scope, authority and provenance

Reviewed `05d674b2d..1792332bfc6bf500241156d26f746e2c16772190` in
`/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`; independently confirmed clean input and
full head. The complete diff adds only the 217-line
[activation receipt](../../executions/3721-netcup-live-activation.md). Rework round0.
Reviewer `/root/netcup_activation_review`, codex-chuck. Dispatch requested
**gpt-6-astra / high**; actual served model and effort are not exposed, hence unknown.

Applied post-audit; read the personal/project instructions, portable conductor
contract, relevant vision/testing/architecture guidance, activation specification,
receipt and frozen OPERATOR runbook. Reused the accepted
[preparation review](3734-netcup-rollout-preparation.md#source-and-artifact-continuity)
at19b60eee for source, build, native fixtures and prefix-aware recovery design.
No new product source review or test campaign. Explicit live approval and the
subsequent Claude-preferred/Goose-fallback pilot ruling supersede the frozen
runbook's historical holds and Grok-only proposal, as documented in the
[activation scope](../../executions/3721-netcup-live-activation.md#scope-and-pins).

Exclusive ownership was limited to this report. All reviewer operations were
read-only except creating and committing this report. No service mutation,
provider invocation, login, new agent, taskctl, push, merge, build, rollback,
fault injection, archive extraction or live SQLite query. No secrets, database
payloads, full configuration or environment dumps were printed.

Evidence labels below distinguish **measured** reviewer checks, **read** retained
executor receipts and **reported** user evidence. Read receipts are not silently
promoted into independently rerun historical actions.

## Six-obligation assessment

### 1. Installed paths and bytes — PASS

**Measured, native at22:17:10 UTC:** `/proc/2212700/exe` resolves to the frozen
package's `buzz-hostd`; executable SHA256 is
`3a31f56b453ec5c6c8d5e1140a5921bd2bfd392b8207c6071f0cd86b3b6f054d`.
Installed controller/template under `/home/bryce/.config/systemd/user/` and
`/var/lib/buzz-host-registration/deployment.json` each match their package bytes;
each is regular1002:1003/0600. Their respective hashes are
`0c5fb6ec7122a38d19d8ca30a0f9d89fdb22e9164072372f872a3e9664702ae0`,
`6c7759314a7da08cd018f29714edffa1b5f078e4d191f1d5d53993094e686394` and
`f019bb6dc7b256cfb931f950fa45d53e95a677f6d138e7b3b7321f90c64e376b`.

Parsed only the installed deployment configuration's local-path section: ACP and
launcher point to this exact package's `buzz-acp` and `buzz-host-runtime`; ordinary
HOME/workspace/PATH/user-unit directory agree with the accepted package. All eight
manifest entries independently rehashed successfully; manifest SHA256 remains
`ec1fb606e835ba238a39021a3ad17d90e1f8c3ea933b568f2fa5db78e5671441`.
Actual build344baa0/production929c continuity is reused from the
[accepted source/artifact assessment](3734-netcup-rollout-preparation.md#source-and-artifact-continuity).
This checks installed references and bytes, without claiming a hosted runtime has
started. Package:
[/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/344baa091ad486653ee99aa8b1de64d4839b4ada/package](/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/344baa091ad486653ee99aa8b1de64d4839b4ada/package).

### 2. Exactly one controller and actual service context — PASS

**Measured:** user manager reports enabled, loaded, active/running, MainPID2212700,
NRestarts0, start22:13:04 UTC, exact installed FragmentPath, no drop-ins,
WorkingDirectory `/home/bryce`, and effective configured `HOME=/home/bryce`.
The process is UID1002, cwd `/home/bryce`, in the user1002 service cgroup:
`/user.slice/user-1002.slice/user@1002.service/app.slice/buzz-host-registration.service`.
This combines manager context with process identity; PID existence alone was not
used as service-context proof.

Root-visible `/proc` executable census found exactly one buzz-hostd, PID2212700.
System manager reports masked/inactive/dead/MainPID0 and no cgroup; the system-unit
path is a `/dev/null` symlink. Old PID43436 is absent. **Read:**
[after-start receipt](/home/bryce/services/buzz-agents/releases/3721-netcup-live-activation-20261110/controller-after-start.txt)
and [stability receipt](/home/bryce/services/buzz-agents/releases/3721-netcup-live-activation-20261110/controller-stable.txt)
match at22:13:04→22:14:04. The independent22:17:10 check extends the same
PID/start/NRestarts observation by over four minutes from start. This establishes
observed activation stability, not reboot or long-term health.

### 3. Binding and state custody — PASS within receipt scope

**Read:** stopped-state
[public binding](/var/backups/buzz-host-registration/3734-trusted-344baa0/public-binding-before.json)
equals the public binding in
[native-after.json](/home/bryce/services/buzz-agents/releases/3721-netcup-live-activation-20261110/native-after.json):
host `66f5d7c51f4dbccc9746204f5860e57c98e30a56b777d05c6f4f209cd7f38d7a`,
owner `f5c79eab2f44284ff278780c34aab621aafb149be4d7dba8f9625bfd99419733`,
generation `6eaf423e2e2fffa23e142135bfb3583ba22dcbadf0daedc57c747fe54ec28b40`,
community `https://buzz.interleaven.com`, active. Pre-start
[registration status](/home/bryce/services/buzz-agents/releases/3721-netcup-live-activation-20261110/registration-before-start.txt)
agrees. Executor reports readonly public-column queries; reviewer did not reopen
the live database. These receipts and the recorded sequence support continuity;
they are not an independent replay of historical custody actions.

**Measured:** current state directory, deployment.json, host.sqlite,
reconciler.lock and hosted/ are1002:1003; directories0700, files0600, no links or
special files, all files single-linked. **Read:** stopped-state metadata records
the original988:988 private tree with only database and reconciler.lock. Stage1
completed22:12:33 and stage2 completed22:13:04. No receipt indicates initialization,
second state tree or stale snapshot restoration. Current installed controller
retains the exact `/var/lib/buzz-host-registration` state path.

### 4. Protected agents and absence of adoption — PASS

**Measured:** live selected metadata equals
[protected-agents-before.txt](/home/bryce/services/buzz-agents/releases/3721-netcup-live-activation-20261110/protected-agents-before.txt)
byte-for-byte. Claude PID641476/startNov3 20:27:07, Grok PID637752/start20:26:09,
Scout PID619136/start20:18:08 UTC; all active/running, NRestarts0. Shared template
and all three named drop-in hashes match
[protected-unit-hashes-before.txt](/home/bryce/services/buzz-agents/releases/3721-netcup-live-activation-20261110/protected-unit-hashes-before.txt).
All four saved transition/final metadata receipts also equal that baseline.
Zero `buzz-hosted-agent@*.service.d` directories exist. This supports unchanged
existing-agent process/unit continuity and no managed adoption; no agent was
prompted to manufacture stronger provider evidence.

### 5. Recovery assets and current-state rule — PASS

**Measured:** backup directory
`/var/backups/buzz-host-registration/3734-trusted-344baa0` remains root:root/0700.
Archive/hash, configuration, public metadata and stage markers are0600. Saved
original unit is root:root/0644, hash
`9cb7a6d7b9df3e18b39d46916d1d7b32140e0567e123305dfe8685a5e8a03bce`.
Saved old executable and configuration hash-identically match their untouched
installed originals: respectively
`de05655071c5f6af3451cc70116969f53da0df0ff64448593a0fc913bf10d3ee` and
`df305f3dfe9d594cdbd2962c9910d5d126db881bed2458999b967d9d5706c14d`.
Retained state-before.tar hashes to its recorded SHA256
`3cd8823d1bf5021019b9f3e74e806701e9c45b2828238ee02899c4f6f1e8d33f`.

That archive is the stopped **pre-cutover** snapshot, not a fresh post-activation
database backup. The unchanged
[OPERATOR.md](/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/344baa091ad486653ee99aa8b1de64d4839b4ada/package/OPERATOR.md)
requires selecting original/moved/masked recovery, proving both controllers and
any pilot quiescent, validating and snapshotting CURRENT state, and restoring
custody988:988 before old-service start. It forbids extracting the old archive
over current journals. **Read/reused:** accepted prefix recovery review.
**Not run:** recovery, restoration or fault injection. Hashes and retained
instructions establish available recovery assets, not executed rollback success.

### 6. Attribution and remaining acceptance — PASS

The [WSL flag-on gate](../../executions/3734-relay-deployment-enabled.md) remains
user-supplied evidence: candidate929c/image5bc5300/schema82, flag-on and WSL review
were not rerun here. The activation receipt labels its NIP-11 measurement as
executor-native evidence and does not turn it into provider acceptance.

**New user-reported evidence, relayed directly by conductor during this review:**
Bryce reopened Windows; netcup remained **Registered**; About showed
**v0.5.29-test.24**. Credit that human registration observation only. It updates the
execution receipt's then-pending Windows observation; no reviewer Windows run or
installer-hash check occurred. No provider/agent flow has run.

Executable presence and accepted source/native fixture results do not prove ACP
adapter readiness, returned models, provider login, subscription entitlement or
chat. These remain unrun, together with actual managed-agent credential delivery,
notify/cgroup behavior, confirmed Delete/Cleanup and reboot.

## Verification, routing and ownership release

Native read-only checks used narrowly approved unsandboxed execution for actual
systemd and root-private metadata/hash access; all completed successfully.
Initial Hermit activation hit its sandbox read-only cache self-update error;
`HERMIT_SELF_UPDATE=false` activated the pinned environment successfully for Git.
No systemd sandbox failure was counted as a regression or a green native gate.
Pinned range and report whitespace checks plus local-link existence checks passed
before signed commit. No suites or mutants were dispatched for this receipt-only
review; accepted builds/source/native tests and full just ci were not rerun.

Recommend conductor acceptance of controller activation. No new task, product
decision, architecture work or executor repair is required. Continue the already
authorized ONE-disposable-agent workflow under its existing readiness, no-login
and confirmed-Cleanup constraints; this review grants no expanded authority.
Minimalism/elegance/correctness9/10 each for the bounded activation and receipt.
Exact installed-byte checks, preserved protected-agent baselines and the explicit
current-journal recovery rule are the material strengths.

Only this report is changed. All reviewer commands are terminal; no background
process, held mutation or test ownership remains. The intended live user service
continues. Release the same workspace to conductor at the signed report commit;
conductor owns board disposition and next-stage routing.

quality: model=unknown effort=unknown stage=post-audit verdict=merge-safe verify=ran commits=05d674b2d..1792332bfc6bf500241156d26f746e2c16772190 findings=C=0/H=0/M=0/L=0 rework_rounds=0
