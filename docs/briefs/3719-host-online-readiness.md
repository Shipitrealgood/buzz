# #3719 / #3721 online readiness — selected T handoff

> **SUPERSEDED — 2026-11-10 trusted-host correction.** The operative plan is
> [#3722 shared-account execution](../specs/3722-main-discovery-native-login.md). Bryce approved normal non-root
> host account/HOME/native logins and networking, with distinct per-agent Buzz
> identities/config/workspaces/service tracking. Earlier per-agent OS isolation,
> private-root/provider-credential separation, root helper/catalog approval and
> exact-DynamicUser native-login-entry requirements are superseded, **not passed**.
> Retain owner authorization and journal/retry/cleanup; the root helper leaves the
> ordinary new-deployment path. The text below is historical only. No live action
> or existing-agent change is authorized.

**Decision, 2026-11-10:** Bryce selected normal host networking (T), keeping existing
per-agent protections and requesting the smallest disposable Grok change. First
complete the Claude reuse assessment; conductor review precedes source execution.
Workspace `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`, audit input pin
`a59b3db1dd4e6c7977a915e9e9e127925eb83e48`.

[Updated decision packet](../specs/3719-host-online-readiness.md) is authoritative.
The prior co-resident network and abstract-socket denial requirements are explicitly
superseded, **not passed**. Filesystem/socket mount boundaries, owner authorization,
narrow root helper, approved catalog runtime, per-agent identity/private HOME and
credentials, retries and cleanup remain. No namespace/veth/nft lifecycle or former
D/450–740-line proposal. Keep the offline default until reviewed source and explicit
live approval. Runtime closure/native execution/provider evidence remain open.

Next: read [Claude reuse audit](../audits/pre-audit/3722-claude-host-reuse.md) and
[forward brief](3722-claude-host-reuse.md); conductor updates roadmap/acceptance and
reviews the small source scope. Fresh executor and independent reviewer share this
worktree sequentially. No live change, existing-agent read, provider call, Windows
rebuild, copied-rootfs test platform, board mutation, push or deployment is implied.

Accepted [checkpoint-2 source review](../audits/post-audit/3721-host-service-discovery.md)
and [Windows native canary](../audits/post-audit/2722-windows-native-canary.md) remain
inputs. Models metadata and synthetic auth results are not subscription proof.
This is a documentation decision update; no product implementation was performed.
Requested audit configuration Astra/high; actual runtime configuration unknown.
Final doc checks, DCO commit, test results and process closure are in the new brief
and conductor return. Human/source/native/provider acceptance is not asserted.
