use crate::{
    asset::{ethereum::FromWei, Erc20, Erc20Quantity},
    btsieve::ethereum::{
        watch_for_contract_creation, watch_for_event, Cache, Event, Topic, Web3Connector,
    },
    ethereum::{Hash, U256},
    herc20::{
        Deployed, Funded, Params, Redeemed, Refunded, WaitForDeployed, WaitForFunded,
        WaitForRedeemed, WaitForRefunded,
    },
    Secret,
};
use chrono::NaiveDateTime;
use tracing_futures::Instrument;

lazy_static::lazy_static! {
    static ref REDEEM_LOG_MSG: Hash = blockchain_contracts::ethereum::rfc003::REDEEMED_LOG_MSG.parse().expect("to be valid hex");
    static ref REFUND_LOG_MSG: Hash = blockchain_contracts::ethereum::rfc003::REFUNDED_LOG_MSG.parse().expect("to be valid hex");
    static ref TRANSFER_LOG_MSG: Hash = blockchain_contracts::ethereum::rfc003::ERC20_TRANSFER.parse().expect("to be valid hex");
}

#[async_trait::async_trait]
impl WaitForDeployed for Cache<Web3Connector> {
    async fn wait_for_deployed(
        &self,
        params: Params,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Deployed> {
        let expected_bytecode = params.clone().bytecode();

        let (transaction, location) =
            watch_for_contract_creation(self, start_of_swap, &expected_bytecode)
                .instrument(tracing::trace_span!(
                    "deployed",
                    expected_bytecode = %hex::encode(&expected_bytecode.0)
                ))
                .await?;

        Ok(Deployed {
            transaction,
            location,
        })
    }
}

#[async_trait::async_trait]
impl WaitForFunded for Cache<Web3Connector> {
    async fn wait_for_funded(
        &self,
        params: Params,
        start_of_swap: NaiveDateTime,
        deployed: Deployed,
    ) -> anyhow::Result<Funded> {
        let event = Event {
            address: params.asset.token_contract,
            topics: vec![
                Some(Topic(*TRANSFER_LOG_MSG)),
                None,
                Some(Topic(deployed.location.into())),
            ],
        };

        let (transaction, log) = watch_for_event(self, start_of_swap, event)
            .instrument(tracing::trace_span!("funded"))
            .await?;

        let quantity = Erc20Quantity::from_wei(U256::from_big_endian(log.data.0.as_ref()));
        let asset = Erc20::new(log.address, quantity);

        Ok(Funded {
            transaction,
            asset,
            deploy_transaction: deployed.transaction,
            location: deployed.location,
        })
    }
}

#[async_trait::async_trait]
impl WaitForRedeemed for Cache<Web3Connector> {
    async fn wait_for_redeemed(
        &self,
        start_of_swap: NaiveDateTime,
        deployed: Deployed,
    ) -> anyhow::Result<Redeemed> {
        let event = Event {
            address: deployed.location,
            topics: vec![Some(Topic(*REDEEM_LOG_MSG))],
        };

        let (transaction, log) = watch_for_event(self, start_of_swap, event)
            .instrument(tracing::info_span!("redeemed"))
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
impl WaitForRefunded for Cache<Web3Connector> {
    async fn wait_for_refunded(
        &self,
        start_of_swap: NaiveDateTime,
        deployed: Deployed,
    ) -> anyhow::Result<Refunded> {
        let event = Event {
            address: deployed.location,
            topics: vec![Some(Topic(*REFUND_LOG_MSG))],
        };

        let (transaction, _) = watch_for_event(self, start_of_swap, event)
            .instrument(tracing::info_span!("refunded"))
            .await?;

        Ok(Refunded { transaction })
    }
}
