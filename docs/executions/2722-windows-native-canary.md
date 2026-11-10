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
