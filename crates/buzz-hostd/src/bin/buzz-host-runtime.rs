//! The isolated worker UID can read only this unit's systemd credential.
use anyhow::{anyhow, ensure, Result};

use std::{env, os::unix::process::CommandExt, path::PathBuf, process::Command};
fn main() -> Result<()> {
    ensure!(
        !rustix::process::geteuid().is_root(),
        "refusing to run an agent as root"
    );
    let creds = PathBuf::from(
        env::var_os("CREDENTIALS_DIRECTORY")
            .ok_or_else(|| anyhow!("systemd credentials required"))?,
    );
    let home = PathBuf::from(
        env::var_os("STATE_DIRECTORY")
            .ok_or_else(|| anyhow!("private state directory required"))?,
    );
    let cred: buzz_hostd::Prepared =
        serde_json::from_slice(&buzz_hostd::bounded(&creds.join("instance"), 131072)?)?;
    let i = cred.instance;
    let (catalog, rev) =
        buzz_hostd::Catalog::read(std::path::Path::new(buzz_hostd::OPERATOR_CATALOG))?;
    i.validate(&catalog, &rev)?;
    let rt = &catalog.runtimes[&i.runtime_id];
    rt.harness.verify()?;
    rt.adapter.verify()?;
    ensure!(catalog.current(&rev)?, "catalog changed before exec");
    let socket = env::var_os("NOTIFY_SOCKET")
        .ok_or_else(|| anyhow!("paired unit requires systemd readiness"))?;
    let error = Command::new(&rt.harness.path)
        .env_clear()
        .env("NOTIFY_SOCKET", socket)
        .env(
            "BUZZ_PAIRED_STARTUP_CHANNELS",
            serde_json::to_string(&i.channels)?,
        )
        .current_dir(&home)
        .env("HOME", &home)
        .env("PATH", "/usr/local/bin:/usr/bin:/bin")
        .envs(cred.provider)
        .env("BUZZ_PRIVATE_KEY", i.secret)
        .env("BUZZ_AUTH_TAG", i.oa)
        .env(
            "BUZZ_RELAY_URL",
            i.scope.community.replacen("https://", "wss://", 1),
        )
        .env("BUZZ_ACP_AGENT_OWNER", i.scope.owner.to_hex())
        .env("BUZZ_ACP_AGENT_COMMAND", &rt.adapter.path)
        .env("BUZZ_ACP_AGENT_ARGS", rt.args.join(","))
        .env("BUZZ_ACP_RESPOND_TO", i.policy.respond_to)
        .env(
            "BUZZ_ACP_RESPOND_TO_ALLOWLIST",
            i.policy.respond_to_allowlist.join(","),
        )
        .exec();
    Err(error.into())
}
