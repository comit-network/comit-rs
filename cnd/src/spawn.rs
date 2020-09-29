use crate::{
    connectors::Connectors,
    halbit, hbit, herc20,
    local_swap_id::LocalSwapId,
    storage::{commands, Load, SwapContext},
    Role, Side, Storage,
};
use anyhow::{Context, Result};
use diesel::SqliteConnection;
use futures::prelude::*;
use time::OffsetDateTime;
use tokio::{runtime::Handle, task::JoinHandle};

#[derive(Clone, Copy, Debug)]
pub struct Swap<A, B> {
    pub role: Role,
    pub alpha: A,
    pub beta: B,
    pub start_of_swap: OffsetDateTime,
}

impl<A, B> Swap<A, B> {
    fn into_context_pair(
        self,
        swap_context: SwapContext,
    ) -> (ProtocolContext<A>, ProtocolContext<B>) {
        let alpha = ProtocolContext {
            id: swap_context.id,
            start_of_swap: self.start_of_swap,
            role: swap_context.role,
            side: Side::Alpha,
            params: self.alpha,
        };
        let beta = ProtocolContext {
            id: swap_context.id,
            start_of_swap: self.start_of_swap,
            role: swap_context.role,
            side: Side::Beta,
            params: self.beta,
        };

        (alpha, beta)
    }
}

/// The context within which a protocol can be spawned into a runtime.
pub struct ProtocolContext<P> {
    pub id: LocalSwapId,
    pub start_of_swap: OffsetDateTime,
    pub side: Side,
    pub role: Role,
    pub params: P,
}

pub async fn spawn(
    connectors: Connectors,
    storage: Storage,
    handle: Handle,
    swap_context: SwapContext,
) -> anyhow::Result<()> {
    within_swap_context!(swap_context, {
        let swap = Load::<Swap<AlphaParams, BetaParams>>::load(&storage, swap_context.id).await?;
        let swap_id = swap_context.id;
        let (alpha, beta) = swap.into_context_pair(swap_context);

        let alpha_handle = alpha
            .spawn(connectors.clone(), storage.clone(), handle.clone())
            .context("failed to spawn protocol for alpha ledger")?;
        let beta_handle = beta
            .spawn(connectors.clone(), storage.clone(), handle.clone())
            .context("failed to spawn protocol for alpha ledger")?;

        handle.spawn(swap_result_handler(
            alpha_handle,
            beta_handle,
            storage,
            swap_id,
        ));
    });

    Ok(())
}

async fn swap_result_handler(
    alpha: JoinHandle<Result<()>>,
    beta: JoinHandle<Result<()>>,
    storage: Storage,
    swap_id: LocalSwapId,
) {
    let db_update: Box<dyn Fn(&SqliteConnection) -> Result<()> + Send> =
        match future::try_join(alpha, beta).await {
            // Join successful and both protocols finish successfully
            Ok((Ok(()), Ok(()))) => {
                tracing::info!(swap = %swap_id, "swap completed");

                Box::new(move |conn| {
                    commands::update_order_of_swap_to_closed(conn, swap_id)?;
                    commands::mark_swap_as_completed(conn, swap_id, OffsetDateTime::now_utc())?;

                    Ok(())
                })
            }
            // Join successful but one of the protocols failed
            Ok((Err(e), _)) | Ok((_, Err(e))) => {
                tracing::error!(swap = %swap_id, "failed to complete swap: {:#}", e);

                Box::new(move |conn| {
                    commands::update_order_of_swap_to_failed(conn, swap_id)?;
                    // we don't mark a swap as completed in case of failure so that a
                    // restart of the node will respawn the swap

                    Ok(())
                })
            }
            // Join unsuccessful
            Err(e) => {
                tracing::error!(swap = %swap_id, "failed to join protocol futures: {:?}", e);
                return;
            }
        };

    if let Err(e) = storage.db.do_in_transaction(db_update).await {
        tracing::warn!("failed to update db state: {:#}", e)
    }
}

impl ProtocolContext<herc20::Params> {
    fn spawn(
        self,
        connectors: Connectors,
        storage: Storage,
        handle: Handle,
    ) -> Result<JoinHandle<Result<()>>> {
        let task = herc20::new(
            self.id,
            self.params,
            self.start_of_swap,
            self.role,
            self.side,
            storage,
            connectors.ethereum(),
        );

        Ok(handle.spawn(task))
    }
}

impl ProtocolContext<hbit::Params> {
    fn spawn(
        self,
        connectors: Connectors,
        storage: Storage,
        handle: Handle,
    ) -> Result<JoinHandle<Result<()>>> {
        let task = hbit::new(
            self.id,
            self.params,
            self.start_of_swap,
            self.role,
            self.side,
            storage,
            connectors.bitcoin(),
        );

        Ok(handle.spawn(task))
    }
}

impl ProtocolContext<halbit::Params> {
    fn spawn(
        self,
        connectors: Connectors,
        storage: Storage,
        handle: Handle,
    ) -> Result<JoinHandle<Result<()>>> {
        match (self.role, self.side) {
            (Role::Alice, Side::Alpha) | (Role::Bob, Side::Beta) => {
                let task = halbit::new(
                    self.id,
                    self.params,
                    self.role,
                    self.side,
                    storage,
                    connectors.lnd_as_sender()?,
                );

                Ok(handle.spawn(task))
            }
            (Role::Bob, Side::Alpha) | (Role::Alice, Side::Beta) => {
                let task = halbit::new(
                    self.id,
                    self.params,
                    self.role,
                    self.side,
                    storage,
                    connectors.lnd_as_receiver()?,
                );

                Ok(handle.spawn(task))
            }
        }
    }
}
