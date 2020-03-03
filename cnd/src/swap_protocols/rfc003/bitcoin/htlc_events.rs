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
use tracing_futures::Instrument;

#[async_trait::async_trait]
impl<B> HtlcFunded<B, asset::Bitcoin, htlc_location::Bitcoin, identity::Bitcoin>
    for Cache<BitcoindConnector>
where
    B: bitcoin::Bitcoin + bitcoin::Network,
{
    async fn htlc_funded(
        &self,
        _htlc_params: &HtlcParams<B, asset::Bitcoin, identity::Bitcoin>,
        htlc_deployment: &Deployed<htlc_location::Bitcoin, transaction::Bitcoin>,
        _start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Funded<asset::Bitcoin, transaction::Bitcoin>> {
        let tx = &htlc_deployment.transaction;
        let asset =
            asset::Bitcoin::from_sat(tx.output[htlc_deployment.location.vout as usize].value);

        Ok(Funded {
            transaction: tx.clone(),
            asset,
        })
    }
}

#[async_trait::async_trait]
impl<B> HtlcDeployed<B, asset::Bitcoin, htlc_location::Bitcoin, identity::Bitcoin>
    for Cache<BitcoindConnector>
where
    B: bitcoin::Bitcoin + bitcoin::Network,
{
    async fn htlc_deployed(
        &self,
        htlc_params: &HtlcParams<B, asset::Bitcoin, identity::Bitcoin>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Deployed<htlc_location::Bitcoin, transaction::Bitcoin>> {
        let connector = self.clone();

        let (transaction, out_point) =
            watch_for_created_outpoint(connector, start_of_swap, htlc_params.compute_address())
                .instrument(tracing::info_span!("htlc_deployed"))
                .await?;

        Ok(Deployed {
            location: out_point,
            transaction,
        })
    }
}

#[async_trait::async_trait]
impl<B> HtlcRedeemed<B, asset::Bitcoin, htlc_location::Bitcoin, identity::Bitcoin>
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
        let connector = self.clone();

        let transaction =
            watch_for_spent_outpoint(connector, start_of_swap, htlc_deployment.location, vec![
                vec![1u8],
            ])
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
impl<B> HtlcRefunded<B, asset::Bitcoin, htlc_location::Bitcoin, identity::Bitcoin>
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
        let connector = self.clone();

        let transaction =
            watch_for_spent_outpoint(connector, start_of_swap, htlc_deployment.location, vec![
                vec![],
            ])
            .instrument(tracing::info_span!("htlc_refunded"))
            .await?;

        Ok(Refunded { transaction })
    }
}
