use crate::{
    ledger_query_service::{EthereumQuery, EventMatcher, QueryEthereum, Topic},
    swap_protocols::{
        asset::Asset,
        ledger::Ethereum,
        rfc003::{
            self,
            ethereum::{extract_secret::extract_secret, REDEEM_LOG_MSG, REFUND_LOG_MSG},
            events::{Deployed, Funded, HtlcEvents, RedeemedOrRefunded},
            state_machine::HtlcParams,
            FundTransaction, RedeemTransaction, RefundTransaction,
        },
    },
};
use ethereum_support::{
    web3::types::Address, CalculateContractAddress, Erc20Token, EtherQuantity, Transaction,
};
use futures::{
    future::{self, Either},
    Future,
};
use std::sync::Arc;

impl HtlcEvents<Ethereum, EtherQuantity> for Arc<dyn QueryEthereum + Send + Sync + 'static> {
    fn htlc_deployed(
        &self,
        htlc_params: HtlcParams<Ethereum, EtherQuantity>,
    ) -> Box<Deployed<Ethereum>> {
        let query_ethereum = Arc::clone(&self);
        let query = query_ethereum
            .create(EthereumQuery::Transaction {
                from_address: None,
                to_address: None,
                is_contract_creation: Some(true),
                transaction_data: Some(htlc_params.bytecode()),
                transaction_data_length: None,
            })
            .map_err(rfc003::Error::LedgerQueryService);

        let transaction = query.and_then(move |query_id| {
            query_ethereum
                .transaction_first_result(&query_id)
                .map_err(rfc003::Error::LedgerQueryService)
        });

        let htlc_location = transaction.and_then(move |tx| {
            let actual_value = EtherQuantity::from_wei(tx.value);
            let required_value = htlc_params.asset;

            if actual_value < required_value {
                Err(rfc003::Error::InsufficientFunding)
            } else {
                Ok(calcualte_contract_address_from_deployment_transaction(tx))
            }
        });

        Box::new(htlc_location)
    }

    fn htlc_funded(
        &self,
        _htlc_params: HtlcParams<Ethereum, EtherQuantity>,
        _htlc_location: &Address,
    ) -> Box<Funded<Ethereum>> {
        // It's already funded when it's deployed
        Box::new(future::ok(None))
    }

    fn htlc_redeemed_or_refunded(
        &self,
        htlc_params: HtlcParams<Ethereum, EtherQuantity>,
        htlc_location: &Address,
    ) -> Box<RedeemedOrRefunded<Ethereum>> {
        htlc_redeemed_or_refunded(Arc::clone(&self), htlc_params, htlc_location)
    }
}

use crate::swap_protocols::rfc003::ethereum::TRANSFER_LOG_MSG;
impl HtlcEvents<Ethereum, Erc20Token> for Arc<dyn QueryEthereum + Send + Sync + 'static> {
    fn htlc_deployed(
        &self,
        htlc_params: HtlcParams<Ethereum, Erc20Token>,
    ) -> Box<Deployed<Ethereum>> {
        let query_ethereum = Arc::clone(&self);
        let query = query_ethereum
            .create(EthereumQuery::Transaction {
                from_address: None,
                to_address: None,
                is_contract_creation: Some(true),
                transaction_data: Some(htlc_params.bytecode()),
                transaction_data_length: None,
            })
            .map_err(rfc003::Error::LedgerQueryService);

        let transaction = query.and_then(move |query_id| {
            query_ethereum
                .transaction_first_result(&query_id)
                .map_err(rfc003::Error::LedgerQueryService)
        });

        let htlc_location = transaction.map(calcualte_contract_address_from_deployment_transaction);

        Box::new(htlc_location)
    }

    fn htlc_funded(
        &self,
        htlc_params: HtlcParams<Ethereum, Erc20Token>,
        htlc_location: &Address,
    ) -> Box<Funded<Ethereum>> {
        let query_ethereum = Arc::clone(&self);
        // TODO: Validate the amount that was transferred
        let query = self
            .create(EthereumQuery::Event {
                event_matchers: vec![EventMatcher {
                    address: Some(htlc_params.asset.token_contract()),
                    data: None,
                    topics: vec![
                        Some(Topic(TRANSFER_LOG_MSG.into())),
                        None,
                        Some(Topic(htlc_location.into())),
                    ],
                }],
            })
            .map_err(rfc003::Error::LedgerQueryService);

        let funding_transaction = query.and_then(move |query_id| {
            query_ethereum
                .transaction_first_result(&query_id)
                .map_err(rfc003::Error::LedgerQueryService)
                .map(|tx| Some(FundTransaction(tx)))
        });

        Box::new(funding_transaction)
    }

    fn htlc_redeemed_or_refunded(
        &self,
        htlc_params: HtlcParams<Ethereum, Erc20Token>,
        htlc_location: &Address,
    ) -> Box<RedeemedOrRefunded<Ethereum>> {
        htlc_redeemed_or_refunded(Arc::clone(&self), htlc_params, htlc_location)
    }
}

fn calcualte_contract_address_from_deployment_transaction(tx: Transaction) -> Address {
    tx.from.calculate_contract_address(&tx.nonce)
}

fn htlc_redeemed_or_refunded<A: Asset>(
    query_ethereum: Arc<dyn QueryEthereum + Send + Sync + 'static>,
    htlc_params: HtlcParams<Ethereum, A>,
    htlc_location: &Address,
) -> Box<RedeemedOrRefunded<Ethereum>> {
    let refunded_tx = {
        let query_ethereum = Arc::clone(&query_ethereum);
        let refunded_query = query_ethereum
            .create(EthereumQuery::Event {
                event_matchers: vec![EventMatcher {
                    address: Some(*htlc_location),
                    data: None,
                    topics: vec![Some(Topic(REFUND_LOG_MSG.into()))],
                }],
            })
            .map_err(rfc003::Error::LedgerQueryService);

        refunded_query
            .and_then(move |query_id| {
                query_ethereum
                    .transaction_first_result(&query_id)
                    .map_err(rfc003::Error::LedgerQueryService)
            })
            .map(RefundTransaction::<Ethereum>)
    };

    let redeemed_tx = {
        let query_ethereum = Arc::clone(&query_ethereum);
        let redeemed_query = query_ethereum
            .create(EthereumQuery::Event {
                event_matchers: vec![EventMatcher {
                    address: Some(*htlc_location),
                    data: None,
                    topics: vec![Some(Topic(REDEEM_LOG_MSG.into()))],
                }],
            })
            .map_err(rfc003::Error::LedgerQueryService);

        redeemed_query.and_then(move |query_id| {
            query_ethereum
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
