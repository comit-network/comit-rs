use crate::{
    asset,
    btsieve::bitcoin::{
        matching_transaction, BitcoindConnector, Cache, TransactionExt, TransactionPattern,
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
use ::bitcoin::OutPoint;
use anyhow::Context;
use chrono::NaiveDateTime;

#[async_trait::async_trait]
impl<Bitcoin: bitcoin::Bitcoin + bitcoin::Network>
    HtlcFunded<Bitcoin, asset::Bitcoin, ::bitcoin::Transaction> for Cache<BitcoindConnector>
{
    async fn htlc_funded(
        &self,
        _htlc_params: HtlcParams<Bitcoin, asset::Bitcoin>,
        htlc_deployment: &Deployed<Bitcoin>,
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
        htlc_params: HtlcParams<Bitcoin, asset::Bitcoin>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Deployed<Bitcoin>> {
        let connector = self.clone();
        let pattern = TransactionPattern {
            to_address: Some(htlc_params.compute_address()),
            from_outpoint: None,
            unlock_script: None,
        };

        let transaction = matching_transaction(connector, pattern, start_of_swap)
            .await
            .context("failed to find transaction to deploy htlc")?;

        let (vout, _txout) = transaction
            .find_output(&htlc_params.compute_address())
            .expect("Deployment transaction must contain outpoint described in pattern");

        Ok(Deployed {
            location: OutPoint {
                txid: transaction.txid(),
                vout,
            },
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
        htlc_params: HtlcParams<Bitcoin, asset::Bitcoin>,
        htlc_deployment: &Deployed<Bitcoin>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Redeemed<Bitcoin>> {
        let connector = self.clone();
        let pattern = TransactionPattern {
            to_address: None,
            from_outpoint: Some(htlc_deployment.location),
            unlock_script: Some(vec![vec![1u8]]),
        };

        let transaction = matching_transaction(connector, pattern, start_of_swap)
            .await
            .context("failed to find transaction to redeem from htlc")?;
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
        _htlc_params: HtlcParams<Bitcoin, asset::Bitcoin>,
        htlc_deployment: &Deployed<Bitcoin>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Refunded<Bitcoin>> {
        let connector = self.clone();
        let pattern = TransactionPattern {
            to_address: None,
            from_outpoint: Some(htlc_deployment.location),
            unlock_script: Some(vec![vec![]]),
        };
        let transaction = matching_transaction(connector, pattern, start_of_swap)
            .await
            .context("failed to find transaction to refund from htlc")?;

        Ok(Refunded { transaction })
    }
}
