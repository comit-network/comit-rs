//! Htlc ERC20 Token atomic swap protocol.

use crate::{
    actions, asset,
    asset::{ethereum::FromWei, Erc20, Erc20Quantity},
    btsieve::{
        ethereum::{
            watch_for_contract_creation, watch_for_event, GetLogs, ReceiptByHash, TransactionByHash,
        },
        BlockByHash, ConnectedNetwork, LatestBlock,
    },
    ethereum,
    ethereum::{Block, ChainId, Hash, U256},
    htlc_location, identity,
    timestamp::Timestamp,
    Secret, SecretHash,
};
use anyhow::Result;
use blockchain_contracts::ethereum::herc20::Htlc;
use conquer_once::Lazy;
use std::cmp::Ordering;
use thiserror::Error;
use time::OffsetDateTime;
use tracing_futures::Instrument;

static REDEEM_LOG_MSG: Lazy<Hash> = Lazy::new(|| {
    blockchain_contracts::ethereum::REDEEMED_LOG_MSG
        .parse()
        .expect("to be valid hex")
});
static REFUND_LOG_MSG: Lazy<Hash> = Lazy::new(|| {
    blockchain_contracts::ethereum::REFUNDED_LOG_MSG
        .parse()
        .expect("to be valid hex")
});
static TRANSFER_LOG_MSG: Lazy<Hash> = Lazy::new(|| {
    blockchain_contracts::ethereum::ERC20_TRANSFER
        .parse()
        .expect("to be valid hex")
});

/// Represents the data available at said state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Deployed {
    pub transaction: ethereum::Hash,
    pub location: htlc_location::Ethereum,
}

#[derive(Debug, Clone)]
pub struct Funded {
    pub transaction: ethereum::Hash,
    pub asset: asset::Erc20,
}

#[derive(Debug, Clone, Error)]
#[error("herc20 HTLC was incorrectly funded, expected {expected} but got {got}")]
pub struct IncorrectlyFunded {
    pub expected: asset::Erc20,
    pub got: asset::Erc20,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Redeemed {
    pub transaction: ethereum::Hash,
    pub secret: Secret,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Refunded {
    pub transaction: ethereum::Hash,
}

#[async_trait::async_trait]
pub trait WatchForDeployed {
    async fn watch_for_deployed(
        &self,
        params: Params,
        utc_start_of_swap: OffsetDateTime,
    ) -> Deployed;
}

#[async_trait::async_trait]
pub trait WatchForFunded {
    async fn watch_for_funded(
        &self,
        params: Params,
        deploy_event: Deployed,
        utc_start_of_swap: OffsetDateTime,
    ) -> Result<Funded, IncorrectlyFunded>;
}

#[async_trait::async_trait]
pub trait WatchForRedeemed {
    async fn watch_for_redeemed(
        &self,
        params: Params,
        deploy_event: Deployed,
        utc_start_of_swap: OffsetDateTime,
    ) -> Redeemed;
}

pub async fn watch_for_deployed<C>(
    connector: &C,
    params: Params,
    start_of_swap: OffsetDateTime,
) -> Result<Deployed>
where
    C: LatestBlock<Block = Block>
        + BlockByHash<Block = Block, BlockHash = Hash>
        + ReceiptByHash
        + ConnectedNetwork<Network = ChainId>,
{
    let expected_bytecode = params.clone().bytecode();

    let (transaction, location) =
        watch_for_contract_creation(connector, start_of_swap, &expected_bytecode)
            .instrument(tracing::info_span!("", action = "deploy"))
            .await?;

    Ok(Deployed {
        transaction: transaction.hash,
        location,
    })
}

pub async fn watch_for_funded<C>(
    connector: &C,
    params: Params,
    start_of_swap: OffsetDateTime,
    deployed: Deployed,
) -> Result<Result<Funded, IncorrectlyFunded>>
where
    C: LatestBlock<Block = Block>
        + BlockByHash<Block = Block, BlockHash = Hash>
        + ReceiptByHash
        + TransactionByHash
        + ConnectedNetwork<Network = ChainId>
        + GetLogs,
{
    use crate::btsieve::ethereum::Event;

    let event = Event {
        address: params.asset.token_contract,
        topics: vec![
            Some(*TRANSFER_LOG_MSG),
            None,
            Some(deployed.location.into()),
        ],
    };

    let (transaction, log) = watch_for_event(connector, start_of_swap, event)
        .instrument(tracing::info_span!("", action = "fund"))
        .await?;

    let expected_asset = &params.asset;

    let quantity = Erc20Quantity::from_wei(U256::from_big_endian(&log.data.0));
    let asset = Erc20::new(log.address, quantity);

    match expected_asset.cmp(&asset) {
        Ordering::Equal => Ok(Ok(Funded {
            transaction: transaction.hash,
            asset,
        })),
        _ => Ok(Err(IncorrectlyFunded {
            expected: params.asset,
            got: asset,
        })),
    }
}

pub async fn watch_for_redeemed<C>(
    connector: &C,
    start_of_swap: OffsetDateTime,
    deployed: Deployed,
) -> Result<Redeemed>
where
    C: LatestBlock<Block = Block>
        + BlockByHash<Block = Block, BlockHash = Hash>
        + ReceiptByHash
        + TransactionByHash
        + ConnectedNetwork<Network = ChainId>
        + GetLogs,
{
    use crate::btsieve::ethereum::Event;

    let event = Event {
        address: deployed.location,
        topics: vec![Some(*REDEEM_LOG_MSG)],
    };

    let (transaction, log) = watch_for_event(connector, start_of_swap, event)
        .instrument(tracing::info_span!("", action = "redeem"))
        .await?;

    let secret =
        Secret::from_vec(&log.data.0).expect("Must be able to construct secret from log data");

    Ok(Redeemed {
        transaction: transaction.hash,
        secret,
    })
}

pub async fn watch_for_refunded<C>(
    connector: &C,
    start_of_swap: OffsetDateTime,
    deployed: Deployed,
) -> Result<Refunded>
where
    C: LatestBlock<Block = Block>
        + BlockByHash<Block = Block, BlockHash = Hash>
        + ReceiptByHash
        + TransactionByHash
        + ConnectedNetwork<Network = ChainId>
        + GetLogs,
{
    use crate::btsieve::ethereum::Event;

    let event = Event {
        address: deployed.location,
        topics: vec![Some(*REFUND_LOG_MSG)],
    };

    let (transaction, _) = watch_for_event(connector, start_of_swap, event)
        .instrument(tracing::info_span!("", action = "refund"))
        .await?;

    Ok(Refunded {
        transaction: transaction.hash,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Params {
    pub asset: asset::Erc20,
    pub redeem_identity: identity::Ethereum,
    pub refund_identity: identity::Ethereum,
    pub expiry: Timestamp,
    pub secret_hash: SecretHash,
    pub chain_id: ChainId,
}

impl Params {
    pub fn bytecode(&self) -> Vec<u8> {
        Htlc::from(self.clone()).into()
    }

    pub fn build_deploy_action(&self) -> actions::ethereum::DeployContract {
        let chain_id = self.chain_id;
        let htlc = Htlc::from(self.clone());
        let gas_limit = Htlc::deploy_tx_gas_limit();

        actions::ethereum::DeployContract {
            data: htlc.into(),
            amount: asset::Ether::zero(),
            gas_limit,
            chain_id,
        }
    }

    pub fn build_fund_action(
        &self,
        htlc_location: htlc_location::Ethereum,
    ) -> actions::ethereum::CallContract {
        let to = self.asset.token_contract;
        let htlc_address = blockchain_contracts::ethereum::Address(htlc_location.into());
        let data =
            Htlc::transfer_erc20_tx_payload(self.asset.clone().quantity.into(), htlc_address);
        let data = Some(data);

        let gas_limit = Htlc::fund_tx_gas_limit();
        let min_block_timestamp = None;

        actions::ethereum::CallContract {
            to,
            data,
            gas_limit,
            chain_id: self.chain_id,
            min_block_timestamp,
        }
    }

    pub fn build_refund_action(
        &self,
        htlc_location: htlc_location::Ethereum,
    ) -> actions::ethereum::CallContract {
        let data = None;
        let gas_limit = Htlc::refund_tx_gas_limit();
        let min_block_timestamp = Some(self.expiry);

        actions::ethereum::CallContract {
            to: htlc_location,
            data,
            gas_limit,
            chain_id: self.chain_id,
            min_block_timestamp,
        }
    }

    pub fn build_redeem_action(
        &self,
        htlc_location: htlc_location::Ethereum,
        secret: Secret,
    ) -> actions::ethereum::CallContract {
        let data = Some(secret.into_raw_secret().to_vec());
        let gas_limit = Htlc::redeem_tx_gas_limit();
        let min_block_timestamp = None;

        actions::ethereum::CallContract {
            to: htlc_location,
            data,
            gas_limit,
            chain_id: self.chain_id,
            min_block_timestamp,
        }
    }
}

impl From<Params> for Htlc {
    fn from(params: Params) -> Self {
        let refund_address = blockchain_contracts::ethereum::Address(params.refund_identity.into());
        let redeem_address = blockchain_contracts::ethereum::Address(params.redeem_identity.into());
        let token_contract_address =
            blockchain_contracts::ethereum::Address(params.asset.token_contract.into());

        Htlc::new(
            params.expiry.into(),
            refund_address,
            redeem_address,
            params.secret_hash.into(),
            token_contract_address,
            params.asset.quantity.into(),
        )
    }
}

pub fn build_erc20_htlc(
    asset: asset::Erc20,
    redeem_identity: identity::Ethereum,
    refund_identity: identity::Ethereum,
    expiry: Timestamp,
    secret_hash: SecretHash,
) -> Htlc {
    let refund_address = blockchain_contracts::ethereum::Address(refund_identity.into());
    let redeem_address = blockchain_contracts::ethereum::Address(redeem_identity.into());
    let token_contract_address =
        blockchain_contracts::ethereum::Address(asset.token_contract.into());

    Htlc::new(
        expiry.into(),
        refund_address,
        redeem_address,
        secret_hash.into(),
        token_contract_address,
        asset.quantity.into(),
    )
}

#[cfg(feature = "quickcheck")]
impl quickcheck::Arbitrary for Params {
    fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
        Self {
            asset: asset::Erc20::arbitrary(g),
            redeem_identity: ethereum::Address::arbitrary(g),
            refund_identity: ethereum::Address::arbitrary(g),
            expiry: Timestamp::arbitrary(g),
            secret_hash: SecretHash::arbitrary(g),
            chain_id: ChainId::arbitrary(g),
        }
    }
}
