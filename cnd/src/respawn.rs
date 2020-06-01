//! This module deals with respawning swaps upon startup of cnd.
//!
//! "Respawning" spawns refers to the feature of _spawning_ tasks into a runtime
//! for watching the necessary ledgers of all swaps that we know about in the
//! database which have not been completed yet.

use crate::{
    protocol_spawner::{ProtocolSpawner, Spawn},
    storage::{Load, LoadAll, Storage},
    swap_protocols::{halight, herc20},
    LocalSwapId, Side,
};
use chrono::Utc;
use comit::{Protocol, Role};

/// Describes a swap that needs to be respawned.
///
/// We define this as a new type within this module instead of reusing a
/// different one so we can semantically differentiate between swaps that need
/// to be re-spawned and simply "all" swaps.
#[derive(Debug)]
pub struct Swap<A, B> {
    pub id: LocalSwapId,
    pub role: Role,
    pub alpha: A,
    pub beta: B,
}

/// Respawn the protocols for all swaps that are not yet done.
pub async fn respawn(storage: Storage, protocol_spawner: ProtocolSpawner) -> anyhow::Result<()> {
    let swaps = storage.load_all().await?;

    for swap in swaps {
        match swap {
            Swap {
                id,
                role,
                alpha: Protocol::Herc20,
                beta: Protocol::Halight,
            } => {
                let swap: Swap<herc20::Params, halight::Params> = match storage.load(id).await {
                    Ok(swap) => swap,
                    Err(e) => {
                        tracing::warn!("failed to load data for swap {}: {:?}", id, e);
                        continue;
                    }
                };

                protocol_spawner.spawn(id, swap.alpha, Utc::now().naive_local(), Side::Alpha, role);
                protocol_spawner.spawn(id, swap.beta, Utc::now().naive_local(), Side::Beta, role);
            }
            _ => tracing::warn!("unsupported swap combination"),
        }
    }

    Ok(())
}
