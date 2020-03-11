use crate::{
    asset,
    btsieve::bitcoin::{
        watch_for_created_outpoint, watch_for_spent_outpoint, BitcoindConnector, Cache,
    },
    htlc_location, identity,
    swap_protocols::{
        ledger::bitcoin,
        rfc003::{
            bitcoin::extract_secret::extract_secret,
            create_swap::HtlcParams,
            events::{
                Deployed, Funded, HtlcDeployed, HtlcFunded, HtlcRedeemed, HtlcRefunded, Redeemed,
                Refunded,
            },
        },
    },
    transaction,
};
use chrono::NaiveDateTime;
use std::cmp::Ordering;
use tracing_futures::Instrument;

#[async_trait::async_trait]
impl<B>
    HtlcFunded<B, asset::Bitcoin, htlc_location::Bitcoin, identity::Bitcoin, transaction::Bitcoin>
    for Cache<BitcoindConnector>
where
    B: bitcoin::Bitcoin + bitcoin::Network,
{
    async fn htlc_funded(
        &self,
        htlc_params: &HtlcParams<B, asset::Bitcoin, identity::Bitcoin>,
        htlc_deployment: &Deployed<htlc_location::Bitcoin, transaction::Bitcoin>,
        _start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Funded<asset::Bitcoin, transaction::Bitcoin>> {
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
impl<B>
    HtlcDeployed<B, asset::Bitcoin, htlc_location::Bitcoin, identity::Bitcoin, transaction::Bitcoin>
    for Cache<BitcoindConnector>
where
    B: bitcoin::Bitcoin + bitcoin::Network,
{
    async fn htlc_deployed(
        &self,
        htlc_params: &HtlcParams<B, asset::Bitcoin, identity::Bitcoin>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Deployed<htlc_location::Bitcoin, transaction::Bitcoin>> {
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
impl<B>
    HtlcRedeemed<B, asset::Bitcoin, htlc_location::Bitcoin, identity::Bitcoin, transaction::Bitcoin>
    for Cache<BitcoindConnector>
where
    B: bitcoin::Bitcoin + bitcoin::Network,
{
    async fn htlc_redeemed(
        &self,
        htlc_params: &HtlcParams<B, asset::Bitcoin, identity::Bitcoin>,
        htlc_deployment: &Deployed<htlc_location::Bitcoin, transaction::Bitcoin>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Redeemed<transaction::Bitcoin>> {
        let (transaction, _) =
            watch_for_spent_outpoint(self, start_of_swap, htlc_deployment.location, vec![vec![
                1u8,
            ]])
            .instrument(tracing::info_span!("htlc_redeemed"))
            .await?;

        let secret = extract_secret(&transaction, &htlc_params.secret_hash)
            .expect("Redeem transaction must contain secret");

        Ok(Redeemed {
            transaction,
            secret,
        })
    }
}

#[async_trait::async_trait]
impl<B>
    HtlcRefunded<B, asset::Bitcoin, htlc_location::Bitcoin, identity::Bitcoin, transaction::Bitcoin>
    for Cache<BitcoindConnector>
where
    B: bitcoin::Bitcoin + bitcoin::Network,
{
    async fn htlc_refunded(
        &self,
        _htlc_params: &HtlcParams<B, asset::Bitcoin, identity::Bitcoin>,
        htlc_deployment: &Deployed<htlc_location::Bitcoin, transaction::Bitcoin>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Refunded<transaction::Bitcoin>> {
        let (transaction, _) =
            watch_for_spent_outpoint(self, start_of_swap, htlc_deployment.location, vec![vec![]])
                .instrument(tracing::info_span!("htlc_refunded"))
                .await?;

        Ok(Refunded { transaction })
    }
}
