pub use comit::{
    hbit::*,
    swap::hbit::{Funded, IncorrectlyFunded, Params},
};

use crate::{
    database::{Load, Save},
    swap::{bitcoin::Wallet, Database},
    SwapId,
};
use backoff::{backoff::Constant, future::FutureOperation};
use comit::{
    ledger,
    swap::hbit::{WatchForFunded, WatchForRedeemed},
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
    async fn wait_until_confirmed(&self, tx: bitcoin::Txid, network: ledger::Bitcoin) {
        let operation = || {
            self.wallet
                .inner
                .wait_until_confirmed(tx, network)
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
impl WatchForFunded for Facade {
    async fn watch_for_funded(
        &self,
        params: &Params,
        start_of_swap: OffsetDateTime,
    ) -> Result<Funded, IncorrectlyFunded> {
        if let Ok(Some(Funded { location, asset })) = self.db.load(self.swap_id) {
            self.wait_until_confirmed(location.txid, params.shared.network)
                .await;

            return Ok(Funded { asset, location });
        }

        let operation = || {
            comit::hbit::watch_for_funded(
                self.wallet.connector.as_ref(),
                &params.shared,
                start_of_swap,
            )
            .map_err(backoff::Error::Transient)
        };

        let funded = operation
            .retry_notify(Constant::new(Duration::from_secs(1)), |e, _| {
                tracing::warn!("failed to watch for hbit funding, retrying ...: {:#}", e)
            })
            .await
            .expect("transient error is never returned")?;

        let _ = self.db.save(funded, self.swap_id).await;

        Ok(funded)
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
        if let Ok(Some(Redeemed {
            transaction,
            secret,
        })) = self.db.load(self.swap_id)
        {
            self.wait_until_confirmed(transaction, params.shared.network)
                .await;

            return Redeemed {
                transaction,
                secret,
            };
        }

        let operation = || {
            watch_for_redeemed(
                self.wallet.connector.as_ref(),
                &params.shared,
                fund_event.location,
                start_of_swap,
            )
            .map_err(backoff::Error::Transient)
        };

        let redeemed = operation
            .retry_notify(Constant::new(Duration::from_secs(1)), |e, _| {
                tracing::warn!("failed to watch for hbit redeem, retrying ...: {:#}", e)
            })
            .await
            .expect("transient error is never returned");

        let _ = self.db.save(redeemed, self.swap_id).await;

        redeemed
    }
}
