use crate::{
    halight, hbit, herc20, Load, LocalSwapId, Protocol, ProtocolSpawner, Role, Side, Spawn, Storage,
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
