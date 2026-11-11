# #3721 credential quoting repair

2026-11-11. **SOURCE AND NONLIVE PACKAGE PREPARED FOR INDEPENDENT REVIEW.**
Native credential-value interpretation remains unproved; live approval required.
Executor `/root/hostd_credential_fix`, codecodex; requested Astra/high, actual
served model/reasoning metadata not exposed. Execute and brief applied.

## Scope and source

Worktree `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`. Clean baseline
`09bbb43096fae862dee9b3f03e4cbcc9295c7d24`; no unrelated dirty files.
Implementation/test DCO commit `46f53f692a0d03779bd9e9168d6e1b986b32c1d1`.
Only user_service.rs, its existing fixture file, this receipt and review brief.
No live configuration/state, keys, providers, services, external network, board,
push or merge. Existing Claude/Grok/Scout excluded. Parent owns independent gate.

Inputs: narrow approved dispatch, [ratified trusted-account contract](../specs/3722-main-discovery-native-login.md),
[accepted executor receipt](3721-trusted-host-executor.md),
[pilot preparation](3721-trusted-host-pilot-preparation.md),
[live activation receipt](3721-netcup-live-activation.md), project AGENTS,
VISION/agent/remote-agent intent, architecture registration section, TESTING,
portable conductor guidance. Authorization/journal/account model remains accepted.
No new product/architecture judgment, path language, or framework.

## Failure evidence and repair

**Conductor-measured, supplied:** Windows test.26 created one disposable Claude,
Jeff, public key `d8b6601f766c2275631332d99d2f22aefef1279f7a244faa0a2c2228775f0ba6`.
Unit `buzz-hosted-agent@f3f8a7cc742088673e11e48aef77f484a5b581650d647cc6a3abdd851809f986.service`
fails243/CREDENTIALS before runtime. D-Bus LoadCredential a(ss) has leading literal
quotes on IDs and trailing quotes on source paths. Actual files exist0600 inside
0700 directories under `/var/lib/buzz-host-registration/hosted/<id>`.
No live reads repeated. This is serializer evidence, not login failure.

**Read:** install quoted each whole `name:/path` with the ExecStart helper.
Installed systemd257 parser call sequence splits at the first colon and applies
specifier expansion to the remaining path, without shell quote/backslash
unescaping. A bounded exported-symbol/disassembly inspection informed that
reading; no private ABI calls or parser harness were built. New output keeps the
source path literal except `%` → `%%`; ExecStart keeps its distinct escaping.
The same non-UTF8/control rejection remains, including newline/tab. Fixed final
credential names prevent trailing whitespace/backslash line-trimming issues.

`install` now compares the complete existing override against new bytes OR the
exact prior generator output for the same local paths/unit. Existing prepare
identity/revision/location/inactivity fences precede install. Arbitrary edits,
foreign source/credential names, changed ExecStart and mixed extra content still
refuse. Same-instance reopen/retry retains identity; no unconditional overwrite.
Registration, owner/auth, journal and cleanup implementation are unchanged.

**Measured source delta:** production +35/-4; existing tests +148/-0;
implementation commit +183/-4 (literal Git counts).
New tests cover two path shapes including spaces, percent, backslash, quote,
dollar and colon; exact old repair, idempotent new replay, identity preservation,
five foreign shapes and no start; control/non-UTF8 refusal; optional offline
systemd syntax. Mutation checks bind the generator and ownership guard.

## Finite verification

Commands used exported `HERMIT_SELF_UPDATE=false`, `. ./bin/activate-hermit`,
Cargo `--offline --locked`. Disposable local socket tests used narrow approved
sandbox escalation. No installs or toolchain updates. Toolchain rustc/cargo1.98.1.

| Command / gate | Measured result |
| --- | --- |
| `cargo test --offline --locked -p buzz-hostd --lib credential_ -- --test-threads=1` | PASS2; repeated after exact mutation restoration |
| `cargo test --offline --locked -p buzz-hostd --all-targets -- --test-threads=1` | PASS library28, activation1, controller13; library3 ignored. Runtime/examples0 cases. Library8.72s, controller34.09s |
| `cargo test --offline --locked -p buzz-hostd --lib generated_credentials_parse_offline -- --ignored --test-threads=1` | PASS1; native systemd --user --test, private HOME/runtime/unit path, generators disabled; no service execution |
| `BUZZ_TEST_RUNTIME=<frozen>/buzz-host-runtime BUZZ_TEST_ACP=<frozen>/buzz-acp cargo test --offline --locked -p buzz-hostd --lib actual_user_runtime_prepare_refresh_selected_conversation_and_cleanup -- --ignored --test-threads=1` | PASS1, existing synthetic provider/manager fixture; expected runtime refusals included |
| Three source mutations, each focused regression | Old output, removed legacy repair, and unconditional foreign acceptance each DETECTED exit101 with failed assertion; exact source restored in finally; focused2 then pass |
| `cargo fmt -p buzz-hostd -- --check`; `git diff --check` | PASS |
| `cargo clippy --offline --locked --no-deps -p buzz-hostd --all-targets` | Exit0; existing request/onboard/transport/CLI warnings only, none in changed files. Strict warning-free lint not claimed |
| `cargo build --offline --locked --release -p buzz-hostd --bin buzz-hostd` | PASS optimized23.67s |
| Candidate `buzz-hostd --help`, `registration run --help` | PASS exit0 each; no state/provider execution |
| OPERATOR four shell blocks `bash -n` | PASS syntax only; none executed |
| Candidate manifest7/7; original frozen manifest8/8 | PASS; original artifacts unchanged |

**Native limits:** initial sandbox --test hit SO_PASSCRED EPERM; disposable-socket
escalation succeeded. First full generated fixture lacked basic.target; adding
private basic/shutdown targets resolved that fixture dependency. Two bounded
private manager startup probes exited1 without diagnostics and ended. No
persistent manager/scaffolding or live-manager calls. Native --test omits parsed
credential values and accepts even the known broken form: it proves syntax only.
Per conductor, exploration stopped rather than extending private ABI tooling.
Actual corrected a(ss) values and credential delivery are **NOT RUN**, not green.
The finite next probe is in the rollout below and requires live approval.

**Other NOT RUN:** full just ci, strict -D warnings lint (accepted baseline warnings),
Windows/native builds, real provider/login/models/chat, Jeff retry/Cleanup, live
rollback, reboot or human acceptance. Independent review still required. No PR
readiness/`buzz-review-completed` claim.

## Hostd-only artifact and continuity

Candidate root:
`/home/bryce/services/buzz-agents/releases/3721-credential-quoting/46f53f692a0d03779bd9e9168d6e1b986b32c1d1/`.
Its `package/` is immutable after staging: binary0555/text0444. `evidence/`
contains build and mutation logs. SOURCE.json records provenance and limits.

| File | SHA256 |
| --- | --- |
| buzz-hostd (18,214,768 bytes) | `97590329514963e3e7c72efa9e33d53619560b52f453109a0c8036a3c0a0da59` |
| MANIFEST.sha256 | `d020fb8959017fbd54c92eb8e179275aaf4b652b8b18082811d5a8895249ab0f` |
| OPERATOR.md | `e1d25f43592cec240a0b06ceeb98bb71ffd8921fd3e1d5d03bb742d8b906f45c` |
| buzz-host-registration.service | `47e311c61bffe916951f61076a9f437a14f0917958c6085c85e17424ff63b62c` |
| jeff.before.conf | `fd4402216afbe1bc0fbb0c5aedbb8274e300f4115123d4204db97c8653bd6425` |
| jeff.after.conf | `8f1b6f8f272c8934e6e89c0b6ea5f99c8d1ccac62d0d4f60666bf29d30cc2b1c` |

Frozen source/package remains
`/home/bryce/services/buzz-agents/releases/3721-trusted-host-pilot/344baa091ad486653ee99aa8b1de64d4839b4ada/package`.
Retained runtime SHA256 `98404e872a266d94457083bde657e179f3ab19fcd563771ac4e41f994c7bc16a`;
ACP `b7fe18f1604db51dfeffc8d392676d8755f872662be710b87e84c11fc1e83f33`.
Neither rebuilt nor copied into candidate. **Read/measured:** hostd/core/sdk/ws
and build inputs had no diff from344baa to baseline09bbb. Runtime calls only
unchanged UserService installed/catalog/directory/private_file; install/quote
are unreachable and absent in frozen runtime symbols. Frozen runtime synthetic
fixture passed against new executor. This supports same-source runtime behavior,
not a claim that a rebuild would yield identical bytes. Retaining its exact path
also preserves prepare's recorded launcher guard. deployment.json, template,
HOME/PATH/workspace and catalog revision remain unchanged.

## Exact future rollout and rollback — not executed

Full fixed-path commands and prefix-aware rollback are in
[immutable OPERATOR.md](/home/bryce/services/buzz-agents/releases/3721-credential-quoting/46f53f692a0d03779bd9e9168d6e1b986b32c1d1/package/OPERATOR.md).
Four proposed texts are frozen: original/replacement controller and original/
replacement Jeff override. Controller differs only by hostd executable path;
Jeff differs only by removing outer quotes from its two LoadCredential values.
No live files were used to manufacture a broader matching rule: future byte
comparison against the expected old texts is mandatory and mismatches stop.

1. After independent review and explicit approval, verify both manifests, exact
   current config/template/controller/Jeff bytes, file custody, unit shapes,
   masked legacy system controller and protected-agent metadata/hashes. Back up
   the two exact old texts under the private fixed OPERATOR rollback directory.
2. Stop only the user controller; require it and Jeff quiescent with zero PIDs/
   tasks. Recheck target bytes, then atomically replace only the controller unit
   and Jeff override using the frozen after texts. Preserve all current state.
3. Reload user manager; inspect native D-Bus LoadCredential a(ss) for exactly
   `instance` and `execution` with their literal expected paths, before restarting
   controller or invoking any provider. Failure stops for rollback/assessment.
   This is the smallest outstanding native interpretation gate.
4. Start candidate controller only under approval covering same-Jeff automatic
   reconciliation/retry. Verify source hash, unchanged public registration and
   one-minute stability. Use existing Windows Jeff Retry/Refresh if needed;
   retain identity/unit. Keep existing agents unchanged. No second agent/login.

**Rollback caveat:** old hostd rejects a corrected override as foreign on prepare.
Containment rollback must quiesce controller/Jeff and restore the exact saved old
Jeff override as well as the old controller unit, then reload. Only before/after
packet bytes may be replaced; third-party shapes stop. This restores the known
243/CREDENTIALS defect, not working Jeff. Keep current journal/identity/credentials;
never restore a state snapshot. OPERATOR handles interruptions before/between
writes and after controller restart; old restart is conditional on recorded
original state and acceptance of its existing reconciliation. No system-unit
unmask, custody migration or existing-agent action.

Next: independent reviewer uses [review brief](../briefs/3721-credential-quoting-review.md),
then conductor grades and seeks precise live approval. Source/build/test processes
are terminal at handoff; no source mutation held. Final doc commit and clean
worktree receipt are returned to conductor. No open product decision; native
interpretation is the named unrun gate, not a hidden prerequisite workaround.
