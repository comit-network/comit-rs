use crate::{
    btsieve::{EthereumQuery, EventMatcher, QueryEthereum, Topic},
    swap_protocols::{
        asset::Asset,
        ledger::Ethereum,
        rfc003::{
            self,
            ethereum::extract_secret::extract_secret,
            events::{
                Deployed, DeployedFuture, Funded, FundedFuture, HtlcEvents, Redeemed,
                RedeemedOrRefundedFuture, Refunded,
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
    ) -> Box<DeployedFuture<Ethereum>> {
        let query_ethereum = Arc::clone(&self);
        let deployed_future = query_ethereum
            .create(EthereumQuery::contract_deployment(htlc_params.bytecode()))
            .and_then(move |query_id| query_ethereum.transaction_first_result(&query_id))
            .map_err(rfc003::Error::Btsieve)
            .map(|tx| Deployed {
                location: calcualte_contract_address_from_deployment_transaction(&tx),
                transaction: tx,
            });

        Box::new(deployed_future)
    }

    fn htlc_funded(
        &self,
        _htlc_params: HtlcParams<Ethereum, EtherQuantity>,
        deploy_transaction: &Deployed<Ethereum>,
    ) -> Box<FundedFuture<Ethereum, EtherQuantity>> {
        Box::new(future::ok(Funded {
            transaction: deploy_transaction.transaction.clone(),
            asset: EtherQuantity::from_wei(deploy_transaction.transaction.value),
        }))
    }

    fn htlc_redeemed_or_refunded(
        &self,
        htlc_params: HtlcParams<Ethereum, EtherQuantity>,
        htlc_deployment: &Deployed<Ethereum>,
        htlc_funding: &Funded<Ethereum, EtherQuantity>,
    ) -> Box<RedeemedOrRefundedFuture<Ethereum>> {
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
    htlc_deployment: &Deployed<Ethereum>,
    _: &Funded<Ethereum, A>,
) -> Box<RedeemedOrRefundedFuture<Ethereum>> {
    let refunded_future = {
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
            .map(Refunded::<Ethereum>::new)
    };

    let redeemed_future = {
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
                Ok(Redeemed {
                    transaction,
                    secret,
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

mod erc20 {
    use super::*;
    // keccak('Transfer(address,address,uint256)')
    const TRANSFER_LOG_MSG: &str =
        "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";

    impl HtlcEvents<Ethereum, Erc20Token> for Arc<dyn QueryEthereum + Send + Sync + 'static> {
        fn htlc_deployed(
            &self,
            htlc_params: HtlcParams<Ethereum, Erc20Token>,
        ) -> Box<DeployedFuture<Ethereum>> {
            let query_ethereum = Arc::clone(&self);
            let deployed_future = query_ethereum
                .create(EthereumQuery::contract_deployment(htlc_params.bytecode()))
                .and_then(move |query_id| query_ethereum.transaction_first_result(&query_id))
                .map_err(rfc003::Error::Btsieve)
                .map(|tx| Deployed {
                    location: calcualte_contract_address_from_deployment_transaction(&tx),
                    transaction: tx,
                });

            Box::new(deployed_future)
        }

        fn htlc_funded(
            &self,
            htlc_params: HtlcParams<Ethereum, Erc20Token>,
            deployment: &Deployed<Ethereum>,
        ) -> Box<FundedFuture<Ethereum, Erc20Token>> {
            let query_ethereum = Arc::clone(&self);
            let funded_future = self
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
                    Funded { transaction, asset }
                })
                .map_err(rfc003::Error::Btsieve);

            Box::new(funded_future)
        }

        fn htlc_redeemed_or_refunded(
            &self,
            htlc_params: HtlcParams<Ethereum, Erc20Token>,
            htlc_deployment: &Deployed<Ethereum>,
            htlc_funding: &Funded<Ethereum, Erc20Token>,
        ) -> Box<RedeemedOrRefundedFuture<Ethereum>> {
            htlc_redeemed_or_refunded(
                Arc::clone(&self),
                htlc_params,
                htlc_deployment,
                htlc_funding,
            )
        }
    }
}
