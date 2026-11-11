# #3721 credential quoting repair — independent post-audit

2026-11-11. **MERGE-SAFE within the reviewed source/nonlive preparation scope:
source and packet v2 are ready for separately approved activation.** Open findings
C=0/H=0/M=0/L=0; historical M-1 is closed below. No source merge blocker or
foundation gap found. This is not acceptance of the live pilot or authorization
to activate it; native parsed values and delivery remain unproved.

Reviewer `/root/hostd_credential_review`, codex-chuck. Requested configuration:
gpt-6-astra/high; actual served model and effort not exposed. Initial review plus
one packet-only re-gate, rework rounds 1. Applied post-audit; no implementation
edits or board mutations.

## Pin, contract and scope

Workspace `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events`, initially clean at
`8e831be04193b4ade4e46b860cd2fdf6690938c8`.
Reviewed `09bbb43096fae862dee9b3f03e4cbcc9295c7d24..8e831be04193b4ade4e46b860cd2fdf6690938c8`;
implementation `46f53f692a0d03779bd9e9168d6e1b986b32c1d1` changes production
+35/-4 lines and tests +148. Inputs: [dispatch brief](../../briefs/3721-credential-quoting-review.md),
[executor receipt](../../executions/3721-credential-quoting-fix.md), project
AGENTS/VISION/agent and remote-agent intent/architecture/testing, portable conductor
contract and [ratified trusted-account contract](../../specs/3722-main-discovery-native-login.md).

Review covers serializer semantics, exact legacy repair, frozen runtime retention,
package provenance and rollback prefixes. Existing owner/journal semantics,
normal-account trust, main discovery/ACP and provider policy remain accepted.
No live manager calls, configuration/journal/key reads or writes, providers,
existing-agent operations, relay/Windows work, installs, push or deployment ran.
Only this audit artifact is written and signed off.

## M-1 (Medium, high confidence) — closed; stale approved-configuration pin; rides #3721

**Read and measured.** [OPERATOR line 33](/home/bryce/services/buzz-agents/releases/3721-credential-quoting/46f53f692a0d03779bd9e9168d6e1b986b32c1d1/package/OPERATOR.md:33)
compares live deployment.json to the original frozen package. That file hashes
to `f019bb6dc7b256cfb931f950fa45d53e95a677f6d138e7b3b7321f90c64e376b`.
[SOURCE line 27](/home/bryce/services/buzz-agents/releases/3721-credential-quoting/46f53f692a0d03779bd9e9168d6e1b986b32c1d1/package/SOURCE.json:27)
repeats that obsolete retained-configuration pin.

The conductor supplied the approved PATH correction; I independently read its
[application receipt](/home/bryce/services/buzz-agents/releases/3721-trusted-host-path/node-v22.16.0/OPERATOR-RESULT.md:17)
and hashed its staged deployment.json:
`1e140bed82e738072671e61a74803a79b6eda9e518eba7714a36a4369a77516c`.
Parsed comparison with the original proves the sole change is local.tool_path,
prepending `/home/bryce/.nvm/versions/node/v22.16.0/bin:`. The receipt records
approved installation; I did not re-read live configuration.

**Failure:** against that accepted state, the packet's `set -e` preflight aborts
before quiescence. It fails safely but is not executable as the promised prepared
rollout. Restoring the old configuration to satisfy it would regress the approved
PATH and is not an acceptable remedy.

**Initial proposed disposition:** original executor repairs packet OPERATOR/SOURCE and
associated manifest/receipt pins using the approved corrected file and hash;
explicitly preserve this configuration in both directions. Retain the old packet
as evidence and issue a clearly versioned replacement. No source fix or binary
rebuild is required. Conductor owns routing; reviewer re-gates only that delta.
No product or architecture ruling is needed.

## Source and rollout assessment

**Read:** [override_text](../../../crates/buzz-hostd/src/user_service.rs:421)
uses the correct directive-specific representation. Systemd v257's
[config_parse_load_credential](https://github.com/systemd/systemd/blob/v257/src/core/load-fragment.c#L4456)
splits the credential ID at the first colon, expands the remaining path's
specifiers and stores it without shell unquoting. Literal spaces, quotes,
backslashes, dollar signs and subsequent colons therefore belong in the path;
doubled percent signs preserve literal percent characters. Fixed filename suffixes
avoid trailing-whitespace or terminal-backslash ambiguity. Control/non-UTF8
rejection remains, and ExecStart retains its separate quoting.

**Read/measured:** the legacy branch reproduces the base generator exactly;
[install](../../../crates/buzz-hostd/src/user_service.rs:406) compares the entire
override, not a prefix. The only production caller is prepare after existing
identity/revision/location/inactivity checks. The production-path regression
[test](../../../crates/buzz-hostd/src/user_service_tests.rs:70) exercises two path
shapes, exact old repair, reopened/idempotent replay, unchanged identity, five
foreign forms, no reload on refusal and no start. It is not a Jeff-only exception.

**Read/measured:** the frozen-source-to-base diff contains desktop/documentation
only; hostd/shared Rust/build inputs are unchanged. The runtime entry's
[run_user](../../../crates/buzz-hostd/src/runtime_context.rs:513) reaches installed,
catalog and file/directory checks, not install/override_text/quote. The frozen
runtime/ACP synthetic fixture passed against this executor. Retaining both exact
binaries and the recorded launcher path is supported; rebuilt byte identity and
native credential delivery are not claimed.

**Read:** [OPERATOR lines 57–94](/home/bryce/services/buzz-agents/releases/3721-credential-quoting/46f53f692a0d03779bd9e9168d6e1b986b32c1d1/package/OPERATOR.md:57)
require quiescence, exact bytes/custody, replacement and reload, then exact parsed
a(ss) pairs before controller restart. Automatic same-Jeff reconciliation is
explicit. [Rollback lines 116–146](/home/bryce/services/buzz-agents/releases/3721-credential-quoting/46f53f692a0d03779bd9e9168d6e1b986b32c1d1/package/OPERATOR.md:116)
cover before/between/after-write prefixes and post-restart activity, stop on
foreign shapes, restore both original texts while quiescent and retain current
journals/identity. Restoring the old override intentionally restores the known
243/CREDENTIALS failure; old-hostd containment is not successful Jeff recovery.
No additional supported finding in those prefixes.

Minimalism 9/10, elegance 9/10, source correctness 9/10; packet correctness 9/10
after the M-1 repair below (initially 8/10). The implementation fixes the serializer class at its
production boundary without adding a provider-specific mechanism.

## Independent verification and limits

All Cargo commands used Hermit and `--offline --locked`. These are reviewer runs
at the pin, not transcribed executor green claims:

| Gate | Result |
| --- | --- |
| `cargo test -p buzz-hostd --all-targets -- --test-threads=1` | PASS: 28 library, 1 activation, 13 controller; 3 library ignored. The two credential regressions are included. Library 8.90s; controller 34.10s. |
| `cargo test -p buzz-hostd --lib generated_credentials_parse_offline -- --ignored --test-threads=1` | PASS 1, 0.12s; private systemd --test only, no transaction execution. |
| Frozen BUZZ_TEST_RUNTIME/BUZZ_TEST_ACP with `actual_user_runtime_prepare_refresh_selected_conversation_and_cleanup --ignored` | PASS 1, 0.71s; two expected refusal diagnostics. Synthetic commands only. |
| `cargo fmt -p buzz-hostd -- --check`; `git diff --check` | PASS. |
| `cargo clippy --no-deps -p buzz-hostd --all-targets` | Finished successfully; existing request/onboard/transport/CLI warnings, none in changed files. No strict warning-free claim. |
| `cargo build --release -p buzz-hostd --bin buzz-hostd` | PASS, 0.55s incremental. Candidate and local release binary SHA256 match. |
| Candidate help and registration-run help | PASS exit 0; no stateful invocation. |
| Packet manifests and OPERATOR shell syntax | PASS candidate 7/7, frozen original 8/8; all four bash blocks parse, none executed. |

Initial sandboxed full-suite attempt: 26 library passed, two failed at disposable
UnixDatagram binds with EPERM, three ignored. After narrow approved socket
escalation, the complete suite passed as above. This failure was environmental,
not silently counted green. All reviewer subprocesses are terminal.

**Measured provenance:** candidate manifest
`d020fb8959017fbd54c92eb8e179275aaf4b652b8b18082811d5a8895249ab0f`;
hostd `97590329514963e3e7c72efa9e33d53619560b52f453109a0c8036a3c0a0da59`;
original manifest `ec1fb606e835ba238a39021a3ad17d90e1f8c3ea933b568f2fa5db78e5671441`.
Cargo.lock matches SOURCE.json. Package files remain 0444, binary 0555.
Controller before/after differs only in hostd path; Jeff before/after only in the
two credential values' outer quotes.

**Reused evidence, not rerun:** read the three executor mutation logs under the
[candidate evidence directory](/home/bryce/services/buzz-agents/releases/3721-credential-quoting/46f53f692a0d03779bd9e9168d6e1b986b32c1d1/evidence).
Old output fails the expected-value assertion; no legacy repair fails prepare;
foreign acceptance fails unwrap_err. Unchanged source/test pin and this review's
green unmutated suite support reuse per dispatch. No mutation was held or applied.

**Not run:** actual manager parsed credential values or delivery, provider/model/
chat/Jeff retry/Cleanup, live rollback/reboot, full just ci, Windows and human
acceptance. Offline --test accepts even broken syntax and cannot discharge the
native interpretation gate. That gate remains explicitly future and must precede
provider execution under separate approval; it is not a hidden source PASS.

## M-1 delta-only re-gate — 2026-11-11

**Closed, high confidence.** Re-gated clean
`eb2df36471f699291e08b4ba6930a2cc2771922a` against the original review commit
`66f7c5ad3cb36ae29438450978cc8627c5a0e874`. Git delta contains only the execution
receipt and review brief; production and tests are unchanged. Read their delta
and independently compared both complete OPERATOR/SOURCE texts.

Current instructions are [packet-v2 OPERATOR](/home/bryce/services/buzz-agents/releases/3721-credential-quoting/46f53f692a0d03779bd9e9168d6e1b986b32c1d1/packet-v2/package/OPERATOR.md:26).
Forward and rollback now hash the approved staged configuration at
`1e140bed82e738072671e61a74803a79b6eda9e518eba7714a36a4369a77516c`
and compare it read-only with current configuration. I rehashed the staged file;
live configuration was neither read nor changed. SOURCE pins the same approved
file, and the packet expressly forbids restoring historical configuration.
Original rejected packet remains intact. M-1's concrete failure is repaired.

**Measured delta checks:** v2 manifest 7/7 and original manifest 7/7 passed;
original manifest still hashes to the prior reviewed value. Hostd and all four
unit/override texts are byte-identical across packets. Five v2 bash blocks pass
`bash -n`; none executed. Text modes remain 0444, binary 0555. Verified v2 hashes:

- MANIFEST.sha256: `40ea4ea7ef5f16cab3757f271f13c61ebca52d65668faf3c5dc17b93d7a40afe`.
- OPERATOR.md: `c3e683b3b90d4ace9891bc37290256f230580fe967b27d1ad7b4b5ad0446961b`.
- SOURCE.json: `71b44a39bcf587f138d2be8c210a30071b908cc2709d2203aa2dc606ec117939`.

**Preservation read:** controller still references the original immutable binary;
v2 preflight compares that executable to its identical manifested copy. The
quiescence, exact before/after matching, foreign refusal, native parsed-value gate
before controller restart, same-Jeff automatic reconciliation notice and
prefix-aware rollback are unchanged. Added rollback config check is read-only.
Current journal, identity, credentials, approved PATH and protected agents remain
outside mutation scope. No new supported finding was introduced.

Earlier source/suite/mutation/runtime evidence carries forward on unchanged
source and payload; no rebuild, suite rerun or new probe campaign performed.
Only packet-delta verification ran. All native/live/provider/human gates listed
above remain NOT RUN. Next: conductor may present this concrete packet for scoped
live approval. No additional code or preparation repair is required by this review.
All reviewer commands are terminal; clean workspace ownership is released after
this report's signed commit.

quality: model=unknown effort=unknown stage=post-audit verdict=merge-safe verify=ran commits=09bbb43096fae862dee9b3f03e4cbcc9295c7d24..eb2df36471f699291e08b4ba6930a2cc2771922a findings=C=0/H=0/M=0/L=0 rework_rounds=1
