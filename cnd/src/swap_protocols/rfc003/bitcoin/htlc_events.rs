use crate::{
    asset,
    btsieve::bitcoin::{
        watch_for_created_outpoint, watch_for_spent_outpoint, BitcoindConnector, Cache,
    },
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
};
use chrono::NaiveDateTime;
use tracing_futures::Instrument;

#[async_trait::async_trait]
impl<Bitcoin: bitcoin::Bitcoin + bitcoin::Network> HtlcFunded<Bitcoin, asset::Bitcoin>
    for Cache<BitcoindConnector>
{
    async fn htlc_funded(
        &self,
        _htlc_params: HtlcParams<Bitcoin, asset::Bitcoin, crate::bitcoin::PublicKey>,
        htlc_deployment: &Deployed<::bitcoin::Transaction, ::bitcoin::OutPoint>,
        _start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Funded<::bitcoin::Transaction, asset::Bitcoin>> {
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
impl<Bitcoin: bitcoin::Bitcoin + bitcoin::Network> HtlcDeployed<Bitcoin, asset::Bitcoin>
    for Cache<BitcoindConnector>
{
    async fn htlc_deployed(
        &self,
        htlc_params: HtlcParams<Bitcoin, asset::Bitcoin, crate::bitcoin::PublicKey>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Deployed<::bitcoin::Transaction, ::bitcoin::OutPoint>> {
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
impl<Bitcoin: bitcoin::Bitcoin + bitcoin::Network> HtlcRedeemed<Bitcoin, asset::Bitcoin>
    for Cache<BitcoindConnector>
{
    async fn htlc_redeemed(
        &self,
        htlc_params: HtlcParams<Bitcoin, asset::Bitcoin, crate::bitcoin::PublicKey>,
        htlc_deployment: &Deployed<::bitcoin::Transaction, ::bitcoin::OutPoint>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Redeemed<::bitcoin::Transaction>> {
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
impl<Bitcoin: bitcoin::Bitcoin + bitcoin::Network> HtlcRefunded<Bitcoin, asset::Bitcoin>
    for Cache<BitcoindConnector>
{
    async fn htlc_refunded(
        &self,
        _htlc_params: HtlcParams<Bitcoin, asset::Bitcoin, crate::bitcoin::PublicKey>,
        htlc_deployment: &Deployed<::bitcoin::Transaction, ::bitcoin::OutPoint>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Refunded<::bitcoin::Transaction>> {
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
