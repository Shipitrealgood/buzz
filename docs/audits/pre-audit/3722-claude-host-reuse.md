# #3722 Claude host reuse — bounded reconnaissance

2026-11-10. **Verdict: shared preparation/model machinery is reusable; hosted
Claude authentication and automatic host discovery are not implemented.**
The same Windows Create flow can prepare a new Claude identity **if** hosted
creation is enabled and the host advertises a valid root-approved Claude ACP
installation. It cannot finish Claude setup today. An ordinary installed `claude`
on host PATH does not appear automatically. No production change is proposed for
execution until the conductor/user review this direction.

Input/source pin `a59b3db1dd4e6c7977a915e9e9e127925eb83e48`, worktree
`/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`. Requested Astra/high; actual served model,
reasoning and installed native Claude version are **unknown/not measured**.
Read root/package instructions, supplied checkpoint artifacts and current mission.
Used audit, big-brain and brief; scope is the pipeline below, not a new platform
or broad code-health audit. Counts **C0/H2/M1/L0** describe gaps against the desired
main-reuse behavior, not regressions in accepted checkpoint-2/3 source.

## What works, and where it stops

All source citations describe the input pin; test-only extensions are listed below.
“Read” is control-flow evidence, “measured” is this assignment's synthetic test,
and “inferred” is feasibility rather than native acceptance.

| Stage | Classification / Claude today | Evidence |
| --- | --- | --- |
| Host installed inventory | **Generic approval, not automatic discovery.** Operator must supply an installed ID, `harness_id: "claude"`, pinned `buzz-acp` and Claude ACP adapter, approved args, `credential_profile: "none"`. Native `claude` alone is insufficient. | Read: [Runtime/Catalog](../../../crates/buzz-hostd/src/lib.rs#L63), [executable ownership/hash](../../../crates/buzz-hostd/src/lib.rs#L24), [Status filtering](../../../crates/buzz-hostd/src/deployment.rs#L394). Catalog is `/etc/buzz-hostd/catalog.json`; no PATH scan here. |
| Capability → harness ID | **Generic.** Status advertises installed IDs plus explicit canonical harness mappings. ID suffixes have no meaning; registration alone certifies none of this. | Read: [status mapping](../../../crates/buzz-hostd/src/deployment.rs#L408), [hosted projection](../../../desktop/src-tauri/src/commands/agents_hosted_commands.rs#L141). |
| Windows metadata / Add Agent | **Main reuse.** Claude builtin maps to `claude-agent-acp` (legacy `claude-code-acp` alias), empty args, separate underlying `claude` CLI. Grok is a preset with `agent --always-approve stdio`. Hosted projection uses those same definitions, without local paths/auth probes. | Read: [Claude definition](../../../desktop/src-tauri/src/managed_agents/discovery/catalog.rs#L50), [Grok preset](../../../desktop/src-tauri/src/managed_agents/discovery/presets.rs#L175), [pure projection](../../../desktop/src-tauri/src/managed_agents/discovery.rs#L1083), [hosted form gating](../../../desktop/src/features/agents/ui/AgentDefinitionDialog.tsx#L152). Canonical Rust Claude projection was read, not newly native-tested. |
| Main creation | **Generic with bounded config.** Hosted entry calls shared main phases, generates/saves one identity, and prepares PendingLogin; unknown mapping, custom env/provider/command overrides refuse. Conditional Claude preparation is supported by source, not an actual Windows Claude journey. | Read: [host prepare contract](../../../desktop/src-tauri/src/commands/agents_hosted.rs#L37), [shared phases](../../../desktop/src-tauri/src/commands/agents.rs#L426), [definition projection](../../../desktop/src/features/agents/lib/prepareHostedDefinition.ts#L49). Accepted Windows evidence covers Grok; no Claude native case claimed. |
| Host prepare | **Generic.** Same unit/private HOME/catalog context; PendingLogin refuses conversation start and runs finite models discovery. Catalog approval does not prove Node/CLI/library/CA/tool closure. | Read: [preparation](../../../crates/buzz-hostd/src/activation.rs#L318), [pending start refusal](../../../crates/buzz-hostd/src/activation.rs#L338), [unit mounts](../../../crates/buzz-hostd/deploy/buzz-paired-agent@.service#L10). |
| Native login | **Grok-specific; exact Claude blocker.** `Mode::Login` requires `harness_id == grok`, then invokes the adapter as `login --device-auth`. Claude is refused before spawn. | Read + measured synthetic negative: [login guard](../../../crates/buzz-hostd/src/runtime_context.rs#L205), [fixture](../../../crates/buzz-hostd/src/service_actions_tests.rs#L404). No Claude subscription login ran. |
| `buzz-acp models` | **Generic ACP.** Same fixed command and private context can initialize/session-new either synthetic adapter, with no prompt/MCP/relay. Current Disabled policy turns valid/empty catalogs into NetworkDisabled. | Read + measured: [host models](../../../crates/buzz-hostd/src/runtime_context.rs#L214), [normalization/policy](../../../crates/buzz-hostd/src/runtime_context.rs#L301), [main models command](../../../crates/buzz-acp/src/lib.rs#L6317), [Claude normalization tests](../../../crates/buzz-core/src/agent_models.rs#L148). Synthetic labels do not certify a real Claude adapter. |
| Model picker / conversation | **Generic transport, blocked readiness.** UI requires current matching observation, Succeeded login and Available models. Runtime first rejects Disabled networking, then requires its native-login receipt, fresh selected ID and strict ACP session selection. | Read + measured UI predicate: [picker gate](../../../desktop/src/shared/api/hostedAgents.ts#L118), [runtime order](../../../crates/buzz-hostd/src/runtime_context.rs#L376). Normal networking alone cannot close Claude's missing login receipt. |

**Exact stop:** with a valid approved runtime and native feature enabled, Create
can save/prep the Claude identity and return setup/model-unavailable state. The
first provider-specific action, root-local login, rejects Claude. Independently,
all conversations are intentionally offline-disabled. These are source conclusions;
this audit did not install a catalog, activate the feature, or exercise systemd.

## Routing-changing findings

### H1 — hostd owns a second, incomplete login policy

**High confidence; architecture/dependency; medium-to-large effort pending scope.**
[Host login:205](../../../crates/buzz-hostd/src/runtime_context.rs#L205) replaces
main's auth choice with a Grok check. Main already queries adapter `authMethods`,
selects a currently advertised method and distinguishes terminal/native login from
ACP `authenticate` ([agent_auth.rs:70](../../../desktop/src-tauri/src/commands/agent_auth.rs#L70)).
Claude subscription methods take a native CLI path; main handles both `command`
and `_meta.terminal-auth`, including its existing Claude compatibility rule
([argv:262](../../../desktop/src-tauri/src/commands/agent_auth.rs#L262)). An existing
[test:633](../../../desktop/src-tauri/src/commands/agent_auth.rs#L633) explicitly
routes around unimplemented Claude ACP authenticate. Simply allowing Claude or
calling `buzz-acp authenticate` is not an evidenced fix.

**Route:** reuse that main-owned auth interpretation in a portable seam, with
hostd supplying approved executable resolution, private context and supervision.
Do not add `claude`/`codex` cases to hostd. Main itself has provider-specific
knowledge; keep one owner for it. Its current Tauri/PATH/browser wrappers cannot
be lifted wholesale into the isolated unit. This extraction/context seam is the
prerequisite to a credible multi-harness host login implementation.

### H2 — login history has become mandatory readiness authority

**High confidence; architecture; effort unknown until readiness contract chosen.**
[Runtime:378](../../../crates/buzz-hostd/src/runtime_context.rs#L378) and the
[UI:131](../../../desktop/src/shared/api/hostedAgents.ts#L131) require a successful
host-controlled login event. Main's Claude readiness instead probes `claude auth
status`; Codex has its own probe in the existing main path
([readiness:407](../../../desktop/src-tauri/src/managed_agents/readiness.rs#L407),
[CLI probe caller:43](../../../desktop/src-tauri/src/managed_agents/readiness/cli_login.rs#L43)).
An authenticated private context is not sufficient if it never ran hostd's Grok
login. Conversely, historical login success is not current entitlement evidence.

**Route:** retain login outcome as history; have the shared main readiness/auth
mechanism evaluate the same private context. Do not erase the guard or convert
Unknown/model metadata to Ready. A precise readiness representation must be agreed
before changing UI/host predicates: the current observation has login history and
models, not a separate shared-readiness result. Do not relabel probe success as
“our login command succeeded” to avoid that decision. Native refresh/entitlement
and chat remain separately observed pilot facts.

### M1 — approved inventory does not meet automatic discovery expectations

**High confidence; architecture/UX; small mapping reuse, broader automation unestimated.**
Main local discovery resolves its compiled builtins/presets on the machine, checks
underlying CLIs, and probes authentication
([discovery:921](../../../desktop/src-tauri/src/managed_agents/discovery.rs#L921),
[:1107](../../../desktop/src-tauri/src/managed_agents/discovery.rs#L1107)). Host
Status only enumerates catalog-approved verified paths. `harness_id` is also an
operator-authored mapping. Unknown/custom main registries are not included in the
hosted pure builtin/preset projection.

**Route:** reuse main's resolver/catalog facts for candidate discovery in an
approved runtime search scope; retain root approval/pinning as a separate launch
boundary. Automatic candidate detection must not silently approve ambient PATH
programs or operator homes. Today a root catalog entry is required; calling this
“automatic hosted discovery” would be false. No general installer is prerequisite
to proving this distinction.

## Main reuse: what to keep, adapt and remove

Measured comparison uses **local** `origin/main=e3a0f3d59dfd08696cff80badb2d80d9a5096a61`,
not latest remote main; no fetch ran. `git diff origin/main --` is empty for
`commands/agent_auth.rs` and `discovery/catalog.rs`. Preset changes factor static
metadata plus its Grok test. `runtime.rs` differs only in hosted summary handling;
`buzz-acp/src/lib.rs` differs only by six strict-model field additions. Auth-method
and models commands already belong to main. This supports reuse, not a claim
that the entire fork or installed native runtime matches main.

| Decision | Direction for discussion, not execution authorization |
| --- | --- |
| Keep | Shared Windows form/creation/metadata/model picker; main ACP models/session/run; host owner authorization, root-approved installation, per-agent identity/private HOME/credentials, finite supervisor, existing retry/cleanup. |
| Adapt once | Make main's auth-method interpretation and readiness facts consumable with an explicit execution context and resolver. Reuse existing `buzz-acp auth-methods`/`authenticate` commands where their protocol applies. Share builtin/preset facts if host candidate discovery needs them; preserve a single definition owner. |
| Remove after replacement is tested | Hostd's Grok-only command selection and its use of login-action history as universal readiness. `GROK_HOME` at [context:144](../../../crates/buzz-hostd/src/runtime_context.rs#L144) is another provider fact to move behind shared configuration if retained; it does not itself prevent Claude using HOME. |
| Preserve boundary | Never execute arbitrary adapter-advertised argv as root or accept it as a new root-helper verb. Resolve/authenticate within the approved unprivileged agent context; independently approve the CLI/adapter runtime closure. |

Three approaches considered: (1) add hostd per-provider branches — cheap initially,
repeats this failure for every harness, rejected by user steering; (2) move all
Desktop orchestration to the host — imports Tauri/UI/global machine state and
exceeds this pilot; (3) **recommended: share only main's portable discovery/auth/
readiness decisions, keep hostd as context/approval/process boundary**. This trades
a bounded extraction for one semantic owner. It is not a promise that main already
exports a headless multi-provider setup service.

**Remaining product judgment:** approve a finite shared-main seam next, with root
catalog approval retained, or retain the current explicit Grok-only pilot while
separately scoping multi-harness onboarding. Recommendation/default for discussion
is the shared-main seam; implementation stays stopped, per latest user steering.
No hostd provider matrix, plugin framework, remote auth broker/UI or new transport
is justified by this audit. Exact extraction package/API and readiness wire meaning
need the conductor's review; no measured production LOC estimate exists. The
existing auth DTO/terminal interpretation is roughly 120–150 lines to relocate/
parameterize (inferred from the cited ranges), not a total implementation estimate.

## Native authentication feasibility limit

Current official [Claude CLI reference](https://code.claude.com/docs/en/cli-reference)
documents `claude auth login` and a status command; Console/API billing is distinct
from Claude subscription login. [Authentication guidance](https://code.claude.com/docs/en/authentication)
describes browser login and a possible code-paste fallback. Main's local Claude
wrapper discards stdin/output and waits for exit
([agent_auth:223](../../../desktop/src-tauri/src/commands/agent_auth.rs#L223));
the host supervisor likewise has null stdin and an output-only operator frontend
([context:156](../../../crates/buzz-hostd/src/runtime_context.rs#L156),
[frontend:60](../../../crates/buzz-hostd/src/bin/buzz-host-activate.rs#L60)).
**Inferred:** remote browser callback or interactive code input may need an
operator-local channel the present unit does not offer. This is not a measured
failure of an installed version. If it requires a substantial interactive broker,
stop and scope that boundary; do not write it into this pilot or substitute API keys.
No installed CLI/help/auth command, provider call, credential/home read or runtime
installation ran. Model metadata is never authentication proof.

## Verification and release

Production source stayed at the input pin. Small test-only extensions cover two
harness mappings in the existing finite-action fixture and shared UI model gate.
The ACP fixture validates catalog shapes, initializes/session-news a synthetic
peer six times (normal/empty/error × Grok/Claude), asserts no prompt, and proves
Claude login refusal; it does not verify root-owned executable hashes/systemd.

| Command / gate | Measured result |
| --- | --- |
| Hermit `HERMIT_SELF_UPDATE=false`; `cargo test --offline -p buzz-hostd --lib --no-run`; `cargo build --offline -p buzz-acp --bin buzz-acp` | PASS, current source + test delta built |
| `BUZZ_TEST_ACP=$PWD/target/debug/buzz-acp target/debug/deps/buzz_hostd-0d871f539b36364e actual_acp_models --ignored --test-threads=1` | PASS, **1** outer case / six synthetic model invocations, 0.20s; first sandbox attempt failed on temporary Unix socket EPERM, identical authorized rerun passed |
| `cargo test --offline -p buzz-core --lib agent_models -- --test-threads=1` | PASS, **2** Claude model normalization cases |
| From `desktop`: `node --import ./test-loader.mjs --experimental-strip-types --test --test-isolation=none --test-reporter=tap src/features/agents/ui/whereToRunIntent.test.mjs src/features/agents/lib/instanceInputForDefinition.test.mjs` | PASS, **33** named cases, including both runtime IDs and missing-login refusal. Initial isolated runner reported only two file passes; these are not added to the count. |
| Formatting, differential file sizes, docs links and `git diff --check` | Final results recorded in the forward brief |

**36 distinct behavior cases passed.** Carry accepted [checkpoint-2 review](../post-audit/3721-host-service-discovery.md)
and [checkpoint-3/native canary](../post-audit/2722-windows-native-canary.md) for
unchanged production; no new Windows native build/journey is claimed. NOT RUN:
real Claude/Grok adapter, subscription auth, readiness probe, credentials/refresh,
provider entitlement/chat, native mounts/network/service/reboot, full CI, human
acceptance. No relay/Docker resources were needed; test children exited and temp
fixture directories were dropped. No live/existing-agent/board/deployment changes.

Selected T normal networking is recorded in [the decision packet](../../specs/3719-host-online-readiness.md)
and its [brief](../../briefs/3719-host-online-readiness.md). Network/abstract-socket
denial was superseded, not passed. Offline source defaults remain unchanged.
Next owner: conductor reviews this reconnaissance and [forward brief](../../briefs/3722-claude-host-reuse.md),
then discusses direction with Bryce before any product implementation.
