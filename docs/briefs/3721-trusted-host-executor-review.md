# Independent review: S1 trusted-host executor (#3721)

Date: 2026-11-10. Requester: conductor. Next owner: fresh `codex-chuck` reviewer. Requested implementation configuration Astra/high; actual served configuration not independently exposed.

## 1. Where the work lives

Worktree `/home/bryce/services/buzz-wt-3720-host-registration`, branch `proof/3720-host-registration-events` (not main). Review `b8a9ccd97e62465d804bbc85125ce858a873fe8d..HEAD`; implementation/test commit is `b21fe0ff8`; executor return supplies the final documentation pin. Starting tree was clean, and all edits belong to this S1 assignment. Sequentially reuse this checkout after executor processes exit.

`git log --oneline b8a9ccd97e62465d804bbc85125ce858a873fe8d..HEAD`

## 2. What landed

[Spec S1](../specs/3722-main-discovery-native-login.md#s1--first-finite-code-change-local-user-service-executor) replaces the ordinary agent-deployment root socket with a local fixed user-service executor. It keeps the existing host deployment journal and signed authority flow. The existing runtime executable accepts an explicit user-service invocation, uses configured normal account context and reuses the finite model runner. Conversation remains unavailable until S2; native login is external. [Execution receipt](../executions/3721-trusted-host-executor.md) records exact tests, failures repaired, source counts and unrun native/full-CI gates.

Registration itself was not redesigned: `--deployment=false` follows its existing path, and installed registration service/sandbox files are untouched. The registration CLI/refresh edits are conditional deployment config loading and state-directory propagation only. No existing live agent, host registration or provider state was accessed.

## 3. Your job

Independently review the committed implementation against S1, especially durable partial failures and the approximately 600 net production-bearing lines. Determine whether the fixed executor remains minimal and correct within the approved same-account trust model. This executor's green tests are evidence, not the independent merge gate.

**Special instruction:** Native systemd was deliberately not invoked. Fake manager scripts verify command shape/state transitions, not real user-manager credential loading or cgroup behavior. Keep source correctness and unmeasured native acceptance separate. The existing root helper is a legacy caller only; do not require S1 to delete or migrate live resources.

## 4. Specifically look for

1. **New-route/legacy separation and registration preservation.**
   Probe [deployment.rs:16](../../crates/buzz-hostd/src/deployment.rs#L16), [registration_cli.rs:87](../../crates/buzz-hostd/src/registration_cli.rs#L87), [registration_refresh.rs:269](../../crates/buzz-hostd/src/registration_refresh.rs#L269). Is the alias a single local engine? Do false-deployment registration, owner/scope/journal checks and encrypted receipts remain unchanged?
2. **Prepare replay, identity and configuration fencing.**
   Probe [user_service.rs:137](../../crates/buzz-hostd/src/user_service.rs#L137) and its prepare branch. Compare the stable unit, scope, key and monotonic revision guards with the existing controller's journal ordering. A local deployment.json revision includes the account context; executable availability is checked without root hashes. An existing placement refuses changes to its manager/unit-directory/launcher location.
3. **Finite exclusion, nonce correlation and restart prefixes.**
   Probe [user_service.rs:158](../../crates/buzz-hostd/src/user_service.rs#L158), models/observe/cancel branches. Does a manager start/stop failure retain retryable descriptors? Can a restarted controller replace an active action? Are stale correlation/output paths refused? Finite service units are never enabled and `Restart=no` avoids reboot replay.
4. **Shared HOME with separate generated artifacts and secrets.**
   Probe [runtime_context.rs:472](../../crates/buzz-hostd/src/runtime_context.rs#L472), [user_service.rs:74](../../crates/buzz-hostd/src/user_service.rs#L74), [runtime entry](../../crates/buzz-hostd/src/bin/buzz-host-runtime.rs#L3). Inspect credential/path quoting and environment construction. The ordinary runtime must use normal HOME and assigned workspace; artifact publication must stay under the per-unit directory. Models receive neither Buzz key nor auth tag. No login receipt is synthesized.
5. **Cleanup only after confirmed stop, across partial failure.**
   Probe [user_service.rs:345](../../crates/buzz-hostd/src/user_service.rs#L345), file helpers, and stop/show. Fencing precedes effects; saved local context survives failures. Verify directory/symlink checks, known nonce filenames, exact drop-in removal and absence of recursive deletion. Shared template, HOME, provider fixture, repository and unrelated unit persist. A missing/foreign generated artifact must not authorize stopping another unit.
6. **Bounded subprocesses and source/native evidence boundary.**
   Probe [runtime_context.rs:207](../../crates/buzz-hostd/src/runtime_context.rs#L207), process-group guard, and [user template](../../crates/buzz-hostd/deploy/buzz-hosted-agent@.service#L1). Runner output is bounded; runtime deadline and template lifecycle must cover descendant exit. The source tests exercise a real synthetic descendant timeout and fake manager stop failure; actual user-manager/cgroup behavior is still not measured.

## 5. Spec deviations and implementation decisions

No architecture/protocol deviation. Extra source seam beyond the proposed list: the existing runtime binary's explicit argument branch. Existing service-action fixture visibility and Catalog initializers changed to reuse tests without another platform. `Catalog.local` is optional only for legacy decoding and mandatory on the new route. The shared template and lock/tombstone remain after per-unit cleanup, because they are shared execution infrastructure and resurrection fences.

No root code was copied into a second provider manager. The executor calls existing instance validation, execution/result types, inactive-state predicate, finite runner and normalization, while supplying account-local custody/manager transactions. Review the retained helper alias and generated drop-in choice for unnecessary complexity.

## 6. What NOT to chase

S2 registry/readiness extraction, S3 UI/DTO work, Windows builds, online native activation, new provider adapters, auth orchestration, legacy migration, installed-service hardening and live-agent adoption remain outside S1. Same-account trust and normal networking are ratified; private HOME/rootfs/provider isolation requirements are superseded, not failed tests to repair. Full CI and native acceptance are not green.

## 7. Evidence and next action

Final relevant components: hostd lib 23 pass + existing ignored ACP case run separately and passed; activation binary 1 pass; controller binary 13 pass; core host_deployment 12 pass. Existing legacy tests were retained. Normal clippy with `--no-deps` passed with pre-existing warnings only; strict lint is blocked by unchanged upstream buzz-core warnings. Scoped format, differential size scripts and diff checks passed. The receipt preserves initial sandbox and fixture failures instead of claiming an uninterrupted green aggregate run.

Return all concrete findings together to the conductor with exact source references. No push/merge/deployment or taskctl authority is transferred. If repairs are needed, return this same pinned checkout to the executor for its own rework after review processes stop.

## M-1 scoped re-gate — 2026-11-10

The conductor returned the sole M-1 finding for repair in this same assignment. Review this repair against clean input `88594cc425b9b3e574217860c07dedad7964757a`; final repair commit comes in the executor return. Earlier sections describe the initial implementation; all other accepted review obligations remain valid. The independent report was not modified.

Production delta is only [user_service.rs:40](../../crates/buzz-hostd/src/user_service.rs#L40), [constructor:102](../../crates/buzz-hostd/src/user_service.rs#L102), and [cleanup:357](../../crates/buzz-hostd/src/user_service.rs#L357). Construction now checks state custody without requiring launch config. Cleanup checks recorded manager/unit location, then preserves its existing identity/stop/path checks. Launch callers still load and fully validate the current catalog. There is no replacement manager or root fallback.

**Probe M-1 and introduced defects:** verify moved/deleted HOME/workspace/launcher no longer block cleanup, but missing/unsafe manager/unit location still does. Verify cleanup after controller reopening reaches the recorded manager without usable current configuration, including failed-stop durable retry. Verify stage/prepare/models remain closed when launch validation or revision binding fails.

[New regression:399](../../crates/buzz-hostd/src/user_service_tests.rs#L399) covers updated/missing/malformed config with moved paths and deleted launcher, failed stop then reopened retry, and preservation of moved provider/repo fixtures. The [existing controller case:387](../../crates/buzz-hostd/src/registration_refresh_tests.rs#L387) now enters the real `run(deployment=true)` constructor with absent config and reopens with malformed config, retaining its 40-unit durable-failure/fairness assertions. No new test platform.

Repair size: +12 net production lines, +89 net test lines. [Receipt M-1 section](../executions/3721-trusted-host-executor.md#m-1-repair--2026-11-10) records targeted and final affected suites, retained prior evidence and unrun gates. Review stays source-only; no S2/S3, native activation, live provider or existing-agent authority is granted. Executor releases the same checkout and processes on return.
