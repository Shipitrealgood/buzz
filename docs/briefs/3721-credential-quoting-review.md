# Review scope: credential quoting repair (#3721)

2026-11-11. From codecodex `/root/hostd_credential_fix` to independent Chuck.
Requested Astra/high; actual served configuration not exposed.

## 1. Where work lives

Worktree `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events` (not main). Base09bbb43096fae862dee9b3f03e4cbcc9295c7d24;
implementation46f53f692a0d03779bd9e9168d6e1b986b32c1d1 plus documentation commit.
`git diff 09bbb43096fae862dee9b3f03e4cbcc9295c7d24..HEAD` is the entire review.
Clean baseline, no unrelated edits; exclusive source/test ownership released.

## 2. What landed

LoadCredential now uses literal paths with doubled percent specifiers; ExecStart
retains command-line escaping. Same-instance prepare accepts only exact old or
new generated override bytes; foreign edits still refuse. Existing fixture tests
cover path characters, legacy replay, identity retention and foreign rejection.
Hostd-only immutable candidate retains existing runtime/ACP/config paths.
[Receipt](../executions/3721-credential-quoting-fix.md) carries commands/hashes,
limits and links to exact frozen rollout/rollback OPERATOR.
Source contract: [trusted account S1](../specs/3722-main-discovery-native-login.md).

## 3. Your job

Independently post-audit source and candidate/runbook, especially whether the
repair preserves the narrow ownership guard and same-instance recovery.
**Special instruction:** offline systemd --test accepts even the broken form and
omits credential values. Its PASS is syntax only; do not promote it to native
interpretation/delivery proof. That gate is explicitly pending live approval.

## 4. Specifically look for

1. Serializer semantics — `user_service.rs:425` and `:553`.
   **Probe:** LoadCredential splits first colon, then expands specifiers; does
   literal source text plus %% correctly preserve accepted path characters?
   Check fixed filename suffixes and unchanged control/non-UTF8 refusal.
2. Exact legacy recognition — `user_service.rs:406`.
   **Probe:** compare the legacy rendering byte-for-byte against base09bbb;
   verify full override matching, never a prefix or arbitrary overwrite.
   Prepare's identity/location/inactive fences remain at `user_service.rs:224`.
3. Falsifiable production seam — `user_service_tests.rs:70` and `:139`.
   **Probe:** old-output/no-repair/foreign-accept mutations each failed; inspect
   exact test assertions rather than relying on reported counts. Same frozen
   identity and normal fake-manager no-start assertions must remain meaningful.
4. Native evidence boundary — `user_service_tests.rs:168`.
   **Probe:** private HOME/runtime/unit path and disabled generators; --test
   computes a transaction without starting services. The fixture proves no
   parsed-value roundtrip. Evaluate the explicit future a(ss) gate in OPERATOR.
5. Retained runtime — `runtime_context.rs:513` and binary entry unchanged.
   **Probe:** changed install/render methods are unreachable from runtime;
   unchanged dependency/build input diff and frozen-runtime synthetic test
   support retention. No new launcher path/catalog revision needed.
6. Rollout/rollback packet — immutable OPERATOR linked from receipt.
   **Probe:** exact before/after byte checks, quiescence before edits, native
   parse inspection before controller restart/provider activity, automatic
   reconciliation acknowledged. Old hostd rejects corrected overrides, so
   containment restores the old override and its known failure while retaining
   current journals/identity. Assess interrupted prefixes and foreign refusal.

## 5. Deviations

Native credential-value proof could not be obtained within bounded safe probes:
--test omits values; private manager attempts exited1. Conductor explicitly
bounded exploration and accepted return of this named unrun gate. No ABI harness
or new framework. OPERATOR proposes exact Jeff replacement while stopped to
inspect a(ss) before provider work; code's normal same-instance legacy repair
remains tested. Review these limitations independently.

## 6. What NOT to chase

Scope is these two source/test files, candidate provenance and exact runbook.
Registration authorization/journal, normal account trust, provider readiness,
existing Claude/Grok/Scout, relay/Windows builds and cleanup product design retain
their accepted contracts. No live operations or additional campaign authorized.
Existing unrelated clippy warnings are reported, not patched.

## 7. Deliverables and preflight

Return independent findings and disposition to conductor; executor evidence is
not a gate. Read receipt and OPERATOR before grading. Verify correct worktree/pin,
no unrelated dirty files, and candidate/original manifests; any new finding may
justify a narrow test. Finite tests already passed: hostd42, focused2, native
syntax1, retained-runtime synthetic1, three killed mutations, fmt, no-deps clippy,
offline locked release and help smoke. Full CI/live/native values not run.
No merge/push/deploy/board authority is granted by this brief.
