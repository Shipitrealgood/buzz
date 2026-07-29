//! Provider mode: invoked by the Buzz desktop with a JSON request on stdin.
//!
//! `info` is answered locally (static schema). `deploy` is forwarded verbatim
//! over `ssh` to the remote helper (`buzz-backend-ssh remote-deploy`), whose
//! JSON response is passed back through on stdout. Forwarding the raw request
//! bytes — not a re-serialization — keeps the provider transparent to payload
//! fields it does not know about.

use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::Context;

use crate::protocol::{
    info_response, load_defaults_file, respond_err, respond_ok, ProviderConfig, ProviderRequest,
};

/// Leave headroom under the desktop's own 600s deploy timeout so the user
/// sees our actionable ssh error, not a generic provider timeout.
const DEPLOY_TIMEOUT: Duration = Duration::from_secs(540);

/// Cap captured remote output; the desktop caps its own read at 1 MB.
const OUTPUT_CAP: usize = 1024 * 1024;

pub fn run(raw_request: &str) -> anyhow::Result<()> {
    let request: ProviderRequest = match serde_json::from_str(raw_request) {
        Ok(req) => req,
        Err(e) => {
            respond_err(&format!("invalid provider request JSON: {e}"));
            return Ok(());
        }
    };

    match request.op.as_str() {
        "info" => {
            respond_ok(info_response(&load_defaults_file()));
            Ok(())
        }
        "deploy" => {
            match forward_deploy(raw_request, &request) {
                Ok(response) => {
                    // Trust but verify: the remote helper speaks the same
                    // protocol, so pass its response through untouched. A
                    // non-JSON response was already rejected in forward_deploy.
                    println!("{response}");
                }
                Err(e) => respond_err(&format!("{e:#}")),
            }
            Ok(())
        }
        other => {
            respond_err(&format!("unsupported op '{other}'"));
            Ok(())
        }
    }
}

/// Forward the raw deploy request to the remote helper over ssh and return
/// its JSON response.
fn forward_deploy(
    raw_request: &str,
    request: &ProviderRequest,
) -> anyhow::Result<serde_json::Value> {
    let config = ProviderConfig::from_value(request.provider_config.as_ref())
        .merge_defaults(&load_defaults_file());
    let argv = ssh_argv(&config)?;

    // Forward the request with the merged provider config so the remote
    // helper sees the same effective values (acp_path, remote_path_prepend)
    // this side resolved. Agent payload fields pass through untouched.
    let forwarded = {
        let mut value: serde_json::Value =
            serde_json::from_str(raw_request).context("re-parsing request for forwarding")?;
        value["provider_config"] =
            serde_json::to_value(&config).context("serializing merged provider config")?;
        serde_json::to_string(&value).context("serializing forwarded request")?
    };

    let mut cmd = Command::new(&argv[0]);
    cmd.args(&argv[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .with_context(|| format!("failed to run '{}' — is OpenSSH installed?", argv[0]))?;

    // Write the request and close stdin so the remote helper sees EOF.
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(forwarded.as_bytes())
            .context("failed to write request to ssh stdin")?;
    }

    // Drain stdout/stderr on threads to avoid pipe-buffer deadlock, then
    // poll for exit with a deadline.
    let stdout_handle = capture_thread(child.stdout.take());
    let stderr_handle = capture_thread(child.stderr.take());

    let deadline = Instant::now() + DEPLOY_TIMEOUT;
    let status = loop {
        match child.try_wait().context("failed to wait for ssh")? {
            Some(status) => break status,
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                anyhow::bail!(
                    "remote deploy timed out after {}s (host '{}')",
                    DEPLOY_TIMEOUT.as_secs(),
                    config.host
                );
            }
            None => std::thread::sleep(Duration::from_millis(50)),
        }
    };

    let stdout = join_capture(stdout_handle);
    let stderr = join_capture(stderr_handle);

    if !status.success() {
        let detail = String::from_utf8_lossy(&stderr);
        let detail = detail.trim();
        let code = status
            .code()
            .map(|c| c.to_string())
            .unwrap_or_else(|| "signal".to_string());
        if detail.is_empty() {
            anyhow::bail!("ssh to '{}' failed (exit {code})", config.host);
        }
        anyhow::bail!(
            "ssh to '{}' failed (exit {code}): {}",
            config.host,
            truncate_chars(detail, 2048)
        );
    }

    let stdout_str = String::from_utf8_lossy(&stdout);
    // The remote helper writes exactly one JSON line; tolerate leading noise
    // from remote shell init by scanning lines, mirroring the desktop parser.
    stdout_str
        .lines()
        .find_map(|line| serde_json::from_str(line.trim()).ok())
        .or_else(|| serde_json::from_str(stdout_str.trim()).ok())
        .ok_or_else(|| {
            let err = String::from_utf8_lossy(&stderr);
            anyhow::anyhow!(
                "remote helper produced no JSON response — is '{}' installed on '{}'? stderr: {}",
                config.remote_command,
                config.host,
                truncate_chars(err.trim(), 2048)
            )
        })
}

/// Character-boundary-safe truncation (byte slicing can panic mid-UTF-8).
fn truncate_chars(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

/// Build the ssh argv. BatchMode fails fast instead of hanging on an
/// interactive prompt the desktop can never answer.
fn ssh_argv(config: &ProviderConfig) -> anyhow::Result<Vec<String>> {
    let host = config.host.trim();
    if host.is_empty() {
        anyhow::bail!("provider config is missing 'host'");
    }
    // A leading '-' would be parsed as an ssh option; there is no legitimate
    // host starting with '-'.
    if host.starts_with('-') {
        anyhow::bail!("invalid ssh host '{host}'");
    }
    let remote_command = config.remote_command.trim();
    if remote_command.is_empty() {
        anyhow::bail!("provider config 'remote_command' must not be empty");
    }
    if remote_command.contains(char::is_whitespace) {
        // The remote command line is re-parsed by the remote shell; a path
        // with spaces would need quoting we deliberately do not attempt.
        anyhow::bail!("'remote_command' must be a single path without spaces");
    }

    let mut argv = vec![
        "ssh".to_string(),
        "-o".to_string(),
        "BatchMode=yes".to_string(),
        "-o".to_string(),
        "ConnectTimeout=15".to_string(),
    ];
    argv.extend(config.ssh_args.split_whitespace().map(String::from));
    argv.push(host.to_string());
    argv.push(remote_command.to_string());
    argv.push("remote-deploy".to_string());
    Ok(argv)
}

fn capture_thread(
    pipe: Option<impl Read + Send + 'static>,
) -> Option<std::thread::JoinHandle<Vec<u8>>> {
    pipe.map(|mut reader| {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            let mut chunk = [0u8; 8192];
            loop {
                match reader.read(&mut chunk) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if buf.len() < OUTPUT_CAP {
                            buf.extend_from_slice(&chunk[..n]);
                        }
                    }
                }
            }
            buf
        })
    })
}

fn join_capture(handle: Option<std::thread::JoinHandle<Vec<u8>>>) -> Vec<u8> {
    handle.and_then(|h| h.join().ok()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(host: &str) -> ProviderConfig {
        serde_json::from_value(serde_json::json!({ "host": host })).unwrap_or_default()
    }

    #[test]
    fn ssh_argv_shape() {
        let argv = ssh_argv(&config("agentbox")).expect("valid");
        assert_eq!(argv[0], "ssh");
        assert!(argv.contains(&"BatchMode=yes".to_string()));
        assert_eq!(argv[argv.len() - 3], "agentbox");
        assert_eq!(argv[argv.len() - 2], "buzz-backend-ssh");
        assert_eq!(argv[argv.len() - 1], "remote-deploy");
    }

    #[test]
    fn ssh_argv_rejects_empty_and_option_like_hosts() {
        assert!(ssh_argv(&config("")).is_err());
        assert!(ssh_argv(&config("-oProxyCommand=evil")).is_err());
    }

    #[test]
    fn ssh_argv_includes_extra_args_before_host() {
        let cfg: ProviderConfig = serde_json::from_value(
            serde_json::json!({ "host": "agentbox", "ssh_args": "-p 2222" }),
        )
        .expect("valid config");
        let argv = ssh_argv(&cfg).expect("valid");
        let p_pos = argv.iter().position(|a| a == "-p").expect("has -p");
        let host_pos = argv.iter().position(|a| a == "agentbox").expect("has host");
        assert!(p_pos < host_pos);
        assert_eq!(argv[p_pos + 1], "2222");
    }
}
