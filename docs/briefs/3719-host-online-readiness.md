# #3719 / #3721 online readiness — conductor decision handoff

**Next reader:** persistent conductor resolving the trust/egress choice; then the same spec assignment may be continued to finalize that choice. This is **not** a ready executor dispatch.
**Date:** 2026-11-10 UTC. Requested `gpt-6-astra/high`; actual served configuration unknown.

## Where the work lives

- Worktree `/home/bryce/services/buzz-wt-3720-host-registration`.
- Branch `proof/3720-host-registration-events`.
- Source baseline `f4b24f42c340dc55f081a8cfd95893fdf1ff098a`, clean at handoff in.
- [Decision packet/spec](../specs/3719-host-online-readiness.md).
- This assignment owns only that spec and this brief. No source/test/probe edits, board operations, live changes or provider/relay calls.

## What is in the working tree

Two documentation artifacts, committed with DCO on the above branch. Product source remains exactly at the source baseline. No tests/builds/background processes were launched.

The accepted offline foundation remains credible: [#3721 re-gate](../audits/post-audit/3721-host-service-discovery.md:3) closes its source defects; [Windows native canary](../audits/post-audit/2722-windows-native-canary.md:104) supplies 101 tests/ACL/build. Neither closes online service or Grok entitlement. The newer [delivery map](/home/bryce/services/buzz-agents/docs/plans/2026-11-06-host-completion.md) supersedes stale Windows-pending pilot text.

Bounded gap verdict: **pending decision; C0/H2/M1/L0 activation/verification gaps**. N1 is a missing selected/enforced online policy; R1 is unpinned/unmeasured installed runtime closure; V1 is actual native boundary/reboot evidence. These are not claims that the accepted offline implementation regressed.

## Sequence

1. **Resolve HOST-TRUST / N1-EGRESS.** Main's [operator shell trust](../../VISION_AGENT.md:57) and [substrate trust](../../VISION_REMOTE_AGENTS.md:31) do not imply per-agent OS confidentiality. Conductor measurements of current agents show shared `bryce` UID and absence of the listed unit confinement directives. Their similarity to main does not meet the stronger accepted #3719 contract. Bryce has asked for this comparison; D is not yet selected.
2. **Discuss T first under the latest provisioning/migration steering.** Bryce describes verified relay placement/easy provisioning and eventual migration of the existing agents. Preserve hostd authority/helper, DynamicUser/private HOME/credentials/filesystem controls while using normal host networking. This needs only an existing policy/unit/helper adaptation, not D's namespace lifecycle addition; no measured T line count exists. Explicitly disclaim co-resident-network/host-abstract-socket isolation and revise prior #3719 acceptance before code. Neither option is approved. **Only if retaining stronger network isolation, consider D with its actual limits.** Dedicated namespace/private state; public IPv4 HTTPS and chosen public DNS; host/private/local/metadata/sibling paths denied; IPv6 off. Public HTTPS, DoH and outbound data transfer remain allowed. No invented provider/CDN allowlist. A strict hostname-only requirement is a different, larger prerequisite.
3. **Finalize the bounded contract only after choice.** Pin actual artifact paths, root-owned online configuration, address/resource ownership, nft placement, restart/cleanup and native runner. Proposed D costs 450–740 production LOC excluding tests/docs, one root oneshot plus fixed methods in the existing helper. It is not authorized implementation. No manual namespace-per-agent gap is acceptable.
4. **Fresh executor, then independent reviewer.** Keep this worktree for sequential ownership. Use accepted source evidence with explicit continuity; run only affected finite gates. Native/live work follows a concrete reviewed approval request, never source approval alone.

## Pre-flight evidence and limits

Read complete #3719/#3721/#3722 supplied exports, including histories and amendments; project AGENTS/VISION/TESTING, accepted service artifacts, runtime/helper/unit code and relevant personal operations/idea notes. Read primary installed `systemd.exec(5)`, `ip-netns(8)`, `network_namespaces(7)` and upstream nft manual. Semantics verified: NetworkNamespacePath requires a pre-existing namespace and overrides PrivateNetwork; abstract Unix sockets follow the network namespace; filesystem sockets follow mounts; systemd entry does not supply ip-netns DNS mounts.

Conductor reports actual-host systemd 257.13, nft 1.1.3, iproute2 6.15.0, Debian 13/kernel 6.12.115. Sandbox PID 1 differs from real netcup PID 1. No native runner or confinement pass follows from installed tools. No disposable reboot-capable host has been established. Do not revive the stopped copied-rootfs proof or silently create a test platform.

## Things to flag back before powering through

- Any attempt to treat a new online enum as enforcement: [current runtime](../../crates/buzz-hostd/src/runtime_context.rs:90) is only disabled; the kernel boundary must accompany it.
- Any network dependency taking the existing activation flock: [activation.rs:171](../../crates/buzz-hostd/src/activation.rs:171) holds it through [systemctl start:382](../../crates/buzz-hostd/src/activation.rs:382), so recursive acquisition deadlocks.
- Exact drop-in recognition must preserve [finite-mode custody](../../crates/buzz-hostd/src/service_actions.rs:48), [retirement ordering](../../crates/buzz-hostd/src/service_actions.rs:170) and [cleanup](../../crates/buzz-hostd/src/service_actions.rs:303); do not accept arbitrary overrides to enable a namespace.
- Host post-DNAT/public-address/container/VPN denial needs actual evidence. Namespace output rules alone do not establish it. Missing external host/metadata inventory is an activation blocker.
- Endpoint restrictions that require a proxy, generalized network controller or DNS/CDN maintenance exceed the proposed D scope. Return the requirement rather than build that foundation implicitly.
- Missing runtime tools/libraries must be pinned/mounted as approved runtime material; no operator HOME, credentials, broad root mount or mutable auto-update fallback.
- Native host tests/reboot/provider login need named resources and explicit authorization. Only the conductor can route/record task changes.

## What not to chase

Preserve main creation/ACP, private protocol, registration identity/journal, Windows artifacts and existing agents. Exclude adoption, generic installer/fleet/lifecycle, old HTTP retirement, upstream catch-up, new daemon/proxy/firewall framework, and standalone copied-rootfs proof. General full-product claims are outside this one pilot. Do not infer provider entitlement from native login success or models output.

## Done criteria and release

This documentation assignment is complete when both artifacts have valid local links/citations, `git diff --check` passes, a signed-off documentation commit exists, and status is clean. Product verification S1/S2/N1–N4/P1 in the spec is **NOT RUN here**. No merge-ready/full-CI/native/provider/human acceptance is asserted.

Documentation validation: local Markdown targets and cited source line numbers checked; content self-review completed; whitespace/diff check passed. Commit identity and final clean-status evidence accompany the return. No writer/test/process remains active; exclusive ownership returns to the conductor after the final git check. Next action is the trust/egress decision, specifically T versus stronger isolation after the latest user clarification, not source execution.
