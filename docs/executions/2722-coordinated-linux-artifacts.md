# #2722 coordinated Linux artifacts — execution and conductor brief

**Matched Linux artifacts prepared and verified. Live activation remains held.**
No live update, installer execution, real provider/login or agent operation.
Requested configuration gpt-6-astra/high; runtime identity not independently exposed.

## Ownership and accepted inputs

Primary workspace `/home/bryce/services/buzz-wt-3720-host-registration`, clean
handoff HEAD `336b1d9ff42904a4b4cb2675472c47ad2c3ecc2d`. Conductor transferred
exclusive write/test/cache ownership after independent integration review and all
prior processes terminated. Read [independent integration acceptance](../audits/post-audit/2722-coordinated-delivery-preparation.md),
[accepted discovery source review](../audits/post-audit/2722-host-discovery-replacement.md),
and the prior credential-quoting execution/accepted live-application receipt.
Portable conductor, applicable AGENTS, architecture/testing/release and execute/
brief guidance read. No production/test edits; only this report is owned here.

Cohort source `d7714be862f4514822c303b124a347f5e07dc8c5`; protected secondary
checkout `/home/bryce/services/buzz-wt-2722-delivery-build` owns the Windows report.
Windows success recorded in local DCO commit `3e0210bbc2d6360809f659acba34f2fc71eddfd2`;
fork remains exact d771. Integration additions at actual Linux checkout are exactly
two relay test files and four documents, no production/dependency/schema/runner
input change. This continuity is measured, not relabeling the checkout.

Artifact root:
`/home/bryce/services/buzz-agents/releases/2722-coordinated-delivery/d7714be862f4514822c303b124a347f5e07dc8c5/`.

## Hostd-only candidate

Measured `cargo build --offline --locked -j2 --release -p buzz-hostd --bin buzz-hostd`
with Hermit/HERMIT_SELF_UPDATE=false at actual 336b: passed in 50.09s.
Candidate `linux-host/package/buzz-hostd`: 18,449,200 bytes, SHA-256
`f07a7f7a79b7ef02cd9c0ed237ba71a44ce203169a3b48fc5e7ce7f497c13a97`.
ELF64 x86-64 Linux, dynamically linked, interpreter /lib64/ld-linux-x86-64.so.2;
`--help` and `registration run --help` exited 0 without state/provider access.
Measured local rustc1.98.1 (48a229cea 2026-09-01), cargo1.98.1; no host dependency installation.

Measured frozen runtime hash
`98404e872a266d94457083bde657e179f3ab19fcd563771ac4e41f994c7bc16a`, ACP
`b7fe18f1604db51dfeffc8d392676d8755f872662be710b87e84c11fc1e83f33`, matching
last accepted 344baa package. Existing fixture command with those exact
BUZZ_TEST_RUNTIME/BUZZ_TEST_ACP paths:
`cargo test --offline --locked -j2 -p buzz-hostd --lib user_service::tests::actual_ -- --ignored --test-threads=1`
passed **2/0/0**, 1.31s. Existing prepare/refresh/selected conversation/cleanup and
new host-first discovery fixture both ran with synthetic provider/manager only.
Expected negative-child `managed runtime unavailable` output is part of refusal
assertions, not a failure. No new matrix, provider probe or runtime build.

Read: new pre-Create discovery runs inside candidate hostd. Runtime finite Models
code was extracted but existing run_user/launch/selection projection remains;
ACP and its normalizer inputs unchanged. Synthetic compatibility substantiates
preserving exact old runtime/ACP and recorded launcher paths. It is not real
native login/provider conversation evidence.

`linux-host/package/` contains binary0555, SOURCE.json, before/after controller,
OPERATOR.md and MANIFEST.sha256 (texts0444). Five-file manifest passed. Six Bash
operator blocks passed `bash -n`; none executed. `linux-host/controller.diff`
changes exactly one executable path in ExecStart; all args/environment preserved.
Last accepted deployment/template/runtime/ACP pins are future drift checks, not
new live measurements. No live files were read to manufacture expected contents.

## Journal rollback limitation accepted by conductor

Read: host journal/schema/user_version unchanged. New model replies are rows in
existing sent table. Candidate registration pending_generation expires/prunes
status+models; old 46f53f only handles status. Its flush_pending uses old wire
Header operation verification which rejects models. A pending models row can
therefore block old reconciliation even after expiry.

Concrete proposal: close Add Agent/setup, keep matching new relay while candidate
normally delivers/expires new models obligations, stop controller, then read-only
COUNT sent WHERE op=models AND pending=1 must equal zero before old restart.
Delivered rows can stay; preserve all journal/identity/credentials. If drain fails,
leave controller stopped for assessment; no row deletion, database restore,
re-enrollment, old-binary patch or rollback-success claim. This is source-derived,
not a rehearsed live rollback. Earlier quoting correction/Jeff override stays.

## Old-relay data-semantic rollback boundary

Read exact 929c read_registry and handler reply path: messages/evidence SQL has
no active-binding filter for retained models-query/models. Existing principal
checks still apply, but a non-recovery authorized owner/host with a dead binding
can reach those retained rows. Handler page_event passes p.data unchanged into
wire::reply, which validates the envelope/query/size, not inner event operations.
Therefore old929c can restore historical model-evidence reads after revocation.
New-write rejection and schema parity do not establish security-equivalent
rollback. No new test/matrix was requested or run.

The prepared packet explicitly retains the new relay for safe containment while
hostd/setup are held. Post-model old-image recovery is unproven and requires
separate conductor assessment, not history deletion/restore, flag changes or a
compatibility service. Conductor accepted source-derived containment as sufficient
for preparation; no product defect repair is introduced here.

## Relay build ledger

Conductor initially considered WSL, then selected one local build because the
user is remote without WSL access. No external build launched. Exact `git archive`
d771 is stdin context; dirty/report content excluded. Unchanged root Dockerfile,
linux/amd64, target runtime, compiled BUZZ_SOURCE_SHA=d771; truthful local build ID
and build URL unknown. No replacement Dockerfile/Caddy/image design or publication.

Task-owned docker-container builder `buzz-2722-delivery-d771` verified Memory and
MemorySwap 6,442,450,944 bytes; CpuPeriod100000/CpuQuota200000; effective OCI
max-parallelism1. Docker29.9.2/buildx0.38.1. Build wrapper session54670 owns command,
logs and five-second disk monitoring, with a 12GiB free floor. Initial free about50GiB.
No unrelated cache/prune. Normal public base/dependency downloads approved.
No host package installation. **Build passed first attempt**, 17:26:01–17:57:26UTC,
**1885.17s (31m25s)**, wrapper exit0. Standard dependency cook and both web bundles
passed, then final locked optimized relay/admin/pair-relay build and stripped
runtime assembly passed. No repeat build, source repair or alternative image.

Verified local image / OCI index
`sha256:3e7da8876903702f11898dc467d4603a39eae57249e13e335cacbcb175cbced3`,
local tag `buzz-2722-coordinated:d7714be8`, **373,643,470 bytes**. Linux/amd64,
user buzz:buzz, entrypoint /usr/local/bin/buzz-relay, existing web/admin paths.
Never-started network-none extraction container yielded the relay ELF and both
web index files. ELF64 x86-64, stripped, **67,361,240 bytes**, SHA-256
`a017d6e8552ce9e9b51c461f4601ea1a57ca2c6d023e98a12757204b523a09c1`.
Measured ELF .comment: rustc1.98.1 (48a229cea 2026-09-01), LLD22.1.8, GCC12.2.0.
Compiled d771 source and local build ID strings present, matching provenance
invocation; build URL is truthfully unknown. This is static artifact evidence,
not a runtime/readiness smoke or application connection.

Source tree `576d1f28511c7d958dbf0edfc473034e727aade5`;
exact source.tar SHA-256
`b5ef1abf2cfee7443eb0f4e83244350b5c6c519a82bea9c8423e590a3f196152`.
Root Dockerfile base resolutions (also in metadata/provenance):

| Base | Resolved SHA-256 |
|---|---|
| rust:1.98-bookworm | 93ce27a88655056a51dbdd8f5f2d7ddc071c7b0070fb288a37b5a285fc83971e |
| node:24-bookworm-slim | dc26ae92c9fb15cf6b1d63e20f0944b2efe0f115b20e00be48fb23fceb061c54 |
| debian:bookworm-slim | 734a72b3d6142a71993b4bc580ca01d8ca687f77a2c422d96a961d3376b179bc |

Archive `relay/buzz-relay-d7714be8-linux-amd64.tar.gz`: **98,378,289 bytes**,
SHA-256 **3705c0a5d79288ce72bb3d70419e00d2180d3eb7fdf72f7879358ef6491901c4**.
Archive index references the exact image, Docker manifest/config confirms platform
and tag, all referenced layer members exist and all SHA-256-addressed blobs match
contents. Not loaded on any live host, not registry-published.

Owned builder, its cache volume and never-started extraction container removed;
scoped container/volume listings empty. Image/archive preserved. Minimum monitored
free space **38.825GiB**; after cleanup **49GiB** free. No unrelated prune. All
build/export/extraction processes terminal. Metadata, base resolutions, source
archive, build command/log/terminal/resource receipts, ELF/toolchain, image inspect,
archive and verification.json remain under relay/.

Measured schema comparison: `git diff --quiet 929c2cd890fbc2c0993549c3cbc64a2714232832..d7714be862f4514822c303b124a347f5e07dc8c5 -- migrations schema`
exit0. Current Hetzner readiness remains unmeasured; no SSH/access changes.

## Not run and next gate

No full CI/strict lint rerun, broad test matrix, installer/native UI/human journey,
real provider/login/chat/systemd credential execution, rollback rehearsal, live
unit/config/journal reads/writes, existing-agent operation, deployment or registry
push. No taskctl, PR or merge. Exact old pins require fresh read-only preflight
before explicit user-approved activation. No new capacity policy/gate introduced.
Conductor independently reviews finite artifacts and exact update/rollback plan;
explicit live approval remains separate. Windows/native artifact hashes were
independently checked by conductor; source/integration acceptance remains separate
from any future human/provider acceptance. No merge/deployment readiness claim.

## Final handoff and process accounting

`cohort-manifest.sha256` indexes the complete frozen artifact/evidence root;
`linux-host/package/MANIFEST.sha256` is the five-file host packet manifest.
`relay/verification.json` binds image/source/ELF/archive and cleanup; Windows
verification/index remain at root. Report relative links and git diff whitespace
checked. Only this document is committed on primary; protected checkout has only
its separately committed Windows report. No pre-existing dirty files here; main
repository's unrelated untracked audits remain untouched.

All owned sessions terminal: host build55829, frozen-runtime fixtures74998,
source archive36232, builder bootstrap88619, relay monitor/build54670,
extraction/builder cleanup45396 and image export38303. Earlier Windows sessions
are terminal per its report. No owned build/test/container process remains.
No source/test mutation, signed live event, service call or real agent data access.

Recommended next gate: conductor checks manifest/image/host packet against these
pins and obtains fresh narrow read-only deployment readiness and drift evidence.
Then seek explicit coordinated activation approval with hostd pending-model drain
and old-relay historical-read rollback limits clearly disclosed. Keep the new
relay for containment after discovery traffic; do not infer old-image rollback
safety from schema parity. No additional product change/test framework requested.
Requested model remains gpt-6-astra/high; runtime identity unknown.
