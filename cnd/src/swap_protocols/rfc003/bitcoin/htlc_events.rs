use crate::{
    btsieve::{BitcoinQuery, QueryBitcoin},
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
use bitcoin_support::{Amount, FindOutput, OutPoint};
use futures::{
    future::{self, Either},
    Future,
};
use std::sync::Arc;

impl HtlcEvents<Bitcoin, Amount> for Arc<dyn QueryBitcoin + Send + Sync> {
    fn htlc_deployed(
        &self,
        htlc_params: HtlcParams<Bitcoin, Amount>,
    ) -> Box<DeployedFuture<Bitcoin>> {
        let query_bitcoin = Arc::clone(&self);
        let deployed_future = self
            .create(BitcoinQuery::deploy_htlc(htlc_params.compute_address()))
            .and_then(move |query_id| query_bitcoin.transaction_first_result(&query_id))
            .map_err(rfc003::Error::Btsieve)
            .and_then(move |tx| {
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
            });

        Box::new(deployed_future)
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
        _: &Funded<Bitcoin, Amount>,
    ) -> Box<RedeemedOrRefundedFuture<Bitcoin>> {
        let refunded_future = {
            let query_bitcoin = Arc::clone(&self);

            let refunded_query = self
                .create(BitcoinQuery::refund_htlc(htlc_deployment.location))
                .map_err(rfc003::Error::Btsieve);

            refunded_query
                .and_then(move |query_id| {
                    query_bitcoin
                        .transaction_first_result(&query_id)
                        .map_err(rfc003::Error::Btsieve)
                })
                .map(Refunded::<Bitcoin>::new)
        };

        let redeemed_future = {
            let query_bitcoin = Arc::clone(&self);
            let redeemed_query = self
                .create(BitcoinQuery::redeem_htlc(htlc_deployment.location))
                .map_err(rfc003::Error::Btsieve);

            redeemed_query.and_then(move |query_id| {
                query_bitcoin
                    .transaction_first_result(&query_id)
                    .map_err(rfc003::Error::Btsieve)
                    .and_then(move |transaction| {
                        let secret = extract_secret(&transaction, &htlc_params.secret_hash)
                            .ok_or_else(|| {
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
                    })
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
