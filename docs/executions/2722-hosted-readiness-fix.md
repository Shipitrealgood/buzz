# #2722 hosted readiness fix — execution and review brief

2026-11-11 UTC. Executor `/root/hosted_readiness_fix`; requested Astra/high;
runtime model/effort not independently exposed. Next agent: independent Chuck.

## 1. Where the work lives

Worktree `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`. Clean starting HEAD (measured):
`706cedb40a4e8010ba1c3d27610649a0f6966817`. Review that base through the signed-off
commit containing this brief; exact final hash is in the executor return.

```sh
cd /home/bryce/services/buzz-wt-3720-host-registration
git diff 706cedb40a4e8010ba1c3d27610649a0f6966817..HEAD --stat
```

Root owns board/roadmap; no unrelated baseline edits. Scoped files: selector,
its existing jsdom fixture, existing hosted Playwright fixture, agent guidance,
and this combined execution/review brief. No changes to Rust or native authority.

## 2. What landed

`WhereToRunSection.tsx:81` requires an exact current owner/community paired row
and an available, supported, error-free directory. It stops requiring the
independent registration challenge's `fresh` bit. Current paired rows say
Registered; retained rows from an unavailable directory say Saved registration ·
unavailable. Runtime/name validation, hostedReady conjunction, expiry timer and
stale-response cancellation remain unchanged.

The conductor's bounded dispatch is the accepted repair specification; the
[hosted-flow spec](../specs/2722-windows-hosted-flow.md) and
[prior catalog review](../audits/post-audit/2722-hosted-catalog-session-fix.md)
remain upstream inputs. The latter records test25 native/ACL evidence. Native
source is identical to baseline (measured diff); those results are continuity
input, not newly run native evidence.

## 3. Your job

Independently review this bounded repair and its production-bound fixtures.
Challenge whether directory registration eligibility remains distinct from the
verified hosted catalog and runtime authority. The executor's self-review found
no additional scoped defect; it is not the independent merge gate.

## 4. Specifically look for

1. **Does registration remain exact and fail closed?** Probe
   `WhereToRunSection.tsx:73–96`: existing session filter and destination key
   select the row; supported/available/error checks must all remain. The jsdom
   cases at `WhereToRunSection.jsdom-test.mjs:99,187,218` include paired false/true
   freshness, pending, expired, revoked, missing, wrong owner/community and a
   thrown refetch retaining the previously successful row.
2. **Does registration alone remain insufficient?** Probe unchanged
   `whereToRunIntent.ts:65` and `AgentDialog.tsx:251,346,357`: hostAvailable AND
   hostedReady, with token plus current exact-scope catalog. The existing 14-case
   intent fixture covers the truth table, exact expiry, capability and scope.
3. **Does the actual form enable Prepare for the reported case?** Probe
   `tests/e2e/hosted-agent-setup.spec.ts:140,365`: native-shaped `fresh:false`,
   Fizz, available native/grok, token and verified-catalog-shaped IPC response.
   The real dialog becomes enabled, empty name blocks, restored name enables,
   then the existing timer disables without interaction or a catalog refetch.
4. **Do negative hosted inputs still block?** Probe the existing-fixture table
   at `hosted-agent-setup.spec.ts:407`: pending token, missing catalog,
   wrong-generation catalog, stale catalog and explicitly selected unavailable
   runtime. Wrong owner/community catalog and exact expiry boundaries remain
   covered by the production predicate's existing unit cases. Native verification
   is mocked at the IPC boundary here; these tests do not prove signatures.
5. **Does the native boundary remain unchanged?** Read
   `agents_hosted.rs:165` (guarded nonce status query and opened signed status),
   `agents.rs:435` (inventory check before create), and the prior review above.
   `AgentDialog.tsx:223–252` retains cancellation and one-shot expiry. No timer,
   Device cache, native freshness semantic, or runtime metadata mapping changed.
6. **Is the wording honest and narrowly scoped?** Probe selector lines 85,105,214.
   Paired-host management's separate registration-challenge wording remains in
   its existing helper; the Add Agent selector no longer inherits its false
   deployment blocker. No Online claim or synthesized fresh=true is introduced.

## 5. Decisions / deviations

No product-contract deviation. The conductor explicitly added ownership of the
existing Playwright file and authorized its local frontend build. No new test
framework or dialog mocking layer was introduced. Combined brief stays at the
requested execution path. Unavailable runtimes are not auto-selected by the real
form; the fixture now explicitly selects that choice before asserting blocked.

## 6. What NOT to audit / chase

This increment is frontend readiness only. Goose config_supported=false,
provider matrices, host PATH configuration, native clock/retry reliability,
registration challenge lifetime, frozen runbooks and Windows distribution are
separate work. No provider execution, service or protected journal/key access,
push, board mutation, deployment or live acceptance occurred here.

## 7. Verification and handoff

All commands below run in `desktop/`, except the final git diff check. Hermit
activation was attempted but its metadata cache write was sandbox-denied. Tests
used the already-installed pinned Node 24.15.0 and Biome 2.4.7 directly:

```sh
export PATH=/home/bryce/.cache/hermit/pkg/node-24.15.0/bin:$PATH
node node_modules/typescript/bin/tsc --noEmit
node --import ./test-loader.mjs --experimental-strip-types --test-isolation=none --test src/features/agents/ui/whereToRunIntent.test.mjs
node --import ./test-jsdom-setup.mjs --import ./test-loader.mjs --experimental-strip-types --test-force-exit --test-isolation=none --test src/features/agents/ui/WhereToRunSection.jsdom-test.mjs
node --import ./test-jsdom-setup.mjs --import ./test-loader.mjs --experimental-strip-types --test-force-exit --test src/features/agents/ui/WhereToRunSection.jsdom-test.mjs src/features/agents/ui/PairedHostsSection.jsdom-test.mjs
/home/bryce/.cache/hermit/pkg/biome-2.4.7/biome check src/features/agents/ui/WhereToRunSection.tsx src/features/agents/ui/WhereToRunSection.jsdom-test.mjs tests/e2e/hosted-agent-setup.spec.ts
node node_modules/vite/bin/vite.js build --mode e2e
node node_modules/@playwright/test/cli.js test --project=smoke tests/e2e/hosted-agent-setup.spec.ts --workers=1 --reporter=list
```

Measured: TypeScript PASS (exit 0); intent tests 14/14; selector jsdom 5/5;
isolated selector + paired-host suites 2/2 files; scoped Biome PASS; E2E build
PASS (existing chunk-size warning). Browser local server needed approved socket
access outside the sandbox. Final Playwright PASS: 10/10, 43.6s, exit 0; all six readiness cases and
all four existing continuation/cleanup journeys. `git diff --check` PASS.

Failing-before receipt: modified selector fixture against unchanged production
failed 2/5: old deployment-unavailable wording and `false !== true` for
fresh:false/current registration. After repair, 5/5 passed. No mutation campaign.
One exploratory combined `--test-isolation=none` run collided through two global
Tauri bridges; normal per-file isolation passed, and individual selector run
provides case counts. Initial browser run was 9/10: unavailable runtime fixture
incorrectly expected automatic selection. An accidental rerun preceded the
fixture correction and repeated 9/10; the corrected case alone passed 1/1.
These were fixture/runner issues, not suppressed product failures.

NOT RUN: full `just ci`, native Rust/Windows tests or installer build/execution,
real hosted IPC/relay/provider/login/chat flow, human pilot acceptance. No PR
readiness attestation. The next gate is independent source review, then the
conductor's separately authorized Windows build/pilot. No named open product
question remains.

All test/build processes reached terminal exit; no source mutations or detached
processes remain held. Final owned changes are committed together; ownership
returns to the conductor/independent reviewer at that revision.
