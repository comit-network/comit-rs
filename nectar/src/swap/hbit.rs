pub use crate::swap::comit::hbit::*;

use crate::{
    database::{Load, Save},
    swap::{bitcoin::Wallet, Database},
    SwapId,
};
use comit::ledger;
use std::sync::Arc;
use time::OffsetDateTime;

pub struct Facade {
    pub wallet: Wallet,
    pub db: Arc<Database>,
    pub swap_id: SwapId,
}

impl Facade {
    async fn wait_until_confirmed(&self, tx: bitcoin::Txid, network: ledger::Bitcoin) {
        loop {
            match self.wallet.inner.wait_until_confirmed(tx, network).await {
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
impl WatchForFunded for Facade {
    async fn watch_for_funded(
        &self,
        params: &Params,
        start_of_swap: OffsetDateTime,
    ) -> Result<Funded, IncorrectlyFunded> {
        match self.db.load(self.swap_id) {
            Ok(Some(Funded { location, asset })) => {
                self.wait_until_confirmed(location.txid, params.shared.network)
                    .await;

                Ok(Funded { asset, location })
            }
            _ => loop {
                match comit::hbit::watch_for_funded(
                    self.wallet.connector.as_ref(),
                    &params.shared,
                    start_of_swap,
                )
                .await
                {
                    Ok(comit::hbit::Funded::Correctly {
                        asset, location, ..
                    }) => {
                        let event = Funded { location, asset };

                        let _ = self.db.save(event, self.swap_id).await;
                        return Ok(event);
                    }
                    Ok(comit::hbit::Funded::Incorrectly { asset, .. }) => {
                        return Err(IncorrectlyFunded {
                            expected: params.shared.asset,
                            got: asset,
                        })
                    }
                    Err(e) => {
                        tracing::warn!("failed to watch for hbit funding, retrying ...: {:#}", e)
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
        params: &Params,
        fund_event: Funded,
        start_of_swap: OffsetDateTime,
    ) -> Redeemed {
        match self.db.load(self.swap_id) {
            Ok(Some(Redeemed {
                transaction,
                secret,
            })) => {
                self.wait_until_confirmed(transaction, params.shared.network)
                    .await;

                Redeemed {
                    transaction,
                    secret,
                }
            }
            _ => loop {
                match watch_for_redeemed(
                    self.wallet.connector.as_ref(),
                    &params.shared,
                    fund_event.location,
                    start_of_swap,
                )
                .await
                {
                    Ok(event) => {
                        let _ = self.db.save(event, self.swap_id).await;
                        return event;
                    }
                    Err(e) => {
                        tracing::warn!("failed to watch for hbit redeem, retrying ...: {:#}", e)
                    }
                }
            },
        }
    }
}
