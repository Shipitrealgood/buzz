# Execution receipt: #3721 host service and discovery

Status: implementation in progress; no activation or independent acceptance.
Workspace `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`; clean start measured at
`db829357458e90ed434af1487039bedbe0a5ad5f`. Product baseline `da66e8d1c`.
Requested configuration: Astra/high; independent runtime identity metadata unavailable.

Accepted inputs: [spec](../specs/3721-host-service-discovery.md),
[execution brief](../briefs/3721-host-service-discovery.md), accepted M-3 repair.
Exclusive source writer; no pre-existing dirty files or handed-over processes.

## Contract checkpoint

Moved main's pure model normalization and DTOs into buzz-core, preserving desktop
exports and camelCase representation. Added bounded immutable encrypted observation
body with legacy result decoding and unchanged public result header. Added optional
approved installed-runtime/harness mapping. No execution enabled.

Measured gates (Hermit with `HERMIT_SELF_UPDATE=false`):

- `cargo test -p buzz-core --lib host_deployment`: 12 passed.
- `cargo test -p buzz-core --lib agent_models`: 2 passed (existing normalization cases).
- `cargo check -p buzz-hostd -p buzz-paired-owner --all-targets`: passed.

## Fixed execution boundary agreed with conductor

The installed finite-mode entrypoint will run the real supervisor under the shipped
`PrivateNetwork=yes` restriction. A root-owned policy credential accepts only the
disabled policy. Offline command/model facts cannot authorize online conversation;
observations report network-disabled and conversation refuses. This is trusted
configuration, not proof of native isolation. N1 later requires a reviewed online
policy variant, unit network configuration and native evidence; none is enabled here.
Root reads output using anchored no-follow file descriptors, never runtime paths.

## Remaining gates

V1–V5, source integration, strict ACP, native service/network/reboot/provider/Windows,
full CI and human testing are NOT RUN at this checkpoint. PID 1 is not systemd.
No host-manager/provider/live-relay actions or existing-agent credentials used.

## Service/context checkpoint

Added the real fixed finite supervisor and short existing-lock helper transactions,
shared per-unit context, root-local login frontend, exact finite-mode override,
anchored output custody, opt-in same-journal binding and offline unit filesystem
configuration. Live raw login output is confined to RuntimeDirectory; terminal
StateDirectory results omit it. The existing conversation construction is retained
behind the offline refusal and fresh-selection gate.

Corrections found by tests: canonical `grok` harness keys are not namespaced runtime
IDs (separate bounded key validator); owned drop-in directory creation now sets 0700
explicitly rather than depending on caller umask. Initial local socket tests returned
sandbox EPERM, so the exact compiled synthetic suite ran with approved local socket
access. No manager, provider, installed service or existing-agent credential was used.

- `cargo check -p buzz-hostd --all-targets`: passed.
- `target/debug/deps/buzz_hostd-0d871f539b36364e service_actions --test-threads=1`:
  4 passed, 1.08s. Covers fixed artifacts, busy/correlation/root-only login,
  symlink/hardlink refusal, real finite login subprocess, same HOME persistence,
  inherited-provider/Buzz-key exclusion, combined output cap and deadline.
- These tests inject manager responses and use disposable context paths. They do
  not prove systemd mounts, cgroups, native login or installed-entrypoint startup.

Conductor clarification: V4's source selected-start leg must end in truthful
network-disabled refusal. Online selected-start is N1/native NOT RUN; an injected
manager cannot prove it. The spec now records this boundary explicitly.
