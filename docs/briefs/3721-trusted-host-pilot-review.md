# Review scope: trusted netcup pilot preparation (#3721)

**Date:** 2026-11-10. Requester: conductor `/root`.
**Next agent:** independently dispatched codex-chuck; requested Astra/high.
**Executor:** codecodex `/root/netcup_trusted_pilot_prepare`, requested Astra/high;
served model/reasoning not exposed. This is preparation, not activation approval.

## 1. Where the work lives

Worktree `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events` (not main), exclusive ownership released
at the signed documentation commit supplied in the return. Review range begins
`c97256e20893b2688c1cc1c6bc2e23361aab707e`; only two documents changed. No baseline
dirt. All build/check sessions terminal; no detached owned process.

```sh
cd /home/bryce/services/buzz-wt-3720-host-registration
export HERMIT_SELF_UPDATE=false
. ./bin/activate-hermit
git log --oneline c97256e20893b2688c1cc1c6bc2e23361aab707e..HEAD
```

Artifacts/evidence live under
`/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/c97256e20893b2688c1cc1c6bc2e23361aab707e/`.
`package/MANIFEST.sha256` SHA256 is
`02e2ffe9adcb06a8c83b51b51921c0c83e93654de6ad0b222f2b4318b1655490`.
Package source is baseline c97256e, production/build-identical to Windows23d2dc.

## 2. What landed

[Execution packet](../executions/3721-trusted-host-pilot-preparation.md) commits
exact staged config/unit/runbook text, build pins/hashes, measured installation
context, finite verification and two concrete activation blockers. Package has
matched release hostd/runtime/ACP, automatic main catalog with capacity1, ordinary
bryce HOME/PATH/workspace, exact source agent template and guarded operator plan.
No product source or live state changed. Inputs include
[ratified spec](../specs/3722-main-discovery-native-login.md), accepted
[S1](../audits/post-audit/3721-trusted-host-executor.md),
[S2/S3](../audits/post-audit/3721-host-discovery-windows.md) and
[test.21](../audits/post-audit/2722-shared-discovery-windows-canary.md) reviews.

## 3. Your job

Adversarially review preparation safety, source/artifact match and truthfulness.
Do not turn preparation acceptance into pilot readiness: missing public43104 and
missing owner/setup Cleanup are real gates. No broad suite or provider/login loop
is needed. The current code's accepted tests are carry-forward evidence.

**Special attention:** rollback after a created agent only contains the exact
unit and retains its journal/placement; it cannot claim completed product teardown.
A successful disposable pilot must wait for the narrow confirmed cleanup path.

## 4. Specifically look for

1. **Single controller and key custody.** Is same-state transfer explicit and
   reversible without generating an identity or racing old StateDirectory logic?
   **Probe:** compare runbook stop → backup → saved regular unit → mask → chown →
   user start against `registration_cli.rs:110` lock and `store.rs:58` ownership.
   Existing root-helper drop-in is excluded. Old system service is masked before
   changing custody, and no second state is used.
2. **Actual baseline and rollback binary.** Are November7 receipts mistakenly
   substituted for the November9 refresh?
   **Probe:** current installed SHA is de056550…, not06ef06b8…. System unit remains
   hash9cb7a6d7…. Old binary/config stay installed. Rollback restores current
   journal/custody, not stale snapshot data; `store.rs:117` and
   `deployment.rs:89` explain additive tables/version1 compatibility.
3. **Ordinary account runtime configuration.** Do paths support the trusted model
   without touching the current agents?
   **Probe:** deployment.json uses local.buzz_acp/runtimes:{} (main discovery),
   capacity1, HOME/home/bryce, normal tool directories and UID1002 runtime dir.
   `user_service.rs:388` requires exact private template; package template mode0600
   and bytes must match source. New pilot workspace is ordinary, not an auth store.
4. **Relay capability hold.** Could public absence be overstated as known old code
   or resolved by a flag assumption?
   **Probe:** public-nip11.json has registry v2 but no host_deployment; compare
   `nip11.rs:423` and `config.rs:678,1471`. Current serving source/config remains
   unknown. Separate owner must establish code/schema and exact true-valued flag.
5. **Cleanup affordance gap.** Is emergency stop being misrepresented as cleanup,
   or is a supported existing path overlooked?
   **Probe:** HostedAction at agents_hosted_commands.rs:256 has four non-cleanup
   actions; HostedAgentSetup.tsx:197 merely closes. Owner deployment.rs:61,96
   rejects non-Deploy; continuation.rs:226 produces Deploy. agents.rs:1037,1253
   refuses lifecycle/delete. Underlying host deployment.rs:453 handles Cleanup.
   Proposed minimal #3724/#3725 repair retains owner intent and identity until a
   current authorized acknowledgement; no new tool or manual DB alteration here.
6. **Evidence boundaries and interruption.** Are syntax and help results promoted
   to native acceptance, or are partial-cutover states unsafe?
   **Probe:** syntax-verification.json records two exit0 checks, not LoadCredential
   or cgroup proof. Initial sandbox failures are disclosed. Inspect each interruption
   prefix and make sure user controller/unit are stopped before original custody
   and system unit are restored. Provider/native chat/reboot/rollback remain not run.

Code paths above are relative to `crates/buzz-hostd/src/` unless another module is
named: relay paths under `crates/buzz-relay/src/`; owner paths under
`crates/buzz-paired-owner/src/`; Desktop command/UI paths are linked in execution.

## 5. Decisions and deviations

No change to ratified product architecture. Concrete local choices: keep existing
state path and transfer custody instead of copying identity; put executables at
immutable source-named release path; automatic main catalog capacity1; new empty
ordinary pilot workspace; exact user controller file plus preserved source template.
The existing system unit must be saved before masking because it is a regular
`/etc` file. ACP is packaged to match source/runtime without replacing existing
agents' older harness. Are these choices sufficiently explicit and reversible?

Found two missing live prerequisites, returned to conductor without implementing
workarounds. #3767 is legacy cleanup, not this pilot's missing cleanup path.
No new root helper, socket RPC, installer or provider broker was added.

## 6. What not to chase

Accepted shared discovery/Windows architecture and99 source/110 native cases plus
ACL remain prior evidence; no rerun or redesign. Existing agent adoption #3727,
legacy cleanup #3767, broad lifecycle UI, relay deployment and actual service/provider
activation are separate assignments. Preserve current live registration and agents.
No private provider/key/journal reads, authenticated relay requests, SSH, live
mutation, new agent test or board writes are authorized for this review.

## 7. Deliverables and preflight

Return a committed independent report at
`docs/audits/post-audit/3721-trusted-host-pilot-preparation.md`, severity findings,
measured/read/inferred distinction, preparation verdict and exact remaining holds.
No activation-ready verdict unless both external gaps are actually resolved by
separately authorized work. Conductor owns routing and final approval request.

Preflight: clean correct worktree/pin; compare manifest with recorded hash and
staged bytes; read execution packet and both source reviews; preserve prior evidence.
Finite rechecks may inspect hashes/help/syntax and document links. No independent
live/provider action is authorized. All commands must finish before ownership return.
