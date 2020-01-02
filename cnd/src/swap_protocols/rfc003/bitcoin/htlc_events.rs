use crate::{
    btsieve::bitcoin::{
        matching_transaction, BitcoindConnector, TransactionExt, TransactionPattern,
    },
    swap_protocols::{
        ledger::Bitcoin,
        rfc003::{
            self,
            bitcoin::extract_secret::extract_secret,
            events::{
                Deployed, DeployedFuture, Funded, FundedFuture, HtlcEvents, Redeemed,
                RedeemedOrRefundedFuture, Refunded,
            },
            state_machine::HtlcParams,
        },
    },
};
use bitcoin::{Amount, OutPoint};
use futures::{
    future::{self, Either},
    Future,
};
use futures_core::future::{FutureExt, TryFutureExt};

impl HtlcEvents<Bitcoin, Amount> for BitcoindConnector {
    fn htlc_deployed(
        &self,
        htlc_params: HtlcParams<Bitcoin, Amount>,
    ) -> Box<DeployedFuture<Bitcoin>> {
        let future = {
            let connector = self.clone();
            let pattern = TransactionPattern {
                to_address: Some(htlc_params.compute_address()),
                from_outpoint: None,
                unlock_script: None,
            };

            async {
                matching_transaction(
                    connector,
                    pattern,
                    None,
                )
                .await
            }
            .boxed()
            .compat()
            .map_err(|_| rfc003::Error::Btsieve)
            .and_then({
                move |tx| {
                    let (vout, _txout) = tx.find_output(&htlc_params.compute_address())
                        .ok_or_else(|| {
                            rfc003::Error::Internal(
                                "Query returned Bitcoin transaction that didn't match the requested address".into(),
                            )
                        })?;
                    Ok(Deployed {
                        location: OutPoint {
                            txid: tx.txid(),
                            vout,
                        },
                        transaction: tx,
                    })
                }
            })
        };

        Box::new(future)
    }

    fn htlc_funded(
        &self,
        _htlc_params: HtlcParams<Bitcoin, Amount>,
        htlc_deployment: &Deployed<Bitcoin>,
    ) -> Box<FundedFuture<Bitcoin, Amount>> {
        let tx = &htlc_deployment.transaction;
        let asset = Amount::from_sat(tx.output[htlc_deployment.location.vout as usize].value);
        Box::new(future::ok(Funded {
            transaction: tx.clone(),
            asset,
        }))
    }

    fn htlc_redeemed_or_refunded(
        &self,
        htlc_params: HtlcParams<Bitcoin, Amount>,
        htlc_deployment: &Deployed<Bitcoin>,
        _htlc_funding: &Funded<Bitcoin, Amount>,
    ) -> Box<RedeemedOrRefundedFuture<Bitcoin>> {
        let refunded_future = {
            let connector = self.clone();
            let pattern = TransactionPattern {
                to_address: None,
                from_outpoint: Some(htlc_deployment.location),
                unlock_script: Some(vec![vec![]]),
            };

            async { matching_transaction(connector, pattern, None).await }
                .boxed()
                .compat()
                .map_err(|_| rfc003::Error::Btsieve)
                .and_then(|transaction| Ok(Refunded { transaction }))
        };

        let redeemed_future = {
            let connector = self.clone();
            let pattern = TransactionPattern {
                to_address: None,
                from_outpoint: Some(htlc_deployment.location),
                unlock_script: Some(vec![vec![1u8]]),
            };

            async { matching_transaction(connector, pattern, None).await }
                .boxed()
                .compat()
                .map_err(|_| rfc003::Error::Btsieve)
                .and_then({
                    move |tx| {
                        let secret =
                            extract_secret(&tx, &htlc_params.secret_hash).ok_or_else(|| {
                                log::error!(
                                    "Redeem transaction didn't have secret it in: {:?}",
                                    tx
                                );
                                rfc003::Error::Internal(
                                    "Redeem transaction didn't have the secret in it".into(),
                                )
                            })?;

                        Ok(Redeemed {
                            transaction: tx,
                            secret,
                        })
                    }
                })
        };

        Box::new(
            redeemed_future
                .select2(refunded_future)
                .map(|tx| match tx {
                    Either::A((tx, _)) => Either::A(tx),
                    Either::B((tx, _)) => Either::B(tx),
                })
                .map_err(|either| match either {
                    Either::A((error, _)) => error,
                    Either::B((error, _)) => error,
                }),
        )
    }
}
