# 2722 verification failure receipts and restoration

Source baseline: 93ea3f2a29f663e9c53cd560df33e73a785dc37a plus this task's
uncommitted implementation at execution. All failures below were observed,
not inferred from upstream history. Runtime configuration requested Astra/high.

- Initial Hermit invocation scoped `HERMIT_SELF_UPDATE=false` only to activation;
  Cargo then attempted read-only metadata update. Exporting the existing flag
  fixed execution; no tool installation or environment file edit.
- Initial host unit parallel run: 23 passed / 5 failed / 3 ignored. Two tests
  failed at Unix socket bind with EPERM; three returned EAGAIN under parallel
  synthetic process load. Narrow escalation and `--test-threads=1`: 28 passed,
  zero failed, three ignored. No production change was made for those failures.
- `cargo test --offline --locked -j2 -p buzz-relay --lib --no-run` fails at
  `external_infra_deployment_tests.rs:390` (missing `Catalog.local`) and `:412`
  (removed `Helper::at`). Baseline references confirmed unchanged. Conductor
  explicitly retained this obsolete fixture; no root-helper restoration,
  test deletion or ignore change. Production relay `cargo check` passed.
- Initial DB test build: unchanged `store/deletion.rs:4632` lacked
  `Status.inventory`; conductor approved test-only `None`. New test dynamic SQL
  was corrected to literal queries. DB test binary then compiled and the real
  isolated writer test passed one/zero; DB removed automatically.
- New host-first test initially used invalid runtime ID `missing` in its negative
  fixture, rejected by the codec before the intended host guard. Corrected to
  canonical `main/missing`; actual host-first journey passed. An earlier overly
  exact short-name filter ran zero tests and is explicitly not green evidence.
- New owner socket fixture initially tried to decode an empty close frame; it
  now handles text frames only. A second valid-response failure was caused by
  fixture expiry anchored before the actual request was issued. It now echoes
  the signed request expiry. Seven scenarios then passed: valid response and
  wrong runtime/revision/nonce/parent/host/generation. No production validator
  was loosened to pass either fixture.
- New hook unavailable test initially exposed misleading inherited text about
  built-in fallback models. Hosted failures now use the existing status slot
  with a host-unavailable/retry message; hook suite passed three/zero. Ordinary
  Node test child launch failed in sandbox; `--test-isolation=none` exercised
  the real hook successfully. Related frontend suite passed 57/zero.
- Strict `cargo clippy … --lib -- -D warnings` fails at unchanged
  `paired_message/body.rs:9` (`large_enum_variant`) and `:133`
  (`wrong_self_convention`). Advisory clippy completed with two core, two owner,
  seven hostd, two DB warnings, all on unchanged paths. Strict lint is not green.
- Normal Biome path invocation rejects existing nested root configuration
  `desktop/biome.json`. Direct cached pinned Biome 2.4.7 behaves the same.
  Changed frontend files were formatted through that binary's stdin interface
  using the repository root configuration and matching filename extensions;
  no config/package change. Canonical package lint is not claimed passed.
- Differential size check initially failed because Node could not spawn Git
  (EPERM); identical gate with narrow escalation passed.

No live/provider traffic, credentials, service changes or Windows evidence arose
from these tests. Only fake provider/service processes ran. The standard full
CI/native/relay integration gates remain unrun or blocked as documented in the
execution record, not silently replaced by the scoped checks.
