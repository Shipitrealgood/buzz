# #3722 — main discovery/readiness, native host login

**Status:** proposed smallest source plan; stop before coding. The complete native
journey is **not executable yet**: exact-service native login entry is unresolved.
**Date:** 2026-11-10. **Source:** `3169659626044819e17c7f62fe2d060b890c760e`;
`/home/bryce/services/buzz-wt-3720-host-registration`,
`proof/3720-host-registration-events`. Requested `gpt-6-astra` / `high`;
runtime model/reasoning not independently exposed to this worker.
**Sources:**
- Bryce's latest dispatch: reuse main discovery/readiness; native login on netcup;
  no custom login flow; smallest changes before coding.
- [Accepted audit](../audits/pre-audit/3722-claude-host-reuse.md), H1/H2/M1;
  its auth-orchestration extraction recommendation is superseded by this decision.
- [Selected T](3719-host-online-readiness.md), [handoff](../briefs/3722-claude-host-reuse.md),
  task #3722 amendments 774/780/800/801 (conductor-supplied export).
- [Product](../../VISION.md), [agent](../../VISION_AGENT.md),
  [remote agent](../../VISION_REMOTE_AGENTS.md), [testing](../../TESTING.md).
**Preflight:** ran, bounded to accepted decisions and integration seams. Optional
`~/personal/projects/buzz` is absent; a targeted personal idea-thread search found
no governing Buzz decision. Current user direction controls; no broad re-audit.

## Outcome and no-surprise scope

Use main's canonical harness facts and availability/readiness interpretation on
netcup, with netcup's explicit execution context. Keep the existing Add Agent,
desktop-generated identity, encrypted deployment, model picker and ACP launch.
Native provider sign-in belongs to the operator on netcup, outside Buzz.

| Keep | Change | Defer / excluded |
| --- | --- | --- |
| Owner/revocation checks, approved executable pins, narrow helper, private per-agent identity/HOME/credentials, bounded actions, journal/idempotence/cleanup | Share portable main discovery/readiness facts; report host detection separately from approval; replace login-history gate with current readiness; explicit normal-host-network opt-in | Auth orchestration extraction, broker/PTY/login UI, provider-specific hostd registry, Windows remote login, installer/adoption, HTTP retirement, main catch-up |
| Main model IDs, normalization and strict selected-model enforcement | Truthful separate readiness and models in existing private messages | Entitlement inferred from model listing, universal support inferred from a shared registry |
| Live Claude/Grok/Scout, pairing, installed controller and credentials | Source/doc changes only after a later implementation dispatch | Any live access, installation, provider call, service activation or credential copying in this assignment |

Main is not universal authentication: Grok preset reports `NotApplicable` and no
login hint; Claude has `claude auth status`; Codex has `codex login status`.
Read: [preset:87](../../desktop/src-tauri/src/managed_agents/discovery/presets.rs#L87),
[catalog:50](../../desktop/src-tauri/src/managed_agents/discovery/catalog.rs#L50),
[readiness:407](../../desktop/src-tauri/src/managed_agents/readiness.rs#L407).
“Main readiness permits an attempt” and “provider account authenticated / entitled”
are separate claims. Grok can permit an attempt without claiming the second.

## Smallest file-level changes and sequence

Paths below are proposed edits, not files changed by this planning assignment.
First resolve a concrete exact-context native-login approach by source/OS design,
without live action (see H-native below). Then implement S1 → S2 → S3 → S4; S5 is
a separate source-policy commit. Native demonstration follows source review and
approved rollout. No new task/project is required.

| Files | Fix | Bounded purpose |
| --- | --- | --- |
| New `crates/buzz-core/src/agent_runtime_discovery.rs` (split pure data/logic siblings only if size requires), `crates/buzz-core/src/lib.rs` | S1 | Shared canonical discovery facts and pure classifiers in an already shared crate; no process spawning or Tauri dependencies |
| `desktop/src-tauri/src/managed_agents/discovery.rs`, `discovery/{catalog,presets,runtime_metadata}.rs`, `readiness/{cli_login,cli_probe}.rs` | S1 | Consume moved facts/classifiers; keep desktop PATH augmentation, caches, installer and UI wrappers local |
| `crates/buzz-hostd/src/{lib,deployment,runtime_context}.rs` | S2/S3 | Host candidate resolution, approved native-CLI pins, same-service readiness probes and result projection |
| `crates/buzz-core/src/host_deployment.rs`, `host_deployment/observation.rs` | S2/S3 | Optional bounded host inventory and independent readiness observation; existing status/result envelopes |
| `crates/buzz-hostd/src/{service_actions,activation}.rs`, `src/bin/buzz-host-activate.rs`, `deploy/buzz-paired-agent@.service` | S3/S5 | Retire custom login execution/history dependency; preserve finite action custody; validate exact trusted-network configuration |
| `desktop/src-tauri/src/commands/agents_hosted_commands.rs`, `desktop/src/shared/api/hostedAgents.ts`, `desktop/src/features/agents/ui/{AgentDialog,HostedAgentSetup}.tsx` | S4 | Merge remote observations with shared metadata; display approval/readiness/model states and remove custom login command copy |
| `crates/buzz-hostd/src/runtime_context.rs`, `deploy/execution-policy.json`, optional fixed reviewed unit template/drop-in | S5 | Named trusted-host networking opt-in; shipped offline default retained |
| Existing core/hostd/paired-owner/desktop discovery and hosted-flow tests; relevant agent guidance | All | Focused production-seam regressions and changed contract documentation |

### S1 — share main decisions, inject machine context

**Severity:** H1/H2 foundation, narrowed by user; depends on none.
**Problem (read):** desktop discovery mixes static definitions and pure
[classification:759](../../desktop/src-tauri/src/managed_agents/discovery.rs#L759)
with global resolution/caches. Its
[CLI runner:51](../../desktop/src-tauri/src/managed_agents/readiness/cli_probe.rs#L51)
inherits process environment; lifting it into hostd would read the wrong HOME.

**Fix:** move only canonical IDs/aliases, command candidates, underlying CLI,
default ACP arguments, probe descriptors and availability/version/output
interpretation required by this slice into the shared module. Desktop definitions
reference those facts; do not leave a second commands/probes table behind. Keep
presentation/install metadata and desktop environment/caches where they are.
Preserve Codex's existing minimum-adapter version policy as shared data/logic.
Do not relocate the 1,287-line discovery or 1,712-line readiness module wholesale.
Use the existing `buzz-core` dependency in desktop and hostd; a new framework or
crate graph is not necessary for pure facts and classifiers.

Pass resolver and probe results explicitly. Host execution supplies absolute
approved paths, HOME, cwd, cleared/allowlisted env, PATH/interpreter roots, bounded
output, deadline and cancellation. Desktop continues its own platform wrappers.
Share the provider interpretation, not the unsafe assumption that the caller's
HOME/PATH is correct. Host probes have no global desktop auth cache; results are
scoped to agent/catalog/revision/action. Config-invalid diagnostics become a fixed
sanitized category in remote messages, never raw native stderr.

**Verify:** existing main registry/args/availability/version and Claude/Codex probe
classification tests still pass; host and desktop consume the same facts. Fake
resolver context A and context B yield different installations without mutating
process globals. Grok yields no probe and `NotApplicable`, never `LoggedIn`.
**Watch:** Goose/buzz-agent config readiness is richer than CLI login. Discover
these normally but mark unsupported hosted configuration honestly where the
existing restricted deploy payload cannot express it; do not add configuration
support or treat a missing known rule as authenticated/ready.

### S2 — detection is evidence, approval is authority

**Severity:** M1; depends on S1.
**Problem (read):** [host status:394](../../crates/buzz-hostd/src/deployment.rs#L394)
lists verified catalog entries only; [desktop:141](../../desktop/src-tauri/src/commands/agents_hosted_commands.rs#L141)
projects static metadata. That cannot describe netcup's actual installed state.

**Fix:** resolve canonical main command candidates within explicitly approved
host search roots. Resolve presence without executing unapproved candidates or
reading operator/sibling HOME. Retain missing CLI/adapter distinctions. Version
and auth subprocesses run only after their executable closure is approved, inside
the prepared agent service. Never let discovered paths replace catalog pins.
The current [Runtime:63](../../crates/buzz-hostd/src/lib.rs#L63) pins only harness
and adapter. Add one optional bounded map of extra approved executables keyed by
canonical command name for underlying CLI/status/version probes; use the existing
`Executable::verify`, no arbitrary argv/provider registry. Missing required pins
mean setup unavailable, not an ambient PATH fallback.

Add optional `inventory` to existing encrypted `Status`: at most 64 unique
canonical harness records, with detection/availability, validation status and
optional approved installed-runtime binding. No paths, secrets, diagnostic text
or account readiness in host inventory. Keep `Capability::Supported.runtimes`
and its catalog hash as the sole approval authority; inventory never changes
`Status::permits`. A record's approved binding must exist in that capability and
match its canonical harness mapping. Bound the full existing 8 KiB status body;
oversize/failed inventory is explicitly unavailable, never silently truncated.

**User experience:** an unapproved detected harness is shown in the existing
host section as “Detected on netcup — host approval required”, with partial
installation details where known. It is not a selectable launch runtime. After
operator catalog approval and refresh, the approved runtime becomes selectable.
Missing or unvalidated components remain visible as setup requirements. No
installer, approval button, auto-adoption or local Windows discovery fallback.

**Verify:** detected/unapproved cannot prepare or launch; approved but missing
underlying CLI cannot pass readiness; revoked/stale status still refuses. Host
inventory changes when injected netcup roots change, regardless of Windows PATH.

### S3 — current readiness replaces custom login history

**Severity:** H2; depends on S1/S2.
**Problem (read):** [runtime:378](../../crates/buzz-hostd/src/runtime_context.rs#L378)
requires `native-login`; [finite:205](../../crates/buzz-hostd/src/runtime_context.rs#L205)
is Grok-only. [Observation:61](../../crates/buzz-core/src/host_deployment/observation.rs#L61)
has login history and models but no independent readiness fact.

**Fix:** extend the existing finite models/refresh action to evaluate shared
readiness first in the same service context, then main `buzz-acp models` as
appropriate; retain a single finite-action deadline/bounded process tree. Add
optional `readiness` to `ActionResult` and encrypted `Observation`, with bounded
availability and auth states (`logged-in`, `logged-out`, `not-applicable`,
`unknown`, `config-invalid`) plus fixed execution/timeout/unsupported reason.
Absent observation is unknown. Preserve existing scope, catalog, operation,
revision, nonce and 120-second freshness checks and 2 KiB observation cap.
Models stay independent: Available / Empty / Unsupported / Unavailable(reason).

A launch attempt requires approved/available runtime, shared readiness permitting
an attempt, fresh nonempty models and selected ID membership. Claude/Codex require
a successful current status probe. Grok `NotApplicable` means authentication not
checked by main, permits the attempt when other gates pass, and is never labeled
authenticated. Unknown, timeout, invalid config or unsupported readiness blocks.
Recheck readiness and models in the service immediately before conversation.
Model enumeration is not subscription/entitlement proof; real chat remains pilot
evidence. Never set `login=Succeeded` to make these checks pass.

Retire host `--login`, `Mode::Login`, Grok argv dispatch and credential requirement;
remove its live output/history production branches only after replacement tests.
Retain legacy login fields as historical decode compatibility (new records use
NotObserved/None); leave cleanup able to remove old `.login` artifacts. Do not
reinterpret old history as current readiness. PendingLogin/NeedsLogin wire names
may stay for this small change; user copy says “Host setup” and describes facts.
Move any necessary `GROK_HOME` fact from [context:144](../../crates/buzz-hostd/src/runtime_context.rs#L144)
behind the shared main-owned runtime environment facts, or omit it only after
proving native HOME semantics; never maintain a second host provider switch.

**Verify:** successful native status with no Buzz login receipt permits selection;
old login success with logged-out/unknown status refuses. Grok metadata without a
probe never produces authenticated. Probe timeout kills descendants and preserves
retry/cleanup. Stale/other-agent/catalog/revision observations refuse unchanged.

### S4 — reuse existing UI and private transport

**Severity:** H2 consumption; depends on S2/S3.
Replace [currentHostedModels:118](../../desktop/src/shared/api/hostedAgents.ts#L118)
login-history predicate with the current readiness predicate. Existing hosted
setup displays availability, readiness and model errors separately; remove
[custom login instructions:125](../../desktop/src/features/agents/ui/HostedAgentSetup.tsx#L125).
Show native-login-on-netcup guidance identifying this agent's service/HOME, never
an invented terminal command. Keep one Refresh action and the existing model
picker/deploy/start path. Main still generates the new keypair and encrypted
payload; operator login never supplies an identity or copied credentials.

**Compatibility seam (read):** `Status` and `Observation` use
`deny_unknown_fields` ([status:115](../../crates/buzz-core/src/host_deployment.rs#L115),
[observation:62](../../crates/buzz-core/src/host_deployment/observation.rs#L62)).
New optional fields with serde defaults let new clients read old messages as
unknown; old clients reject new messages. This is **not** full rolling-version
compatibility. Proposed smallest rollout: keep hosted creation disabled until
matched hostd and Desktop/paired-owner builds are installed and reviewed. Test
old→new decoding and old-reader rejection; do not relax arbitrary-field validation
or invent a protocol negotiation layer. If rolling mixed-version support is
required, return that decision before implementing a version protocol.

Only existing encrypted status/result bodies change. Existing wire admission,
signed headers, scopes, transport, relay resources and schema remain unchanged;
verify host status/result ciphertext admission still passes the existing tests.
**Verify:** approved/discovered, absent readiness, each auth state, stale/error/
empty models, refresh and retry through existing hosted-flow production tests.

### S5 — selected T is a small explicit source opt-in

**Severity:** retained N1 from #3719; depends on S3 for truthful launch gating.
[Policy:90](../../crates/buzz-hostd/src/runtime_context.rs#L90) only supports Disabled;
[unit:18](../../crates/buzz-hostd/deploy/buzz-paired-agent@.service#L18) has
PrivateNetwork=yes. Add an explicit TrustedHost policy, plus a fixed reviewed
online unit variant/configuration setting PrivateNetwork=no. Retain the offline
shipped policy. Make helper validation recognize only the exact approved source
configuration and policy pairing; unknown/mismatched/foreign overrides refuse.
Existing [shape:46](../../crates/buzz-hostd/src/service_actions.rs#L46) accepts only
its finite-mode drop-in: do not silently weaken that check for arbitrary overrides.
No namespace/veth/nft, proxy or provider-only egress claim. Preserve private
filesystem/state/identity, pins, process bounds and owner authority. Normal host
networking permits host/sibling network and abstract-socket access subject to
those services' own controls; those earlier denials were waived, not passed.
**Verify:** offline still refuses; matching root-approved T passes policy checks;
unknown policy and unexpected unit configuration refuse. Actual mounts, runtime
closure, DNS, network and provider access remain native evidence, not source proof.

## Unavoidable native seam and remaining judgments

**H-native — native-entry design gates the next implementation dispatch; actual
demonstration gates live acceptance.** Read: canonical unit uses
DynamicUser, private RootDirectory/StateDirectory and null output
([unit:10](../../crates/buzz-hostd/deploy/buzz-paired-agent@.service#L10)); child stdin
is null ([context:156](../../crates/buzz-hostd/src/runtime_context.rs#L156)). Existing
local frontend supplies output only and Grok login. There is no evidenced generic
interactive native entry into this exact service context. `sudo -u bryce`, a
separate HOME, copying tokens, or guessing a recyclable numeric UID does not solve
it. Preserve the service identity/state across restarts, not an assumed fixed UID.

**Recommendation:** next, before coding the dependent path, the conductor/operator
resolves a concrete finite native OS administration approach by source/OS design.
It must run the provider's own
CLI in this new agent's actual service account, HOME, mounts and env, with no
concurrent conversation, no operator credentials and confirmed cleanup/restore.
Direct native tooling is preferred; do not prescribe untested nsenter/su/systemd
commands here. Demonstrate the procedure only in the later approved disposable
pilot. If the design requires significant new privileged interactive machinery,
stop and return this named prerequisite. Options: approve a separately specified
minimal native-entry change, or hold the pilot. Default: hold dependent implementation and the pilot; no broker hidden in
this plan and no return to the copied-rootfs test platform.

Other explicit proposals for plan approval: pure shared module in existing
buzz-core; disabled detected-but-unapproved UI; matched gated activation for the
two optional wire fields. These are the smallest proposed engineering choices,
not silently implemented product decisions. Native login availability for Claude,
Codex or other harnesses is not certified by registry reuse; one approved disposable
Grok pilot remains the first acceptance target.

## Verification, effort and built-in review

Finite later source gates: core contract/discovery tables; existing hostd action/
helper/lifecycle tests with bounded fake CLI probes; paired-owner receipt tests;
desktop discovery/readiness Rust tests and existing hosted creation/model tests.
Exercise falsifiable production guards, preserve valid unchanged checkpoint
receipts, and stop after affected acceptance is answered. No new test platform,
relay/Docker stack or provider calls for this source slice. Follow normal repository
format/type/lint and required pre-PR gates when execution reaches a PR; report any
not run. Independent reviewer follows implementation; executor is not its own gate.

Later, only after explicit approval: one disposable main-created agent; native
login in its verified exact context; Refresh shows truthful readiness/models;
select and launch; reply with Windows closed; persistence/re-auth/cancel/recovery;
confirmed teardown. Pin artifacts/mounts/policy, rollback and redacted receipts
before requesting activation. Existing agents and pairing remain untouched.

**Effort (read-derived, not a quote):** small shared-fact extraction plus host/wire/
UI integration, not “add Claude to a switch.” Existing sources measured 156 lines
for builtin catalog, 642 for presets including tests, 1,287 for discovery, 1,712
for readiness. Only relevant facts/classifiers move. A reliable moved/new/deleted
LOC total is unavailable before a patch; claiming the audit's 120–150 auth lines
as this plan's size would be false because auth orchestration is now excluded.
Custom host login branches are deletions; observation/context integration is new.

**Self-review:** no prerequisite hidden as a shim; native entry remains a named
pilot blocker. No duplicate registry, no ambient context, no unapproved spawn,
no metadata-as-entitlement, and no false old-client compatibility. Empty/error/
unknown states block as specified; Grok NotApplicable is explicit. Existing
identity, retry, freshness and cleanup contracts survive. Unchased questions:
exact native terminal entry (pickup before rollout), provider refresh/entitlement
(pickup at approved pilot), mixed-version negotiation (only if required).

**This docs assignment:** source inspection/preflight/self-review completed;
document links and `git diff --check` are the only intended executable gates.
No builds, product tests, native commands, provider probes, service or credential
access ran. Prior accepted audit tests are carried as history, not rerun or claimed
as validation of this proposed implementation. Human plan acceptance is pending.
