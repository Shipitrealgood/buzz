# #2722 hosted Edit + explicit Start — execution receipt

Source implementation complete for independent review; native Windows compilation,
independent review, and human acceptance remain pending. This is not release acceptance.

## Scope and source

Workspace `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`. Ownership began clean at
`bd46993f30c9395d24b971e4520706a7f8f0dba7`; predecessor F2/F3 commits are preserved.
Requested role/settings: codecodex, Astra/high; actual hidden runtime identity and
reasoning configuration are unknown. No live agents, services, provider calls,
relay/hostd production changes, schema, push, release, CI dispatch or taskctl actions.

Active source: `/home/bryce/services/buzz-agents/docs/specs/2722-hosted-save-next-spawn.md`,
top execution authorization and superseding narrowing amendment. The human approved
continuation up to **13 production files / net +450** after the first +370 estimate
proved too small. Historical online Save validation, desired/applied UI and 18-file
proposals are superseded. The approved contract is:

- Edit uses the exact selected hosted instance and its host context. Canonical harness
  metadata stays distinct from the installed runtime ID. Legacy instances resolve one
  exact retained context or fail visibly; no local-machine fallback.
- Save performs ordinary definition and selected-record writes offline, retains an
  omitted installed runtime, preserves the draft on partial failure, and says saved
  changes take effect on next start. Save neither probes deployment nor interrupts.
- Explicit Start projects main's current effective settings and current identity,
  validates current host runtime/model facts, and uses the existing revisioned Deploy.
  Pending retries retain immutable bytes. Current revision, peer, cleanup and scope
  protections remain. The `existing_start` marker prevents recovery from overwriting
  later saved settings. Host active-body refusal and inactive higher-revision reuse
  remain existing behavior; existing `!shutdown` is separate from this change.

## Commits and measured production budget

`8c0603fe8bf57155c5e4250bd6ba2791c7ec4894` preserved the paused native WIP.
`c542979eb` completes native projection/recovery and focused owner/host/native fixtures.
`dcd9658aa` completes the editor, ordinary Save feedback and browser coverage.
All carry DCO sign-off. The WIP deliberately bypassed hooks to honor the stop;
completion commits used ordinary `git commit -s` (no installed pre-commit/commit-msg
hooks in this checkout). Documentation follows in a separate signed commit.

Measured with `git diff bd46993 --numstat`; production excludes existing test files,
AGENTS guidance and docs. Normal rustfmt/Biome formatting; no compression to fit.

| Production file | Added | Removed | Net |
| --- | ---: | ---: | ---: |
| `crates/buzz-paired-owner/src/deployment/continuation.rs` | 75 | 9 | 66 |
| `desktop/src-tauri/src/commands/agent_models_update.rs` | 30 | 1 | 29 |
| `desktop/src-tauri/src/commands/agents.rs` | 10 | 4 | 6 |
| `desktop/src-tauri/src/commands/agents_hosted.rs` | 224 | 30 | 194 |
| `desktop/src-tauri/src/commands/agents_hosted_cleanup.rs` | 2 | 8 | -6 |
| `desktop/src-tauri/src/managed_agents/types.rs` | 3 | 0 | 3 |
| `desktop/src-tauri/src/managed_agents/types/requests.rs` | 3 | 0 | 3 |
| `desktop/src/features/agents/ui/AgentDefinitionDialog.tsx` | 12 | 4 | 8 |
| `desktop/src/features/profile/ui/UserProfilePanel.tsx` | 10 | 2 | 8 |
| `desktop/src/features/profile/ui/UserProfilePanelPersonaSubmit.ts` | 31 | 5 | 26 |
| `desktop/src/features/profile/ui/UserProfilePersonaDialogs.tsx` | 132 | 24 | 108 |
| `desktop/src/features/profile/ui/useAgentLifecycleActions.ts` | 5 | 3 | 2 |
| `desktop/src/shared/api/types.ts` | 2 | 0 | 2 |
| **13 files** | **539** | **90** | **449** |

Approved scope-neutral substitution: `AgentDefinitionDialog.tsx` replaces planned
`AgentDialog.tsx`, since the shared definition editor required fresh models for Edit.
The unchanged saved-selection exception is restricted to hosted Edit; Create and
changed selections retain model evidence requirements. The existing profile wrapper
uses the existing run-location provider directly. Its next-start description is
rendered explicitly because the shared shell did not display its description prop.
The native adapter exceeded its initial estimate because it must compose existing
locking, session/catalog, retained-intent, current-record projection, model admission,
recovery and dispatch seams; no new state/wire/store mechanism was introduced.

## Verification actually run

Commands ran with `HERMIT_SELF_UPDATE=false` and `. ./bin/activate-hermit`.
Local socket fixtures needed sandbox escalation; no external endpoint was used.

| Check | Measured result and scope |
| --- | --- |
| `cargo test -p buzz-paired-owner --lib` | **17 passed**, 0 failed; includes expanded existing revision/exact-retry journey |
| `cargo test -p buzz-paired-owner --lib deployment_current_revision_refresh_launch_and_exact_retry -- --nocapture` | Restored source **1 passed**, 16 filtered, 12.44s |
| `cargo test -p buzz-hostd --lib prepare_replay_restart_and_identity_fences_precede_effects -- --nocapture` | **1 passed**, 31 filtered; synthetic manager verifies same unit/key, active refusal, inactive higher revision |
| `cargo test -p buzz-hostd --lib service_actions::tests:: -- --nocapture` | **7 passed, 1 existing ignored**, 24 filtered; ignored actual ACP case requires `BUZZ_TEST_ACP` |
| `pnpm typecheck` in desktop | **PASS** |
| `pnpm build:e2e` in desktop | **PASS**, including final restored build (`tsc` + Vite); ordinary chunk warnings |
| `pnpm exec playwright test --project=smoke hosted-agent-setup.spec.ts` | **18 passed**, 1.5m; actual UI, mocked IPC boundary |
| Same Playwright command with `-g 'profile hosted Edit'` | Final strengthened and restored case **1 passed**, 13.4s; offline Save, exact runtime, partial failure/draft, Start wording/identity |
| Biome check on six changed frontend production files plus hosted browser spec | **PASS**, 7 files, no fixes |
| `rustfmt --check --edition 2021 --config skip_children=true` on all eleven touched Rust production/test files | **PASS** |
| `git diff --check` | **PASS** |

The full browser suite ran before final test-only strengthening (closing/reopening
profile to discard local model cache and asserting Start). The final case then passed
separately with those assertions, and passed again after mutation restoration. Other
17 cases and production source were unchanged by that strengthening.

Falsification: temporarily bypassing owner `evidence.current` admission made the
existing journey fail at the `advanced` assertion; restored bytes passed. Temporarily
disabling the hosted unchanged-selection exception made the final profile journey
fail at the offline Save enabled assertion (`hosted-agent-setup.spec.ts:1118`). Both
mutations were restored byte-for-byte; restored browser bundle and case passed.
Raw local logs are `/tmp/2722-edit-{owner-tests,owner-mutation,owner-restored,host-test,service-guards,browser-suite,profile-final,browser-mutation,restored-build,restored-browser}.log`;
this committed receipt is the durable sanitized evidence index.

Historical failures: paused WIP owner test attempted `Command.clone()` and did not
compile; replaced with existing serialization/restore and the compiled suite passed.
Initial browser iteration exposed a Create-name reset from an added effect dependency;
the dependency was removed with a narrow lint explanation, and full Create cases passed.
Sandbox loopback denial was environmental; allowed local reruns passed.

## Not run and next gate

Desktop native Linux/Windows compile/tests were **not run** for this implementation.
The known Linux GLib/WebKit dependencies are unavailable; no environment installation
was attempted. The existing Windows-filtered
`hosted_native_create_recovery_and_session_fence` fixture now covers latest definition
projection, exact installed runtime, mapping/inventory rejection, immutable retained
Start recovery, later Save preservation, identity/auth/missing-record refusal and
session fencing. Its cleanup sibling remains in the existing workflow. Source
inspection covered RegisteredHost constructors/callers; this is not compilation proof.
The fixture exercises candidate/recovery seams, not the entire live Start transport.
Browser coverage cannot prove native persistence or network delivery.

`just ci`, native platform gates, real provider/service runs, human acceptance and
independent review are **not run**. The next gate is fresh independent Astra Chuck
review of the conductor's combined `f95d075..HEAD` range, then the existing Windows
workflow as authorized by the conductor. See [review brief](../briefs/2722-hosted-edit-start-review.md).
No product decision remains open within this bounded contract. Final process scan found no task-owned test/build/browser process and `/proc/net/tcp*`
contained no port 4173 listener. All tool sessions were terminal before the signed
documentation commit; clean source is checked again in the final handoff.
