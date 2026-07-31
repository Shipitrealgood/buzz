# Post-audit: full virtual-mint pipeline (T3–T5: buzz-seal, buzz-mint, buzz-provision)

Reviewer: chuck (Fable 5). Date: 2026-07-31. Scope: commits `b5b14b61` (seal+mint),
`552b8408` (provision agent), `75a7a46e` (Design-A flip), `88e34f11` (retire), in
worktree `feat/buzz-provision`. Specs applied as binding:
`virtual-mint-v1-design.md` (⚑ A-ruling header, §2c, §3) and
`sealed-mint-bundle-spec.md` (7-step unseal order, key-handling rules, 8-case test
spec). Prior audit `2684-2685-virtual-mint-t1-t2.md` read for context; its findings
are not re-reported. Hard boundaries honored: no live unit touched, nothing
published to the production relay, live env files inspected by key NAME only.

Gates run (measured): `cargo test -p buzz-seal` 19/19, `-p buzz-mint` 15/15,
`-p buzz-provision` unit+integration 41/41, and all three local-relay e2e
(`e2e_local_relay -- --ignored`, dockerized postgres/redis, fresh relay binary
built from this tree) 3/3 green. Relay/desktop/mobile contract claims verified by
reading the counterpart code in this same tree (file:line cited per finding).

Verdict: **0 Critical / 3 High / 3 Medium / 6 Low. No merge-blocker for landing
the branch; H-1 and H-3 are must-fix BEFORE first production use** (H-1 before any
retire runs on the production host, H-3 before the #2692 live-agent migration).
A-mode (the default ceremony) is production-shaped; B-mode is measurably broken
against the current relay (H-2) but is the ruled-secondary mode.

---

## High

### H-1 (High, read; fix-now, small) — `--no-systemd` silently disables retire's live-agent guard — rides #2688

`retire.rs:153-157`: the runtime-active guard runs only `if !args.no_systemd`.
`--no-systemd` is the flag every test and out-of-band invocation uses habitually
(all three e2e use it), and it is not named like a force flag — yet passing it
turns off the ONLY protection standing between a mistyped/copied command and the
two production agents. Trace: `buzz-provision retire interleaven-claude
--no-systemd` on this host finds env+credential (both real, post-T2), skips the
guard, publishes the 9022 leaves and the 10100 tombstone, **shreds the live
credential**, and archives the env file — while the unit is still running.
`--force-live` (retire.rs:567-581) is correct but never consulted on this path.
Fix shape: always attempt `unit_active_state` (it is read-only); degrade to a
warning only when systemctl itself is unavailable (spawn error), not when the
flag is passed; keep `--no-systemd` scoped to skipping stop/disable.

### H-2 (High, measured; route-to-task) — B-mode provision cannot complete against the current relay, and never converges — rides #2687

`agent.rs:445-457` hard-fails on ANY publish error, and the B-mode pipeline
submits the pre-signed owner kind:9000s authenticated as the AGENT. The relay
structurally refuses that: `ingest.rs:1538` ("event pubkey does not match
authenticated identity") plus the ±15 min freshness window at `ingest.rs:1519`
for any bundle older than 15 minutes. Measured in this audit's e2e run:
"RESULT: relay REFUSED host-side (agent-authenticated) submission of the
owner-pre-signed 9000". Consequence: every B-mode provision run publishes
kind:0+10100, then dies at the first 9000; re-runs re-load `events/` and die at
the same step forever (sealed.json is already shredded, so the state is
half-provisioned: credential+env exist, unit never enabled). The e2e captures the
rejection explicitly but `agent.rs` was never updated to match the #2694 pinned
relay rule. Sealed-spec test 7 (idempotent re-provision) is therefore unmet for
B. Fix shape: in B-mode treat 9000 `Rejected` as warn-and-verify (mirror the
A-mode Phase 5b), or stop submitting 9000s in B-mode entirely and verify — the
owner batch can only ever land owner-authenticated.

### H-3 (High, measured; route-to-task, must-fix before #2692) — provision `--force` over an existing agent silently drops env keys it cannot express, with no backup — rides #2687

`render_env_file` (`agent.rs:601-650`) regenerates the env file from
manifest-only fields and hardcodes `BUZZ_ACP_SUBSCRIBE=mentions`. The live
production env files carry keys provision has no way to express — measured (key
names only): `interleaven-grok.env` has `BUZZ_ACP_AGENT_ARGS` and
`BUZZ_ACP_MULTIPLE_EVENT_HANDLING`. The planned migration path (#2692:
re-provision the two live agents A-style) requires `--force` (env exists,
`agent.rs:185-191`), which clobbers atomically with `Clobber::Allow`
(`agent.rs:376`) — no backup — then restarts the unit. The agent comes back
"healthy" (journal markers all fire) with silently changed behavior: harness
args gone, event-handling mode reset. Fix shape: (a) on `--force`, archive the
existing env as `<name>.env.pre-provision` first (retire already has the rename
pattern, `retire.rs:371`); (b) add an extra-env passthrough (`--env KEY=VALUE`
repeatable, control-char-checked like the other values) or carry unknown
`BUZZ_ACP_*` keys forward from the existing env file.

## Medium

### M-1 (Medium, read; route-to-task) — provision's 10100 lacks the same-second tie-break guard retire has — rides #2687

Retire reads the live 10100 and signs the tombstone strictly after it
(`retire.rs:227-249`, measured rationale in the commit). Provision's kind:10100
(and kind:0) use default `created_at = now` (`agent.rs:387-401`). A
retire→re-provision inside one second — exactly what a scripted rotate ceremony
does — leaves the fresh 10100 tying the tombstone's `created_at` and losing
NIP-01's lowest-id tie-break ~50% of the time: unit healthy, agent permanently
invisible/not-mentionable with zero error output. Fix: mirror retire's
read-live-and-bump in provision Phase 4 (one shared helper; same fix class).

### M-2 (Medium, read; route-to-task) — A-mode never checks `manifest.host_pubkey` against this host — rides #2694

The A-mode branch (`agent.rs:245-265`) checks credential existence, owner
pinning, and credential↔manifest agent-pubkey agreement — but never compares
`manifest.host_pubkey` to `host_keys.public_key()`. B-mode gets host binding
structurally from the seal (unseal steps 1+3); A-mode has no analogue. A bundle
attested with the wrong `--host` (owner typo, or the right bundle copied to the
wrong same-owner host that happens to hold a same-named credential) provisions
silently, and the published `host` lineage (own key, `agent.rs:593`) diverges
from the owner's bundle record. One `bail!` (or at minimum a loud warning) on
mismatch closes the last unchecked manifest field.

### M-3 (Medium, read; route-to-task) — `keygen --force` has no live-unit guard — rides #2688

`keygen` (`main.rs:355-408`) refuses to clobber without `--force` and prints the
existing pubkey (good), but `--force` replaces the credential with only a
generic warning. `keygen --force --name interleaven-claude` on this host
overwrites a live production agent's credential; the running unit survives until
its next restart, at which point it comes up as a different identity. Retire
grew `--force-live` for exactly this class of typo-vs-production accident;
keygen (and provision-with-`--force` replacing a different identity's
credential, `agent.rs:297-307`) should consult the same `unit_active_state`
guard. Fix the class once: shared guard helper, used by retire, keygen, and the
B-mode credential-replace path.

## Low

### L-1 (Low, read) — failed first provision demands `--force` and mislabels the state — rides #2687

The env file is written in Phase 3, before any publish or systemd action; any
later failure (publish rejected, journal timeout) leaves an env file that makes
the re-run refuse with "this instance looks provisioned" (`agent.rs:185-191`).
The instance is half-provisioned, and the operator's recovery step (`--force`)
is also the flag that authorizes identity replacement. Wording + ordering nit;
consider a success marker or "a previous run may have failed partway" phrasing.

### L-2 (Low, read) — `redact_key_material` misses key material embedded in a mixed token — rides #2686

`buzz-seal/src/lib.rs:61-91` tokenizes on non-alphanumerics and redacts a token
only if ALL its bytes are hexdigits (len ≥32) or it starts with `nsec1`/`npub1`.
A pasted value glued to a letter (`x<64hex>`, or `key<64hex>`) forms one token
containing non-hex chars and passes through unredacted. Narrow (requires
argv junk fused to the key with no separator), but the redactor's contract says
"token that looks like key material" — a substring scan for 32+ hex runs inside
tokens would close it.

### L-3 (Low, read) — retire with a hand-deleted credential claims convergence it cannot know — rides #2688

`retire.rs:322-325`: when the credential is gone but env/pub remain, retire
prints "relay teardown was done by a previous run; skipping". True for the
converged path (credential is shredded only after the publishes), false if the
operator deleted the credential manually — live 10100 + memberships then remain
on the relay with a message that says otherwise. Say "cannot re-verify" instead.

### L-4 (Low, read) — no 429 handling in the bridge client — rides #2687

`enforce_http_admission` (relay `bridge.rs:24-56`) rate-limits per pubkey per
minute; the bridge client (`buzz-seal/src/bridge.rs`) has no retry/backoff, so a
mid-batch 429 hard-fails an attest/provision/retire. Re-runs converge, and the
batch sizes here are small; acceptable, noted for the first big-channel-count
agent.

### L-5 (Low, read) — provision assumes the unit template is already installed — rides #2687

Phase 6 runs `enable`/`restart buzz-agent@<name>` (`agent.rs:498-505`); the v2
template is a T2 host artifact not shipped by any crate. On a fresh host,
provision fails after publishing. Fine on this host; one runbook line ("host
prerequisites: host-init + unit template") for D2 (#2672).

### L-6 (Low, read) — normal-error tempfile cleanup unlinks without shredding — accept (same class as T1-5, rides #2687)

`write_atomic_0600` failures drop the `NamedTempFile`, which unlinks but does
not overwrite; only ≥1h-old crash orphans get `shred_file` via
`sweep_stale_tempfiles` (`util.rs:151-175`). Best-effort-shred is already the
documented posture; noted for completeness.

## Production first-run: go/no-go

**GO for the A-mode ceremony (keygen → attest → provision) and the retire pair,
with the must-fix-first list and preconditions below. NO-GO for any B-mode
(`buzz-mint new`) run against production until H-2 is fixed.**

Must fix first:
1. **H-1** before any `retire` invocation on the production host (one-line class
   of change; until then, treat `--no-systemd` as forbidden on this host).
2. **H-3** before the #2692 migration of interleaven-claude/grok (env backup +
   extra-key passthrough); a plain new-agent provision is unaffected.
3. **M-1** if the first production exercise includes any scripted
   retire→re-provision; manual ceremonies with seconds between steps are safe.

Preconditions to verify on the day (all fail closed if wrong — errors, not
damage):
- `BUZZ_ALLOW_NIP_OA_AUTH=true` on the production relay (#2690; default false,
  `relay config.rs:200-201`). Without it every agent-authenticated publish 403s
  — the bridge's 403 hint names this exact cause.
- Clock sanity both ends: NIP-98 is ±60s (`buzz-auth/src/nip98.rs:77-85`) —
  tighter than the ±15min event window; kind:9035 is ±120s
  (`identity_archive.rs:141-149`). The owner-side Windows box is the likely
  offender; check NTP before the ceremony.
- Proxy path: NIP-98 `u` binding uses the request Host + wss→https scheme
  (`relay api/bridge.rs:195-207`), which the live agents' daily bridge usage
  through Caddy already proves; `x-auth-tag` is a plain header (forwarded by
  default). No new exposure found.
- TLS: `reqwest` is workspace-pinned to rustls (`Cargo.toml`), so `https://`
  from `wss://` works; local e2e only exercised `http://`, this is
  read-verified, not measured.
- Phase 6 (`verify_journal`) has never run against a real unit — every e2e used
  `--no-systemd`. The three markers exist verbatim in buzz-acp
  (`lib.rs:131,1354,1511`) and the live units emit them, so the risk is a
  timeout-shaped false negative, not damage; keep `--verify-timeout-secs 90`.

## Clean surfaces (checked, not assumed)

- **Client 10100 contract**: all content fields snake_case and typed to match
  desktop's normalizer (`nostr_convert.rs:439-497` — absent fields defaulted,
  pubkey always overwritten from the event author) and `RelayAgent`
  (`types.ts:288-298`); mobile parses the same names
  (`mention_candidates.dart:35-41`). `host` is additive-safe in both (object
  passthrough / keyed map reads). Tombstone `respond_to:"nobody"` is outside the
  desktop `RespondToMode` union but fails every mentionability branch closed —
  exactly the ruled behavior; `"nobody"` is a first-class harness mode
  (`buzz-acp/config.rs:95-101`).
- **kind:0 parity**: verify-then-embed auth tag (`agent.rs:567-572`),
  agent-signed, NIP-98 + `x-auth-tag` — matches the native `build_profile_event`
  semantics; harness never republishes kind:0 or 10100 (grep-verified), so
  retire's publish-before-stop ordering cannot be raced by the dying agent.
- **9035 shape**: matches `verify_owner_consent` exactly (single 4-element auth
  tag, owner==signer, live kind:0 attesting the same owner,
  `identity_archive.rs:253-299`); host retire structurally cannot emit kind:0
  (assert + test).
- **Unseal order**: spec steps 1–6 implemented literally and in order
  (`buzz-seal/src/lib.rs:478-569`); all 8 spec test cases present and green,
  including splice, wrapped-foreign-attestation, and rogue-owner cases.
- **Key handling**: no secret ever formatted — newtypes with redacted `Debug`,
  no `Display`, deliberate `expose_*` accessors; owner key via 0600-checked file
  or silent prompt, held only through signing; clap argv redaction on both
  binaries (leak test green); serde/nip44 residuals documented, matching the
  pairing-crate posture. Panic paths drop `Zeroizing` buffers on unwind.
- **Fail-closed A-mode**: foreign manifest owner, missing credential,
  wrong-credential-pubkey, tampered manifest, empty pin list, allowlist without
  pubkeys — all refuse before any publish/systemd action (18 integration tests
  exercise them). `--trust-owner` is run-scoped exactly as documented
  (`agent.rs:212-218`).
- **Retire convergence**: leaves converge on "not an active member", tombstone
  is a replaceable republish with the measured same-second tie-break fix, re-run
  detects `.env.retired` and exits 0 reprinting the owner-half command (e2e
  asserts all of it, including `.env.retired` mode 0600 and manifest survival).

## What's done well

- The e2e that PINS the relay's rejection of host-submitted owner events (with
  an owner-authenticated control) is exactly how a cross-service contract should
  be nailed down — it just needs its conclusion folded back into the B-mode code
  path (H-2).
- Retire's tie-break guard, kind:0 structural assert, and `.env.retired`-as-
  idempotence-token are all correctly derived from measured relay behavior, not
  pattern-matched.
- One redactor, one bundle parser, one atomic-write helper, one canonical-hex
  authority — the T1-audit "never grow a second parser" rule held across three
  crates.

## Residual disposition table

| ID | Severity | Finding | Disposition |
|---|---|---|---|
| H-1 | High | retire `--no-systemd` bypasses the live-agent guard entirely | rides #2688 |
| H-2 | High | B-mode 9000 submission structurally refused by relay; provision non-convergent | rides #2687 |
| H-3 | High | provision `--force` drops inexpressible env keys (grok: AGENT_ARGS, MULTIPLE_EVENT_HANDLING), no backup | rides #2687 |
| M-1 | Medium | provision 10100/kind:0 lack retire's same-second tie-break guard | rides #2687 |
| M-2 | Medium | A-mode never checks manifest.host_pubkey against this host | rides #2694 |
| M-3 | Medium | keygen `--force` (and B credential-replace) lack the live-unit guard | rides #2688 |
| L-1 | Low | failed partial provision demands --force, message mislabels state | rides #2687 |
| L-2 | Low | redactor misses key material fused into a mixed token | rides #2686 |
| L-3 | Low | retire overclaims convergence when credential hand-deleted | rides #2688 |
| L-4 | Low | no 429 retry in bridge client | rides #2687 |
| L-5 | Low | unit template assumed pre-installed | rides #2687 |
| L-6 | Low | normal-error tempfile unlink without shred | rides #2687 |

quality: model=claude-fable-5 effort=default stage=post-audit verdict=rework verify=ran commits=b5b14b61,552b8408,75a7a46e,88e34f11 findings=C=0/H=3/M=3/L=6 rework_rounds=0
