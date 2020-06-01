use crate::{
    db::{ForSwap, Save, Swap},
    network::comit::RemoteData,
    protocol_spawner::{ProtocolSpawner, Spawn},
    storage::{Load, Storage},
    swap_protocols::{halight, herc20},
    LocalSwapId,
};
use ::comit::{
    network::{WhatAliceLearnedFromBob, WhatBobLearnedFromAlice},
    Protocol, Role, Side,
};
use chrono::offset::Utc;

pub async fn start_swap(
    storage: Storage,
    spawner: ProtocolSpawner,
    id: LocalSwapId,
    data: RemoteData,
) -> anyhow::Result<()>
where
    ProtocolSpawner: Spawn<herc20::Params> + Spawn<halight::Params>,
{
    let start_of_swap = Utc::now().naive_local();
    let swap = storage.load(id).await?;

    match (swap, data) {
        (
            Swap {
                alpha: Protocol::Herc20,
                beta: Protocol::Halight,
                role: role @ Role::Alice,
            },
            RemoteData {
                ethereum_identity: Some(ethereum_identity),
                lightning_identity: Some(lightning_identity),
                // Do not make this None, secret_hash is in the behaviour event for Alice.
                secret_hash: _,
            },
        ) => {
            storage
                .save(ForSwap {
                    local_swap_id: id,
                    data: WhatAliceLearnedFromBob {
                        alpha_redeem_identity: ethereum_identity,
                        beta_refund_identity: lightning_identity,
                    },
                })
                .await?;

            let herc20_params: herc20::Params = storage.load(id).await?;
            let halight_params: halight::Params = storage.load(id).await?;

            spawner.spawn(id, herc20_params, start_of_swap, Side::Alpha, role);
            spawner.spawn(id, halight_params, start_of_swap, Side::Beta, role);
        }
        (
            Swap {
                alpha: Protocol::Herc20,
                beta: Protocol::Halight,
                role: role @ Role::Bob,
            },
            RemoteData {
                ethereum_identity: Some(ethereum_identity),
                lightning_identity: Some(lightning_identity),
                secret_hash: Some(secret_hash),
            },
        ) => {
            storage
                .save(ForSwap {
                    local_swap_id: id,
                    data: WhatBobLearnedFromAlice {
                        secret_hash,
                        alpha_refund_identity: ethereum_identity,
                        beta_redeem_identity: lightning_identity,
                    },
                })
                .await?;

            let herc20_params: herc20::Params = storage.load(id).await?;
            let halight_params: halight::Params = storage.load(id).await?;

            spawner.spawn(id, herc20_params, start_of_swap, Side::Alpha, role);
            spawner.spawn(id, halight_params, start_of_swap, Side::Beta, role);
        }
        (
            Swap {
                alpha: Protocol::Halight,
                beta: Protocol::Herc20,
                role: role @ Role::Alice,
            },
            RemoteData {
                ethereum_identity: Some(ethereum_identity),
                lightning_identity: Some(lightning_identity),
                ..
            },
        ) => {
            storage
                .save(ForSwap {
                    local_swap_id: id,
                    data: WhatAliceLearnedFromBob {
                        alpha_redeem_identity: lightning_identity,
                        beta_refund_identity: ethereum_identity,
                    },
                })
                .await?;

            let halight_params: halight::Params = storage.load(id).await?;
            let herc20_params: herc20::Params = storage.load(id).await?;

            spawner.spawn(id, halight_params, start_of_swap, Side::Alpha, role);
            spawner.spawn(id, herc20_params, start_of_swap, Side::Beta, role);
        }
        (
            Swap {
                alpha: Protocol::Halight,
                beta: Protocol::Herc20,
                role: role @ Role::Bob,
            },
            RemoteData {
                ethereum_identity: Some(ethereum_identity),
                lightning_identity: Some(lightning_identity),
                secret_hash: Some(secret_hash),
            },
        ) => {
            storage
                .save(ForSwap {
                    local_swap_id: id,
                    data: WhatBobLearnedFromAlice {
                        secret_hash,
                        alpha_refund_identity: lightning_identity,
                        beta_redeem_identity: ethereum_identity,
                    },
                })
                .await?;

            let halight_params: halight::Params = storage.load(id).await?;
            let herc20_params: herc20::Params = storage.load(id).await?;

            spawner.spawn(id, halight_params, start_of_swap, Side::Alpha, role);
            spawner.spawn(id, herc20_params, start_of_swap, Side::Beta, role);
        }
        _ => tracing::info!("attempting to start an unsupported swap"),
    };

    Ok(())
}
