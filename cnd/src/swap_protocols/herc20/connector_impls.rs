use crate::{
    asset::{ethereum::FromWei, Erc20, Erc20Quantity},
    btsieve::ethereum::{
        watch_for_contract_creation, watch_for_event, Cache, Event, Topic, Web3Connector,
    },
    ethereum::{Hash, U256},
    swap_protocols::{
        herc20::{
            Deployed, Funded, Params, Redeemed, Refunded, WaitForDeployed, WaitForFunded,
            WaitForRedeemed, WaitForRefunded,
        },
        rfc003::Secret,
    },
};
use std::cmp::Ordering;
use tracing_futures::Instrument;

lazy_static::lazy_static! {
    static ref REDEEM_LOG_MSG: Hash = blockchain_contracts::ethereum::rfc003::REDEEMED_LOG_MSG.parse().expect("to be valid hex");
    static ref REFUND_LOG_MSG: Hash = blockchain_contracts::ethereum::rfc003::REFUNDED_LOG_MSG.parse().expect("to be valid hex");
    /// keccak('Transfer(address,address,uint256)')
    static ref TRANSFER_LOG_MSG: Hash = "ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef".parse().expect("to be valid hex");
}

#[async_trait::async_trait]
impl WaitForDeployed for Cache<Web3Connector> {
    async fn wait_for_deployed(&self, params: Params) -> anyhow::Result<Deployed> {
        let expected_bytecode = params.clone().bytecode();

        let (transaction, location) =
            watch_for_contract_creation(self, params.start_of_swap, &expected_bytecode)
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
    async fn wait_for_funded(&self, params: Params, deployed: Deployed) -> anyhow::Result<Funded> {
        let event = Event {
            address: params.asset.token_contract,
            topics: vec![
                Some(Topic(*TRANSFER_LOG_MSG)),
                None,
                Some(Topic(deployed.location.into())),
            ],
        };

        let (transaction, log) = watch_for_event(self, params.start_of_swap, event)
            .instrument(tracing::trace_span!("funded"))
            .await?;

        let expected_asset = &params.asset;

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
impl WaitForRedeemed for Cache<Web3Connector> {
    async fn wait_for_redeemed(
        &self,
        params: Params,
        deployed: Deployed,
    ) -> anyhow::Result<Redeemed> {
        let event = Event {
            address: deployed.location,
            topics: vec![Some(Topic(*REDEEM_LOG_MSG))],
        };

        let (transaction, log) = watch_for_event(self, params.start_of_swap, event)
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
        params: Params,
        deployed: Deployed,
    ) -> anyhow::Result<Refunded> {
        let event = Event {
            address: deployed.location,
            topics: vec![Some(Topic(*REFUND_LOG_MSG))],
        };

        let (transaction, _) = watch_for_event(self, params.start_of_swap, event)
            .instrument(tracing::info_span!("refunded"))
            .await?;

        Ok(Refunded { transaction })
    }
}
