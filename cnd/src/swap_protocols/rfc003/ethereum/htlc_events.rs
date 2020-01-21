use crate::{
    asset::{self, Asset},
    btsieve::ethereum::{matching_transaction, Event, Topic, TransactionPattern, Web3Connector},
    ethereum::{Address, CalculateContractAddress, Transaction, TransactionAndReceipt, H256},
    swap_protocols::{
        ledger::Ethereum,
        rfc003::{
            create_swap::HtlcParams,
            events::{Deployed, Funded, HtlcEvents, Redeemed, Refunded},
            Secret,
        },
    },
};
use anyhow::Context;
use asset::ethereum::FromWei;
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
impl HtlcEvents<Ethereum, asset::Ether> for Web3Connector {
    async fn htlc_deployed(
        &self,
        htlc_params: HtlcParams<Ethereum, asset::Ether>,
    ) -> anyhow::Result<Deployed<Ethereum>> {
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
            matching_transaction(connector, pattern, None)
                .await
                .context("failed to find transaction for htlc deployment")?;

        Ok(Deployed {
            location: calculate_contract_address_from_deployment_transaction(&transaction),
            transaction,
        })
    }

    async fn htlc_funded(
        &self,
        _htlc_params: HtlcParams<Ethereum, asset::Ether>,
        deploy_transaction: &Deployed<Ethereum>,
    ) -> anyhow::Result<Funded<Ethereum, asset::Ether>> {
        Ok(Funded {
            transaction: deploy_transaction.transaction.clone(),
            asset: asset::Ether::from_wei(deploy_transaction.transaction.value),
        })
    }

    async fn htlc_redeemed_or_refunded(
        &self,
        htlc_params: HtlcParams<Ethereum, asset::Ether>,
        htlc_deployment: &Deployed<Ethereum>,
        htlc_funding: &Funded<Ethereum, asset::Ether>,
    ) -> anyhow::Result<Either<Redeemed<Ethereum>, Refunded<Ethereum>>> {
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
) -> anyhow::Result<Either<Redeemed<Ethereum>, Refunded<Ethereum>>> {
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
            } = matching_transaction(connector, pattern, None)
                .await
                .context("failed to find transaction to redeem from htlc")?;
            let log = receipt
                .logs
                .into_iter()
                .find(|log| log.topics.contains(&*REDEEM_LOG_MSG))
                .expect("Redeem transaction receipt must contain redeem logs");
            let log_data = log.data.0.as_ref();
            let secret =
                Secret::from_vec(log_data).expect("Must be able to construct secret from log data");

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
            matching_transaction(connector, pattern, None)
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

mod erc20 {
    use super::*;
    use crate::ethereum::U256;
    use asset::ethereum::FromWei;

    #[async_trait::async_trait]
    impl HtlcEvents<Ethereum, asset::Erc20> for Web3Connector {
        async fn htlc_deployed(
            &self,
            htlc_params: HtlcParams<Ethereum, asset::Erc20>,
        ) -> anyhow::Result<Deployed<Ethereum>> {
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
                matching_transaction(connector, pattern, None)
                    .await
                    .context("failed to find transaction to deploy htlc")?;

            Ok(Deployed {
                location: calculate_contract_address_from_deployment_transaction(&transaction),
                transaction,
            })
        }

        async fn htlc_funded(
            &self,
            htlc_params: HtlcParams<Ethereum, asset::Erc20>,
            htlc_deployment: &Deployed<Ethereum>,
        ) -> anyhow::Result<Funded<Ethereum, asset::Erc20>> {
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
            .await
            .context("failed to find transaction to fund htlc")?;
            let log = receipt
                .logs
                .into_iter()
                .find(|log| log.topics.contains(&*super::TRANSFER_LOG_MSG))
                .expect("Fund transaction receipt must contain transfer events");

            let quantity =
                asset::Erc20Quantity::from_wei(U256::from_big_endian(log.data.0.as_ref()));
            let asset = asset::Erc20::new(log.address, quantity);

            Ok(Funded { transaction, asset })
        }

        async fn htlc_redeemed_or_refunded(
            &self,
            htlc_params: HtlcParams<Ethereum, asset::Erc20>,
            htlc_deployment: &Deployed<Ethereum>,
            htlc_funding: &Funded<Ethereum, asset::Erc20>,
        ) -> anyhow::Result<Either<Redeemed<Ethereum>, Refunded<Ethereum>>> {
            htlc_redeemed_or_refunded(self.clone(), htlc_params, htlc_deployment, htlc_funding)
                .await
        }
    }
}
