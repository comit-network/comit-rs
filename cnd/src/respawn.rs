//! This module deals with respawning swaps upon startup of cnd.
//!
//! "Respawning" spawns refers to the feature of _spawning_ tasks into a runtime
//! for watching the necessary ledgers of all swaps that we know about in the
//! database which have not been completed yet.

use crate::{
    connectors::Connectors,
    spawn::spawn,
    storage::{commands, queries::get_active_swap_contexts, Storage},
};
use tokio::runtime::Handle;

/// Respawn the protocols for all swaps that are not yet done.
pub async fn respawn(
    storage: Storage,
    connectors: Connectors,
    handle: Handle,
) -> anyhow::Result<()> {
    let swaps = storage
        .db
        .do_in_transaction(get_active_swap_contexts)
        .await?;

    for swap in swaps {
        let id = swap.id;
        if let Err(e) = spawn(connectors.clone(), storage.clone(), handle.clone(), swap).await {
            tracing::warn!(swap_id = %id, "failed to spawn swap {:#}", e);
            continue;
        };

        if let Err(e) = storage
            .db
            .do_in_transaction(|conn| commands::update_order_of_swap_to_settling(conn, id))
            .await
        {
            tracing::warn!(swap_id = %id, "failed to update order state for swap {:#}", e);
        }
    }

    Ok(())
}
