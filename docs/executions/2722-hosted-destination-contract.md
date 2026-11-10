# Hosted destination step 1 contract and storage decision

2026-11-09 · #2722 / #3720 · Inert contract/scaffold, no deployment admission.
Accepted input: `/home/bryce/services/buzz-agents/docs/specs/2722-first-hosted-agent.md`, slice 1; user authorization is limited to this slice. Source starts at `a2722aa9bdac7acefc8a01376151cdab6a249305`.

## Present behavior

Windows Add Agent's existing Advanced / Run on selector includes the current
owner/community's registration rows and Add Host. The latter opens the existing
registration UI. Local and external provider mappings remain the existing paths.
Registration-only, pending, expired, revoked and unknown states are projected
from actual registry data. Query failure/unavailability is not host offline.
Legacy runtime/capacity fields do not grant deployment capability. Non-Windows
uses the existing provider selector and performs no Windows-only registration IPC.

Host selection is an explicit `registered-host` destination containing the shared
registration `Scope`. It is separate from persisted `BackendKind`: no runnable
host record or lifecycle match is added yet. Both ordinary dialog submission and
shared instance/create boundaries refuse it. The native create command rejects
it before validation, store synchronization, key generation or publication.
Malformed IPC destinations fail typed decoding. No capability can enable this
scaffold, including supported fixtures. Selecting a host does not launch a
provider probe. A stale selection stays blocked instead of becoming local.

## Shared data contract

`buzz-core::host_deployment` contains the strict typed destination, capability,
status, deploy/cleanup command, authority commitment, result and protected intent.
This is data validation, not a wire codec or admission implementation. The current
registration v2 `Body`, kind, tags and handler are unchanged and will reject these
operations. No kind number or advertised capability is assigned in this slice.

- Reuse `paired_message::Scope` and `Policy`, registry canonical origin and
  32-byte lowercase operation/generation identifiers, and the existing hostd
  `plugin/runtime` IDs and SHA-256 catalog-byte hash. The slash is the existing
  catalog key convention, not a new per-VM plugin architecture.
- Capability distinguishes registration-only, explicitly unsupported, and
  supported (including empty approved runtime lists). Availability independently
  distinguishes unknown/current/stale/unavailable/revoked. The current UI has no
  deployment status feed and therefore does not invent those observations.
- Authority binds version, community/owner/host/enrollment, agent, deploy or
  cleanup action, stable operation ID, expected placement revision, exclusive
  expiry, and SHA-256 of exact encrypted content bytes. Validators check trusted
  scope/revision, bounded content and expiry. They do **not** check current
  membership/revocation, signing identity, or cryptographic ciphertext validity.
- The future signed envelope must bind every Authority field, with inner action
  cross-checked after NIP-44 opening. Signed authority is mandatory in slice 2;
  constructing/deserializing Authority alone never authorizes execution.
- Deploy reuses `private_managed_agent::PrivateIdentity` and its existing NIP-OA
  validator. The agent nsec must derive exactly the bound agent, distinct from
  host/owner; its auth tag must attest that agent from this owner. The owner key
  never appears in the payload. The encrypted command contains only this new
  agent identity and structured resolved config, not command/path/env overrides.
- Structured config is an adapter target for main's existing effective config
  and runtime metadata, not a new runtime catalog. The actual adapter/discovery
  remains slice 2. Preserve main validation for visible instructions and runtime
  capability/effort/model compatibility before constructing it; the core shape
  validator only validates the bounded fields it documents.
- PendingLogin is an explicit nonlaunchable model state. Installed host metadata
  must not use operator or sibling provider sessions. Native new-agent auth,
  authenticated model discovery and model-picker wiring remain later slices.
- Resume uses Deploy with the same identity and placement. Cleanup fences before
  stop/disable/removal and retains working data. Pending/failed results preserve
  retry state; Cleaned requires observed completion and a retained tombstone.
  The current helper's abort/settle behavior is not asserted to satisfy this.

## Exact persistence decision for slice 2 (no SQL in step 1)

Preserve deployed migration 81 byte-for-byte and registration tables/readers.
Use additive private storage, outside generic events/search/fanout:

1. Relay `paired_agent_placements`: primary key `(community_id, agent_pubkey)`;
   immutable `owner_pubkey`, `host_pubkey`, `generation`; monotonic `revision`;
   immutable `unit_id` (the original create operation ID, reusing hostd request/unit
   identity), state (`pending`, `applied`, `cleanup-pending`, `cleaned`, `failed`),
   `last_operation_id`, and nullable `cleanup_operation_id`. A cleaned row remains
   a tombstone. Keeping the primary key independent of host/owner prevents a
   second placement for the same community agent. Re-enrollment cannot silently
   transfer or resurrect it.
2. Relay `paired_agent_operations`: primary key `(community_id, owner_pubkey,
   operation_id)`; exact immutable agent/host/generation/action/expected_revision/
   expires_at/content_sha256 and signed event bytes; durable state and correlated
   result bytes. Identical retries return the retained outcome; any changed bytes
   under that operation ID conflict. No plaintext key/config. Admission and
   operation/placement writes share an application-fenced transaction with current
   registration/membership checks. No foreign keys, triggers, backfills or new
   DB functions. Slice 2 chooses the then-current additive migration number and
   updates desired schema alongside it under `docs/database.md`.
3. Desktop existing protected device-local `paired-hosts/owner.sqlite`: additive
   `deployment_intents` keyed by `(community, owner, operation_id)` with immutable
   serialized `ProtectedCreateIntent`, plus monotonic phase and durable publication/
   dispatch/cleanup receipts. Persist one native intent containing exact NIP-44
   encrypted Command bytes before any profile/channel publication or dispatch.
   Reuse the existing Windows ACL gate and SQLite FULL transaction discipline.
   No WebView storage or plaintext duplicate key. Recovery requires the original
   native owner signer and exact host key; ciphertext alone is insufficient.
4. Main's native creation remains the one key/config authority. It generates the
   candidate once, seals the new-agent identity to the host, and journals it before
   materializing the normal managed-agent key/record and publications. Existing
   main key persistence is a replayable materialization of that same identity;
   recovery must never call key generation again. That integration/atomicity is
   **not implemented** here. Missing signer leaves recovery pending.
5. Hostd extends its existing durable journal, catalog hash checks, request/unit
   identity and privileged helper path. No second daemon, runtime table or unit
   naming scheme. The slice 2 adapter must bind one stable unit to the accepted
   agent placement (retain its original create operation ID across redeploys), persist cleanup fences before helper calls, and recheck relay
   execution authority before applying queued work. Do not equate the legacy v1
   request workflow or registration refresh loop with this deployment path.

Compatibility: step 1 writes no new intent/placement data and changes no storage
schema. Old local/provider create requests omit `destination` and retain their
behavior. Existing applications must not see future host placements projected as
local/provider runnable records. Slice 2 must introduce explicit backend/version
handling only when its lifecycle dispatch is implemented and independently gated.

## Handoff boundary

Next: independent review of this candidate, then an authorized slice 2 dispatch
covering signed transport, protected reads, atomic placement/retry, main identity
materialization and host journal/helper integration. Runtime/login/service/network
policy, native Windows journey and disposable pilot are separate gates. No live
changes, provider authentication, historical fixture repair, or teardown occurred.

## Slice 2 refinement (2026-11-10, conductor approved)

The authorized implementation is recorded in
[3738-3739-hosted-deployment-step2.md](3738-3739-hosted-deployment-step2.md).
Kind43104 and additive migration82 implement the private management extension;
registration81 remains unchanged. One bounded host-signed capability table is
additional to the two placement/operation tables; it is not grant authority.

Owner intents now atomically include exact signed command bytes and an
owner-self NIP44 materialization snapshot bound to that command's full authority.
The snapshot omits the agent key; recovery obtains it from the original Command.
Main's normal record still requires its existing key field. A persisted
RegisteredHost backend has safe pending summary and refused unsupported lifecycle
routes. UI creation remains blocked; the native entry point is default off.

Current-authority permit issuance is the revoke linearization point. One fresh,
short-lived nonce is durably consumed before each privileged deploy attempt.
Revocation prevents later permits; it does not kill an already-running agent.
Only a persisted explicit cleanup intent authorizes restrictive offline retry.
Expired commands retain identity and exact bytes; failed expired work leaves the
poll queue and requires an explicit new operation, never an implicit new key.

The actual fixed helper body is shared with isolated tests; its executable keeps
root/peer authentication and fixed production paths. Today's registration-only
service installation fails the execution precondition. Slice3 owns compatible
service/account/mount integration; native Windows and actual systemd acceptance
remain separate gates. No installed service or provider was changed.

## Slice 2 review repair: restrictive retention and progress

Conductor-approved refinement, 2026-11-10 (#3738/#3739, H-1/H-2/M-1/M-2):
ordinary Deploy history remains capped at 1,024 operations per owner/community
and 4,096 per community. Each existing placement separately reserves at most two
Cleanup rows: its immutable first cleanup anchor and one current replacement.
A new currently authorized, unexpired Cleanup at the exact placement revision
may replace that second row transactionally. `cleanup_operation_id` remains the
first anchor permanently; unit/agent identity and the Deploy fence never change.
No schema, migration, extra table, digest chain or retention service is added.

Replacement IDs use the reserved `c1ea0001` prefix plus 56 hex digits of SHA-256
(224-bit binding) over a domain-separated canonical serialization of Authority
with operation_id empty, event created_at, and kind43104. Deploy rejects that
namespace; every Cleanup using it must validate its binding, including a first
cleanup on another placement. The extension is unshipped/default-off; this does
not change registration IDs. Native create rejects an explicitly supplied reserved
Deploy ID before signer capture/materialization. There is no production allocator
yet; the later caller must exclude this prefix when allocating new Deploy IDs. Authority
includes version, exact community/owner/host/generation, agent, Cleanup action,
expected revision, expiry and ciphertext commitment; recipient is the scope host.
Retained rows still compare the entire original event bytes, including signature
and tag spelling. Retired replacements lose relay retrieval of their original
bytes/outcomes and envelope-conflict diagnostics; their caller must retain that
envelope if history is needed. An operation read returns explicit
`missing-or-superseded`, never a receipt or success. Old revisions cannot be
readmitted or executed; changing authority/time under a retired ID fails its
semantic binding. This is deliberately not a claim to remember forgotten bytes.
The original anchor remains retrievable exactly. Host storage stays one row per
agent; Cleanup adds no consumed-permit nonce history or owner create intents.

Pending reads use an optional operation_id as an exclusive rotation point and
wrap over bounded pages. Host pass and offline cleanup cursors persist before
attempts in namespaced entries of the existing work_cursor journal. The central
registration controller runs at most four independent cleanup generation tasks;
helper failure remains actionable per item and does not terminate discovery.
This improves progress, not authority: an expired first-receipt command never
invokes the helper, and only a previously persisted explicit cleanup intent may
retry offline. Denied management REQs retire an existing same-ID holder through
the established lifecycle lock before CLOSED; EVENT and COUNT keep their prior
response behavior.
