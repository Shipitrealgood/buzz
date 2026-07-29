//! `buzz-backend-ssh` — a generic SSH backend provider for the Buzz desktop.
//!
//! The desktop discovers any executable named `buzz-backend-*` on `PATH` and
//! offers it as a "Run on" destination when creating a managed agent. This
//! binary implements that provider protocol (JSON request on stdin → JSON
//! response on stdout) by forwarding deploys over `ssh` to a Linux host,
//! where the same binary installs the agent as a systemd user service
//! running `buzz-acp`.
//!
//! Renaming (or symlinking) the binary changes the backend's display id:
//! `buzz-backend-agentbox` shows up as "agentbox" in the desktop dropdown.
//!
//! Modes (selected by argv):
//! - no arguments: provider mode, invoked by the desktop. Reads one JSON
//!   request (`op: "info"` or `op: "deploy"`) from stdin; `info` is answered
//!   locally, `deploy` is forwarded to the remote helper over ssh.
//! - `remote-deploy`: remote helper mode, invoked over ssh. Reads the same
//!   deploy request from stdin, persists the agent config, installs and
//!   (re)starts the `buzz-remote-agent@<id>` systemd user unit.
//! - `run-agent <id>`: unit entry point. Rebuilds the agent environment from
//!   the persisted config and execs `buzz-acp`.

mod protocol;
mod provider;
#[cfg(unix)]
mod remote;

use std::io::Read;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match dispatch(&args) {
        Ok(()) => 0,
        Err(e) => {
            // Provider-mode errors are reported as `ok: false` JSON on stdout
            // inside `dispatch`; anything surfacing here is a hard failure.
            eprintln!("buzz-backend-ssh: {e:#}");
            1
        }
    };
    std::process::exit(code);
}

fn dispatch(args: &[String]) -> anyhow::Result<()> {
    match args.first().map(String::as_str) {
        None => provider::run(&read_stdin()?),
        Some("-V" | "--version") => {
            println!("buzz-backend-ssh {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        #[cfg(unix)]
        Some("remote-deploy") => remote::deploy(&read_stdin()?),
        #[cfg(unix)]
        Some("run-agent") => match args.get(1) {
            Some(id) => remote::run_agent(id),
            None => anyhow::bail!("usage: buzz-backend-ssh run-agent <agent-id>"),
        },
        #[cfg(not(unix))]
        Some("remote-deploy" | "run-agent") => {
            anyhow::bail!("remote-deploy/run-agent only run on the (unix) agent host")
        }
        Some(other) => anyhow::bail!(
            "unknown mode '{other}' (expected no args, 'remote-deploy', or 'run-agent <id>')"
        ),
    }
}

/// Read the request from stdin, bounded so a misbehaving caller cannot OOM us.
/// Deploy payloads carry a system prompt and env vars but stay well under this.
fn read_stdin() -> anyhow::Result<String> {
    const STDIN_CAP: u64 = 4 * 1024 * 1024;
    let mut buf = String::new();
    std::io::stdin()
        .take(STDIN_CAP)
        .read_to_string(&mut buf)
        .map_err(|e| anyhow::anyhow!("failed to read request from stdin: {e}"))?;
    Ok(buf)
}
