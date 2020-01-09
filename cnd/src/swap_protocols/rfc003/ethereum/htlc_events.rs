use crate::{
    btsieve::ethereum::{matching_transaction, Event, Topic, TransactionPattern, Web3Connector},
    ethereum::{
        Address, CalculateContractAddress, Erc20Token, EtherQuantity, Transaction,
        TransactionAndReceipt, H256,
    },
    swap_protocols::{
        asset::Asset,
        ledger::Ethereum,
        rfc003::{
            self,
            create_swap::HtlcParams,
            events::{Deployed, Funded, HtlcEvents, Redeemed, Refunded},
            Secret,
        },
    },
};
use futures_core::future::{self, Either};

lazy_static::lazy_static! {
    /// keccak256(Redeemed())
    pub static ref REDEEM_LOG_MSG: H256 = "B8CAC300E37F03AD332E581DEA21B2F0B84EAAADC184A295FEF71E81F44A7413".parse().expect("to be valid hex");
    /// keccak256(Refunded())
    pub static ref REFUND_LOG_MSG: H256 = "5D26862916391BF49478B2F5103B0720A842B45EF145A268F2CD1FB2AED55178".parse().expect("to be valid hex");
    /// keccak('Transfer(address,address,uint256)')
    pub static ref TRANSFER_LOG_MSG: H256 = "ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef".parse().expect("to be valid hex");
}

#[async_trait::async_trait]
impl HtlcEvents<Ethereum, EtherQuantity> for Web3Connector {
    async fn htlc_deployed(
        &self,
        htlc_params: HtlcParams<Ethereum, EtherQuantity>,
    ) -> Result<Deployed<Ethereum>, rfc003::Error> {
        let connector = self.clone();
        let pattern = TransactionPattern {
            from_address: None,
            to_address: None,
            is_contract_creation: Some(true),
            transaction_data: Some(htlc_params.bytecode()),
            transaction_data_length: None,
            events: None,
        };
        let TransactionAndReceipt { transaction, .. } =
            matching_transaction(connector, pattern, None).await;

        Ok(Deployed {
            location: calculate_contract_address_from_deployment_transaction(&transaction),
            transaction,
        })
    }

    async fn htlc_funded(
        &self,
        _htlc_params: HtlcParams<Ethereum, EtherQuantity>,
        deploy_transaction: &Deployed<Ethereum>,
    ) -> Result<Funded<Ethereum, EtherQuantity>, rfc003::Error> {
        Ok(Funded {
            transaction: deploy_transaction.transaction.clone(),
            asset: EtherQuantity::from_wei(deploy_transaction.transaction.value),
        })
    }

    async fn htlc_redeemed_or_refunded(
        &self,
        htlc_params: HtlcParams<Ethereum, EtherQuantity>,
        htlc_deployment: &Deployed<Ethereum>,
        htlc_funding: &Funded<Ethereum, EtherQuantity>,
    ) -> Result<Either<Redeemed<Ethereum>, Refunded<Ethereum>>, rfc003::Error> {
        htlc_redeemed_or_refunded(self.clone(), htlc_params, htlc_deployment, htlc_funding).await
    }
}

fn calculate_contract_address_from_deployment_transaction(tx: &Transaction) -> Address {
    tx.from.calculate_contract_address(&tx.nonce)
}

async fn htlc_redeemed_or_refunded<A: Asset>(
    connector: Web3Connector,
    _htlc_params: HtlcParams<Ethereum, A>,
    htlc_deployment: &Deployed<Ethereum>,
    _: &Funded<Ethereum, A>,
) -> Result<Either<Redeemed<Ethereum>, Refunded<Ethereum>>, rfc003::Error> {
    let redeemed = {
        let connector = connector.clone();
        async {
            let pattern = TransactionPattern {
                from_address: None,
                to_address: None,
                is_contract_creation: None,
                transaction_data: None,
                transaction_data_length: None,
                events: Some(vec![Event {
                    address: Some(htlc_deployment.location),
                    data: None,
                    topics: vec![Some(Topic(*REDEEM_LOG_MSG))],
                }]),
            };

            let TransactionAndReceipt {
                transaction,
                receipt,
            } = matching_transaction(connector, pattern, None).await;
            let log = receipt
                .logs
                .into_iter()
                .find(|log| log.topics.contains(&*REDEEM_LOG_MSG))
                .ok_or_else(|| {
                    rfc003::Error::Internal(format!(
                        "transaction receipt {:?} did not contain a REDEEM log",
                        transaction.hash
                    ))
                })?;
            let log_data = log.data.0.as_ref();
            let secret = Secret::from_vec(log_data).map_err(|e| {
                rfc003::Error::Internal(format!(
                    "failed to construct secret from data in transaction receipt {:?}: {:?}",
                    transaction.hash, e
                ))
            })?;

            Ok(Redeemed {
                transaction,
                secret,
            })
        }
    };

    let refunded = async {
        let pattern = TransactionPattern {
            from_address: None,
            to_address: None,
            is_contract_creation: None,
            transaction_data: None,
            transaction_data_length: None,
            events: Some(vec![Event {
                address: Some(htlc_deployment.location),
                data: None,
                topics: vec![Some(Topic(*REFUND_LOG_MSG))],
            }]),
        };

        let TransactionAndReceipt { transaction, .. } =
            matching_transaction(connector, pattern, None).await;

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

mod erc20 {
    use super::*;
    use crate::ethereum::{Erc20Quantity, U256};

    #[async_trait::async_trait]
    impl HtlcEvents<Ethereum, Erc20Token> for Web3Connector {
        async fn htlc_deployed(
            &self,
            htlc_params: HtlcParams<Ethereum, Erc20Token>,
        ) -> Result<Deployed<Ethereum>, rfc003::Error> {
            let connector = self.clone();
            let pattern = TransactionPattern {
                from_address: None,
                to_address: None,
                is_contract_creation: Some(true),
                transaction_data: Some(htlc_params.bytecode()),
                transaction_data_length: None,
                events: None,
            };
            let TransactionAndReceipt { transaction, .. } =
                matching_transaction(connector, pattern, None).await;

            Ok(Deployed {
                location: calculate_contract_address_from_deployment_transaction(&transaction),
                transaction,
            })
        }

        async fn htlc_funded(
            &self,
            htlc_params: HtlcParams<Ethereum, Erc20Token>,
            htlc_deployment: &Deployed<Ethereum>,
        ) -> Result<Funded<Ethereum, Erc20Token>, rfc003::Error> {
            let connector = self.clone();
            let events = Some(vec![Event {
                address: Some(htlc_params.asset.token_contract),
                data: None,
                topics: vec![
                    Some(Topic(*super::TRANSFER_LOG_MSG)),
                    None,
                    Some(Topic(htlc_deployment.location.into())),
                ],
            }]);
            let TransactionAndReceipt {
                transaction,
                receipt,
            } = matching_transaction(
                connector,
                TransactionPattern {
                    from_address: None,
                    to_address: None,
                    is_contract_creation: None,
                    transaction_data: None,
                    transaction_data_length: None,
                    events,
                },
                None,
            )
            .await;
            let log = receipt
                .logs
                .into_iter()
                .find(|log| log.topics.contains(&*super::TRANSFER_LOG_MSG))
                .ok_or_else(|| {
                    log::warn!(
                        "receipt for transaction {:?} did not contain any Transfer events",
                        transaction.hash
                    );
                    rfc003::Error::IncorrectFunding
                })?;

            let quantity = Erc20Quantity(U256::from_big_endian(log.data.0.as_ref()));
            let asset = Erc20Token::new(log.address, quantity);

            Ok(Funded { transaction, asset })
        }

        async fn htlc_redeemed_or_refunded(
            &self,
            htlc_params: HtlcParams<Ethereum, Erc20Token>,
            htlc_deployment: &Deployed<Ethereum>,
            htlc_funding: &Funded<Ethereum, Erc20Token>,
        ) -> Result<Either<Redeemed<Ethereum>, Refunded<Ethereum>>, rfc003::Error> {
            htlc_redeemed_or_refunded(self.clone(), htlc_params, htlc_deployment, htlc_funding)
                .await
        }
    }
}
