# #3725 confirmed hosted Cleanup execution

Status: implementation in progress. Executor owns exclusive write/test work in
`/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`. Starting clean pin
`ee3bbc8d5f77f45634040e41dafa3e6dfb7b159a`.

Contract: [ratified spec](../specs/3725-confirmed-hosted-cleanup.md), F1–F3/G1–G4.
Requested Astra/high; served identity is not independently exposed.

Conductor approved narrow use of existing fallible key/tombstone operations;
strict hosted finalization keeps phase 3 on failure. No storage API addition was
needed: `try_delete_agent_key` is already crate-visible. Ordinary local/provider
best-effort deletion remains unchanged. Owner-only new Cleanup sibling and native
shared deletion/cleanup siblings keep responsibilities and file size bounded.

Measured early gate: `HERMIT_SELF_UPDATE=false`, activated Hermit,
`cargo test --offline --locked -p buzz-paired-owner deployment -- --test-threads=1`
PASS 2/2 including new Cleanup fixture extension. Initial sandbox run failed at
loopback bind; identical authorized socket-escalated command passed. This is an
intermediate source result, not final acceptance.

Conductor subsequently authorized existing Windows canary verification and the
narrow canary branch fast-forward/dispatch, after coherent local checks. Source
must be committed first; no main/tag/release/live mutation. Native Windows,
browser, final local gates and independent Chuck review remain pending.
