//! Shared helpers: config-dir resolution, canonical-hex validation, atomic
//! 0600 writes, owner-pin parsing, host-key loading, tempfile sweeping, and
//! argv redaction.
//!
//! Key-handling rules (binding): no function here
//! ever formats secret material into an error, log, or stdout surface —
//! pubkeys, paths, and lengths only.

use std::fs::{self, File};
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use anyhow::{bail, Context, Result};
use nostr::{Keys, PublicKey, SecretKey};
use zeroize::Zeroizing;

/// Filename of the host secret key (canonical 64-lowercase-hex, 0600).
pub const HOST_KEY_FILE: &str = "host.key";
/// Filename of the owner pin list (one 64-hex pubkey per line).
pub const ALLOWED_OWNERS_FILE: &str = "allowed-owners";
/// Prefix of the atomic-write tempfiles this binary creates.
pub const TEMPFILE_PREFIX: &str = ".buzz-provision.";

/// Resolve the config directory: explicit flag override, else
/// `$HOME/.config/buzz-provision`.
///
/// Deliberately NOT env-activatable: a stray environment variable must never
/// silently redirect the trust root (pin list + host key) at runtime
/// (chuck post-audit T1-3).
pub fn config_dir(explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(dir) = explicit {
        return Ok(dir.to_path_buf());
    }
    let home = std::env::var_os("HOME")
        .filter(|h| !h.is_empty())
        .context("HOME is not set; pass --config-dir explicitly")?;
    Ok(PathBuf::from(home).join(".config").join("buzz-provision"))
}

/// True iff `s` is exactly 64 ASCII lowercase-hex characters.
pub fn is_canonical_64_hex(s: &[u8]) -> bool {
    buzz_provision::is_canonical_64_hex(s)
}

/// Strictly validate a public-key argument (one authority: the lib
/// validator; the value is never echoed back in errors).
pub fn validate_pubkey_arg(s: &str, what: &str) -> Result<PublicKey> {
    buzz_provision::validate_pubkey_arg(s, what, None).map_err(|e| anyhow::anyhow!(e))
}

/// Ensure `dir` exists with mode 0700.
///
/// Existing contents are never touched; if the directory already exists only
/// its own mode is corrected to 0700.
pub fn ensure_dir_0700(dir: &Path) -> Result<()> {
    if dir.symlink_metadata().is_ok() {
        let meta =
            fs::metadata(dir).with_context(|| format!("cannot stat dir {}", dir.display()))?;
        if !meta.is_dir() {
            bail!("path {} exists but is not a directory", dir.display());
        }
        if meta.permissions().mode() & 0o7777 != 0o700 {
            fs::set_permissions(dir, fs::Permissions::from_mode(0o700))
                .with_context(|| format!("cannot set 0700 on {}", dir.display()))?;
        }
    } else {
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)
            .with_context(|| format!("cannot create dir {}", dir.display()))?;
    }
    Ok(())
}

/// Whether an atomic write may replace an existing file.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Clobber {
    /// Replace whatever is at the destination (merge-style files).
    Allow,
    /// Fail if the destination exists at rename time — closes the
    /// check-then-write TOCTOU on refuse-if-exists paths (chuck T1-2).
    Refuse,
}

/// Atomically write `bytes` to `path` with mode 0600: tempfile in the same
/// directory, permissions set before content lands, fsync, rename, dir fsync.
pub fn write_atomic_0600(path: &Path, bytes: &[u8], clobber: Clobber) -> Result<()> {
    let dir = path
        .parent()
        .with_context(|| format!("{} has no parent directory", path.display()))?;
    let mut tmp = tempfile::Builder::new()
        .prefix(TEMPFILE_PREFIX)
        .tempfile_in(dir)
        .with_context(|| format!("cannot create tempfile in {}", dir.display()))?;
    // NamedTempFile is created 0600 on Unix already; set it explicitly so the
    // invariant does not rest on a platform default.
    tmp.as_file()
        .set_permissions(fs::Permissions::from_mode(0o600))
        .context("cannot set 0600 on tempfile")?;
    tmp.as_file_mut()
        .write_all(bytes)
        .context("cannot write tempfile")?;
    tmp.as_file().sync_all().context("cannot fsync tempfile")?;
    match clobber {
        Clobber::Allow => tmp
            .persist(path)
            .map_err(|e| e.error)
            .with_context(|| format!("cannot rename tempfile into {}", path.display()))?,
        Clobber::Refuse => tmp
            .persist_noclobber(path)
            .map_err(|e| e.error)
            .with_context(|| {
                format!(
                    "cannot rename tempfile into {} (a concurrent run may have created it first)",
                    path.display()
                )
            })?,
    };
    // Make the rename durable.
    File::open(dir)
        .and_then(|d| d.sync_all())
        .with_context(|| format!("cannot fsync directory {}", dir.display()))?;
    Ok(())
}

/// Remove stale `.buzz-provision.*` tempfiles from `dir` (chuck T1-5: a hard
/// crash between write and rename can orphan a 0600 tempfile holding secret
/// material; nothing else ever sweeps them).
///
/// Only entries older than one hour are removed so a concurrent run's
/// in-flight tempfile is never yanked out from under its rename. Best-effort:
/// sweep failures are not fatal to the caller's real work.
pub fn sweep_stale_tempfiles(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let cutoff = SystemTime::now() - Duration::from_secs(3600);
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if !name.starts_with(TEMPFILE_PREFIX) {
            continue;
        }
        let path = entry.path();
        let stale = fs::metadata(&path)
            .and_then(|m| m.modified())
            .map(|mtime| mtime < cutoff)
            .unwrap_or(false);
        if stale {
            let _ = shred_file(&path);
            eprintln!(
                "note: removed stale orphaned tempfile {} (crash leftover)",
                path.display()
            );
        }
    }
}

/// Best-effort secure delete: overwrite the file's bytes with zeros, fsync,
/// then unlink (`shred -u` semantics, in-process so provisioning never
/// depends on a coreutils binary being on PATH).
///
/// Best-effort by nature on journaling/CoW filesystems — same caveat as
/// coreutils shred itself documents.
pub fn shred_file(path: &Path) -> Result<()> {
    let meta = fs::metadata(path).with_context(|| format!("cannot stat {}", path.display()))?;
    let len = meta.len() as usize;
    {
        let mut f = fs::OpenOptions::new()
            .write(true)
            .open(path)
            .with_context(|| format!("cannot open {} for overwrite", path.display()))?;
        f.write_all(&vec![0u8; len])
            .with_context(|| format!("cannot overwrite {}", path.display()))?;
        f.sync_all()
            .with_context(|| format!("cannot fsync {}", path.display()))?;
    }
    fs::remove_file(path).with_context(|| format!("cannot remove {}", path.display()))?;
    Ok(())
}

/// Merge owner pins: existing file entries first (validated), then new
/// owners, order-preserving, deduplicated.
pub fn merge_owners(owners_path: &Path, new: &[String]) -> Result<Vec<String>> {
    let mut merged: Vec<String> = Vec::new();
    if owners_path.exists() {
        let existing = fs::read_to_string(owners_path)
            .with_context(|| format!("cannot read {}", owners_path.display()))?;
        for line in existing.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            validate_pubkey_arg(line, "pin-file").with_context(|| {
                format!(
                    "existing pin file {} contains an invalid entry",
                    owners_path.display()
                )
            })?;
            if !merged.iter().any(|m| m == line) {
                merged.push(line.to_owned());
            }
        }
    }
    for owner in new {
        if !merged.contains(owner) {
            merged.push(owner.clone());
        }
    }
    Ok(merged)
}

/// Write the merged owner-pin list (atomic, clobbering — merge semantics).
pub fn write_owners_file(owners_path: &Path, owners: &[String]) -> Result<()> {
    let mut content = String::with_capacity(owners.len() * 65);
    for owner in owners {
        content.push_str(owner);
        content.push('\n');
    }
    write_atomic_0600(owners_path, content.as_bytes(), Clobber::Allow)
}

/// Read and strictly validate the host key file; returns the parsed keys.
///
/// The raw buffer is zeroized on drop. No error path formats file content.
pub fn load_host_keys(key_path: &Path) -> Result<Keys> {
    let raw = Zeroizing::new(
        fs::read(key_path).with_context(|| format!("cannot read {}", key_path.display()))?,
    );
    if !is_canonical_64_hex(&raw) {
        bail!(
            "{} is not canonical: expected exactly 64 lowercase hex bytes \
             (no newline, no bech32); refusing to use it",
            key_path.display()
        );
    }
    // Charset already validated; from_utf8 cannot fail but stay fail-closed.
    let sk_str = std::str::from_utf8(&raw)
        .map_err(|_| anyhow::anyhow!("{} is not valid UTF-8", key_path.display()))?;
    let secret = SecretKey::from_hex(sk_str).map_err(|_| {
        // Deliberately drop the underlying error: it could echo input bytes.
        anyhow::anyhow!(
            "{} does not contain a valid secp256k1 secret key",
            key_path.display()
        )
    })?;
    Ok(Keys::new(secret))
}

// One redactor for every argv-parsing mint binary.
pub use buzz_provision::redact_key_material;

/// Resolve the agent env-file directory: explicit flag override, else
/// `$HOME/services/buzz-agents` (the deployment convention the systemd
/// units read `EnvironmentFile=` from).
pub fn default_env_dir(explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(dir) = explicit {
        return Ok(dir.to_path_buf());
    }
    let home = std::env::var_os("HOME")
        .filter(|h| !h.is_empty())
        .context("HOME is not set; pass --env-dir explicitly")?;
    Ok(PathBuf::from(home).join("services").join("buzz-agents"))
}

/// Derive the pubkey a credential file corresponds to, if it parses.
pub fn credential_pubkey(cred_path: &Path) -> Option<PublicKey> {
    let raw = Zeroizing::new(fs::read(cred_path).ok()?);
    if !is_canonical_64_hex(&raw) {
        return None;
    }
    let secret = SecretKey::from_hex(std::str::from_utf8(&raw).ok()?).ok()?;
    Some(Keys::new(secret).public_key())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redaction_covers_hex_and_bech32_tokens() {
        let sk = "f5c79eab2f44284ff278780c34aab621aafb149be4d7dba8f9625bfd99419733";
        let msg = format!("error: unexpected argument '{sk}' found");
        let red = redact_key_material(&msg);
        assert!(!red.contains(sk));
        assert!(red.contains("<redacted-key-material>"));
        assert!(red.contains("unexpected argument"));

        let nsec = "nsec1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq";
        let red = redact_key_material(&format!("bad value {nsec} here"));
        assert!(!red.contains(nsec));

        // Short/ordinary tokens survive.
        let red = redact_key_material("error: invalid value 'abc' for '--name'");
        assert!(red.contains("'abc'"));
    }

    #[test]
    fn redaction_handles_multibyte_text() {
        let red = redact_key_material("héllo — nsec1qqqqqqqqqqqqqqqqqqqq — done");
        assert!(red.contains("héllo"));
        assert!(!red.contains("nsec1"));
    }
}
