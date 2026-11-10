# #3734 — attributed WSL preparation receipt summary

Recorded November 10, 2026 from Bryce's pasted WSL-agent receipt. This file is a
summary, not the original receipt or an independent verification of its bytes.
The netcup conductor cannot read the WSL-local evidence paths directly.

**Disposition:** accept completed preparation evidence; overall live readiness
remains partial. Actual registration handshake/authenticated protocol recovery
was not exercised. SQL-seeded host binding survival and advertised capability do
not substitute for that check. [Bounded follow-up](../briefs/3734-wsl-registration-addendum.md)
uses existing owner/host clients; no product implementation is requested.

## Reported provenance

- WSL evidence: `/tmp/buzz-hosted-prep-929c-w39MsZNb`, mode0700.
- Original `sanitized-receipt.txt` SHA256:
  `5900307f2796b37a72288f402e19b601d3cce9e9216157ad5d451532167f4ff0`.
- Manifest covering53files:
  `f582512ab74b284c7bf0211dc03ab294078daa554f175f3c27addf19470e018e`.
- Source `929c2cd890fbc2c0993549c3cbc64a2714232832`, tree
  `1f1d902c2a80e528aaf639449fc1a114ea1129ad`.
- Candidate linux/amd64 image:
  `sha256:5bc530077c757a8b058c08ef6db9e17eba8f0551118d88d17388db972fceaa0f`.
- Candidate gzip archive98,349,855bytes, SHA256:
  `dd7a82ce65b83db6fb92f9dd573cd6b0ac57c41b54f1cd5d2ccb23b2ae9abd39`.
- Exact old linux/amd64 image:
  `sha256:1da22b2856a998de760a12b0767b30f0daf30e65b580ff193054fa17a4c85044`.
  Source f316fb46 association comes from November7 receipt, not an image label.
- Unchanged Dockerfile/runtime build with locked/frozen dependencies, truthful
  source metadata, Docker29.6.1,6GiB/2CPU. Hermit bootstrap unavailable; Docker
  build succeeded. Missing host Hermit is not a new product requirement.

## Credited results and limits

One synthetic database persisted through exact-old/81, exact-old/82 after the
candidate admin migration, candidate off/on/off, and exact-old rollback/82.
Signed chat writes/authenticated accumulated history passed at every stage.
Memberships and SQL-seeded host binding survived; registration-v2 capability
remained. Deployment-v1 capability appeared only for candidate-on.

All original81 ledger checksums matched source. Migration82 matched; catalog
comparison covered31 orderedcolumns,3primarykeys,5valid/readyindexes. No restore
or drop occurred between transitions. Fixture egress denial was measured, no
published ports, cleanup found no owned containers/errors. Local stage duration
2.026–2.687seconds includes probes and is not a live outage estimate.

Final read-only live check at19:19UTC matched hostname, primary image/container,
seven service IDs/images/runningstates, labels, five-file Compose order, flags
and ledger. Initial capture lacked start-time/config-hash fields; equality of
those fields is not claimed. Live schema81 and no deployment tables persisted.
Registry=true; deployment unset/absent; auto-migrate=false; v1=false; NIP-FI
unset/defaultoff; auth-token=true; relay-membership=true. Public relay identity
unchanged. No live changes or netcup/provider access occurred.

WSL operator-plan contains candidate-off/on, exact-old recovery and nonexecuted
commands. Clean-environment Compose comparator/execution restrict differences
to primary relay image/four flags; full five-file prefix retained. Candidate
admin migrates once, primary off then on, no dependency recreation. Normal
rollback retains schema82/current journals, first candidate-off, then exact old
only as temporary degraded recovery (deployment unavailable, community deletion
fails closed, auto-migrate=false). Flag-off is not cancellation of admitted host
work. Recovery checkpoint and live drift gates are future approved operations.

Separate WSL Astra/high review reported zero remaining preparation findings;
`astra-review-final.json` SHA256
`405b96c19538bce521377072c545b919ea3fcf4c0c358e317092a7ddba1013f6`.
It accepted bounded partial preparation, not deployment readiness. Native Windows
human observation, real host/runtime/provider behavior and live rollout remain
unrun. Completed source/Windows automated reviews remain credited separately.

## Combined status

Netcup P1 remains independently accepted at19b60eee with manifest
`ec1fb606e835ba238a39021a3ad17d90e1f8c3ea933b568f2fa5db78e5671441`.
No rebuild or implementation is needed from this receipt. #3734 remains open,
awaiting the narrow registration addendum, then a concrete combined live approval
request. Existing Claude/Grok/Scout and real netcup registration stay untouched.
