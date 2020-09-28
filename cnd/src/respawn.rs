//! This module deals with respawning swaps upon startup of cnd.
//!
//! "Respawning" spawns refers to the feature of _spawning_ tasks into a runtime
//! for watching the necessary ledgers of all swaps that we know about in the
//! database which have not been completed yet.

use crate::{
    protocol_spawner::ProtocolSpawner,
    spawn::spawn,
    storage::{queries::get_all_swap_contexts, Storage},
};

/// Respawn the protocols for all swaps that are not yet done.
pub async fn respawn(storage: Storage, spawner: ProtocolSpawner) -> anyhow::Result<()> {
    let swaps = storage.db.do_in_transaction(get_all_swap_contexts).await?;

    for swap in swaps {
        let id = swap.id;
        match spawn(&spawner, &storage, swap).await {
            Err(e) => {
                tracing::warn!("failed to load data for swap {}: {:?}", id, e);
                continue;
            }
            _ => continue,
        };
    }

    Ok(())
}
