use crate::{
    asset::{ethereum::FromWei, Erc20, Erc20Quantity, Ether},
    btsieve::ethereum::{
        watch_for_contract_creation, watch_for_event, Cache, Event, Topic, Web3Connector,
    },
    ethereum::{Address, Transaction, H256, U256},
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
use chrono::NaiveDateTime;
use tracing_futures::Instrument;

lazy_static::lazy_static! {
    static ref REDEEM_LOG_MSG: H256 = blockchain_contracts::ethereum::rfc003::REDEEMED_LOG_MSG.parse().expect("to be valid hex");
    static ref REFUND_LOG_MSG: H256 = blockchain_contracts::ethereum::rfc003::REFUNDED_LOG_MSG.parse().expect("to be valid hex");
    /// keccak('Transfer(address,address,uint256)')
    static ref TRANSFER_LOG_MSG: H256 = "ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef".parse().expect("to be valid hex");
}

#[async_trait::async_trait]
impl HtlcFunded<Ethereum, Ether> for Cache<Web3Connector> {
    async fn htlc_funded(
        &self,
        _htlc_params: &HtlcParams<'_, Ethereum, Ether, Address>,
        deploy_transaction: &Deployed<Transaction, Address>,
        _start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Funded<Transaction, Ether>> {
        Ok(Funded {
            transaction: deploy_transaction.transaction.clone(),
            asset: Ether::from_wei(deploy_transaction.transaction.value),
        })
    }
}

#[async_trait::async_trait]
impl HtlcDeployed<Ethereum, Ether> for Cache<Web3Connector> {
    async fn htlc_deployed(
        &self,
        htlc_params: &HtlcParams<'_, Ethereum, Ether, Address>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Deployed<Transaction, Address>> {
        let connector = self.clone();
        let (transaction, location) =
            watch_for_contract_creation(connector, start_of_swap, htlc_params.bytecode())
                .instrument(tracing::info_span!("htlc_deployed"))
                .await?;

        Ok(Deployed {
            transaction,
            location,
        })
    }
}

#[async_trait::async_trait]
impl HtlcRedeemed<Ethereum, Ether> for Cache<Web3Connector> {
    async fn htlc_redeemed(
        &self,
        _htlc_params: &HtlcParams<'_, Ethereum, Ether, Address>,
        htlc_deployment: &Deployed<Transaction, Address>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Redeemed<Transaction>> {
        let connector = self.clone();
        let event = Event {
            address: htlc_deployment.location,
            topics: vec![Some(Topic(*REDEEM_LOG_MSG))],
        };

        let (transaction, log) = watch_for_event(connector, start_of_swap, event)
            .instrument(tracing::info_span!("htlc_redeemed"))
            .await?;

        let log_data = log.data.0.as_ref();
        let secret =
            Secret::from_vec(log_data).expect("Must be able to construct secret from log data");

        Ok(Redeemed {
            transaction,
            secret,
        })
    }
}

#[async_trait::async_trait]
impl HtlcRefunded<Ethereum, Ether> for Cache<Web3Connector> {
    async fn htlc_refunded(
        &self,
        _htlc_params: &HtlcParams<'_, Ethereum, Ether, Address>,
        htlc_deployment: &Deployed<Transaction, Address>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Refunded<Transaction>> {
        let connector = self.clone();
        let event = Event {
            address: htlc_deployment.location,
            topics: vec![Some(Topic(*REFUND_LOG_MSG))],
        };

        let (transaction, _) = watch_for_event(connector, start_of_swap, event)
            .instrument(tracing::info_span!("htlc_refunded"))
            .await?;

        Ok(Refunded { transaction })
    }
}

#[async_trait::async_trait]
impl HtlcFunded<Ethereum, Erc20> for Cache<Web3Connector> {
    async fn htlc_funded(
        &self,
        htlc_params: &HtlcParams<'_, Ethereum, Erc20, Address>,
        htlc_deployment: &Deployed<Transaction, Address>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Funded<Transaction, Erc20>> {
        let connector = self.clone();

        let event = Event {
            address: htlc_params.asset.token_contract,
            topics: vec![
                Some(Topic(*TRANSFER_LOG_MSG)),
                None,
                Some(Topic(htlc_deployment.location.into())),
            ],
        };

        let (transaction, log) = watch_for_event(connector, start_of_swap, event)
            .instrument(tracing::info_span!("htlc_funded"))
            .await?;

        let quantity = Erc20Quantity::from_wei(U256::from_big_endian(log.data.0.as_ref()));
        let asset = Erc20::new(log.address, quantity);

        Ok(Funded { transaction, asset })
    }
}

#[async_trait::async_trait]
impl HtlcDeployed<Ethereum, Erc20> for Cache<Web3Connector> {
    async fn htlc_deployed(
        &self,
        htlc_params: &HtlcParams<'_, Ethereum, Erc20, Address>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Deployed<Transaction, Address>> {
        let connector = self.clone();

        let (transaction, location) =
            watch_for_contract_creation(connector, start_of_swap, htlc_params.clone().bytecode())
                .instrument(tracing::info_span!("htlc_deployed"))
                .await?;

        Ok(Deployed {
            transaction,
            location,
        })
    }
}

#[async_trait::async_trait]
impl HtlcRedeemed<Ethereum, Erc20> for Cache<Web3Connector> {
    async fn htlc_redeemed(
        &self,
        _htlc_params: &HtlcParams<'_, Ethereum, Erc20, Address>,
        htlc_deployment: &Deployed<Transaction, Address>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Redeemed<Transaction>> {
        let connector = self.clone();
        let event = Event {
            address: htlc_deployment.location,
            topics: vec![Some(Topic(*REDEEM_LOG_MSG))],
        };

        let (transaction, log) = watch_for_event(connector, start_of_swap, event)
            .instrument(tracing::info_span!("htlc_redeemed"))
            .await?;

        let log_data = log.data.0.as_ref();
        let secret =
            Secret::from_vec(log_data).expect("Must be able to construct secret from log data");

        Ok(Redeemed {
            transaction,
            secret,
        })
    }
}

#[async_trait::async_trait]
impl HtlcRefunded<Ethereum, Erc20> for Cache<Web3Connector> {
    async fn htlc_refunded(
        &self,
        _htlc_params: &HtlcParams<'_, Ethereum, Erc20, Address>,
        htlc_deployment: &Deployed<Transaction, Address>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Refunded<Transaction>> {
        let connector = self.clone();
        let event = Event {
            address: htlc_deployment.location,
            topics: vec![Some(Topic(*REFUND_LOG_MSG))],
        };

        let (transaction, _) = watch_for_event(connector, start_of_swap, event)
            .instrument(tracing::info_span!("htlc_refunded"))
            .await?;

        Ok(Refunded { transaction })
    }
}
