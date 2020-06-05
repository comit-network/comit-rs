use crate::{
    db::{ForSwap, Save},
    halight, hbit, herc20,
    network::{RemoteData, WhatAliceLearnedFromBob, WhatBobLearnedFromAlice},
    Load, LocalSwapId, Protocol, ProtocolSpawner, Role, Side, Spawn, Storage,
};
use chrono::NaiveDateTime;

#[derive(Clone, Copy, Debug)]
pub struct DecisionSwap {
    pub id: LocalSwapId,
    pub role: Role,
    pub alpha: Protocol,
    pub beta: Protocol,
}

#[derive(Clone, Copy, Debug)]
pub struct Swap<A, B> {
    pub role: Role,
    pub alpha: A,
    pub beta: B,
    pub start_of_swap: NaiveDateTime,
}

pub async fn save_and_start_swap(
    storage: Storage,
    spawner: ProtocolSpawner,
    id: LocalSwapId,
    data: RemoteData,
) -> anyhow::Result<()> {
    let swap = storage.load(id).await?;
    match (&swap, data) {
        (
            DecisionSwap {
                alpha: Protocol::Herc20,
                beta: Protocol::Halight,
                role: Role::Alice,
                ..
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
                        alpha_redeem_identity: ethereum_identity,
                        beta_refund_identity: lightning_identity,
                    },
                })
                .await?;
        }
        (
            DecisionSwap {
                alpha: Protocol::Herc20,
                beta: Protocol::Halight,
                role: Role::Bob,
                ..
            },
            RemoteData {
                ethereum_identity: Some(ethereum_identity),
                lightning_identity: Some(lightning_identity),
                secret_hash: Some(secret_hash),
                ..
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
        }
        (
            DecisionSwap {
                alpha: Protocol::Halight,
                beta: Protocol::Herc20,
                role: Role::Alice,
                ..
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
        }
        (
            DecisionSwap {
                alpha: Protocol::Halight,
                beta: Protocol::Herc20,
                role: Role::Bob,
                ..
            },
            RemoteData {
                ethereum_identity: Some(ethereum_identity),
                lightning_identity: Some(lightning_identity),
                secret_hash: Some(secret_hash),
                ..
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
        }
        (
            DecisionSwap {
                alpha: Protocol::Herc20,
                beta: Protocol::Hbit,
                role: Role::Alice,
                ..
            },
            RemoteData {
                ethereum_identity: Some(ethereum_identity),
                bitcoin_identity: Some(bitcoin_identity),
                ..
            },
        ) => {
            storage
                .save(ForSwap {
                    local_swap_id: id,
                    data: WhatAliceLearnedFromBob {
                        alpha_redeem_identity: ethereum_identity,
                        beta_refund_identity: bitcoin_identity,
                    },
                })
                .await?;
        }
        (
            DecisionSwap {
                alpha: Protocol::Herc20,
                beta: Protocol::Hbit,
                role: Role::Bob,
                ..
            },
            RemoteData {
                ethereum_identity: Some(ethereum_identity),
                bitcoin_identity: Some(bitcoin_identity),
                secret_hash: Some(secret_hash),
                ..
            },
        ) => {
            storage
                .save(ForSwap {
                    local_swap_id: id,
                    data: WhatBobLearnedFromAlice {
                        secret_hash,
                        alpha_refund_identity: ethereum_identity,
                        beta_redeem_identity: bitcoin_identity,
                    },
                })
                .await?;
        }
        (
            DecisionSwap {
                alpha: Protocol::Hbit,
                beta: Protocol::Herc20,
                role: Role::Alice,
                ..
            },
            RemoteData {
                bitcoin_identity: Some(bitcoin_identity),
                ethereum_identity: Some(ethereum_identity),
                ..
            },
        ) => {
            storage
                .save(ForSwap {
                    local_swap_id: id,
                    data: WhatAliceLearnedFromBob {
                        alpha_redeem_identity: bitcoin_identity,
                        beta_refund_identity: ethereum_identity,
                    },
                })
                .await?;
        }
        (
            DecisionSwap {
                alpha: Protocol::Hbit,
                beta: Protocol::Herc20,
                role: Role::Bob,
                ..
            },
            RemoteData {
                ethereum_identity: Some(ethereum_identity),
                bitcoin_identity: Some(bitcoin_identity),
                secret_hash: Some(secret_hash),
                ..
            },
        ) => {
            storage
                .save(ForSwap {
                    local_swap_id: id,
                    data: WhatBobLearnedFromAlice {
                        secret_hash,
                        alpha_refund_identity: bitcoin_identity,
                        beta_redeem_identity: ethereum_identity,
                    },
                })
                .await?;
        }
        _ => tracing::info!("attempting to save for an unsupported swap"),
    };

    start_swap(&spawner, &storage, swap).await?;

    Ok(())
}

pub async fn start_swap(
    spawner: &ProtocolSpawner,
    storage: &Storage,
    meta_swap: DecisionSwap,
) -> anyhow::Result<()> {
    match meta_swap {
        DecisionSwap {
            alpha: Protocol::Herc20,
            beta: Protocol::Halight,
            ..
        } => {
            let swap =
                Load::<Swap<herc20::Params, halight::Params>>::load(storage, meta_swap.id).await?;
            spawner.spawn(
                meta_swap.id,
                swap.alpha,
                swap.start_of_swap,
                Side::Alpha,
                swap.role,
            );
            spawner.spawn(
                meta_swap.id,
                swap.beta,
                swap.start_of_swap,
                Side::Beta,
                swap.role,
            );
        }
        DecisionSwap {
            alpha: Protocol::Halight,
            beta: Protocol::Herc20,
            ..
        } => {
            let swap =
                Load::<Swap<halight::Params, herc20::Params>>::load(storage, meta_swap.id).await?;
            spawner.spawn(
                meta_swap.id,
                swap.alpha,
                swap.start_of_swap,
                Side::Alpha,
                swap.role,
            );
            spawner.spawn(
                meta_swap.id,
                swap.beta,
                swap.start_of_swap,
                Side::Beta,
                swap.role,
            );
        }
        DecisionSwap {
            alpha: Protocol::Herc20,
            beta: Protocol::Hbit,
            ..
        } => {
            let swap =
                Load::<Swap<herc20::Params, hbit::Params>>::load(storage, meta_swap.id).await?;
            spawner.spawn(
                meta_swap.id,
                swap.alpha,
                swap.start_of_swap,
                Side::Alpha,
                swap.role,
            );
            spawner.spawn(
                meta_swap.id,
                swap.beta,
                swap.start_of_swap,
                Side::Beta,
                swap.role,
            );
        }

        DecisionSwap {
            alpha: Protocol::Hbit,
            beta: Protocol::Herc20,
            ..
        } => {
            let swap =
                Load::<Swap<hbit::Params, herc20::Params>>::load(storage, meta_swap.id).await?;
            spawner.spawn(
                meta_swap.id,
                swap.alpha,
                swap.start_of_swap,
                Side::Alpha,
                swap.role,
            );
            spawner.spawn(
                meta_swap.id,
                swap.beta,
                swap.start_of_swap,
                Side::Beta,
                swap.role,
            );
        }
        _ => tracing::info!("attempting to start an unsupported swap"),
    };

    Ok(())
}
