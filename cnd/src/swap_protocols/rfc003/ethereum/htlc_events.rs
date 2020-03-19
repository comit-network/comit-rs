use crate::{
    asset,
    asset::{ethereum::FromWei, Erc20, Erc20Quantity, Ether},
    btsieve::ethereum::{
        watch_for_contract_creation, watch_for_event, Cache, Event, Topic, Web3Connector,
    },
    ethereum::{H256, U256},
    htlc_location, identity,
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
    transaction,
};
use chrono::NaiveDateTime;
use std::cmp::Ordering;
use tracing_futures::Instrument;

lazy_static::lazy_static! {
    static ref REDEEM_LOG_MSG: H256 = blockchain_contracts::ethereum::rfc003::REDEEMED_LOG_MSG.parse().expect("to be valid hex");
    static ref REFUND_LOG_MSG: H256 = blockchain_contracts::ethereum::rfc003::REFUNDED_LOG_MSG.parse().expect("to be valid hex");
    /// keccak('Transfer(address,address,uint256)')
    static ref TRANSFER_LOG_MSG: H256 = "ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef".parse().expect("to be valid hex");
}

#[async_trait::async_trait]
impl
    HtlcFunded<
        Ethereum,
        asset::Ether,
        htlc_location::Ethereum,
        identity::Ethereum,
        transaction::Ethereum,
    > for Cache<Web3Connector>
{
    async fn htlc_funded(
        &self,
        htlc_params: &HtlcParams<Ethereum, asset::Ether, identity::Ethereum>,
        deploy_transaction: &Deployed<htlc_location::Ethereum, transaction::Ethereum>,
        _start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Funded<asset::Ether, transaction::Ethereum>> {
        let expected_asset = &htlc_params.asset;

        let asset = Ether::from_wei(deploy_transaction.transaction.value);

        let event = match expected_asset.cmp(&asset) {
            Ordering::Equal => Funded::Correctly {
                transaction: deploy_transaction.transaction.clone(),
                asset,
            },
            _ => Funded::Incorrectly {
                transaction: deploy_transaction.transaction.clone(),
                asset,
            },
        };

        Ok(event)
    }
}

#[async_trait::async_trait]
impl
    HtlcDeployed<
        Ethereum,
        asset::Ether,
        htlc_location::Ethereum,
        identity::Ethereum,
        transaction::Ethereum,
    > for Cache<Web3Connector>
{
    async fn htlc_deployed(
        &self,
        htlc_params: &HtlcParams<Ethereum, asset::Ether, identity::Ethereum>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Deployed<htlc_location::Ethereum, transaction::Ethereum>> {
        let (transaction, location) =
            watch_for_contract_creation(self, start_of_swap, htlc_params.bytecode())
                .instrument(tracing::info_span!("htlc_deployed"))
                .await?;

        Ok(Deployed {
            transaction,
            location,
        })
    }
}

#[async_trait::async_trait]
impl
    HtlcRedeemed<
        Ethereum,
        asset::Ether,
        htlc_location::Ethereum,
        identity::Ethereum,
        transaction::Ethereum,
    > for Cache<Web3Connector>
{
    async fn htlc_redeemed(
        &self,
        _htlc_params: &HtlcParams<Ethereum, asset::Ether, identity::Ethereum>,
        htlc_deployment: &Deployed<htlc_location::Ethereum, transaction::Ethereum>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Redeemed<transaction::Ethereum>> {
        let event = Event {
            address: htlc_deployment.location,
            topics: vec![Some(Topic(*REDEEM_LOG_MSG))],
        };

        let (transaction, log) = watch_for_event(self, start_of_swap, event)
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
impl
    HtlcRefunded<
        Ethereum,
        asset::Ether,
        htlc_location::Ethereum,
        identity::Ethereum,
        transaction::Ethereum,
    > for Cache<Web3Connector>
{
    async fn htlc_refunded(
        &self,
        _htlc_params: &HtlcParams<Ethereum, asset::Ether, identity::Ethereum>,
        htlc_deployment: &Deployed<htlc_location::Ethereum, transaction::Ethereum>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Refunded<transaction::Ethereum>> {
        let event = Event {
            address: htlc_deployment.location,
            topics: vec![Some(Topic(*REFUND_LOG_MSG))],
        };

        let (transaction, _) = watch_for_event(self, start_of_swap, event)
            .instrument(tracing::info_span!("htlc_refunded"))
            .await?;

        Ok(Refunded { transaction })
    }
}

#[async_trait::async_trait]
impl
    HtlcFunded<
        Ethereum,
        asset::Erc20,
        htlc_location::Ethereum,
        identity::Ethereum,
        transaction::Ethereum,
    > for Cache<Web3Connector>
{
    async fn htlc_funded(
        &self,
        htlc_params: &HtlcParams<Ethereum, asset::Erc20, identity::Ethereum>,
        htlc_deployment: &Deployed<htlc_location::Ethereum, transaction::Ethereum>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Funded<asset::Erc20, transaction::Ethereum>> {
        let event = Event {
            address: htlc_params.asset.token_contract,
            topics: vec![
                Some(Topic(*TRANSFER_LOG_MSG)),
                None,
                Some(Topic(htlc_deployment.location.into())),
            ],
        };

        let (transaction, log) = watch_for_event(self, start_of_swap, event)
            .instrument(tracing::info_span!("htlc_funded"))
            .await?;

        let expected_asset = &htlc_params.asset;

        let quantity = Erc20Quantity::from_wei(U256::from_big_endian(log.data.0.as_ref()));
        let asset = Erc20::new(log.address, quantity);

        let event = match expected_asset.cmp(&asset) {
            Ordering::Equal => Funded::Correctly { transaction, asset },
            _ => Funded::Incorrectly { transaction, asset },
        };

        Ok(event)
    }
}

#[async_trait::async_trait]
impl
    HtlcDeployed<
        Ethereum,
        asset::Erc20,
        htlc_location::Ethereum,
        identity::Ethereum,
        transaction::Ethereum,
    > for Cache<Web3Connector>
{
    async fn htlc_deployed(
        &self,
        htlc_params: &HtlcParams<Ethereum, asset::Erc20, identity::Ethereum>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Deployed<htlc_location::Ethereum, transaction::Ethereum>> {
        let (transaction, location) =
            watch_for_contract_creation(self, start_of_swap, htlc_params.clone().bytecode())
                .instrument(tracing::info_span!("htlc_deployed"))
                .await?;

        Ok(Deployed {
            transaction,
            location,
        })
    }
}

#[async_trait::async_trait]
impl
    HtlcRedeemed<
        Ethereum,
        asset::Erc20,
        htlc_location::Ethereum,
        identity::Ethereum,
        transaction::Ethereum,
    > for Cache<Web3Connector>
{
    async fn htlc_redeemed(
        &self,
        _htlc_params: &HtlcParams<Ethereum, Erc20, identity::Ethereum>,
        htlc_deployment: &Deployed<htlc_location::Ethereum, transaction::Ethereum>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Redeemed<transaction::Ethereum>> {
        let event = Event {
            address: htlc_deployment.location,
            topics: vec![Some(Topic(*REDEEM_LOG_MSG))],
        };

        let (transaction, log) = watch_for_event(self, start_of_swap, event)
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
impl
    HtlcRefunded<
        Ethereum,
        Erc20,
        htlc_location::Ethereum,
        identity::Ethereum,
        transaction::Ethereum,
    > for Cache<Web3Connector>
{
    async fn htlc_refunded(
        &self,
        _htlc_params: &HtlcParams<Ethereum, Erc20, identity::Ethereum>,
        htlc_deployment: &Deployed<htlc_location::Ethereum, transaction::Ethereum>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Refunded<transaction::Ethereum>> {
        let event = Event {
            address: htlc_deployment.location,
            topics: vec![Some(Topic(*REFUND_LOG_MSG))],
        };

        let (transaction, _) = watch_for_event(self, start_of_swap, event)
            .instrument(tracing::info_span!("htlc_refunded"))
            .await?;

        Ok(Refunded { transaction })
    }
}
