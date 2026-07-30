# Post-audit: T2 (#2685) live secrets migration + T1 (#2684) buzz-provision host-init

Reviewer: chuck (Fable 5). Date: 2026-07-30. Read-only on live systems; no live
unit restarted, nothing under `~/.config` or `~/services/buzz-agents` modified.
Specs applied as binding: `virtual-mint-v1-design.md` §2a, `sealed-mint-bundle-spec.md`
(key-handling rules, §5 owner pinning). Gates run: `cargo test -p buzz-provision`
in `/home/bryce/services/buzz-wt-provision` (hermit) — 12/12 green. Live-state
verification via `systemctl --user show/cat`, `loginctl`, journal grep, and a
key-value filesystem sweep (grep for the actual credential bytes, never echoed).

Verdict: **0 Critical / 1 High / 2 Medium / 6 Low / 3 Minor. No merge-blockers.**
T2's live posture and T1's crate are sound foundations for T3/T4, with the High
(stray key copies) requiring action via #2692 before the "secrets migrated"
claim is true, and two one-line T1 fixes recommended before T4 builds on the crate.

## Findings

### T2-1 (High, measured) — backup-hygiene sweep incomplete: live nsecs outside the .v1bak accounting — rides #2692

The migration's at-rest story accounts for `*.env.v1bak` (shred ~Aug 3, #2692).
A byte-level sweep for the actual key values found additional durable copies:

- **grok nsec in 13 files** under `~/.grok/sessions/%2Fhome%2Fbryce%2Fprojects/`
  (4 sessions, Jul 28–29: `chat_history.jsonl`, `updates.jsonl`,
  `terminal/call-*.log`; file modes 644/664, dir chain 755/775 — shielded only
  by `/home/bryce` = 750).
- **grok nsec plaintext** in
  `/tmp/claude-1002/-home-bryce-services/db34bba6-*/scratchpad/e2e-grok.json`
  (0664, Jul 29; an e2e deploy request with `private_key_nsec` inline).
- **claude + grok nsecs** in
  `~/.claude/projects/-home-bryce-services/db34bba6-*.jsonl` and **claude nsec**
  in `~/.claude/projects/-home-bryce-projects/c12e203f-*.jsonl` (0600).

These are exactly the durable/backup/sync-visible class T2 exists to end. Both
keys transited chat transcripts, so shredding is only partial containment —
**rotation at the T3/T4 re-mint is the only complete remedy**. Disposition:
route-to-task — extend #2692 scope to these paths; orchestrator/Bryce to rule
on rotate-at-re-mint. Filed as comment #6704 on #2685.

Verified clean in the same sweep: both live env files secret-free; user journal
0 hits since 07-25; `~/.bash_history` 0 value hits (8 benign `BUZZ_PRIVATE_KEY`
name mentions); `windows/`, `tools/` clean; `buzz-agent@.service.v1bak`'s
`BUZZ_PRIVATE_KEY` match is a comment line, not a value.

### T2-2 (Medium, read) — restart-loop variants beyond the known missing-credential case — rides #2685 (comment #6705)

`StartLimitIntervalSec=0` + `Restart=on-failure` + `RestartSec=5` loops forever
on ANY nonzero exit. Known case: missing credential (243). Unguarded variants:
(a) shim's own exit **78** for an empty credential file loops despite being a
deliberate, distinctive code — `RestartPreventExitStatus=78` would fail-stop it;
(b) credential present but malformed (spaces, interior junk after `tr -d '\r\n'`)
→ harness `Keys::parse` failure → generic nonzero exit → same infinite loop,
indistinguishable from a transient relay outage. (b) needs a harness exit-code
contract; fold into T6 (`BUZZ_PRIVATE_KEY_FILE` upstream patch). Disposition:
route-to-task (unit tweak + T6 note).

### T2-3 (Low, read) — shim strips only `\r\n`, not spaces/tabs — accept (rides #2685, comment #6705)

A hand-edited credential with a trailing space passes the shim's non-empty check
and dies in `Keys::parse`, entering the T2-2 loop. T4 writes credentials
byte-exact (`printf %s`), so accept.

### T2-4 (Low, read) — LoadCredential snapshots at unit start — accept, T4/T5 note (rides #2687)

Rewriting `creds/%i.nsec` does nothing to a running unit; re-provision flows
must restart the unit (daemon-reload is insufficient). Not written down in the
design's provision steps. One line in T4/T5's flow.

### T2 clean surfaces (checked, not assumed)

- **Template v2 vs v1** (diffed against `.service.v1bak`): pure superset —
  PATH, WorkingDirectory, Restart, RestartSec, TimeoutStopSec, SyslogIdentifier,
  Type=exec, WantedBy all preserved; only LoadCredential + shim ExecStart added.
- **Live state** (measured): both units on the v2 template
  (`FragmentPath`/`ExecStart` = shim), `NRestarts=0`, active since 19:53/19:57;
  no drop-ins; only the two expected instances enabled.
- **Boot path**: `Linger=yes`; `~/.config` and `~/services` on the same fs
  (`/dev/root`); credential source read directly at unit start — no reboot
  ordering hazard.
- **Shim internals**: `set -eu` + `:?` guard behave correctly on missing
  `CREDENTIALS_DIRECTORY`; command-substitution failure aborts; `exec` preserves
  PID so Restart/TimeoutStop/SIGTERM semantics are unchanged; stale+fresh
  credential coexistence impossible (single id `nsec`, per-service dir wiped
  each start).
- **Migration consistency** (measured): each `creds/*.nsec` is byte-identical to
  the corresponding old env value — no divergence between rollback files and
  live credentials.

### T1-1 (Medium, read) — owner-pin append requires rotating the host key — rides #2684 (comment #6706) / #2687

`host-init` refuses when `host.key` exists (`main.rs:291-299`) and the refusal
message steers to `--force` — which mints a NEW host identity (permanently
orphaning every sealed blob) as a side effect of merely wanting to pin a second
owner. The commit message advertises "appends and dedups when the pin file
already exists", but that path is unreachable on a healthy initialized host
without `--force`. Judgment call for the orchestrator:

- (a) allow owner-append without `--force` when `host.key` exists — smallest
  change, but overloads `host-init` semantics;
- (b) explicit `pin-owner`/`--add-owner` subcommand (T4/T5) — cleanest;
- (c) rely solely on spec §5's provision-time `--trust-owner` — already planned,
  but per-provision trust extension is not the same as a durable pin.

Default: (c) exists in spec. **Recommendation (confidence: high): (b) later,
plus fix the refusal message now** so it stops pointing owner-changes at
`--force`.

### T1-2 (Low, read; 1-line fix) — refuse-if-exists is TOCTOU with a clobbering rename — rides #2684 (comment #6706)

`rotating = key_path.exists()` (`main.rs:290`) then `tmp.persist()`
(`main.rs:187`, rename semantics, clobbers). Two racing `host-init`s both pass
the check; the loser's printed pubkey no longer matches disk, and blobs sealed
to it are discovered dead only at provision time. `persist_noclobber()` on the
non-`--force` path closes it. Low likelihood (single-operator, run-once), fix
is one line — fix-now.

### T1-3 (Low, read; 1-line fix) — hidden test flag is env-activatable at runtime — rides #2684 (comment #6706)

`--config-dir` is `hide = true` but `env = "BUZZ_PROVISION_CONFIG_DIR"`
(`main.rs:75-81`). A stray env var silently redirects the trust root (pin list
+ host key) for `host-init`, `--show`, and — since T4 lands in this same binary
— future provision runs. Tests don't need it (they pass the flag explicitly and
`env_remove` the var, `tests/host_init.rs:21`). Drop `env =` — fix-now.

### T1-4 (Low, measured) — no-secret-echo is not total across argv error paths — rides #2684 (comment #6707), accept+document

The binary's own error paths are clean (all `bail!`/`context` sites print
lengths and paths only; `from_hex` errors deliberately swallowed at
`main.rs:131-136`, `245-251`). But clap's argv layer echoes:
`buzz-provision host-init <pasted-value>` → `error: unexpected argument
'nsec1FAKE…' found` (reproduced against the built binary). A secret pasted
without its `--allow-owner` prefix lands on stderr — and stderr lands in
session transcripts, which T2-1 shows is a real leak channel. Options: accept +
document, or pre-scan argv for 64-hex/`nsec1` positionals before `Cli::parse`.

### T1-5 (Low, read) — crash-orphaned tempfile can hold the new secret — rides #2687 (T4 sweep note)

Hard crash (SIGKILL/power) between `write_all` and `persist` leaves
`.buzz-provision.XXXXXX` (0600) containing the new secret in the config dir;
nothing ever sweeps the prefix. Normal error paths are covered by
NamedTempFile's drop-unlink. T4 should sweep `.buzz-provision.*` at start.

### T1 Minor

- Pin-file parser accepts blank lines but has no comment syntax; any `#` or
  uppercase line hard-fails both append and `--show` (fail-closed — acceptable;
  document, and T4 must reuse `merge_owners`, not grow a second parser). (no-finding severity: Minor, rides #2687)
- Secret erasure on `Keys`/`SecretKey` drop is secp256k1 `non_secure_erase`
  (nostr-0.44.6 `secret_key.rs:162`) — best-effort by its own name; acceptable
  at process-exit lifetime alongside `Zeroizing` on the hex buffer.
- Test gaps: no e2e for a pre-existing pin file with blank/invalid lines, none
  for `--show` on a non-canonical `host.key` (trailing newline), none proving
  the env-var override works. All are defensive paths that exist in code.

### T1 clean surfaces (checked, not assumed)

- **Atomic write**: tempfile created in the destination dir (same-fs rename
  guaranteed), 0600 set before content, fsync file → rename → fsync dir
  (`main.rs:170-195`). Correct.
- **Write order under interruption** (traced both paths): fresh init crash
  between owners and key leaves owners-only — plain re-run repairs (merge is
  idempotent), `--show` gives correct guidance. `--force` crash leaves OLD key
  + merged owners — coherent (old identity intact), re-run completes. The
  owners-first deviation delivers what it claims.
- **Zeroization** (verified at crate source): `sk_hex` in `Zeroizing`;
  `Keys`/`SecretKey` Drop impls erase (nostr-0.44.6 `key/mod.rs:251`,
  `key/secret_key.rs:162`); `to_secret_hex` returns a moved String (no lingering
  heap copy); no serde/`format!` of secret material anywhere in the crate; no
  Debug/Display derives on secret-bearing types.
- **Owner parsing**: dedup is safe against case games because validation
  rejects anything but lowercase hex before comparison; `PublicKey::from_hex`
  enforces curve validity.
- **Tests**: 12/12 green (ran). The no-leak test's sliding-16-char-window
  design is genuinely strong.

## What's done well

- The sealed-bundle spec's "no value ever echoed" rule is engineered, not
  asserted: length-only errors, deliberately swallowed library errors, and a
  sliding-window leak test across all four CLI surfaces.
- Owners-before-key write order with the key as the "initialized" marker is a
  correctly-reasoned crash-consistency design, and it holds under trace.
- The shim is minimal, documents *why* it exists (env-file ordering, CRLF
  class), and preserves PID semantics via exec.

## T4 bite list (consolidated — no fixes now)

1. Empty/absent `allowed-owners` MUST be refuse-all in unseal step 5; the file
   can legitimately be absent while `host.key` exists.
2. Credential rewrite requires unit restart (LoadCredential snapshot).
3. Reuse `merge_owners` for `--trust-owner`; never a second parser.
4. `BUZZ_PROVISION_CONFIG_DIR` reaches runtime unless T1-3 is fixed.
5. Sweep `.buzz-provision.*` orphans at start (T1-5).

## Residual disposition table

| ID | Severity | Finding | Disposition |
|---|---|---|---|
| T2-1 | High | Stray live-key copies (~/.grok sessions ×13, /tmp scratchpad, 2 Claude transcripts) outside .v1bak accounting; rotation the only full remedy | rides #2692 |
| T2-2 | Medium | Restart-loop variants: shim exit 78 + malformed-cred harness exit both loop forever; RestartPreventExitStatus=78 + T6 exit-code contract | rides #2685 |
| T1-1 | Medium | Owner-pin append unreachable without --force key rotation; refusal message steers at --force | rides #2684 |
| T2-3 | Low | Shim strips only \r\n | rides #2685 |
| T2-4 | Low | LoadCredential snapshot ⇒ restart on re-provision | rides #2687 |
| T1-2 | Low | TOCTOU clobbering rename on refuse-if-exists; persist_noclobber | rides #2684 |
| T1-3 | Low | BUZZ_PROVISION_CONFIG_DIR env-activates hidden flag at runtime | rides #2684 |
| T1-4 | Low | clap argv errors echo pasted values verbatim (measured) | rides #2684 |
| T1-5 | Low | Crash-orphaned secret-bearing tempfile never swept | rides #2687 |

quality: model=claude-fable-5 effort=default stage=post-audit verdict=merge-safe verify=ran commits=20cf97df findings=C=0/H=1/M=2/L=6 rework_rounds=0
