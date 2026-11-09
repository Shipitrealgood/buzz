# #3719/#3722: bounded service-context proof

Prepared 2026-11-09 for independent Astra/high review. **Offline namespace gate
passes; full isolation and native subscription gates remain open.** This is a
reviewable test fixture, not a production launcher, installer or authorization
boundary. The fixture files are operator-controlled source; no untrusted caller
can edit a runtime descriptor in the tested model.

## Where the work lives

Worktree `/home/bryce/services/buzz-wt-3720-host-registration`, branch
`proof/3720-host-registration-events` (not main). Starting HEAD
`62fe54c5bf323504e641dd24ec2acb59471a22ef`; initial status clean. Only this directory
is owned. The separate non-Git receipt is
`/home/bryce/services/buzz-agents/docs/audits/3719-3722-grok-service-context.md`.
Requested configuration: `gpt-6-astra`, high; runtime model routing not independently
exposed. The conductor owns integration and the independent review gate.

Accepted upstream inputs: operations `docs/specs/2722-first-hosted-agent.md`,
`docs/audits/3722-grok-subscription-proof.md`,
`docs/briefs/3719-isolation-proof-completion.md`, and
`docs/audits/spec-review/3737-keep-adapt-replace.md`. This work preserves their
filesystem/network High holds and authentication Medium hold.

## What landed

`run-proof.py` builds a private temporary rootfs from explicit executable/loader
copies and a limited Python standard library. It invokes `context.py` under
bubblewrap with separate user, PID, network, IPC and mount namespaces. Two
writable binds contain only the fixture's home and work directory; `/run` and
`/tmp` are private. No real home, `/etc`, live sockets or broad `/usr` is mounted.
The receipt pins every copied file, the driver, source adapter and bubblewrap.
Grok is copied from the one permitted binary path and hash-checked, **never
executed**. No credentials are read or copied.

The fixed runner accepts `service-proof-agent` and enumerated proof modes. All
provider modes return 78. Unknown references/modes return 64. Its model mode reads
the same runtime descriptor shape (harness, command, args) as the future native
mode and delegates to `buzz-acp models --json`; it contains no model table or
normalizer. The unchanged `crates/buzz-acp/tests/fixtures/task_agent.py` supplies
the synthetic ACP response. Its `old`/`chosen` models are fixture data, not Grok
availability. Login/conversation are not simulated as authenticated success.

The existing implementation seams read for continuity are:

- `desktop/src-tauri/src/commands/agent_model_process.rs`: actual generic process
  interface and raw payload; `agent_models.rs` retains desktop normalization.
- `desktop/src-tauri/src/managed_agents/discovery/presets.rs`: Grok argument order
  `agent --always-approve stdio`; no separate adapter or API fallback.
- `crates/buzz-hostd/src/lib.rs`: catalog pins and trusted ownership checks.
- `crates/buzz-hostd/src/bin/buzz-host-runtime.rs`: cleared environment and the
  approved descriptor driving conversation. This proof does not invoke it or
  invent a substitute privileged catalog/journal admission path.

## Measured gates and limitations

Run from the pinned worktree (requires the already-authorized scoped sandbox
escalation because the default environment denies sockets):

```sh
python3 docs/proofs/grok-service-context/run-proof.py > docs/proofs/grok-service-context/receipt.json
systemd-analyze verify docs/proofs/grok-service-context/buzz-service-proof.service
```

The JSON is sanitized and has a 64 KiB capture bound per subprocess, a 15-second
normal deadline, a 1 MiB per-file write limit, no core files and 128 descriptors.
The context is private disposable state, not an adversarial resource-exhaustion
certification. Each command has a PID namespace; timeout/cancellation kills its
init and detached descendants. No shell command is accepted from the caller.

| Gate | Measured result |
|---|---|
| Real ACP runtime and dynamic loader in copied rootfs | PASS for artifact below |
| Generic model discovery through real ACP and existing source fixture | PASS; stable model payload retained in receipt |
| Same UID/HOME/GROK_HOME across process restart | PASS, UID 1002; synthetic state only |
| Own persistent state and Unix socket | PASS |
| Operator/sibling/non-home/config synthetic reads | Denied; each canary readable outside |
| Outside IPv4 loopback, IPv6 loopback, filesystem and abstract Unix sockets | Denied; each listener connected successfully outside |
| Deliberately expose canary / share outside network | Both cause the actual probe to fail |
| Cleared environment and absent machine config/systemd socket | PASS; strict keys and exact PWD; inherited synthetic poison excluded |
| Concurrent model mode while hold owns lock | Rejected 75; lock available after exit/failure/cancel |
| Cancellation including detached descendant, timeout and retry | PASS; namespace processes dead; no running child retained |
| Unknown commands/references and native modes | Rejected; provider modes deliberately disabled |
| Cleanup | Temporary root removed; sockets closed; direct children reaped |
| Native systemd actual unit | NOT RUN |
| Non-loopback private/link-local/public IPv4/IPv6 selective egress | NOT RUN; network-none is deny-all, not an online policy |
| Native login, authenticated models, conversation, refresh | NOT RUN |
| Provider credential persistence, entitlement, concurrent subscription impact | NOT RUN |
| Production tool/dependency closure, CA/DNS/TLS | NOT RUN; offline models needs no provider/relay |
| Full CI/native Windows/human/independent review | NOT RUN by executor |

The ACP binary is the saved canary artifact at
`buzz-agents/releases/paired-hosts-canary-723c7e849/netcup/bin/buzz-acp`, hash
`fa3f1a36919d3d1c34fd523f40695495bb58cd00e7f6b52b62e59edc140341fb`.
Its PREPARATION.md attributes it to source
`723c7e849a65af766a7db8ae8c2b62821f7550b4`, Rust 1.98.1, frozen lockfile/cache.
**Read source continuity:** ACP, SDK, persona, git helpers and root Cargo.toml
have no diff to starting HEAD; core, ws-client and Cargo.lock do differ. This is
an old exact-artifact preparatory gate, not execution of current HEAD. A binary
built from the final approved source and its complete dependency closure must
repeat the gate before deployment. Grok hash is
`22d281c5964c5444147328096ba2a221f3f0874c6858e27c825656a80f6e58e9`.

Initial default-sandbox run failed creating a synthetic socket (EPERM), before
any namespace or Grok launch. Escalated fixture iterations repaired an overly
narrow environment assertion (bwrap injects PWD) and changed child observation
from namespace CWD to owned `/proc/.../children`. These were test defects, not
relaxed boundary claims. The final probe requires exact environment keys and
PWD=/state/work. The native unit syntax check first hit sandbox SO_PASSCRED
EPERM; read-only escalation is required here too.

## Candidate service difference and approval boundary

`buzz-service-proof.service` is an **uninstalled offline test candidate**, using
a closed `RootDirectory`, explicit own-state binds, `PrivateNetwork=yes`, cleared
exec environment and control-group cleanup. Compared with
`crates/buzz-hostd/deploy/buzz-paired-agent@.service`, it removes readiness notify,
real instance credentials, restart loops and the canonical runtime helper. It
retains resource restrictions and adds a closed root plus network-none. It is
not a drop-in replacement and does not prove the canonical DynamicUser mapping,
credential injection or production startup. Its read mode requires the prepared
synthetic marker/probe fixtures; a bare unit installation is insufficient.

Measured environment: PID 1 is `codex`; systemd markers/socket exist but read-only
`env -i PATH=/usr/bin:/bin timeout 3s systemctl show --property=Version --value`
returns EPERM. A disposable native manager cannot be established with existing
permitted resources. No container/privileged mount/new account/unit was created.

Proposed **next native offline action**, requiring separate approval before host
mutation: on a disposable systemd machine, populate
`/run/buzz-service-proof/rootfs` from this driver’s exact copy inventory and
`/var/lib/buzz-service-proof/{home,work}` as fresh synthetic directories. Keep
four synthetic listeners and canaries alive with outside positive controls;
write their exact addresses into `/proof/probes.json`. Use this one unit name,
run write → read → models by approved operator-only ExecStart substitutions,
then stop/restart, contention, timeout and failure variants. Capture effective
UID, state mapping and cgroup emptiness. Install nothing in the existing paired
agent namespace. Cleanup: stop the proof unit, remove only its runtime unit file,
reload the manager, terminate the four owned listeners, remove its exact `/run`
root and its synthetic StateDirectory. No invocation is authorized by this doc.
The current driver is a namespace runner, not this native staging orchestrator;
implementing that small staging adaptation is part of the next approved gate.

Proposed **online/native action is blocked on an egress decision**, not ready to
execute: maintain a dedicated persistent identity/context and one exclusive lock
for fixed commands `grok login --device-auth`, `buzz-acp models --json` with the
approved Grok descriptor, and the normal `buzz-acp` conversation invocation.
Each must use the same UID, HOME/GROK_HOME, working directory, immutable runtime,
private sockets and scrubbed environment. Provider-facing login prompts remain
operator-local. Native tools alone own tokens/refresh; no custom parser. The
current runner deliberately rejects these modes.

Required prerequisite: choose and prove an online network enforcement mechanism
permitting reviewed provider/relay/DNS/TLS destinations while rejecting synthetic
co-resident/private/alternate-family endpoints, including redirect/address changes.
Known documentation names `auth.x.ai` and `cli-chat-proxy.grok.com` are inputs, not
a verified complete policy. This proof builds no proxy/firewall framework and
cannot substitute PrivateNetwork=yes for that gate. Impacted resources for later
approval: one dedicated test service/state root/runtime, reviewed online policy,
provider device request and account approval, subsequent model calls, and a
separately approved test relay identity/conversation. Existing agents remain out
of scope. #3719/#3721/#3722 remain open.

## Reviewer attention and handoff

The next agent’s job is independent post-audit of this finite artifact range.
Doubt the evidence boundaries, especially old-artifact versus current-source
claims and synthetic state versus authentication. Read the source fixture and
receipt, not only the prose summary.

1. `run-proof.py:bounded`: do deadlines and output limits own the entire namespace?
2. `context.py:main`: do enum/reference rejection and lock behavior admit any
   arbitrary executable/home/environment? Provider branches must remain disabled.
3. `run-proof.py:command`: does either weakening control leak into ordinary modes?
4. `run-proof.py` rootfs inventory: does the actual ACP run use only pinned copied
   dependencies and the unchanged upstream adapter? Verify receipt hashes.
5. Cancellation probe: does its descendant enumeration cover the setsid child,
   and does subsequent lock acquisition prove cleanup without killing unrelated PIDs?
6. Candidate unit: env-clearing and offline constraints are deliberate; native
   execution and online enforcement must not be inferred from syntax verification.

Deviations: no native systemd execution is possible within current resources;
old canary ACP used with conductor approval and explicit source drift; proof-only
brief lives here to honor scoped ownership. There is no production implementation.
Native authentication, deployment, UI/protocol and relay work belong to later
approved assignments. Return findings to the conductor; executor self-review
is not an independent gate. Review reruns need the same scoped test escalation.
