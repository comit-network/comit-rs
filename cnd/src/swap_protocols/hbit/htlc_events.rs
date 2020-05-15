use crate::{
    asset,
    btsieve::bitcoin::{
        watch_for_created_outpoint, watch_for_spent_outpoint, BitcoindConnector, Cache,
    },
    swap_protocols::hbit::{
        events::{
            Deployed, Funded, HtlcDeployed, HtlcFunded, HtlcRedeemed, HtlcRefunded, Redeemed,
            Refunded,
        },
        extract_secret, HtlcParams,
    },
};
use chrono::NaiveDateTime;
use std::cmp::Ordering;
use tracing_futures::Instrument;

#[async_trait::async_trait]
impl HtlcFunded for Cache<BitcoindConnector> {
    async fn htlc_funded(
        &self,
        htlc_params: &HtlcParams,
        htlc_deployment: &Deployed,
        _start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Funded> {
        let expected_asset = htlc_params.asset;

        let tx = &htlc_deployment.transaction;
        let asset =
            asset::Bitcoin::from_sat(tx.output[htlc_deployment.location.vout as usize].value);

        let event = match expected_asset.cmp(&asset) {
            Ordering::Equal => Funded::Correctly {
                transaction: tx.clone(),
                asset,
            },
            _ => Funded::Incorrectly {
                transaction: tx.clone(),
                asset,
            },
        };

        Ok(event)
    }
}

#[async_trait::async_trait]
impl HtlcDeployed for Cache<BitcoindConnector> {
    async fn htlc_deployed(
        &self,
        htlc_params: &HtlcParams,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Deployed> {
        let (transaction, location) =
            watch_for_created_outpoint(self, start_of_swap, htlc_params.compute_address())
                .instrument(tracing::info_span!("htlc_deployed"))
                .await?;

        Ok(Deployed {
            location,
            transaction,
        })
    }
}

#[async_trait::async_trait]
impl HtlcRedeemed for Cache<BitcoindConnector> {
    async fn htlc_redeemed(
        &self,
        htlc_params: &HtlcParams,
        htlc_deployment: &Deployed,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Redeemed> {
        let (transaction, _) = watch_for_spent_outpoint(
            self,
            start_of_swap,
            htlc_deployment.location,
            htlc_params.redeem_identity,
        )
        .instrument(tracing::info_span!("htlc_redeemed"))
        .await?;

        let secret = extract_secret::extract_secret(&transaction, &htlc_params.secret_hash)
            .expect("Redeem transaction must contain secret");

        Ok(Redeemed {
            transaction,
            secret,
        })
    }
}

#[async_trait::async_trait]
impl HtlcRefunded for Cache<BitcoindConnector> {
    async fn htlc_refunded(
        &self,
        htlc_params: &HtlcParams,
        htlc_deployment: &Deployed,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Refunded> {
        let (transaction, _) = watch_for_spent_outpoint(
            self,
            start_of_swap,
            htlc_deployment.location,
            htlc_params.refund_identity,
        )
        .instrument(tracing::info_span!("htlc_refunded"))
        .await?;

        Ok(Refunded { transaction })
    }
}
