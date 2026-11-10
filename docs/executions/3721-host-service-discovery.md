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
