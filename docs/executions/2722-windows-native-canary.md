# #2722 Windows native canary execution

Status: preparation; Windows gates not yet run. Requested executor: Astra/high;
served configuration is not exposed. Exclusive source/test ownership is held in
`/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`. Clean baseline:
`bc827df8c6109eb6972ca2ee8786095bda5c9ee3`.

Accepted input: [source re-gate](../audits/post-audit/2722-windows-hosted-flow.md),
[contract](../specs/2722-windows-hosted-flow.md), and
[prior execution](2722-windows-hosted-flow.md). This bounded follow-on runs the
existing Windows artifact-only workflow plus the two existing hosted native tests.
No product/architecture change is planned. Full CI, human and live acceptance
remain not run. No runtime flag, live service, provider, pairing or agent home
is activated or touched.

Read fixture isolation: the hosted child removes inherited Buzz credentials and
runtime flag; sets temporary home variables; pins nest and the mock app-data
identifier to its temporary directory; bypasses managed-agent keychain storage;
uses a generated in-memory signer and temporary owner journal; pins profile
publication to closed loopback port 9; and requests no spawn. The metadata test
uses a no-local-binary closure and the production static projection. Existing
owner/directory/registration suites and disposable-account SQLite ACL test remain.

Workflow additions require each fully qualified hosted test to appear as passing
and report exactly one pass, zero failures and zero ignored. Per-suite test logs
and a source/run receipt are retained seven days. These logs contain fixture test
names/outcomes, not real credentials. The build stays unsigned NSIS, updater
artifacts disabled, with no release/tag/publication step.

Push-trigger inspection: all 31 workflow YAML documents parsed; push branches are
main, release or verify/paired-hosts-staging, with separately filtered release tags.
None matches build/paired-hosts-canary, and no workflow_run trigger exists.
Only a fast-forward push to the existing fork branch and explicit Windows Canary
dispatch are authorized. The recorded fork head f316fb46d89c3e16eedc6173ae06e85fe224a1a9
is an ancestor of the baseline; remote freshness will be checked before push.

Next: conductor ratification of the minimal patch, then dispatch/monitor to
terminal status. Source changes from actual Windows failures require a fresh
independent review before the resulting artifact is accepted.

## Run ledger

Conductor ratified workflow commit `1bec0dc72c5228b3fcfe73879a5910307596dac4`.
Fresh remote equality and ancestry checks passed; normal fast-forward push
updated only `Shipitrealgood/buzz:build/paired-hosts-canary`. Manual dispatch:
[run 42762758173](https://github.com/Shipitrealgood/buzz/actions/runs/42762758173),
API-confirmed exact head `1bec0dc72c5228b3fcfe73879a5910307596dac4`. Both Windows
jobs started 2026-11-10 05:32 UTC; executor retains monitor ownership.
Preparation gates: YAML parse, embedded Bash syntax, branch-trigger exclusion
check, source ancestor check and `git diff --check` passed. No Windows result
is claimed until the terminal receipt below.

## First native failure and bounded correction

Run 42762758173 test job 129652105479 compiled Windows Tauri successfully, then
passed owner **16**, nostr conversion **73**, relay directory **7**, paired-host
registration **3** tests (all zero failures/ignored) and the SQLite ACL probe.
The hosted phases case ran exactly once and failed at its first phase call:
`agents_hosted_tests.rs:112`, `invalid or unsupported host deployment contract`.
The metadata case was skipped after that failure; it is not counted as passed.

Read diagnosis: `paired_host::canonical_origin` requires `Host::Domain`; the
fixture used the rejected IP-literal scope `https://127.0.0.1:9`. This was a
fixture prerequisite error, not a Windows compiler or production validator defect.
The correction uses `https://localhost:9` / `wss://localhost:9`, asserts the scope
passes production validation before the phases, and replaces only the fixture's
HTTP client with an explicit 127.0.0.1:9 resolution, no proxy and no redirects.
The production canonical-origin rule and all main creation/recovery guards remain
unchanged. Conductor ratified this bounded fixture correction. Scoped Rustfmt
and diff whitespace checks passed; actual correction verification awaits Windows.
The original installer job remains monitored and is not accepted independently.
