use crate::{
    btsieve::bitcoin::{
        matching_transaction, BitcoindConnector, TransactionExt, TransactionPattern,
    },
    swap_protocols::{
        ledger::Bitcoin,
        rfc003::{
            self,
            bitcoin::extract_secret::extract_secret,
            create_swap::HtlcParams,
            events::{Deployed, Funded, HtlcEvents, Redeemed, Refunded},
        },
    },
};
use bitcoin::{Amount, OutPoint};
use futures_core::future::{self, Either};

#[async_trait::async_trait]
impl HtlcEvents<Bitcoin, Amount> for BitcoindConnector {
    async fn htlc_deployed(
        &self,
        htlc_params: HtlcParams<Bitcoin, Amount>,
    ) -> Result<Deployed<Bitcoin>, rfc003::Error> {
        let connector = self.clone();
        let pattern = TransactionPattern {
            to_address: Some(htlc_params.compute_address()),
            from_outpoint: None,
            unlock_script: None,
        };

        let transaction = matching_transaction(connector, pattern, None)
            .await
            .map_err(|_| rfc003::Error::Btsieve)?;

        let (vout, _txout) = transaction
            .find_output(&htlc_params.compute_address())
            .ok_or_else(|| {
                rfc003::Error::Internal(
                    "Query returned Bitcoin transaction that didn't match the requested address"
                        .into(),
                )
            })?;

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
        _htlc_params: HtlcParams<Bitcoin, Amount>,
        htlc_deployment: &Deployed<Bitcoin>,
    ) -> Result<Funded<Bitcoin, Amount>, rfc003::Error> {
        let tx = &htlc_deployment.transaction;
        let asset = Amount::from_sat(tx.output[htlc_deployment.location.vout as usize].value);

        Ok(Funded {
            transaction: tx.clone(),
            asset,
        })
    }

    async fn htlc_redeemed_or_refunded(
        &self,
        htlc_params: HtlcParams<Bitcoin, Amount>,
        htlc_deployment: &Deployed<Bitcoin>,
        _htlc_funding: &Funded<Bitcoin, Amount>,
    ) -> Result<Either<Redeemed<Bitcoin>, Refunded<Bitcoin>>, rfc003::Error> {
        let redeemed = async {
            let connector = self.clone();
            let pattern = TransactionPattern {
                to_address: None,
                from_outpoint: Some(htlc_deployment.location),
                unlock_script: Some(vec![vec![1u8]]),
            };

            let transaction = matching_transaction(connector, pattern, None)
                .await
                .map_err(|_| rfc003::Error::Btsieve)?;
            let secret =
                extract_secret(&transaction, &htlc_params.secret_hash).ok_or_else(|| {
                    log::error!(
                        "Redeem transaction didn't have secret it in: {:?}",
                        transaction
                    );
                    rfc003::Error::Internal(
                        "Redeem transaction didn't have the secret in it".into(),
                    )
                })?;

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
            let transaction = matching_transaction(connector, pattern, None)
                .await
                .map_err(|_| rfc003::Error::Btsieve)?;

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
