use crate::{
    btsieve::{EthereumQuery, EventMatcher, QueryEthereum, Topic},
    swap_protocols::{
        asset::Asset,
        ledger::Ethereum,
        rfc003::{
            self,
            ethereum::extract_secret::extract_secret,
            events::{
                DeployTransaction, Deployed, FundTransaction, Funded, HtlcEvents,
                RedeemTransaction, RedeemedOrRefunded, RefundTransaction,
            },
            state_machine::HtlcParams,
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

// keccak256(Redeemed())
pub const REDEEM_LOG_MSG: &str =
    "0xB8CAC300E37F03AD332E581DEA21B2F0B84EAAADC184A295FEF71E81F44A7413";
// keccak256(Refunded())
pub const REFUND_LOG_MSG: &str =
    "0x5D26862916391BF49478B2F5103B0720A842B45EF145A268F2CD1FB2AED55178";

impl HtlcEvents<Ethereum, EtherQuantity> for Arc<dyn QueryEthereum + Send + Sync + 'static> {
    fn htlc_deployed(
        &self,
        htlc_params: HtlcParams<Ethereum, EtherQuantity>,
    ) -> Box<Deployed<Ethereum>> {
        let query_ethereum = Arc::clone(&self);
        let htlc_location = query_ethereum
            .create(EthereumQuery::contract_deployment(htlc_params.bytecode()))
            .and_then(move |query_id| query_ethereum.transaction_first_result(&query_id))
            .map_err(rfc003::Error::Btsieve)
            .map(|tx| DeployTransaction {
                location: calcualte_contract_address_from_deployment_transaction(&tx),
                transaction: tx,
            });

        Box::new(htlc_location)
    }

    fn htlc_funded(
        &self,
        _htlc_params: HtlcParams<Ethereum, EtherQuantity>,
        deploy_transaction: &DeployTransaction<Ethereum>,
    ) -> Box<Funded<Ethereum, EtherQuantity>> {
        Box::new(future::ok(FundTransaction {
            transaction: deploy_transaction.transaction.clone(),
            asset: EtherQuantity::from_wei(deploy_transaction.transaction.value),
        }))
    }

    fn htlc_redeemed_or_refunded(
        &self,
        htlc_params: HtlcParams<Ethereum, EtherQuantity>,
        htlc_deployment: &DeployTransaction<Ethereum>,
        htlc_funding: &FundTransaction<Ethereum, EtherQuantity>,
    ) -> Box<RedeemedOrRefunded<Ethereum>> {
        htlc_redeemed_or_refunded(
            Arc::clone(&self),
            htlc_params,
            htlc_deployment,
            htlc_funding,
        )
    }
}

fn calcualte_contract_address_from_deployment_transaction(tx: &Transaction) -> Address {
    tx.from.calculate_contract_address(&tx.nonce)
}

fn htlc_redeemed_or_refunded<A: Asset>(
    query_ethereum: Arc<dyn QueryEthereum + Send + Sync + 'static>,
    htlc_params: HtlcParams<Ethereum, A>,
    htlc_deployment: &DeployTransaction<Ethereum>,
    _: &FundTransaction<Ethereum, A>,
) -> Box<RedeemedOrRefunded<Ethereum>> {
    let refunded_tx = {
        let query_ethereum = Arc::clone(&query_ethereum);
        query_ethereum
            .create(EthereumQuery::Event {
                event_matchers: vec![EventMatcher {
                    address: Some(htlc_deployment.location),
                    data: None,
                    topics: vec![Some(Topic(REFUND_LOG_MSG.into()))],
                }],
            })
            .and_then(move |query_id| query_ethereum.transaction_first_result(&query_id))
            .map_err(rfc003::Error::Btsieve)
            .map(RefundTransaction::<Ethereum>::new)
    };

    let redeemed_tx = {
        let query_ethereum = Arc::clone(&query_ethereum);
        query_ethereum
            .create(EthereumQuery::Event {
                event_matchers: vec![EventMatcher {
                    address: Some(htlc_deployment.location),
                    data: None,
                    topics: vec![Some(Topic(REDEEM_LOG_MSG.into()))],
                }],
            })
            .and_then(move |query_id| query_ethereum.transaction_first_result(&query_id))
            .map_err(rfc003::Error::Btsieve)
            .and_then(move |transaction| {
                let secret =
                    extract_secret(&transaction, &htlc_params.secret_hash).ok_or_else(|| {
                        rfc003::Error::Internal(
                            "Redeem transaction didn't have the secret in it".into(),
                        )
                    })?;
                Ok(RedeemTransaction {
                    transaction,
                    secret,
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

mod erc20 {
    use super::*;
    // keccak('Transfer(address,address,uint256)')
    const TRANSFER_LOG_MSG: &str =
        "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";

    impl HtlcEvents<Ethereum, Erc20Token> for Arc<dyn QueryEthereum + Send + Sync + 'static> {
        fn htlc_deployed(
            &self,
            htlc_params: HtlcParams<Ethereum, Erc20Token>,
        ) -> Box<Deployed<Ethereum>> {
            let query_ethereum = Arc::clone(&self);
            let htlc_location = query_ethereum
                .create(EthereumQuery::contract_deployment(htlc_params.bytecode()))
                .and_then(move |query_id| query_ethereum.transaction_first_result(&query_id))
                .map_err(rfc003::Error::Btsieve)
                .map(|tx| DeployTransaction {
                    location: calcualte_contract_address_from_deployment_transaction(&tx),
                    transaction: tx,
                });

            Box::new(htlc_location)
        }

        fn htlc_funded(
            &self,
            htlc_params: HtlcParams<Ethereum, Erc20Token>,
            deployment: &DeployTransaction<Ethereum>,
        ) -> Box<Funded<Ethereum, Erc20Token>> {
            let query_ethereum = Arc::clone(&self);
            let funding_transaction = self
                .create(EthereumQuery::Event {
                    event_matchers: vec![EventMatcher {
                        address: Some(htlc_params.asset.token_contract()),
                        data: None,
                        topics: vec![
                            Some(Topic(TRANSFER_LOG_MSG.into())),
                            None,
                            Some(Topic(deployment.location.into())),
                        ],
                    }],
                })
                .and_then(move |query_id| query_ethereum.transaction_first_result(&query_id))
                .map(move |transaction| {
                    // TODO: Get the actual asset out of response from btsieve
                    let asset = htlc_params.asset;
                    FundTransaction { transaction, asset }
                })
                .map_err(rfc003::Error::Btsieve);

            Box::new(funding_transaction)
        }

        fn htlc_redeemed_or_refunded(
            &self,
            htlc_params: HtlcParams<Ethereum, Erc20Token>,
            htlc_deployment: &DeployTransaction<Ethereum>,
            htlc_funding: &FundTransaction<Ethereum, Erc20Token>,
        ) -> Box<RedeemedOrRefunded<Ethereum>> {
            htlc_redeemed_or_refunded(
                Arc::clone(&self),
                htlc_params,
                htlc_deployment,
                htlc_funding,
            )
        }
    }
}
