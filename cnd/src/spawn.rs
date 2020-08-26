use crate::{
    storage::{Load, SwapContext},
    ProtocolSpawner, Role, Side, Spawn, Storage,
};
use chrono::{DateTime, Utc};

#[derive(Clone, Copy, Debug)]
pub struct Swap<A, B> {
    pub role: Role,
    pub alpha: A,
    pub beta: B,
    pub start_of_swap: DateTime<Utc>,
}

pub async fn spawn(
    spawner: &ProtocolSpawner,
    storage: &Storage,
    swap_context: SwapContext,
) -> anyhow::Result<()> {
    within_swap_context!(swap_context, {
        let swap = Load::<Swap<AlphaParams, BetaParams>>::load(storage, swap_context.id).await?;
        spawner.spawn(
            swap_context.id,
            swap.alpha,
            swap.start_of_swap,
            Side::Alpha,
            swap.role,
        );
        spawner.spawn(
            swap_context.id,
            swap.beta,
            swap.start_of_swap,
            Side::Beta,
            swap.role,
        );
    });

    Ok(())
}
