//! systemd interaction for `buzz-agent@` units: probes, mutations, and the
//! live-unit guard every credential-destroying path must pass.

use std::process::Command;

use anyhow::{bail, Context, Result};

/// The systemd unit name for an agent instance.
pub fn unit_name(name: &str) -> String {
    format!("buzz-agent@{name}.service")
}

/// Read-only probe: the unit's `is-active` state ("active", "inactive", …).
/// Errors only when systemctl itself cannot be spawned.
pub fn unit_active_state(name: &str) -> Result<String> {
    let out = Command::new("systemctl")
        .args(["--user", "is-active", &unit_name(name)])
        .output()
        .context("cannot run systemctl --user (no user manager available?)")?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

/// The live-unit guard, pure and unit-testable: refuse to destroy or replace
/// a running agent's credential/state unless the caller's override flag makes
/// it a conscious decision. This is what stands between a mistyped name and a
/// live production agent. `override_flag` names the authorizing flag when the
/// calling command has one; commands without one refuse outright.
pub fn live_guard_decision(
    name: &str,
    state: &str,
    forced: bool,
    override_flag: Option<&str>,
) -> Result<()> {
    let live = matches!(state, "active" | "activating" | "reloading");
    if live && !forced {
        let hint = override_flag
            .map(|f| format!(" or re-run with {f}"))
            .unwrap_or_default();
        bail!(
            "buzz-agent@{name}.service is currently {state} — refusing to touch a RUNNING \
             agent's credential/state.\n\
             If this is really the agent you mean, stop it first \
             (systemctl --user stop buzz-agent@{name}){hint}."
        );
    }
    if live {
        eprintln!("warning: proceeding on {name:?} while its unit is {state} (override given)");
    }
    Ok(())
}

/// Guard a credential-destroying/replacing operation against a live unit.
///
/// ALWAYS probes — the probe is read-only, so no flag (including
/// `--no-systemd`, which only scopes unit *mutations*) may skip it. Degrades
/// to a warning only when systemctl itself cannot be spawned (host without a
/// user manager), never on operator input.
pub fn guard_live_unit(name: &str, forced: bool, override_flag: Option<&str>) -> Result<()> {
    match unit_active_state(name) {
        Ok(state) => live_guard_decision(name, &state, forced, override_flag),
        Err(e) => {
            eprintln!(
                "warning: cannot probe unit state for {name:?} ({e:#}) — proceeding as if \
                 the unit is not running"
            );
            Ok(())
        }
    }
}

/// Does the unit resolve to a loadable definition? (`systemctl cat` exits
/// non-zero for units with no unit file — including uninstantiable templates.)
pub fn unit_exists(unit: &str) -> Result<bool> {
    Ok(!unit_definition(unit)?.is_empty())
}

/// The unit's rendered definition text, or `None` when it has no unit file.
pub fn unit_definition(unit: &str) -> Result<String> {
    let out = Command::new("systemctl")
        .args(["--user", "cat", "--", unit])
        .output()
        .context("cannot run systemctl --user")?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Ok(String::new())
    }
}

/// Run `systemctl --user <args>`; non-zero exit is a hard error carrying the
/// command's stderr.
pub fn systemctl_user(args: &[&str]) -> Result<()> {
    let out = Command::new("systemctl")
        .arg("--user")
        .args(args)
        .output()
        .context("cannot run systemctl --user (is a user manager available?)")?;
    if !out.status.success() {
        bail!(
            "systemctl --user {} failed ({}): {}",
            args.join(" "),
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}

/// Run `systemctl --user <args>`, tolerating "not loaded"-class failures
/// (converged states on re-runs) but failing loud on anything else.
pub fn systemctl_user_tolerant(args: &[&str]) -> Result<()> {
    let out = Command::new("systemctl")
        .arg("--user")
        .args(args)
        .output()
        .context("cannot run systemctl --user")?;
    if out.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    if stderr.contains("not loaded") || stderr.contains("does not exist") {
        return Ok(());
    }
    bail!(
        "systemctl --user {} failed ({}): {}",
        args.join(" "),
        out.status,
        stderr.trim()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guard_refuses_live_states_and_passes_dead_ones() {
        for live in ["active", "activating", "reloading"] {
            let err = live_guard_decision("prod", live, false, Some("--force-live")).unwrap_err();
            let msg = format!("{err:#}");
            assert!(msg.contains("RUNNING"), "got: {msg}");
            assert!(
                msg.contains("--force-live"),
                "must name the override: {msg}"
            );
            // Override authorizes it.
            live_guard_decision("prod", live, true, Some("--force-live")).unwrap();
        }
        for dead in ["inactive", "failed", "deactivating", "unknown", ""] {
            live_guard_decision("prod", dead, false, Some("--force-live")).unwrap();
        }
    }

    #[test]
    fn guard_without_override_flag_never_advertises_one() {
        let err = live_guard_decision("prod", "active", false, None).unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("stop it first"), "got: {msg}");
        assert!(
            !msg.contains("--force"),
            "no override flag exists on this path: {msg}"
        );
    }
}
