# #3722 Claude host reuse — conductor review brief

## Where the work lives

Worktree `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`; clean input
`a59b3db1dd4e6c7977a915e9e9e127925eb83e48`. Requested Astra/high; served
model/reasoning unknown. This assignment owns four docs and two existing test
files; product source is unchanged. The conductor owns task/roadmap mutations.

## What landed

[Audit](../audits/pre-audit/3722-claude-host-reuse.md): **C0/H2/M1/L0**, gaps against
multi-harness main reuse. [Selected T decision](../specs/3719-host-online-readiness.md)
and [old brief](3719-host-online-readiness.md) now record normal host networking,
retained authorization/private-state/runtime/helper controls, and explicit waiver
of co-resident network/abstract-socket denial. The old D proposal is superseded;
no source policy changed. Two small existing-test extensions add Claude alongside
Grok and demonstrate the current unsupported-login boundary. No implementation fix.

## Your next job

Review these findings with Bryce **before** implementation. Latest steering is
reconnaissance and direction discussion, not “add Claude next, then Codex.” The
same Create flow can prepare Claude only with an approved complete catalog runtime
and native feature enabled. Hostd does not discover installed Claude automatically;
it cannot authenticate Claude today. Accepted checkpoints remain valid for their
scope, not evidence that the desired machine-as-black-box behavior is complete.

Recommended smallest next slice, subject to review: make main's existing auth-method
interpretation callable through an explicit context/resolver seam, with Desktop
still consuming that same implementation. Hostd should own approval, private
execution context and supervision, not provider decisions. Then agree how shared
readiness is represented and how candidate discovery uses the same main registry
under approved search roots. Only afterward connect these to the host. Roughly
120–150 existing lines of DTO/terminal interpretation are extraction candidates;
that is a read-derived move estimate, **not** a total production LOC promise.
A full runtime registry/readiness extraction has no reliable estimate yet.

## Specifically look for

1. **Inventory versus discovery:** [host status:394](../../crates/buzz-hostd/src/deployment.rs#L394)
   iterates the root catalog; [main discovery:921](../../desktop/src-tauri/src/managed_agents/discovery.rs#L921)
   resolves adapters and underlying CLIs. Probe whether the next proposal really
   shares machine discovery or merely adds another manually authored entry.
2. **One owner for auth:** [main auth:80](../../desktop/src-tauri/src/commands/agent_auth.rs#L80)
   chooses terminal versus ACP; [argv:262](../../desktop/src-tauri/src/commands/agent_auth.rs#L262)
   handles the existing Claude compatibility detail. Probe that a shared extraction
   deletes the duplicate host policy rather than introducing Claude/Codex matches.
3. **Readiness is not login history:** [runtime:378](../../crates/buzz-hostd/src/runtime_context.rs#L378)
   and [UI gate:118](../../desktop/src/shared/api/hostedAgents.ts#L118) demand host-login
   Succeeded. Probe the proposed same-context readiness result/representation;
   neither model output nor Unknown may become successful authentication.
4. **Root approval remains meaningful:** [catalog pins:24](../../crates/buzz-hostd/src/lib.rs#L24)
   verify two files, not full CLI/Node/tool closure. Probe shared metadata-driven
   commands against approved runtime resolution; no arbitrary root execution or
   operator-home PATH/auth inheritance.
5. **Interactive boundary:** [stdin null:156](../../crates/buzz-hostd/src/runtime_context.rs#L156)
   and [operator output loop:60](../../crates/buzz-hostd/src/bin/buzz-host-activate.rs#L60)
   cannot accept pasted codes. Probe feasibility only in an explicitly approved
   fresh agent context later. If a large auth broker is needed, name that separate
   boundary; do not build one implicitly or use API keys as subscription proof.

## Deviations and judgment calls

No product code, live actions or framework added. Replaced obsolete conditional D
text with selected T and a historical commit reference. Extended existing synthetic
fixtures instead of running installed Claude or rebuilding Windows. The proposed
shared-main extraction is a recommendation for discussion, not ratified architecture.
No claim that generic ACP `authenticate` already implements Claude terminal login.
Root catalog approval versus automatic candidate enrollment remains an explicit
product choice; recommendation retains approval while sharing discovery facts.

## What not to chase

Preserve main forms, identity/journal/relay lifecycle and accepted tests. Exclude
Windows CI/build, real providers/native CLI, operator/existing-agent homes,
installation, adoption, network namespaces/firewall, rootfs fixture platforms,
new daemon/registry/plugin framework and remote login protocol/UI. #3719/#3721
normal-network source work waits for this discussion; #3722 real provider evidence
still requires a separate approved disposable pilot. Full CI/native/human acceptance
remain NOT RUN. No board, push, merge, deployment or live change authority transfers.

## Verification, deliverable and release

The audit records exact commands: **36 distinct behavior cases passed** (1 real
ACP/synthetic adapter fixture, 2 core normalization, 33 named Node cases). Temporary
Unix socket required the same bounded test's approved unsandboxed rerun; initial
EPERM is retained. Initial Node file-level passes are not extra test cases.
Production stays pinned; these tests do not establish real adapter/auth/native
service behavior. Accepted checkpoint-2/3 evidence carries because product source
is unchanged. No Docker/relay/test platform was launched.

Final checks **PASS**: Rust test-file formatting; all three desktop/web/mobile
differential file-size gates against the input pin; 77 owned-doc local targets/line
bounds; `git diff --check`. File-size checks first met sandbox git-spawn EPERM,
then completed with authorized subprocess access. No full lint/CI claim.
Return DCO commit and clean status to conductor; all owned build/test processes
must have exited. This closes reconnaissance only. Next deliverable is the
conductor's discussion/direction ruling, not an executor launch.
