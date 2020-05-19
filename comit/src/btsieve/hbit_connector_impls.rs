use crate::{
    asset,
    btsieve::bitcoin::{
        watch_for_created_outpoint, watch_for_spent_outpoint, BitcoindConnector, Cache,
    },
    hbit::{
        self, Funded, Params, Redeemed, Refunded, WaitForFunded, WaitForRedeemed, WaitForRefunded,
    },
    htlc_location,
};
use chrono::NaiveDateTime;
use std::cmp::Ordering;
use tracing_futures::Instrument;

#[async_trait::async_trait]
impl WaitForFunded for Cache<BitcoindConnector> {
    async fn wait_for_funded(
        &self,
        params: &Params,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Funded> {
        let expected_asset = params.asset;

        let (transaction, location) =
            watch_for_created_outpoint(self, start_of_swap, params.compute_address())
                .instrument(tracing::info_span!("wait_for_funded"))
                .await?;

        let asset = asset::Bitcoin::from_sat(transaction.output[location.vout as usize].value);

        let event = match expected_asset.cmp(&asset) {
            Ordering::Equal => Funded::Correctly {
                asset,
                transaction,
                location,
            },
            _ => Funded::Incorrectly {
                asset,
                transaction,
                location,
            },
        };

        Ok(event)
    }
}

#[async_trait::async_trait]
impl WaitForRedeemed for Cache<BitcoindConnector> {
    async fn wait_for_redeemed(
        &self,
        params: &Params,
        location: htlc_location::Bitcoin,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Redeemed> {
        let (transaction, _) =
            watch_for_spent_outpoint(self, start_of_swap, location, params.redeem_identity)
                .instrument(tracing::info_span!("wait_for_redeemed"))
                .await?;

        let secret = hbit::extract_secret(&transaction, &params.secret_hash)
            .expect("Redeem transaction must contain secret");

        Ok(Redeemed {
            transaction,
            secret,
        })
    }
}

#[async_trait::async_trait]
impl WaitForRefunded for Cache<BitcoindConnector> {
    async fn wait_for_refunded(
        &self,
        params: &Params,
        location: htlc_location::Bitcoin,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Refunded> {
        let (transaction, _) =
            watch_for_spent_outpoint(self, start_of_swap, location, params.refund_identity)
                .instrument(tracing::info_span!("wait_for_refunded"))
                .await?;

        Ok(Refunded { transaction })
    }
}
