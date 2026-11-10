# Independent review: Windows native canary (#2722)

Date: 2026-11-10 UTC. Requester: conductor `/root`.
Next owner: fresh independent reviewer. Requested executor Astra/high;
actual served configuration unavailable.

## 1. Where the work lives

Worktree `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events` (not main). Review base:
`bc827df8c6109eb6972ca2ee8786095bda5c9ee3`. Workflow commit:
`1bec0dc72c5228b3fcfe73879a5910307596dac4`; fixture correction / built source:
`c4eda2615b72acc5761ef34bb14aed86509c9a14`. Two implementation commits plus the
final docs-only evidence commit. Final handoff pin will be supplied
by the conductor. Baseline clean; no unrelated edits found.

## 2. What landed

The existing artifact-only Windows canary discovery job now explicitly runs both
previously unexecuted hosted tests. Exact named success plus one pass/zero ignored
prevents false green from a misspelled filter. Existing owner/directory/registration/
ACL coverage remains, with separate test logs and a source/run receipt uploaded.
The build job and product code are unchanged. Native execution found that the
existing hosted test supplied an IP-literal scope forbidden by the production
canonical-origin contract. Fixture-only repair `c4eda2615b72acc5761ef34bb14aed86509c9a14`
uses a valid localhost domain and pins its HTTP client to loopback without
proxy/redirects; production validation is unchanged.

Inputs: [accepted source review](../audits/post-audit/2722-windows-hosted-flow.md),
[source contract](../specs/2722-windows-hosted-flow.md),
[source execution](../executions/2722-windows-hosted-flow.md), and
[this run receipt](../executions/2722-windows-native-canary.md).

## 3. Your job

Independently review the canary delta and its native evidence. Distinguish a
passing confined native phase test from full Tauri IPC, service/provider or human
acceptance. The executor's checks do not supply an independent gate.

## 4. Specifically look for

1. Exact test selection (`.github/workflows/windows-canary.yml:274`).
   Probe the full Rust module names, `--exact`, named success line, count assertion
   and pipefail behavior; a zero-match test must fail this job.
2. Fixture confinement (`desktop/src-tauri/src/commands/agents_hosted_tests.rs:6`).
   Probe the child environment scrub, temp app-data identifier/nest/journal,
   keychain bypass and loopback-only relay. At lines 44–53 the repaired client
   uses no proxy, no redirects and explicit localhost → 127.0.0.1:9 resolution.
   At line 68 the scope must pass production validation before main phases.
   Query/policy reads use this replacement `state.http_client`; profile POST uses
   `state.media_fetch_client`, whose existing constructor separately pins localhost
   and disables redirects, but does not explicitly disable proxy inheritance.
   Do not infer blanket proxy immunity from this fixture correction.
   The owner is generated, not loaded
   through persisted identity setup. No native activation env is introduced.
3. Artifact authority (`.github/workflows/windows-canary.yml:16`).
   Probe unchanged repository/ref gate, read-only content permissions, unsigned
   NSIS config and disabled updater artifacts; both jobs must pass before the
   installer is described as ready.
4. Evidence identity (`.github/workflows/windows-canary.yml:247`).
   Probe receipt source/run binding and per-suite file names. Compare downloaded
   installer hash to the build receipt and exact Actions head, rather than branch
   tip after a documentation-only evidence commit.
5. Retained gates (`.github/workflows/windows-canary.yml:255`).
   Probe owner, directory conversion, loopback discovery, paired registration and
   ACL results/counts, including failure logs from any earlier unsuccessful run.

## 5. Deviations

No product/spec deviation. The first Windows run exposed a fixture prerequisite:
`canonical_origin` rejects IP literals. The correction changes only the test
input/client and asserts the prerequisite; it does not relax any guard or change
expected recovery behavior. Tests already exist;
this task wires their supported native execution into the existing manual job.
Per-suite stdout and ACL receipts replace the overwritten discovery log so the
reviewer can inspect actual outcomes. GitHub run logs retain compiler diagnostics.

## 6. What NOT to audit / chase

Prior hosted-flow source acceptance is an input. This work does not reopen the
architecture, hostd, relay, schema, ACP or UI. Full repository CI and human/live
acceptance remain separate. No live flags, provider login, pairing, deployment,
installation, primary-branch merge, tag, release or updater publication occurred.
The installer is unsigned and is not executed by this agent.

## 7. Deliverables and pre-flight

Return independent findings and the pinned accepted/rejected range to the
conductor. Read the final execution receipt for terminal jobs, exact counts and
artifact identity before assessing readiness. Confirm the worktree is clean at
the conductor-supplied handoff and no executor-owned monitor remains active.
Do not infer final acceptance from the initial workflow-only commit or build job
alone. Process closure and final pin will be recorded before ownership release.

### Final measured handoff

[Corrected run 42764261281](https://github.com/Shipitrealgood/buzz/actions/runs/42764261281)
is terminal SUCCESS: 101 Rust tests + ACL, zero failed/ignored; build 37m9s,
tests 14m9s. Same exact source `c4eda2615…` for both jobs. The original run is
terminal failure (native fixture failed; installer passed), preserved as evidence.

[Installer artifact 13739854472](https://github.com/Shipitrealgood/buzz/actions/runs/42764261281/artifacts/13739854472):
`Buzz_0.5.29-test.20_x64-setup.exe`, SHA-256
`47940334b97c5589c6efeb6f3ea51cd488d5305a632387ab4925bfa3d706eb9b`,
expiry `2026-11-17T06:21:05Z`. ZIP digest and installer receipt both verified;
listing only, no installer execution. Raw logs/text receipts/metadata/downloads:
`/home/bryce/services/buzz-agents/releases/2722-windows-canary/c4eda2615b72acc5761ef34bb14aed86509c9a14/42764261281/`.

No model/runtime activation or production source repair. Full CI, human, actual
Tauri IPC/service/provider/live gates remain NOT RUN. Both run watchers exited;
all downloads finished. Writer/test ownership released at the final signed-off
docs commit; no unrelated changes at baseline or final state. The conductor
receives the exact final pin. No open product question or further source fix is
proposed; independently assess the terminal artifact/count evidence now.
