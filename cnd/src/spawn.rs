use crate::{
    connectors::Connectors,
    herc20,
    local_swap_id::LocalSwapId,
    storage::{commands, Load, SwapContext},
    Role, Side, Storage,
};
use anyhow::Result;
use comit::swap::{hbit, Action};
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

impl Swap<hbit::Params, herc20::Params> {
    async fn execute(
        self,
        id: LocalSwapId,
        connectors: Connectors,
        storage: Storage,
    ) -> Result<()> {
        let hbit_facade = crate::hbit::Facade {
            connector: connectors.bitcoin(),
            swap_id: id,
            storage: storage.clone(),
        };
        let herc20_facade = crate::herc20::Facade {
            connector: connectors.ethereum(),
            swap_id: id,
            storage: storage.clone(),
        };

        match self.role {
            Role::Alice => {
                drive(
                    comit::swap::hbit_herc20_alice(
                        hbit_facade,
                        herc20_facade,
                        self.alpha,
                        self.beta,
                        storage.seed.derive_swap_seed(id).derive_secret(),
                        self.start_of_swap,
                    ),
                    storage,
                    id,
                )
                .await
            }
            Role::Bob => {
                drive(
                    comit::swap::hbit_herc20_bob(
                        hbit_facade,
                        herc20_facade,
                        crate::SECP.clone(),
                        self.alpha,
                        self.beta,
                        self.start_of_swap,
                    ),
                    storage,
                    id,
                )
                .await
            }
        }
    }
}

impl Swap<herc20::Params, hbit::Params> {
    async fn execute(
        self,
        id: LocalSwapId,
        connectors: Connectors,
        storage: Storage,
    ) -> Result<()> {
        let hbit_facade = crate::hbit::Facade {
            connector: connectors.bitcoin(),
            swap_id: id,
            storage: storage.clone(),
        };
        let herc20_facade = crate::herc20::Facade {
            connector: connectors.ethereum(),
            swap_id: id,
            storage: storage.clone(),
        };

        match self.role {
            Role::Alice => {
                drive(
                    comit::swap::herc20_hbit_alice(
                        herc20_facade,
                        hbit_facade,
                        crate::SECP.clone(),
                        self.alpha,
                        self.beta,
                        storage.seed.derive_swap_seed(id).derive_secret(),
                        self.start_of_swap,
                    ),
                    storage,
                    id,
                )
                .await
            }
            Role::Bob => {
                drive(
                    comit::swap::herc20_hbit_bob(
                        herc20_facade,
                        hbit_facade,
                        self.alpha,
                        self.beta,
                        self.start_of_swap,
                    ),
                    storage,
                    id,
                )
                .await
            }
        }
    }
}

async fn drive<E>(
    mut swap: impl Stream<Item = Result<Action, E>> + Unpin,
    storage: Storage,
    swap_id: LocalSwapId,
) -> Result<()>
where
    E: std::error::Error + Send + Sync + 'static,
{
    while let Some(action) = swap.try_next().await? {
        storage.next_action.lock().await.insert(swap_id, action);
    }

    Ok(())
}
