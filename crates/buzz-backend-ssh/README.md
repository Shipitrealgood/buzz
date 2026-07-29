# buzz-backend-ssh

A generic SSH **backend provider** for the Buzz desktop: create an agent in
the app, pick this provider under **Run on**, and the agent runs on a remote
Linux host as a supervised systemd user service — with the same native
presence, activity, and lifecycle the desktop shows for any other agent.

The desktop discovers any executable named `buzz-backend-*` on `PATH` and
lists it as a run destination (see `managed_agents/backend.rs` in the desktop
crate). The id shown in the app is whatever follows the prefix — so a copy or
symlink named `buzz-backend-workstation` appears as **workstation**. One
binary, any number of named backends.

```
Buzz desktop (macOS/Windows/Linux)
   │  JSON on stdin (op: info | deploy)
   ▼
buzz-backend-ssh                    ← provider mode, runs where the desktop runs
   │  ssh <host> buzz-backend-ssh remote-deploy   (payload forwarded on stdin)
   ▼
buzz-backend-ssh remote-deploy      ← remote helper, runs on the Linux host
   │  writes ~/.config/buzz-backend-ssh/agents/<id>.json   (0600)
   │  installs ~/.config/systemd/user/buzz-remote-agent@.service
   │  systemctl --user enable + restart buzz-remote-agent@<id>
   ▼
buzz-remote-agent@<id>.service
   └─ buzz-backend-ssh run-agent <id>  →  execs buzz-acp  →  relay
```

After deploy the desktop needs nothing further from this provider: the agent
connects to the relay itself, and status, presence, and the live activity
view all flow back as relay events. `Shutdown` in the app is delivered as an
owner `!shutdown` control message over the relay; the harness exits
gracefully and `Restart=on-failure` correctly leaves it stopped (crashes
still restart).

## Install

**On the machine running the desktop** (this is the only piece the desktop
sees):

```bash
cargo build --release -p buzz-backend-ssh
# name it after the host you deploy to; the name becomes the "Run on" label
cp target/release/buzz-backend-ssh ~/.local/bin/buzz-backend-agentbox
```

On Windows, cross-compile from Linux with the GNU target (pure Rust — no C
toolchain beyond the mingw linker):

```bash
rustup target add x86_64-pc-windows-gnu
CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc \
  cargo build --release --target x86_64-pc-windows-gnu -p buzz-backend-ssh
```

Install it to `%USERPROFILE%\.local\bin` (a directory the desktop always
scans) **without the `.exe` extension** — e.g. `buzz-backend-agentbox`.
Provider ids are derived from the full filename, and the desktop's id
validation rejects dots, so `buzz-backend-agentbox.exe` is discovered but
fails at agent creation with "invalid provider ID 'agentbox.exe'".
`CreateProcess` executes the extensionless PE fine since the desktop always
invokes providers by full path. The provider shells out to `ssh`, so the
stock Windows OpenSSH client and your existing `~/.ssh/config` + keys are
used — configure passwordless (key) auth for the target host; the provider
runs ssh with `BatchMode=yes` and will fail fast rather than prompt.

**On the remote Linux host:**

```bash
cargo build --release -p buzz-backend-ssh
install -m 0755 target/release/buzz-backend-ssh ~/.local/bin/
# keep agents running after you log out
sudo loginctl enable-linger $USER
```

`buzz-acp` (or the `sprig` multicall binary) and your agent harness
(`claude-agent-acp`, `goose`, …) must be present on the remote host; point
the provider config at them.

## Provider configuration (shown as form fields in the app)

| Field | Default | Meaning |
|---|---|---|
| `host` | — (required) | SSH destination: `~/.ssh/config` alias or `user@host` |
| `ssh_args` | empty | Extra ssh options, e.g. `-p 2222` |
| `remote_command` | `buzz-backend-ssh` | Path to this binary on the remote host |
| `acp_path` | `buzz-acp` | Path to `buzz-acp` on the remote host |
| `remote_path_prepend` | empty | Colon-separated dirs prepended to `PATH` for the agent (harness binaries, node, …) |

All fields are non-secret scalars by desktop policy — ssh credentials always
come from your ambient ssh agent/config, never from provider config.

### Operator defaults file

A flat JSON file next to the provider binary named
`<binary-name>.defaults.json` (e.g. `buzz-backend-agentbox.defaults.json`)
pre-fills the form and backstops deploys:

```json
{
  "host": "user@backend-host",
  "remote_command": "/home/user/.local/bin/buzz-backend-ssh",
  "acp_path": "/home/user/buzz/target/release/buzz-acp",
  "remote_path_prepend": "/home/user/.local/bin"
}
```

Values become the schema `default`s (shown pre-filled in the app), and any
field arriving empty in a deploy falls back to them. This matters today
because the desktop's provider form re-probes on every keystroke and resets
typed values to schema defaults (`WhereToRunSection.tsx` — the probe effect
depends on the whole draft); with a defaults file the reset lands on your
correct values. Per-symlink naming means each named backend keeps its own
defaults.

## What the agent process gets

`run-agent` rebuilds the environment the desktop gives a locally spawned
agent (`spawn_agent_child`): identity (`BUZZ_PRIVATE_KEY`, `BUZZ_RELAY_URL`,
`BUZZ_AUTH_TAG`), harness command/args, model, system prompt, respond-to
gates, parallelism and timeouts, `BUZZ_ACP_RELAY_OBSERVER=true` (live
activity in the app), and — when `git-credential-nostr` is on `PATH` —
automatic relay-git auth scoped to the deploy relay, so agents can clone and
push Buzz projects. User `env_vars` are applied first with the desktop's
reserved-key list stripped, so they can never override identity or security
gates.

The agent id (and systemd instance name) is a 16-hex prefix of the agent's
public key: redeploys and renames always map to the same unit, so no stale
processes accumulate.

## Operations & troubleshooting (remote host)

```bash
systemctl --user list-units 'buzz-remote-agent@*'      # what's deployed
journalctl --user -u buzz-remote-agent@<id> -f          # follow one agent
systemctl --user stop buzz-remote-agent@<id>            # manual stop
systemctl --user disable buzz-remote-agent@<id>         # remove from boot
rm ~/.config/buzz-backend-ssh/agents/<id>.json          # forget the agent
```

- **Deploy fails with a bus error** — the helper defaults
  `XDG_RUNTIME_DIR`/`DBUS_SESSION_BUS_ADDRESS` for non-interactive ssh
  sessions, but the user systemd instance must exist; `loginctl
  enable-linger` guarantees it at boot.
- **Agent dies after you disconnect** — lingering is off; the deploy response
  carries this warning. `sudo loginctl enable-linger $USER`.
- **Harness not found** — the unit runs without your login shell's PATH; set
  `remote_path_prepend` (e.g. nvm's bin dir for `claude-agent-acp`).
- **Deleting an agent in the app** — v1 of the provider protocol has no
  `undeploy` op; the app sends `!shutdown` and warns that the remote unit
  remains. Clean up with the `disable` + `rm` commands above.
