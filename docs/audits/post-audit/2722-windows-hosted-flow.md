# #2722 checkpoint 3 — Windows hosted Add Agent post-audit

2026-11-10 UTC. **Verdict: rework; source acceptance held for M-1 and M-2.**
Counts: **C=0 / H=0 / M=2 / L=0**. First review, rework rounds **0**.
Windows/native, full CI and human acceptance remain unrun and independently
prevent a normal merge-ready attestation. They are not invented source findings.

## Scope, authority and evidence identity

Reviewed clean range
`0f7927f24bb79f9a3592dc80a5dbe56ed0f13717..1f0a8668b3fcf173e3c9ff44bdb46962275e49f7`
in `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`. Functional source ends at
`10e9ecd21e8f993ee0e9996b3ccb295a809b5466`; the final commit adds documentation,
evidence and API comments. Requested reviewer configuration: **gpt-6-astra/high**.
Actual served model/reasoning: **unknown**; no authoritative runtime metadata
is exposed. This is an independent reviewer, not the executor.

Read the portable conductor contract, root and feature AGENTS, VISION,
VISION_AGENT, VISION_REMOTE_AGENTS, TESTING, management provenance, complete
[ratified spec](../../specs/2722-windows-hosted-flow.md),
[dispatch brief](../../briefs/2722-windows-hosted-flow-review.md) and
[execution receipt](../../executions/2722-windows-hosted-flow.md).
Used post-audit and a proportional big-brain analysis for M-2 below. This report
also carries the bounded handoff; no additional artifact or board mutation.

Scope is F1–F4/V1–V5: ordinary Add Agent to an existing registered host, native
canonical metadata and protected continuation, shared form/model picker and saved
setup reachability. Accepted [destination](2722-hosted-destination-step1.md) and
[checkpoint-2](3721-host-service-discovery.md) reviews are inputs. Hostd, relay,
schema, core wire and ACP are unchanged; their architecture and broad suites were
not reopened. No live feature activation, service/provider/login/real pairing,
installation, other agents' homes/credentials, merge, push, PR or deployment.
Existing live registration is distinct from this native default-off creation path.

## Required findings

### M-1 (Medium) — changing the visible installed harness silently retries the old ID — fix in lane

**High confidence; independently reproduced in the real form against the final
build, plus source trace.** After a lost prepare response, the form becomes
editable. Select `second/grok` instead of `native/grok`, where both map to the
canonical Grok preset, then press Prepare. The second call still sends
`runtime: native/grok` and advances to the old setup without explaining the
mismatch.

[AgentDialog.tsx:386](../../../desktop/src/features/agents/ui/AgentDialog.tsx#L386)
updates `draft.runtime` only while the draft is unfrozen, but does not reject a
new `chosen.installedId` afterwards. The helper then replaces `input.runtime`
with the retained canonical harness ID **before** comparing frozen inputs
([prepareHostedDefinition.ts:38](../../../desktop/src/features/agents/lib/prepareHostedDefinition.ts#L38)).
Consequently distinct installed IDs compare equal; line 67 sends the original
draft runtime. This violates F1's distinct installed choice and F3/F4's frozen
submitted destination/runtime contract. It does not create a second native key;
the failure is silently acting on a different installed runtime than the form
shows.

Measured reproduction: copied the existing hosted browser fixture into an isolated
temporary test directory, reused its `open(page)` helper and IPC boundary, and
added only these steps:

1. Set `window.__HOSTED_SETUP_FIXTURE__.lose = true`; submit Prepare.
2. Wait for Prepare to become enabled; use the accessible Agent harness picker
   to choose `Grok Build · second/grok`; assert that visible selection.
3. Submit again, observe the setup view, and inspect the recorded prepare args.

The new assertion failed in **3.7s**, with actual runtime calls
`["native/grok","native/grok"]` instead of reflecting/rejecting the new visible
choice. All three existing hosted browser cases passed unmodified first. This
is a discriminating scenario, not a source mutation. The expected safe behavior
is **not** silently creating another identity or switching the existing token.

**Conductor disposition:** fix in lane. Preserve the retained token/definition;
freeze placement/runtime after any submitted attempt, including reply loss, or
explicitly reject exact installed-ID/scope conflict with a clear continuation
path. Canonical harness equality is insufficient. Add the deterministic scenario
to the existing real-form fixture. No new draft coordinator or cleanup workflow.

### M-2 (Medium) — an expired unadmitted follow-on permanently strands the saved setup — fix in lane

**High confidence; read-derived deterministic control-flow failure, not a claim
of executed Windows/native behavior.** The expiry recovery condition in
[continuation.rs:114](../../../crates/buzz-paired-owner/src/deployment/continuation.rs#L114)
only admits `expected_revision == 0`. Follow-ons have a nonzero expected revision
and a ten-minute deadline (same file, authority construction at line 190).

Concrete failure:

1. Initial PendingLogin is admitted and completes at placement revision 1.
2. Refresh creates and durably retains operation B with `expected_revision=1`.
   Profile/publication or relay admission fails; B never reaches the relay.
3. After 600 seconds, permissions/connectivity are restored and an authenticated
   exact operation read confirms B is absent. The original placement is still
   revision 1 with the same unit.
4. Check can report unadmitted. Retry rejects the expired, absent operation at
   [deployment.rs:426](../../../crates/buzz-paired-owner/src/deployment.rs#L426),
   saying an explicit new operation is required. Refresh rejects that new
   operation at `continuation.rs:116`, saying to retry the exact intent.
5. Reopening or supplying the older root cannot escape the cycle:
   [agents_hosted.rs:272](../../../desktop/src-tauri/src/commands/agents_hosted.rs#L272)
   always selects the greatest retained expected revision, including B.

This is verified absence, not an unresolved or admitted operation that should be
blocked. Existing identity and native context survive but **no existing setup
action can continue them**. The same defect applies to an unadmitted Selected
follow-on. The executed owner suite tests expired initial revision-zero intents
([deployment_tests.rs:643](../../../crates/buzz-paired-owner/src/deployment_tests.rs#L643));
it does not cover this nonzero case.

**Root cause and bounded design ruling:** the initial-create-only expiry rule was
carried into the follow-on continuation path. Three approaches were considered:

- **A, recommended and conductor-ratified:** use the existing exact authenticated
  reads of the prior admitted operation to establish its **current** nonzero
  placement revision/unit, and prove the expired attempted operation absent.
  Replace with fresh PendingLogin at that proven revision, retaining the same
  key, unit and protected snapshot. Reuse the existing verified-absent peer checks
  and transaction boundary. This costs bounded history reads and avoids new
  protocol or durable coordination.
- B: explicitly accept a stranded-setup limitation. This weakens F2's recovery
  contract and was declined by the conductor.
- C: add a new placement query/coordinator or reset identity/revision. This is
  unnecessary expansion or unsafe guessing, outside the dispatch.

**Conductor disposition:** fix in lane under existing F2 acceptance. Newer or
unknown placement, and an admitted operation with a lost reply, must remain
conflict/recovery; never reset revision zero. Add a positive nonzero expiry case
and negative newer-placement/admitted-lost-reply cases in the existing finite
owner fixture. No separate design stage, store, protocol or broad campaign.

## Coverage assessment and preserved strengths

| Contract | Independent read/test assessment |
| --- | --- |
| F1 metadata | [discovery.rs:1084](../../../desktop/src-tauri/src/managed_agents/discovery.rs#L1084) joins approved harness keys to shared builtin/preset constructors. Grok remains a preset; hosted projection performs no local executable/auth probe. Installed choices remain separate until the M-1 retry defect. Native pure metadata test is authored, unrun. |
| F2 journal and authority | [deployment.rs:113](../../../crates/buzz-paired-owner/src/deployment.rs#L113) uses an immediate transaction before insertion; conflicting same-token identities cannot reach main save/profile effects. Phase and result writes reload within their transaction. Exact signed event and current admitted revision are reconciled at [continuation.rs:220](../../../crates/buzz-paired-owner/src/deployment/continuation.rs#L220). Failed admission outcomes retain their admitted revision. M-2 is the remaining recovery gap. |
| Expired peers | [continuation.rs:156](../../../crates/buzz-paired-owner/src/deployment/continuation.rs#L156) queries each exact expired peer; [deployment.rs:133](../../../crates/buzz-paired-owner/src/deployment.rs#L133) revalidates scope/agent/revision and verified-absent IDs under the transaction. Local absence of receipt is not treated as remote absence. Executed lost-admission negative retains the admitted peer and refuses replacement. |
| Main/native boundary | [agents_hosted.rs:148](../../../desktop/src-tauri/src/commands/agents_hosted.rs#L148) gates before session/store/network work. Main key/attestation/save/profile phases remain reused. [recover:318](../../../desktop/src-tauri/src/commands/agents_hosted.rs#L318) rejects newer intents and allows only journaled follow-on model differences. [HostedSetup DTO:9](../../../desktop/src-tauri/src/commands/agents_hosted_commands.rs#L9) projects no main response key, auth tag, ciphertext or snapshot. Native compile/execution remains unproved. |
| F3 real shared form | Both [ordinary handler:214](../../../desktop/src/features/agents/ui/usePersonaActions.ts#L214) and [contextual handler:174](../../../desktop/src/features/agents/useAgentManagement.ts#L174) route hosted work before local availability. Synchronous latches precede definition/avatar effects. Source mode disables local discovery and custom/static fallback. Existing definitions, validation, configuration and model picker are reused. |
| F4 continuation | [setup actions:245](../../../desktop/src-tauri/src/commands/agents_hosted_commands.rs#L245) use current permission/evidence, while the local list grants no authority and performs no per-agent network reads. [currentHostedModels:107](../../../desktop/src/shared/api/hostedAgents.ts#L107) binds scope/agent/runtime/operation/revision/login/expiry; disappeared models remain visible but cannot launch. Browser cases exercise close/reopen, delayed old completion, revoked read, model loss/expiry, independent login/model states and truthful offline failure. |
| View and channel ownership | [App.tsx:642](../../../desktop/src/app/App.tsx#L642) keys the community subtree, providing the account/community unmount fence. Setup has its own unmount/action generation. [channel attachment:8](../../../desktop/src/features/agents/useCreatedAgentChannelAttachment.ts#L8) captures relay/signer, uses `ensureRunning:false` and retries the same identity. Runtime-tab continuation is actually reachable at [profile:536](../../../desktop/src/features/profile/ui/UserProfilePanelSections.tsx#L536); linked-definition Edit semantics and provenance remain intact. |

Five focused siblings are justified by owner continuation, sanitized native DTOs,
shared submission, shared setup and typed IPC responsibilities. No new store,
transport, general polling or coordinator is present. The revised production
scope estimate is ratified; file count is not itself a defect. Minimalism **9/10**,
elegance **9/10**, correctness **8/10** pending M-1/M-2 and their named repairs.

The existing 1,024-row cap and five-second per-query deadlines bound work but not
the aggregate action duration. Ambiguous retained peer sets can cause a long
finite action. For this default-off source checkpoint, the documented limitation
is acceptable; it grants no false authority and does not warrant a new framework.

## Independent verification at the final pin

All commands activated repository Hermit with `HERMIT_SELF_UPDATE=false`. Loopback
and Node child-process commands used approved sandbox escalation. No installation.

| Command | Measured outcome / boundary |
| --- | --- |
| `cargo test -p buzz-paired-owner deployment -- --test-threads=1` | **2 passed**, 0 failed, 4.96s; actual Device/Management, temporary protected SQLite and signed/encrypted synthetic WS peer. |
| From desktop: `node --import ./test-loader.mjs --experimental-strip-types --test src/features/agents/lib/instanceInputForDefinition.test.mjs src/features/agents/ui/whereToRunIntent.test.mjs src/features/agents/ui/usePersonaModelDiscovery.test.mjs` | **65 passed**, 0 failed, 0 skipped. |
| From desktop: `node --import ./test-jsdom-setup.mjs --import ./test-loader.mjs --experimental-strip-types --test-force-exit --test src/features/agents/ui/WhereToRunSection.jsdom-test.mjs` | **3 passed**, 0 failed; existing real destination selector and late owner/community query case. |
| `pnpm --dir desktop typecheck`; `pnpm --dir desktop build:e2e` | **PASS**; fixture-mode Vite build with existing dynamic-import/chunk-size warnings. Not a native build. |
| `pnpm --dir desktop exec playwright test --project=smoke hosted-agent-setup.spec.ts where-to-run-config.spec.ts` | **7 passed**, 30.4s: all 3 actual hosted form journeys and 4 provider-form regressions. |
| `pnpm --dir desktop exec playwright test --project=smoke onboarding-agent-defaults.spec.ts welcome-agent-modal-screenshots.spec.ts --grep 'defaults hides model when optional harness has empty discovery\|defaults keeps model control when optional harness discovery fails\|defaults renders only fields supported\|channel-targeted creation preserves'` | **4 passed**, 23.5s: 3 shared-model onboarding cases and ordinary local channel-create failure/retry. |
| `pnpm --dir desktop exec playwright test --project=smoke paired-hosts.spec.ts` | **8 passed**, 31.0s; registration/Add Host and forced-submit refusal. |
| Scoped `rustfmt --edition 2021 --check crates/buzz-paired-owner/src/deployment.rs`; `cargo fmt --manifest-path desktop/src-tauri/Cargo.toml -- --check`; Biome on the 22 changed TS/TSX/MJS files; `CHECK_FILE_SIZES_BASE=0f7927f24bb79f9a3592dc80a5dbe56ed0f13717 node desktop/scripts/check-file-sizes.mjs`; `git diff --check` | **PASS**, no fixes or ceiling changes. |
| Temporary M-1 browser scenario, described above | **1 failed as expected**, concretely exposing the old installed-ID replay. No source mutation. |

Failure receipts are preserved honestly: an initial JS command used the wrong
relative Hermit path and did not execute tests; corrected. Sandboxed Node runs
reported only 3/1 file-level results, so those were **not accepted as case proof**;
approved reruns produced 65/3 individual cases. The first owner run failed at
`deployment_tests.rs:352` with loopback `PermissionDenied` (one other test passed);
the approved rerun passed both. The M-1 failure is a product finding, not an
environment failure. No other independent behavioral invocation failed.

**89 individual tests passed independently**: owner 2 + JS 65 + jsdom 3 + browser 19.
The separate M-1 discriminating probe failed as documented.

The executor's 47-distinct-browser-case evidence is carried only as executor
evidence. Its intermediate profile failure exposed a real missing Runtime-tab
entry, then the five-line shared summary render corrected that seam. Final review
re-executed all hosted journeys including the profile entry and the affected
local/provider/model-selection regression selection. Unselected onboarding and
welcome cases have source continuity through that narrow profile render and
documentation-only final commit; no claim that all 47 reran here.

**NOT RUN:** Windows native tests/build (including
`hosted_native_create_recovery_and_session_fence` and
`hosted_grok_metadata_uses_static_preset_without_local_binary`), actual Tauri
command/native service acceptance, full `just ci`, human acceptance and live
provider/service/login journey. Independently inspected Rust 1.98.1 target-libdir:
`x86_64-pc-windows-gnu/lib` is absent. MinGW alone does not supply it; Linux Tauri
also would not compile Windows-gated commands. No native acceptance or
`buzz-review-completed` marker is supplied. N1/#3719 and native/Grok/#3722 retain
their settled later scope.

## Routing and workspace release

| Finding | Disposition | Next owner / finite closure |
| --- | --- | --- |
| M-1 | fix in lane | Same executor; exact submitted installed-ID/scope freeze or rejection and existing real-form regression; same reviewer re-gates. |
| M-2 | fix in lane | Same executor; prove current nonzero placement before absent expired replacement; positive and newer-placement/admitted-lost-reply negatives in existing owner suite; same reviewer re-gates. |

Only this post-audit artifact is committed by the reviewer. No implementation
or tracked fixture was edited. All owned test sessions exited; Playwright closed
its fixture servers and the owner fixture disposed its temporary journals/socket
peer. The temporary M-1 fixture is removed after recording its result. Final
tracked workspace status is clean after the report commit. Synthetic local
logs under `/tmp/2722-review-*` are ancillary; reproduction steps, outcomes,
failure details and source citations above are the durable evidence.

quality: model=unknown effort=unknown stage=post-audit verdict=rework verify=partial commits=0f7927f24bb79f9a3592dc80a5dbe56ed0f13717..1f0a8668b3fcf173e3c9ff44bdb46962275e49f7 findings=C=0/H=0/M=2/L=0 rework_rounds=0
