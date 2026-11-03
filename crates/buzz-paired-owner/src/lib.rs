//! Native owner-side journal and signed workflow. A caller supplies the unlocked
//! native signer and selected HTTPS origin for each operation.
#![forbid(unsafe_code)]
mod create;
mod device;
mod history;
mod http;
mod ready;
#[cfg(windows)]
mod windows_store;
use anyhow::{ensure, Result};
pub use buzz_core::paired_message::{Packet, Policy, Scope};
pub use create::{Draft, RequestView};
pub use device::{Device, HostView};
pub use history::HistoryView;
pub use http::{Binding, Endpoint, Page};
use nostr::{Event, Keys};
use rand::{rngs::SysRng, TryRng};
use std::time::{SystemTime, UNIX_EPOCH};
#[cfg(windows)]
pub use windows_store::protect_windows_journal;

fn random() -> Result<String> {
    let mut bytes = [0; 32];
    SysRng
        .try_fill_bytes(&mut bytes)
        .map_err(|_| anyhow::anyhow!("random source unavailable"))?;
    Ok(hex::encode(bytes))
}

fn now() -> Result<u64> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}

fn id(packet: &Packet) -> Result<String> {
    Ok(serde_json::from_str::<Event>(packet.event.get())?
        .id
        .to_hex())
}

fn session(scope: &Scope, selected: &str, keys: &Keys) -> Result<()> {
    ensure!(
        scope.community == selected && scope.owner == keys.public_key(),
        "active community or signer changed"
    );
    Ok(())
}

#[cfg(test)]
mod tests;
