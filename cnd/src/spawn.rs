use crate::{
    connectors::Connectors,
    hbit, herc20,
    local_swap_id::LocalSwapId,
    storage::{commands, Load, SwapContext},
    Role, Side, Storage,
};
use anyhow::{Context, Result};
use diesel::SqliteConnection;
use futures::prelude::*;
use time::OffsetDateTime;
use tokio::runtime::Handle;

#[derive(Clone, Copy, Debug)]
pub struct Swap<A, B> {
    pub role: Role,
    pub alpha: A,
    pub beta: B,
    pub start_of_swap: OffsetDateTime,
}

#[derive(Clone, Copy, Debug)]
struct MetaData {
    id: LocalSwapId,
    role: Role,
    side: Side,
    start: OffsetDateTime,
}

pub async fn spawn(
    connectors: Connectors,
    storage: Storage,
    handle: Handle,
    swap_context: SwapContext,
) -> anyhow::Result<()> {
    within_swap_context!(swap_context, {
        let swap = Load::<Swap<AlphaParams, BetaParams>>::load(&storage, swap_context.id).await?;

        handle.spawn(async move {
            let swap_result = swap
                .execute(swap_context.id, connectors.clone(), storage.clone())
                .await;

            handle_swap_result(swap_result, storage, swap_context.id).await;
        });
    });

    Ok(())
}

async fn handle_swap_result(swap_result: Result<()>, storage: Storage, swap_id: LocalSwapId) {
    let db_update: Box<dyn Fn(&SqliteConnection) -> Result<()> + Send> = match swap_result {
        Ok(()) => {
            tracing::info!(swap = %swap_id, "swap completed");

            Box::new(move |conn| {
                commands::update_order_of_swap_to_closed(conn, swap_id)?;
                commands::mark_swap_as_completed(conn, swap_id, OffsetDateTime::now_utc())?;

                Ok(())
            })
        }
        Err(e) => {
            tracing::error!(swap = %swap_id, "failed to complete swap: {:#}", e);

            Box::new(move |conn| {
                commands::update_order_of_swap_to_failed(conn, swap_id)?;
                // we don't mark a swap as completed in case of failure so that a
                // restart of the node will respawn the swap

                Ok(())
            })
        }
    };

    if let Err(e) = storage.db.do_in_transaction(db_update).await {
        tracing::warn!("failed to update db state: {:#}", e)
    }
}

macro_rules! impl_execute {
    ($alpha:ident, $beta:ident) => {
        impl Swap<$alpha::Params, $beta::Params> {
            async fn execute(
                self,
                id: LocalSwapId,
                connectors: Connectors,
                storage: Storage,
            ) -> Result<()> {
                let alpha = $alpha(
                    MetaData {
                        id,
                        side: Side::Alpha,
                        role: self.role,
                        start: self.start_of_swap,
                    },
                    self.alpha,
                    connectors.clone(),
                    storage.clone(),
                );
                let beta = $beta(
                    MetaData {
                        id,
                        side: Side::Beta,
                        role: self.role,
                        start: self.start_of_swap,
                    },
                    self.beta,
                    connectors.clone(),
                    storage.clone(),
                );

                future::try_join(alpha, beta).await.map(|_| ())
            }
        }
    };
}

impl_execute!(herc20, hbit);
impl_execute!(hbit, herc20);

async fn herc20(
    metadata: MetaData,
    params: herc20::Params,
    connectors: Connectors,
    storage: Storage,
) -> Result<()> {
    herc20::new(
        metadata.id,
        params,
        metadata.start,
        metadata.role,
        metadata.side,
        storage.clone(),
        connectors.ethereum(),
    )
    .await
    .with_context(|| {
        format!(
            "failed to complete herc20 as {} ledger protocol",
            metadata.side
        )
    })
}

async fn hbit(
    metadata: MetaData,
    params: hbit::Params,
    connectors: Connectors,
    storage: Storage,
) -> Result<()> {
    hbit::new(
        metadata.id,
        params,
        metadata.start,
        metadata.role,
        metadata.side,
        storage.clone(),
        connectors.bitcoin(),
    )
    .await
    .with_context(|| {
        format!(
            "failed to complete hbit as {} ledger protocol",
            metadata.side
        )
    })
}
