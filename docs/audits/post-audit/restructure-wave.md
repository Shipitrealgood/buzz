# Post-audit: restructure wave (B-strip, crate collapse, class fixes)

Reviewer: chuck (Fable 5). Date: 2026-07-31. Worktree `feat/buzz-provision`.
Scope note (dispatch defense): the dispatch named range `2267003b2..5d01a2d62`
"(8 commits)", but that range yields 7 commits and excludes `2267003b2` itself
— which IS the B-mode deletion commit the shape summary names. Audited
**`f97e12258..5d01a2d62` (8 commits, B-deletion included)** as the evident
intent. Binding refs applied: `mint-pipeline-full.md` (prior audit),
`restructure-wave-spec.md` **with its Phase-4 RECALIBRATION banner** (H-3 =
env-ownership split, not carry-forward), `upstream-primitive-map.md`.
Hard boundaries honored: nothing under `~/.config/buzz-provision` or
`~/services/buzz-agents` touched; production relay untouched; local-relay e2e
only (ports 3034/3035). The "host's installed template is v2 without the
local.env line" premise is taken from the dispatch (could not be verified
without touching the live deployment dir) and is labeled as such below.

Gates run (measured): `cargo test -p buzz-provision` **84/84** green (lib 2,
buzz-mint bin 5, main 11, agent_provision 14, bundle_modes 6, host_init 10,
keygen 5, mint 10, retire 8, sealed_bundle 13; dispatch said "85 tests" —
measured 84 non-ignored). `cargo clippy -p buzz-provision --all-targets`
clean. Both local-relay e2e (`e2e_local_relay -- --ignored`, dockerized
postgres/redis, relay binary from this tree) **2/2** green, including the
new `.pub`-survives-retire assertions and the real owner-consent 9035 path.

Verdict: **0 Critical / 2 High / 3 Medium / 7 Low.** No merge-blocker for
landing the branch. **H-1 must land before the #2692 live-agent migration**
(it re-opens the original audit's H-3 through the tool's own instructions);
H-2 should be fixed before anything scripts against the exit-code contract.
All 11 claimed resolutions from the prior audit verified real in code (table
at the end); two of them (env split, exit codes) have the holes described in
H-1/H-2.

---

## High

### H-1 (High, read + dispatch-premise; fix before #2692) — provision asserts the local.env override works but never verifies the installed template honors it — rides #2692

The env-ownership split (the recalibrated H-3 fix) is correctly implemented
on the file side: `agent.rs:158-163` + `render_env_file` (`agent.rs:501-546`)
make provision the single writer of `<name>.env`, and retire never touches
`<name>.local.env` (read-verified: no code path writes it; `retire.rs:344-349`
documents it). But the merge half of the contract lives in the systemd
template, and the tooling only checks the unit RESOLVES, not what it reads:
`unit_exists` (`systemd.rs:73-79`) is `systemctl --user cat` exit-status only.
Meanwhile the success summary (`agent.rs:624-627`) unconditionally prints
"operator env: … overrides the generated file", and the phase-6 refusal text
(`agent.rs:393-402`) describes the v3 template as a prerequisite in prose only.
No `buzz-agent@.service` template ships anywhere in this repo (measured: no
`*.service` outside hermit checkouts), so template v3 exists nowhere but the
spec.

Trace (on the dispatch's stated host state — installed template v2, no
`EnvironmentFile=-…local.env` line): operator follows the tool's own printed
guidance / the spec's #2692 step and moves grok's `BUZZ_ACP_AGENT_ARGS` +
`BUZZ_ACP_MULTIPLE_EVENT_HANDLING` into `grok.local.env`; provision runs
green, prints "overrides the generated file"; the unit never reads the file;
the agent comes back healthy with silently changed behavior. That is the
original H-3 failure mode, verbatim, resurrected through instructions instead
of clobbering. The spec's own Phase-6 claim ("#2692 … now safe: EnvFile
carries their keys forward") is false until template v3 is installed.

Fix shape: (a) in Phase 6, parse `systemctl --user cat buzz-agent@.service`
output (already fetched by `unit_exists`) for the `%i.local.env`
EnvironmentFile line; hard-fail (or at minimum demote the "overrides" claim
to a loud warning) when absent — provision already refuses on a missing
template, this is the same gate reading one more line; (b) structural: the v3
template is an unowned artifact — land it in-repo (deploy/agents is the
primitive-map's named ladder item) so the contract is checkable, not oral.

### H-2 (High, read) — exit-code contract severed on every retire-side network path; clap errors exit 2 — rides #2697

`exit_code_for` (`lib.rs:43-57`) walks `err.chain()` for a `BridgeError`, but
three call sites format the `BridgeError` into a string, destroying the chain:

- `retire.rs:302` — `anyhow!("publishing {} failed: {e}")`: a **Dominated
  tombstone exits 1**, not 5. This is precisely the "`duplicate:` = LOSS,
  exit-5 semantics" case spec Phase 4.3 names; the semantics hold only on the
  provision path (`agent.rs:348` correctly uses
  `.map_err(anyhow::Error::from).with_context(…)`).
- `retire.rs:222` — membership discovery failure: network/auth → exit 1.
- `buzz-mint.rs:636-645` — the 9035 archive rejection: relay refusal/403 →
  exit 1, so the owner-half revocation can never report auth (3) or relay (2).

Additionally: clap parse errors exit with clap's own code
(`main.rs:178`/`buzz-mint.rs:195` `e.exit_code()` = **2**), colliding with
the contract's "2 = network/relay"; buzz-cli's prior art maps usage errors to
1 (`buzz-cli/lib.rs:40-46`, `error.rs:90-92`). And every non-bridge failure
(systemd, journal timeout, io) exits 1 ("input") where the contract says 4
("other") — buzz-cli reserves 1 for usage/not-found (`error.rs:105-107`).

Fix shape: replace the three formatting sites with the agent.rs idiom (one
line each); map clap `use_stderr()` errors to exit 1; default non-bridge
errors to 4 (or document divergence). Test: assert exit codes, not just
stderr, in the existing unreachable-relay tests (`tests/retire.rs:231-253`
already produces the exact scenario and only checks `!success()`).

## Medium

### M-A (Medium, read) — bridge redirect policy unset; two spec-4.5 hardening items unimplemented — rides #2697

`bridge::client()` (`bridge.rs:60-70`) never sets a redirect policy, so
reqwest follows up to 10 redirects; `x-auth-tag` is a custom header and is
forwarded across ALL redirects including cross-host — the primitive-map §2
item "x-auth-tag never forwarded across redirects" (spec 4.5) is not
implemented. Mitigating (why not High): the attestation is public data (it is
embedded in the published kind:0), and the NIP-98 `u` binding makes a
redirected POST fail at the target anyway — so the realistic impact is
confusing failures, not credential leak. `redirect(Policy::none())` is one
line and matches the bridge's direct-to-relay reality. The second unshipped
4.5 item: the exactly-one-auth-tag post-sign assert exists nowhere on the
kind:0/10100 signing path (`agent.rs:289-306`) — only in a buzz-mint test.

### M-B (Medium, read) — the live-unit guard's WIRING has no regression test — rides #2697

Spec Phase 5 required: retire with `--no-systemd` refuses on an ACTIVE unit;
keygen `--force` refuses on an active unit; spawn-failure degrades to warning
(mock systemctl). What shipped is a pure decision-table test
(`systemd.rs:127-154`) — good, but it cannot catch the regression class H-1
(original) was: a call site dropping or conditionally skipping the guard
(`retire.rs:157`, `main.rs:352-358`). All current integration tests run where
the probe returns "inactive", so the refusal branch has never executed
end-to-end. The guard reads `systemctl` off PATH — a test-local fake systemctl
prepended to PATH makes all three spec cases testable cheaply.

### M-C (Medium, read) — `post_event`'s Dominated / retry behavior untested — rides #2697

Spec Phase 5 required "`duplicate:` response surfaces as error not success".
`bridge.rs` tests cover only `parse_retry_hint_secs` (`bridge.rs:334-343`);
`post_event` — the function carrying the Dominated detection
(`bridge.rs:243-247`), the accepted:false mapping, and the 429/5xx retry loop
— has zero tests (no mock server in the dev-deps). The M-1-class guarantee
("published means live") currently rests on code review alone. A
`std::net::TcpListener`-based one-shot responder (no new deps) covers
Dominated, Rejected, 429-with-hint, and per-attempt re-sign.

## Low

### L-a (Low, read) — `read_secret_file` omits the owner-uid check — rides #2697
`util.rs:234-260` implements O_NOFOLLOW + O_NONBLOCK + fstat + regular-file +
`mode & 0o177 == 0` but not the owner-uid comparison spec 4.7 named from the
git-sign-nostr discipline (`meta.uid() == geteuid()`). Marginal on a
single-user host; one line to close the spec drift.

### L-b (Low, read) — `replaceable_created_at` fallback can fail a legitimate same-second re-run loudly — rides #2697
`bridge.rs:293-328`: when the head query fails but publish works, the event
signs at `now`; a same-second live head then yields `duplicate:` → Dominated
→ exit(-5-intent) failure of a legitimate re-provision/retire. Fails LOUD,
converges on retry, and a first provision is safe (no live head → nothing to
dominate) — so this is accepted behavior worth one sentence in the Dominated
error hint ("re-run usually succeeds"). Adversarial question from dispatch
answered: no false Dominated on first provision.

### L-c (Low, read) — stale `.pub` marker lockout has no recovery hint — rides #2697
Keygen skips writing the marker when the env dir doesn't exist
(`main.rs:426-435`); under name reuse this leaves a stale marker that makes
retire refuse (`retire.rs:409-417` — correct fail-closed direction). The
refusal message names neither the marker file as the mismatch source's other
half nor the recovery step (inspect both, remove the stale marker by hand).

### L-d (Low, read) — comment drift — rides #2697
`e2e_local_relay.rs:814` says "`.pub` removed" directly above assertions
pinning its survival (825-831); `retire.rs:532-534` carries a doc comment
describing `unit_active_state` on `print_summary`.

### L-e (Low, read) — doc-scrub (spec 4.10) stopped at src/ — rides #2697
`src/` is clean (grep-verified); tests still carry auditor/task refs
(`agent_provision.rs:536,559` "chuck T1-4/T1-1", `e2e_local_relay.rs:31`
"#2688").

### L-f (Low, read) — local.env non-touch proven for provision only — rides #2697
`reprovision_regenerates_env_and_never_touches_operator_file`
(`agent_provision.rs:394-430`) covers provision re-runs (dry-run). Spec Phase
5 asked for the full provision→retire cycle with a populated local file.
Retire has no code path that touches the file (read-verified), so the gap is
coverage, not behavior; adding the local file to the retire e2e fixture is
two lines.

### L-g (Low, read) — `credential_pubkey` bypasses the hardened reader — rides #2697
`util.rs:301-308` reads the credential with plain `fs::read` (keygen refusal
path) — no O_NOFOLLOW/mode discipline. It never emits the secret, and the
creds dir is 0700, so exposure is minimal; routing it through
`read_secret_file` unifies the class.

## Prior-audit resolution verification (each checked against code, not commit messages)

| Prior finding | Claimed | Verified | Evidence |
|---|---|---|---|
| H-1 guard bypass via `--no-systemd` | fixed | **yes** (wiring untested → M-B) | `retire.rs:154-157` guard before anything moves; `systemd.rs:58-69` degrades only on spawn failure; `--force-live` sole override |
| H-2 B-mode non-convergent | deleted | **yes** | `bundle.rs:180-184` refuses mode=b; serde default B (`bundle.rs:35-42`) refuses pre-mode manifests fail-closed; `mint new` gone; e2e test 1 replaced |
| H-3 `--force` drops env keys | env split | **yes on files, hole on template** → H-1 above | `agent.rs:158-163,501-546`; `--force` removed from provision entirely (clap rejects it — `agent_provision.rs:294-304`) |
| M-1 same-second 10100 tie | fixed | **yes** | one authority `bridge.rs:293-328`; provision `agent.rs:274-286` + retire `retire.rs:227-233` both route through it; Dominated surfaced `bridge.rs:243-247` (untested → M-C) |
| M-2 host_pubkey unchecked | fixed | **yes** | `agent.rs:181-189` fail-closed, no override, pre-publish; test `agent_provision.rs:253-272` |
| M-3 keygen `--force` unguarded | fixed | **yes** (wiring untested → M-B) | `main.rs:352-358`, no-override refusal (`systemd.rs:146-154` proves no flag advertised) |
| L-1 partial provision demands --force | fixed | **yes** | idempotent regenerate, `agent_provision.rs:394-430` |
| L-2 fused-token redaction | fixed | **yes** | ≥32-hex-run + bech32 substring, `seal.rs:65-104`, tests `seal.rs:617-632` |
| L-3 convergence overclaim | fixed | **yes** | `retire.rs:306-316` "cannot be re-verified" |
| L-4 no 429 handling | fixed | **yes** | `bridge.rs:106-189`: 3 attempts, per-attempt re-sign (fresh created_at+nonce answers the ±60s/replay-guard question), hint capped 30s (`bridge.rs:115-122`) |
| L-5 template assumed installed | fixed | **partially** → H-1 | `agent.rs:391-403` detects + instructs; instruction references a template version that exists nowhere checkable |
| L-6 tempfile unlink | accepted | unchanged | `util.rs:136-160` |

## Adversarial-focus answers not already covered

- **`.pub` survives retire / name reuse**: keygen (fresh or `--force`)
  overwrites the marker when env_dir exists (`main.rs:424-435`, test
  `keygen.rs:231-246`), and provision rewrites it from the manifest
  (`agent.rs:258-263`), so the stale-marker window is only the
  keygen-without-env-dir path (L-c). Retire's marker↔credential cross-check
  fails closed in that window — correct direction.
- **Idempotent regenerate opening anything**: re-provision with a different
  pinned owner's bundle over the same name legitimately re-attests; the
  credential↔manifest derivation gate (`agent.rs:434-463`) still binds the
  identity, and provision touches no credential. No hole found.
- **`unit_exists` template-vs-instance**: it probes the INSTANCE
  (`unit_name(name)`, `agent.rs:391-392`); `systemctl cat` on an instance of
  an uninstalled template exits non-zero → correct detection direction. The
  gap is content, not existence (H-1).
- **Collapsed crate's public surface**: `lib.rs` exposes `bridge`, `bundle`,
  `seal` (`lib.rs:25-27`). `seal`'s unwired seal/unseal are doc-justified
  (`seal.rs:3-13`, spec Phase 1 required exactly this), fail-closed, and
  fully tested (sealed_bundle 13/13). No CLI reaches them (grep-verified).
  Liability accepted by ruling; nothing new to flag.
- **Retry vs NIP-98 window**: worst case 2×30s hint sleeps with re-sign
  before each attempt (`bridge.rs:137-139`) — every attempt's auth event is
  fresh; no window interaction. Nonce per attempt defeats the replay guard.

## What's done well

- The B-mode serde default (`#[default] B` → refused at load) turns every
  pre-mode manifest into a precise refusal instead of a silent A-mode parse —
  fail-closed by construction, not by check.
- `bundle_attested_for_another_host_fails_closed` and the no-bypass-flag
  structural argument (provision has no `--force` for clap to even reject
  past) is the right way to close M-2 permanently.
- The retire e2e forcing the REAL owner-consent path (owner seeded as plain
  `member`, separate relay operator key) pins the one contract that would
  otherwise silently degrade to the admin shortcut.
- One timestamp authority with the loss surfaced as a typed error
  (`Dominated`) is exactly the "fix the class" shape the spec demanded.

## Residual disposition table

| ID | Severity | Finding | Disposition |
|---|---|---|---|
| H-1 | High | template-v3 merge contract unverified; "overrides" claim + #2692 guidance unsafe on v2 hosts; v3 template unowned/un-shipped | rides #2692 |
| H-2 | High | BridgeError chain severed in retire.rs:222,302 + buzz-mint.rs:636 (Dominated→1 not 5); clap exits 2; non-bridge errors 1 not 4 | rides #2697 |
| M-A | Medium | no redirect policy (x-auth-tag crosses redirects); post-sign single-auth-tag assert absent | rides #2697 |
| M-B | Medium | live-unit guard wiring (active-unit refusal, spawn-degrade) untested at call sites | rides #2697 |
| M-C | Medium | post_event Dominated/Rejected/429 paths have zero tests | rides #2697 |
| L-a | Low | read_secret_file lacks owner-uid check | rides #2697 |
| L-b | Low | replaceable_created_at fallback → loud Dominated on same-second re-run | rides #2697 |
| L-c | Low | stale .pub marker refusal lacks recovery hint | rides #2697 |
| L-d | Low | comment drift (e2e:814, retire.rs:532) | rides #2697 |
| L-e | Low | doc scrub incomplete in tests | rides #2697 |
| L-f | Low | local.env non-touch not asserted across retire | rides #2697 |
| L-g | Low | credential_pubkey bypasses hardened reader | rides #2697 |

quality: model=claude-fable-5 effort=default stage=post-audit verdict=rework verify=ran commits=2267003b2,217a8ee8e,0427770e7,278c2fdf8,0df902830,1200f9aeb,d48804f76,5d01a2d62 findings=C=0/H=2/M=3/L=7 rework_rounds=0
