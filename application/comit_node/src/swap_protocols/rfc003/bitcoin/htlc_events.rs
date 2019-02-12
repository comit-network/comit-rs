use crate::{
    ledger_query_service::{BitcoinQuery, QueryBitcoin},
    swap_protocols::{
        ledger::Bitcoin,
        rfc003::{
            self,
            bitcoin::{extract_secret::extract_secret, find_htlc_vout::find_htlc_vout},
            events::{Deployed, Funded, HtlcEvents, RedeemedOrRefunded},
            ledger::{RedeemTransaction, RefundTransaction},
            state_machine::HtlcParams,
        },
    },
};
use bitcoin_support::{BitcoinQuantity, OutPoint};
use futures::{
    future::{self, Either},
    Future,
};

use std::sync::Arc;
impl HtlcEvents<Bitcoin, BitcoinQuantity> for Arc<dyn QueryBitcoin + Send + Sync> {
    fn htlc_deployed(
        &self,
        htlc_params: HtlcParams<Bitcoin, BitcoinQuantity>,
    ) -> Box<Deployed<Bitcoin>> {
        let query_bitcoin = Arc::clone(&self);
        let query = self
            .create(BitcoinQuery::Transaction {
                to_address: Some(htlc_params.compute_address()),
                from_outpoint: None,
                unlock_script: None,
            })
            .map_err(rfc003::Error::LedgerQueryService);

        let transaction = query.and_then(move |query_id| {
            query_bitcoin
                .transaction_first_result(&query_id)
                .map_err(rfc003::Error::LedgerQueryService)
        });

        let htlc_location = transaction.and_then(move |tx| find_htlc_vout(&tx, &htlc_params));

        Box::new(htlc_location)
    }

    fn htlc_funded(
        &self,
        _htlc_params: HtlcParams<Bitcoin, BitcoinQuantity>,
        _htlc_location: &OutPoint,
    ) -> Box<Funded<Bitcoin>> {
        // It's already funded when it's deployed
        Box::new(future::ok(None))
    }

    fn htlc_redeemed_or_refunded(
        &self,
        htlc_params: HtlcParams<Bitcoin, BitcoinQuantity>,
        htlc_location: &OutPoint,
    ) -> Box<RedeemedOrRefunded<Bitcoin>> {
        let refunded_tx = {
            let query_bitcoin = Arc::clone(&self);

            let refunded_query = self
                .create(BitcoinQuery::Transaction {
                    to_address: None,
                    from_outpoint: Some(*htlc_location),
                    unlock_script: Some(vec![vec![0u8]]),
                })
                .map_err(rfc003::Error::LedgerQueryService);

            refunded_query
                .and_then(move |query_id| {
                    query_bitcoin
                        .transaction_first_result(&query_id)
                        .map_err(rfc003::Error::LedgerQueryService)
                })
                .map(RefundTransaction::<Bitcoin>)
        };

        let redeemed_tx = {
            let query_bitcoin = Arc::clone(&self);
            let redeemed_query = self
                .create(BitcoinQuery::Transaction {
                    to_address: None,
                    from_outpoint: Some(*htlc_location),
                    unlock_script: Some(vec![vec![1u8]]),
                })
                .map_err(rfc003::Error::LedgerQueryService);

            redeemed_query.and_then(move |query_id| {
                query_bitcoin
                    .transaction_first_result(&query_id)
                    .map_err(rfc003::Error::LedgerQueryService)
                    .and_then(move |transaction| {
                        let secret = extract_secret(&transaction, &htlc_params.secret_hash)
                            .ok_or_else(|| {
                                error!(
                                    "Redeem transaction didn't have secret it in: {:?}",
                                    transaction
                                );
                                rfc003::Error::Internal(
                                    "Redeem transaction didn't have the secret in it".into(),
                                )
                            })?;
                        Ok(RedeemTransaction {
                            transaction,
                            secret,
                        })
                    })
            })
        };

        Box::new(
            redeemed_tx
                .select2(refunded_tx)
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
