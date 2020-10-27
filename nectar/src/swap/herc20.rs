pub use comit::{
    herc20::{Deployed, Params, Redeemed, Refunded},
    swap::herc20::{Funded, IncorrectlyFunded},
};

use crate::{
    database::{Load, Save},
    swap::{ethereum::Wallet, Database},
    SwapId,
};
use comit::{
    ethereum,
    ethereum::ChainId,
    herc20::{watch_for_deployed, watch_for_funded, watch_for_redeemed},
    swap::herc20::{WatchForDeployed, WatchForFunded, WatchForRedeemed},
};
use std::sync::Arc;
use time::OffsetDateTime;

pub struct Facade {
    pub wallet: Wallet,
    pub db: Arc<Database>,
    pub swap_id: SwapId,
}

impl Facade {
    async fn wait_until_confirmed(&self, tx: ethereum::Hash, chain_id: ChainId) {
        loop {
            match self.wallet.inner.wait_until_confirmed(tx, chain_id).await {
                Ok(()) => return,
                Err(e) => tracing::warn!(
                    "failed to wait for {} getting confirmed, retrying ...: {:#}",
                    tx,
                    e
                ),
            }
        }
    }
}

#[async_trait::async_trait]
impl WatchForDeployed for Facade {
    async fn watch_for_deployed(
        &self,
        params: Params,
        utc_start_of_swap: OffsetDateTime,
    ) -> Deployed {
        match self.db.load(self.swap_id) {
            Ok(Some(Deployed {
                transaction,
                location,
            })) => {
                self.wait_until_confirmed(transaction, params.chain_id)
                    .await;

                Deployed {
                    transaction,
                    location,
                }
            }
            _ => loop {
                match watch_for_deployed(
                    self.wallet.connector.as_ref(),
                    params.clone(),
                    utc_start_of_swap,
                )
                .await
                {
                    Ok(event) => {
                        let _ = self.db.save(event, self.swap_id).await;

                        return event;
                    }
                    Err(e) => tracing::warn!(
                        "failed to watch for herc20 deployment, retrying ...: {:#}",
                        e
                    ),
                }
            },
        }
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
        match self.db.load(self.swap_id) {
            Ok(Some(Funded { transaction, asset })) => {
                self.wait_until_confirmed(transaction, params.chain_id)
                    .await;

                Ok(Funded { transaction, asset })
            }
            _ => loop {
                match watch_for_funded(
                    self.wallet.connector.as_ref(),
                    params.clone(),
                    utc_start_of_swap,
                    deploy_event,
                )
                .await
                {
                    Ok(comit::herc20::Funded::Correctly { transaction, asset }) => {
                        let event = Funded { transaction, asset };
                        let _ = self.db.save(event.clone(), self.swap_id).await;

                        return Ok(event);
                    }
                    Ok(comit::herc20::Funded::Incorrectly { asset, .. }) => {
                        return Err(IncorrectlyFunded {
                            expected: params.asset,
                            got: asset,
                        })
                    }
                    Err(e) => {
                        tracing::warn!("failed to watch for herc20 funding, retrying ...: {:#}", e)
                    }
                }
            },
        }
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
        match self.db.load(self.swap_id) {
            Ok(Some(Redeemed {
                transaction,
                secret,
            })) => {
                self.wait_until_confirmed(transaction, params.chain_id)
                    .await;

                Redeemed {
                    transaction,
                    secret,
                }
            }
            _ => loop {
                match watch_for_redeemed(
                    self.wallet.connector.as_ref(),
                    utc_start_of_swap,
                    deploy_event,
                )
                .await
                {
                    Ok(event) => {
                        let _ = self.db.save(event, self.swap_id).await;

                        return event;
                    }
                    Err(e) => {
                        tracing::warn!("failed to watch for herc20 funding, retrying ...: {:#}", e)
                    }
                }
            },
        }
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
