use crate::{
    connectors::Connectors,
    halbit, hbit, herc20,
    local_swap_id::LocalSwapId,
    storage::{commands, Load, SwapContext},
    Role, Side, Storage,
};
use anyhow::{Context, Result};
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

        handle.spawn(async move {
            let result = future::join(alpha_handle, beta_handle).await;

            match result {
                (Err(e), _) | (_, Err(e)) => {
                    let e = Box::new(e);
                    tracing::error!(
                        error = &e as &(dyn std::error::Error + 'static),
                        swap = %swap_id,
                        "runtime error while executing protocol futures",
                    );
                }
                _ => {}
            }

            if let Err(e) = storage
                .db
                .do_in_transaction(|conn| commands::update_order_of_swap_to_closed(conn, swap_id))
                .await
            {
                tracing::error!("failed to update order state: {:#}", e);
            }
        });
    });

    Ok(())
}

impl ProtocolContext<herc20::Params> {
    fn spawn(
        self,
        connectors: Connectors,
        storage: Storage,
        handle: Handle,
    ) -> Result<JoinHandle<()>> {
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
    ) -> Result<JoinHandle<()>> {
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
    ) -> Result<JoinHandle<()>> {
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
