use crate::{
    asset,
    btsieve::bitcoin::{
        matching_transaction, BitcoinCache, BitcoindConnector, TransactionExt, TransactionPattern,
    },
    swap_protocols::{
        ledger::Bitcoin,
        rfc003::{
            bitcoin::extract_secret::extract_secret,
            create_swap::HtlcParams,
            events::{Deployed, Funded, HtlcEvents, Redeemed, Refunded},
        },
    },
};
use anyhow::Context;
use bitcoin::OutPoint;
use chrono::NaiveDateTime;
use futures_core::future::{self, Either};

#[async_trait::async_trait]
impl HtlcEvents<Bitcoin, asset::Bitcoin> for BitcoinCache<BitcoindConnector> {
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

    async fn htlc_funded(
        &self,
        _htlc_params: HtlcParams<Bitcoin, asset::Bitcoin>,
        htlc_deployment: &Deployed<Bitcoin>,
        _start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Funded<Bitcoin, asset::Bitcoin>> {
        let tx = &htlc_deployment.transaction;
        let asset =
            asset::Bitcoin::from_sat(tx.output[htlc_deployment.location.vout as usize].value);

        Ok(Funded {
            transaction: tx.clone(),
            asset,
        })
    }

    async fn htlc_redeemed_or_refunded(
        &self,
        htlc_params: HtlcParams<Bitcoin, asset::Bitcoin>,
        htlc_deployment: &Deployed<Bitcoin>,
        _htlc_funding: &Funded<Bitcoin, asset::Bitcoin>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Either<Redeemed<Bitcoin>, Refunded<Bitcoin>>> {
        let redeemed = async {
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
        };

        let refunded = async {
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
        };

        futures_core::pin_mut!(redeemed);
        futures_core::pin_mut!(refunded);

        match future::try_select(redeemed, refunded).await {
            Ok(Either::Left((tx, _))) => Ok(Either::Left(tx)),
            Ok(Either::Right((tx, _))) => Ok(Either::Right(tx)),
            Err(either) => {
                let (error, _other_future) = either.factor_first();

                Err(error)
            }
        }
    }
}
