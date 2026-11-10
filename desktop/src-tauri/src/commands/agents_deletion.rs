//! Shared local deletion after each backend establishes its own authority.
use super::*;

// Async so the blocking body (disk reads/writes, process termination, keyring
// delete, nest regeneration) runs off the main UI thread via spawn_blocking.
pub(super) fn run_managed_agent_deletion<T>(
    base_dir: &std::path::Path,
    pubkey: &str,
    records: &mut Vec<ManagedAgentRecord>,
    delete: impl FnOnce(&mut Vec<ManagedAgentRecord>) -> Result<T, String>,
) -> Result<T, String> {
    recover_pending_assignment_cleanup(base_dir, |pending_pubkey| {
        records
            .iter()
            .any(|record| record.pubkey.eq_ignore_ascii_case(pending_pubkey))
    })?;
    with_agent_assignments_cleared(base_dir, pubkey, || delete(records))
}

// Caller holds managed_agents_store_lock; strict hosted completion propagates
// key/tombstone failures, with phase 3 in the existing intent as retry witness.
pub(super) fn finalize_managed_agent_deletion<R: tauri::Runtime>(
    app: &AppHandle<R>,
    state: &AppState,
    pubkey: &str,
    records: &mut Vec<ManagedAgentRecord>,
    confirmed_scope: Option<&crate::managed_agents::retention::RetentionScope>,
    before_remove: impl FnOnce(&mut Vec<ManagedAgentRecord>) -> Result<(), String>,
) -> Result<(), String> {
    let base_dir = managed_agents_base_dir(app)?;
    run_managed_agent_deletion(&base_dir, pubkey, records, |records| {
        before_remove(records)?;
        records.retain(|record| record.pubkey != pubkey);
        save_managed_agents(app, records)
    })?;
    state.clear_agent_session_caches(pubkey);
    if let Some(scope) = confirmed_scope {
        crate::managed_agents::storage::try_delete_agent_key(pubkey)?;
        pending::tombstone_managed_agent_at(&scope.db_path, &scope.owner_keys, pubkey)?;
    } else {
        crate::managed_agents::delete_agent_key(pubkey);
        if let Err(error) = pending::try_tombstone_managed_agent_pending(app, state, pubkey) {
            eprintln!("buzz-desktop: agent-tombstone: {error}");
        }
    }
    Ok(())
}
