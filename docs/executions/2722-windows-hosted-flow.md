# #2722 Windows hosted flow execution

Status: in progress. Executor owns source/tests exclusively in
`/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`. Clean starting revision:
`0f7927f24bb79f9a3592dc80a5dbe56ed0f13717`.

Requested model/reasoning: Astra/high. Runtime identity is not exposed by the
harness. Scope and acceptance: [ratified contract](../specs/2722-windows-hosted-flow.md)
and [execution handoff](../briefs/2722-windows-hosted-flow.md).

Native bridge/owner intents and tests first; real shared form/continuation and
tests second. Default-off remains enforced natively. No live, board, relay,
hostd, schema, installation, push, merge or PR actions are authorized.

All verification gates are NOT RUN at this initial checkpoint. Windows/native,
full CI and human gates require separate evidence; browser fixtures are source
workflow evidence only.

## Scope checkpoint (conductor ruling, #2722 comment 11384)

The original +900–1,500 / −100–250 production estimate was low. After the
first formatted native/owner packet measured +1,041 / −165 (net +876), the
conductor approved a revised provisional **+1,700–2,500 / −200–400** production
range for the same F1–F4 contract. This is scope accounting, not a quota.
No store/protocol/hostd/relay/schema expansion was introduced. Approximately
70 metadata-constructor lines were factored/shared; about 80 existing recovery
lines were rewritten (inferred allocation, not a measured net saving).

The two production siblings are `crates/buzz-paired-owner/src/deployment/continuation.rs`
and `desktop/src-tauri/src/commands/agents_hosted_commands.rs`. They contain
existing-journal continuation/current-operation validation and sanitized native
DTO/command projection. The native list was revised to read validated local
intents without network fan-out: unavailable hosts do not hide other setups.
Explicit actions alone obtain current permission/evidence. A Windows-only
native test sibling uses the existing confined-child/main-create fixture pattern.

Measured owner gate: `cargo test -p buzz-paired-owner deployment -- --test-threads=1`
passed **2 tests**, including real Device and Management with an isolated loopback
WebSocket peer. First run failed at sandbox loopback bind; approved escalation
allowed the fixture. A fixture reply incorrectly used a 15-second lifetime;
production correctly rejected it against the existing 10-second bound. Corrected
the fixture to `wire::PERMIT_SECONDS`, then both tests passed. After atomic
phase/result transaction changes both passed again (3.58 seconds test runtime).

Native checks are NOT RUN: installed MinGW compiler `/usr/bin/x86_64-w64-mingw32-gcc`
exists but the Rust 1.98.1 Windows GNU target library directory does not. No target
or dependency installation was attempted. Prior Linux Tauri GLib/GObject blocker
remains; Linux would not compile Windows-gated commands anyway. Windows-only
authored tests are not native acceptance evidence until run on Windows.

First-group final available checks: owner deployment suite **2 passed, 0 failed**
(3.88 seconds), now including same-token concurrent retention and missing-model /
stale / revoked / unavailable negatives. A test initially attempted `Command::clone`
(the secret-bearing command deliberately is not Clone); corrected it to restore the
candidate through the production protected-command function. Scoped Rustfmt,
Tauri formatting and `git diff --check`: PASS. Differential desktop file-size
check with `CHECK_FILE_SIZES_BASE=0f7927f24bb79f9a3592dc80a5dbe56ed0f13717`:
PASS after approved escalation for sandbox-denied Node→read-only-git subprocess.
These checks do not compile or execute Windows-only source. V2/V3 acceptance
coverage remains incomplete until the final finite matrix is accounted for.


## Native follow-on verification checkpoint

First source commit: `5b0404522` (native bridge/metadata/existing-intent continuation).
The expanded existing owner journey now passes **2/2** (4.68 seconds): real
WebSocket EVENT replay after denied publication and reopen; exact signed bytes;
expired unadmitted replacement; expired admitted receipt reconciliation without
resend; no revision-zero reset after prior admission; expired observations;
post-await signer change; bounded 1,024-row capacity refusal; and two simultaneous
different identity candidates for one new operation with exactly one journal
winner at phase zero. This remains a transport-boundary fixture, not a live relay.
The additional expired-admitted guard checks retained phase/result as well as
initial revision. Native setup projection also withholds unit/observation on
historical placement or stale current runtime/catalog mapping.

Windows native tests now author interrupted follow-on model-save recovery using
an exact signed/encrypted retained fixture row, actual main record recovery and
same key/auth-tag assertions, including refusal to recover the older root.
Execution of this native case remains **NOT RUN**. It does not independently
exercise the Windows top-level transport command or establish native acceptance.


Final native expiry correction: the previous transaction's peer check blocked
repeat replacement after successive expired/unadmitted initial attempts. Current
continuation explicitly reads each expired same-revision peer through the existing
operation query and passes only verified-absent IDs to the transaction. Local
phase/expiry/no-receipt alone never proves non-admission. The transaction still
rejects any new/unverified peer. The lost-admission-reply negative stores the
admitted peer receipt and refuses replacement. Latest native lookup can select an
expired/verified-absent candidate for explicit refresh, but rejects known/historical
admission conflicts. Owner gate **2/2 PASS**, 5.24 seconds. Each existing query has
a five-second deadline; enumeration is capped at 1,024 retained intents. There is
no aggregate action deadline or automatic retry: pathological same-revision peer
sets can require many sequential reads, and an unavailable read fails the action.
