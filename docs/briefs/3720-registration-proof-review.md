# Independent review scope: isolated host registration proof (#3720)

**Date:**2026-11-06. **Requester:** conductor. **Next owner:** fresh independent codex-chuck. This is an execution handoff, not independent acceptance.

## 1. Where the work lives

- Lane: `/home/bryce/services/buzz-wt-3720-host-registration`.
- Branch: `proof/3720-host-registration-events`.
- Base: `723c7e849a65af766a7db8ae8c2b62821f7550b4`.
- Product commits: `949063e57`, `496c4967d`; subsequent commit is artifacts/tooling only. Review base through final HEAD, not today's main.
- The same lane is ready for sequential review. All task services/tests/build processes are stopped. Build cache remains ignored; recreating the fixture requires `up` then `tls`.

```bash
cd /home/bryce/services/buzz-wt-3720-host-registration
git log --oneline 723c7e849a65af766a7db8ae8c2b62821f7550b4..HEAD
```

## 2. What landed

A disabled-by-default fork v2 registration protocol on the actual root WS, with a separate host capability, strict signed/encrypted routing, protected pages/live hints, existing-store binding authority, and actual hostd/Device journal adapters. Existing v1 HTTP/provisioning paths remain. Final14-case runtime passes, but full P1–P12 acceptance is explicitly incomplete; do not convert that receipt into full security/ship readiness.

Read [execution result and exact matrix](3720-registration-proof-execution.md) and [operative snapshot](../specs/3720-host-registration-event-proof.md). The execution result replaces stale progress headlines; failed receipts remain historical evidence.

## 3. Your job

Independently review the committed registration boundary and grade the **finite agreed acceptance matrix**. Determine which stated gaps are actually supported by existing tests/code and which need targeted in-lane probes or repairs. Return concrete blockers first, with exact missing assertions/locations. Do not launch a broad host/platform audit or accept executor self-review as the independent gate.

Conductor requested Astra for demanding security/race review; executor was requested gpt-6.1-sol/high, actual runtime identity not independently exposed. Reviewer configuration belongs to its fresh dispatch.

## 4. Specifically look for

1. **Complete scope, including independently-member hosts.** Probe `crates/buzz-relay/src/handlers/host_registry.rs:63` and `:144`, root connection dispatch and the generic EVENT/REQ/COUNT/artifact guards. Host AUTH clears ordinary scopes and separately pins generation; test both ceilings and reject mixed/unknown scope credentials without normal fallback. Check bind-before-final-reauthorization ordering and owner dependency tracking.

2. **Only one authoritative writer.** Probe `crates/buzz-db/src/store/paired_hosts.rs:91`, `:136`, `:271` and `paired_hosts/registry.rs:50`. Verify all public routing derives from signed fields, operation-ID conflicts reauthorize, v1/v2 generations cannot mix, and ACTIVE/FENCED/DEAD remains monotonic. No new schema/store exists. The final repair rejects a fresh FENCED unpair ID while exact retained retries still succeed.

3. **Private reads and delivery, including cross-node races.** Probe `paired_hosts/registry.rs:83`, `:246`, relay handler `:415`, `:496`, `:624`, generic result/fanout exclusions and HTTP bridge guard. Cursors hide raw seq and bind tenant/principal/view/G/nonce; high-water is opaque. Live reauthorization is primary DB policy on the sending node. Confirm the high-water barriers then distinct post-removal commit prove new sends are denied rather than merely draining old frames.

4. **Real durable host state.** Probe `crates/buzz-hostd/src/registration.rs:53` and `:295`, plus reused store/clock/outbox helpers. The facade links actual daemon modules privately; it is not a copied fake. Inspect proof attempts, local monotonic/boot checks, atomic ack creation, expiry/24h retry distinction, readiness expiry and local tombstone-before-send. Existing daemon CLI remains v1; this is a callable library proof seam, not installed activation.

5. **Owner recovery and terminal outbox convergence.** Probe `crates/buzz-paired-owner/src/registry.rs:163`, `:357`, `:386`, `:615`, `:656`, `:722`. Freshness is process-local and requires a nonce. A missing current-list entry is stale, not proof of DEAD; known pending generations must fetch protected binding and exact host tombstone to retire durable retries. Validate dual consent, immutable parties, labels and no copied invitation/journal. The final journey checks `pending_unpair == None` and no further delivery after DEAD recovery.

6. **Limits and honest compatibility.** Probe the core v2 codec/query/reply bounds, NIP-11 actual signing key, disabled/FI incompatibility gate, shared WS TLS connector and v1 fixtures. Readiness retention is5min for v2 under the reused mailbox quota; latest status/parent and consent/tombstones remain. Confirm no path grants ordinary member rights or falls back to HTTP/stock relay.

7. **Evidence strength and remaining finite work.** Probe the P1–P12 table and `docs/verification/3720/controls/`. Raw complete controls target949063e57; two partial scope removals survived independent protections. The complete scope mutation additionally retains normal NIP-42 scopes and is detected. The unpair/native-recovery repair leaves these exact five mechanisms unchanged; final14-case green covers the delta. Judge that continuity explicitly and repeat only where needed.

## 5. Deviations and candid watchpoints

- Peer payload is a closed nested header/body schema; all duplicated authority is compared. A proposed flat body was not used.
- Numeric global high-water became an opaque scoped HMAC, approved by conductor to prevent traffic-gap inference. Same relay key/cursor machinery, no second authority.
- V2 expired readiness records use5min retention, preserving latest status/parent; v1 remains7days. Confirm cursor reset semantics in the unrun pruning probe.
- Readiness queries are process-local freshness challenges; durable consent/revocation outboxes are separately journalled. Review the spec's operation persistence language against that distinction.
- Broad body/credential fault permutations, explicit DB/Redis loss, timed notification loss/pruning and large concurrency cases were not all run. They remain acceptance work, not waived requirements.
- Strict clippy is baseline-blocked. Supplemental JSON gate checks every primary span/new file against the exact base:40 baseline diagnostics,0 new/error. No global allowances ship.
- Test discovery's out-of-line external-module classification is incomplete at `scripts/check-postgres-test-discovery.py:348`; canonical inline wrapper passed685-file discovery. Conductor owns separate tooling routing; checker source was not changed.
- Root crate Rust files are outside current differential size policy roots. Actual configured gate passed without raising limits; the large matrix test file is not a claim that the policy governs it.

## 6. What not to chase

Keep this review to registration/admission/discovery/revocation and its finite controls. Agent provisioning/key-custody/provider selectors, UI redesign, Windows runtime certification, login/lifecycle/installer/adoption and rollout are separate work. Migration81 history remains a carried #3730/#3731 hold; no schema/migration change was made. Existing host isolation/catalog findings remain separate. Do not merge, push, deploy, use live credentials/endpoints or mutate the board under this worker handoff.

## 7. Deliverables and pre-flight

Return severity-classified findings plus a clear acceptance/missing-probe map to the conductor. Preserve the lane, source pin, failed receipts and exact control deltas. A report alone does not close #3720 or authorize deployment.

Before testing, confirm clean HEAD/branch, read both linked artifacts, inspect `cleanup.json` and verify no previous writer/process remains. Reproduce with the committed task-labelled no-egress fixture script; do not run `just test` or inherited/default service URLs. Use fresh desired-state DBs and two build jobs. Native Windows/full CI/standard nextest wrapper were not run; nextest was unavailable. No human-test/PR-ready claim exists.
