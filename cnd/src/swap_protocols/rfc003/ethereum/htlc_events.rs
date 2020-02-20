use crate::{
    asset::{self, Asset},
    btsieve::ethereum::{
        matching_transaction, Cache, Event, Topic, TransactionPattern, Web3Connector,
    },
    ethereum,
    ethereum::{Address, CalculateContractAddress, Transaction, TransactionAndReceipt, H256},
    swap_protocols::{
        ledger::Ethereum,
        rfc003::{
            create_swap::HtlcParams,
            events::{
                Deployed, Funded, HtlcDeployed, HtlcFunded, HtlcRedeemed, HtlcRefunded, Redeemed,
                Refunded,
            },
            Secret,
        },
    },
};
use anyhow::Context;
use asset::ethereum::FromWei;
use chrono::NaiveDateTime;

lazy_static::lazy_static! {
    pub static ref REDEEM_LOG_MSG: H256 = blockchain_contracts::ethereum::rfc003::REDEEMED_LOG_MSG.parse().expect("to be valid hex");
    pub static ref REFUND_LOG_MSG: H256 = blockchain_contracts::ethereum::rfc003::REFUNDED_LOG_MSG.parse().expect("to be valid hex");
    /// keccak('Transfer(address,address,uint256)')
    pub static ref TRANSFER_LOG_MSG: H256 = "ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef".parse().expect("to be valid hex");
}

#[async_trait::async_trait]
impl HtlcFunded<Ethereum, asset::Ether, ethereum::Transaction> for Cache<Web3Connector> {
    async fn htlc_funded(
        &self,
        _htlc_params: HtlcParams<Ethereum, asset::Ether>,
        deploy_transaction: &Deployed<Ethereum>,
        _start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Funded<ethereum::Transaction, asset::Ether>> {
        Ok(Funded {
            transaction: deploy_transaction.transaction.clone(),
            asset: asset::Ether::from_wei(deploy_transaction.transaction.value),
        })
    }
}

#[async_trait::async_trait]
impl HtlcDeployed<Ethereum, asset::Ether> for Cache<Web3Connector> {
    async fn htlc_deployed(
        &self,
        htlc_params: HtlcParams<Ethereum, asset::Ether>,
        start_of_swap: NaiveDateTime,
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
            matching_transaction(connector, pattern, start_of_swap)
                .await
                .context("failed to find transaction for htlc deployment")?;

        Ok(Deployed {
            location: calculate_contract_address_from_deployment_transaction(&transaction),
            transaction,
        })
    }
}

#[async_trait::async_trait]
impl HtlcRedeemed<Ethereum, asset::Ether> for Cache<Web3Connector> {
    async fn htlc_redeemed(
        &self,
        htlc_params: HtlcParams<Ethereum, asset::Ether>,
        htlc_deployment: &Deployed<Ethereum>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Redeemed<Ethereum>> {
        htlc_redeemed(self.clone(), htlc_params, htlc_deployment, start_of_swap).await
    }
}

#[async_trait::async_trait]
impl HtlcRefunded<Ethereum, asset::Ether> for Cache<Web3Connector> {
    async fn htlc_refunded(
        &self,
        htlc_params: HtlcParams<Ethereum, asset::Ether>,
        htlc_deployment: &Deployed<Ethereum>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Refunded<Ethereum>> {
        htlc_refunded(self.clone(), htlc_params, htlc_deployment, start_of_swap).await
    }
}

fn calculate_contract_address_from_deployment_transaction(tx: &Transaction) -> Address {
    tx.from.calculate_contract_address(&tx.nonce)
}

async fn htlc_redeemed<A: Asset>(
    connector: Cache<Web3Connector>,
    _htlc_params: HtlcParams<Ethereum, A>,
    htlc_deployment: &Deployed<Ethereum>,
    start_of_swap: NaiveDateTime,
) -> anyhow::Result<Redeemed<Ethereum>> {
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
    } = matching_transaction(connector, pattern, start_of_swap)
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

async fn htlc_refunded<A: Asset>(
    connector: Cache<Web3Connector>,
    _htlc_params: HtlcParams<Ethereum, A>,
    htlc_deployment: &Deployed<Ethereum>,
    start_of_swap: NaiveDateTime,
) -> anyhow::Result<Refunded<Ethereum>> {
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
        matching_transaction(connector, pattern, start_of_swap)
            .await
            .context("failed to find transaction to refund from htlc")?;

    Ok(Refunded { transaction })
}

mod erc20 {
    use super::*;
    use crate::ethereum::U256;
    use asset::ethereum::FromWei;

    #[async_trait::async_trait]
    impl HtlcFunded<Ethereum, asset::Erc20, ethereum::Transaction> for Cache<Web3Connector> {
        async fn htlc_funded(
            &self,
            htlc_params: HtlcParams<Ethereum, asset::Erc20>,
            htlc_deployment: &Deployed<Ethereum>,
            start_of_swap: NaiveDateTime,
        ) -> anyhow::Result<Funded<ethereum::Transaction, asset::Erc20>> {
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
                start_of_swap,
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
    }

    #[async_trait::async_trait]
    impl HtlcDeployed<Ethereum, asset::Erc20> for Cache<Web3Connector> {
        async fn htlc_deployed(
            &self,
            htlc_params: HtlcParams<Ethereum, asset::Erc20>,
            start_of_swap: NaiveDateTime,
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
                matching_transaction(connector, pattern, start_of_swap)
                    .await
                    .context("failed to find transaction to deploy htlc")?;

            Ok(Deployed {
                location: calculate_contract_address_from_deployment_transaction(&transaction),
                transaction,
            })
        }
    }

    #[async_trait::async_trait]
    impl HtlcRedeemed<Ethereum, asset::Erc20> for Cache<Web3Connector> {
        async fn htlc_redeemed(
            &self,
            htlc_params: HtlcParams<Ethereum, asset::Erc20>,
            htlc_deployment: &Deployed<Ethereum>,
            start_of_swap: NaiveDateTime,
        ) -> anyhow::Result<Redeemed<Ethereum>> {
            htlc_redeemed(self.clone(), htlc_params, htlc_deployment, start_of_swap).await
        }
    }

    #[async_trait::async_trait]
    impl HtlcRefunded<Ethereum, asset::Erc20> for Cache<Web3Connector> {
        async fn htlc_refunded(
            &self,
            htlc_params: HtlcParams<Ethereum, asset::Erc20>,
            htlc_deployment: &Deployed<Ethereum>,
            start_of_swap: NaiveDateTime,
        ) -> anyhow::Result<Refunded<Ethereum>> {
            htlc_refunded(self.clone(), htlc_params, htlc_deployment, start_of_swap).await
        }
    }
}
