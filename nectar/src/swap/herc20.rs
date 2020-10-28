pub use comit::{
    herc20::{Deployed, Params, Redeemed, Refunded},
    swap::herc20::{Funded, IncorrectlyFunded},
};

use crate::{
    database::{Load, Save},
    swap::{ethereum::Wallet, Database},
    SwapId,
};
use backoff::{backoff::Constant, future::FutureOperation};
use comit::{
    ethereum,
    ethereum::ChainId,
    herc20::{watch_for_deployed, watch_for_funded, watch_for_redeemed},
    swap::herc20::{WatchForDeployed, WatchForFunded, WatchForRedeemed},
};
use futures::TryFutureExt;
use std::{sync::Arc, time::Duration};
use time::OffsetDateTime;

pub struct Facade {
    pub wallet: Wallet,
    pub db: Arc<Database>,
    pub swap_id: SwapId,
}

impl Facade {
    async fn wait_until_confirmed(&self, tx: ethereum::Hash, chain_id: ChainId) {
        let operation = || {
            self.wallet
                .inner
                .wait_until_confirmed(tx, chain_id)
                .map_err(backoff::Error::Transient)
        };

        let _ = operation
            .retry_notify(Constant::new(Duration::from_secs(1)), |e, _| {
                tracing::warn!(
                    "failed to wait for {} getting confirmed, retrying ...: {:#}",
                    tx,
                    e
                )
            })
            .await;
    }
}

#[async_trait::async_trait]
impl WatchForDeployed for Facade {
    async fn watch_for_deployed(
        &self,
        params: Params,
        utc_start_of_swap: OffsetDateTime,
    ) -> Deployed {
        if let Ok(Some(Deployed {
            transaction,
            location,
        })) = self.db.load(self.swap_id)
        {
            self.wait_until_confirmed(transaction, params.chain_id)
                .await;

            return Deployed {
                transaction,
                location,
            };
        }

        let operation = || {
            watch_for_deployed(
                self.wallet.connector.as_ref(),
                params.clone(),
                utc_start_of_swap,
            )
            .map_err(backoff::Error::Transient)
        };

        let deployed = operation
            .retry_notify(Constant::new(Duration::from_secs(1)), |e, _| {
                tracing::warn!(
                    "failed to watch for herc20 deployment, retrying ...: {:#}",
                    e
                )
            })
            .await
            .expect("transient error is never returned");

        let _ = self.db.save(deployed, self.swap_id).await;

        deployed
    }
}

#[async_trait::async_trait]
impl WatchForFunded for Facade {
    async fn watch_for_funded(
        &self,
        params: Params,
        deploy_event: Deployed,
        utc_start_of_swap: OffsetDateTime,
    ) -> Result<Funded, IncorrectlyFunded> {
        if let Ok(Some(Funded { transaction, asset })) = self.db.load(self.swap_id) {
            self.wait_until_confirmed(transaction, params.chain_id)
                .await;

            return Ok(Funded { transaction, asset });
        }

        let operation = || {
            watch_for_funded(
                self.wallet.connector.as_ref(),
                params.clone(),
                utc_start_of_swap,
                deploy_event,
            )
            .map_err(backoff::Error::Transient)
        };

        let funded = operation
            .retry_notify(Constant::new(Duration::from_secs(1)), |e, _| {
                tracing::warn!("failed to watch for herc20 funding, retrying ...: {:#}", e)
            })
            .await
            .expect("transient error is never returned")?;

        let _ = self.db.save(funded.clone(), self.swap_id).await;

        Ok(funded)
    }
}

#[async_trait::async_trait]
impl WatchForRedeemed for Facade {
    async fn watch_for_redeemed(
        &self,
        params: Params,
        deploy_event: Deployed,
        utc_start_of_swap: OffsetDateTime,
    ) -> Redeemed {
        if let Ok(Some(Redeemed {
            transaction,
            secret,
        })) = self.db.load(self.swap_id)
        {
            self.wait_until_confirmed(transaction, params.chain_id)
                .await;

            return Redeemed {
                transaction,
                secret,
            };
        }

        let operation = || {
            watch_for_redeemed(
                self.wallet.connector.as_ref(),
                utc_start_of_swap,
                deploy_event,
            )
            .map_err(backoff::Error::Transient)
        };

        let redeemed = operation
            .retry_notify(Constant::new(Duration::from_secs(1)), |e, _| {
                tracing::warn!("failed to watch for herc20 redeem, retrying ...: {:#}", e)
            })
            .await
            .expect("transient error is never returned");

        let _ = self.db.save(redeemed, self.swap_id).await;

        redeemed
    }
}

#[cfg(all(test, feature = "testcontainers"))]
pub fn params(
    secret_hash: comit::SecretHash,
    chain_id: comit::ethereum::ChainId,
    redeem_identity: comit::identity::Ethereum,
    refund_identity: comit::identity::Ethereum,
    token_contract: comit::ethereum::Address,
    expiry: comit::Timestamp,
) -> Params {
    let quantity = comit::asset::ethereum::FromWei::from_wei(1_000_000_000u64);
    let asset = comit::asset::Erc20::new(token_contract, quantity);

    Params {
        asset,
        redeem_identity,
        refund_identity,
        expiry,
        chain_id,
        secret_hash,
    }
}
