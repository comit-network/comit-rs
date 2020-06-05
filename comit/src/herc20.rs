//! Htlc ERC20 Token atomic swap protocol.

use crate::{
    asset,
    asset::{ethereum::FromWei, Erc20, Erc20Quantity},
    btsieve::{
        ethereum::{watch_for_contract_creation, watch_for_event, ReceiptByHash, Topic},
        BlockByHash, LatestBlock,
    },
    ethereum::{Block, Bytes, ChainId, Hash, U256},
    htlc_location, identity,
    timestamp::Timestamp,
    transaction, Secret, SecretHash,
};
use blockchain_contracts::ethereum::rfc003::Erc20Htlc;
use chrono::NaiveDateTime;
use futures::{
    future::{self, Either},
    Stream,
};
use genawaiter::sync::{Co, Gen};
use std::cmp::Ordering;
use tracing_futures::Instrument;

lazy_static::lazy_static! {
    static ref REDEEM_LOG_MSG: Hash = blockchain_contracts::ethereum::rfc003::REDEEMED_LOG_MSG.parse().expect("to be valid hex");
    static ref REFUND_LOG_MSG: Hash = blockchain_contracts::ethereum::rfc003::REFUNDED_LOG_MSG.parse().expect("to be valid hex");
    static ref TRANSFER_LOG_MSG: Hash = blockchain_contracts::ethereum::rfc003::ERC20_TRANSFER.parse().expect("to be valid hex");
}

/// Data required to create a swap that involves an ERC20 token.
#[derive(Clone, Debug, PartialEq)]
pub struct CreatedSwap {
    pub asset: asset::Erc20,
    pub identity: identity::Ethereum,
    pub chain_id: ChainId,
    pub absolute_expiry: u32,
}

/// Represents the events in the herc20 protocol.
#[derive(Debug, Clone, PartialEq, strum_macros::Display)]
pub enum Event {
    /// The protocol was started.
    Started,

    /// The HTLC was deployed and is pending funding.
    Deployed(Deployed),

    /// The HTLC has been funded with ERC20 tokens.
    Funded(Funded),

    /// The HTLC has been destroyed via the redeem path, token have been sent to
    /// the redeemer.
    Redeemed(Redeemed),

    /// The HTLC has been destroyed via the refund path, token has been sent
    /// back to funder.
    Refunded(Refunded),
}

/// Represents the data available at said state.
#[derive(Debug, Clone, PartialEq)]
pub struct Deployed {
    pub transaction: transaction::Ethereum,
    pub location: htlc_location::Ethereum,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Funded {
    Correctly {
        transaction: transaction::Ethereum,
        asset: asset::Erc20,
    },
    Incorrectly {
        transaction: transaction::Ethereum,
        asset: asset::Erc20,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Redeemed {
    pub transaction: transaction::Ethereum,
    pub secret: Secret,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Refunded {
    pub transaction: transaction::Ethereum,
}

/// Creates a new instance of the herc20 protocol.
///
/// Returns a stream of events happening during the execution.
pub fn new<'a, C>(
    connector: &'a C,
    params: Params,
    start_of_swap: NaiveDateTime,
) -> impl Stream<Item = anyhow::Result<Event>> + 'a
where
    C: LatestBlock<Block = Block> + BlockByHash<Block = Block, BlockHash = Hash> + ReceiptByHash,
{
    Gen::new({
        |co| async move {
            if let Err(error) = watch_ledger(connector, params, start_of_swap, &co).await {
                co.yield_(Err(error)).await;
            }
        }
    })
}

async fn watch_ledger<C, R>(
    connector: &C,
    params: Params,
    start_of_swap: NaiveDateTime,
    co: &Co<anyhow::Result<Event>, R>,
) -> anyhow::Result<()>
where
    C: LatestBlock<Block = Block> + BlockByHash<Block = Block, BlockHash = Hash> + ReceiptByHash,
{
    co.yield_(Ok(Event::Started)).await;

    let deployed = watch_for_deployed(connector, params.clone(), start_of_swap).await?;
    co.yield_(Ok(Event::Deployed(deployed.clone()))).await;

    let funded =
        watch_for_funded(connector, params.clone(), start_of_swap, deployed.clone()).await?;
    co.yield_(Ok(Event::Funded(funded))).await;

    let redeemed = watch_for_redeemed(connector, start_of_swap, deployed.clone());
    let refunded = watch_for_refunded(connector, start_of_swap, deployed);

    futures::pin_mut!(redeemed);
    futures::pin_mut!(refunded);

    match future::try_select(redeemed, refunded).await {
        Ok(Either::Left((redeemed, _))) => {
            co.yield_(Ok(Event::Redeemed(redeemed))).await;
        }
        Ok(Either::Right((refunded, _))) => {
            co.yield_(Ok(Event::Refunded(refunded))).await;
        }
        Err(either) => {
            let (error, _other_future) = either.factor_first();
            return Err(error);
        }
    }

    Ok(())
}

async fn watch_for_deployed<C>(
    connector: &C,
    params: Params,
    start_of_swap: NaiveDateTime,
) -> anyhow::Result<Deployed>
where
    C: LatestBlock<Block = Block> + BlockByHash<Block = Block, BlockHash = Hash> + ReceiptByHash,
{
    let expected_bytecode = params.clone().bytecode();

    let (transaction, location) =
        watch_for_contract_creation(connector, start_of_swap, &expected_bytecode)
            .instrument(tracing::trace_span!(
                "watch_deploy",
                expected_bytecode = %hex::encode(&expected_bytecode.0)
            ))
            .await?;

    Ok(Deployed {
        transaction,
        location,
    })
}

async fn watch_for_funded<C>(
    connector: &C,
    params: Params,
    start_of_swap: NaiveDateTime,
    deployed: Deployed,
) -> anyhow::Result<Funded>
where
    C: LatestBlock<Block = Block> + BlockByHash<Block = Block, BlockHash = Hash> + ReceiptByHash,
{
    use crate::btsieve::ethereum::Event;

    let event = Event {
        address: params.asset.token_contract,
        topics: vec![
            Some(Topic(*TRANSFER_LOG_MSG)),
            None,
            Some(Topic(deployed.location.into())),
        ],
    };

    let (transaction, log) = watch_for_event(connector, start_of_swap, event)
        .instrument(tracing::trace_span!("watch_fund"))
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

async fn watch_for_redeemed<C>(
    connector: &C,
    start_of_swap: NaiveDateTime,
    deployed: Deployed,
) -> anyhow::Result<Redeemed>
where
    C: LatestBlock<Block = Block> + BlockByHash<Block = Block, BlockHash = Hash> + ReceiptByHash,
{
    use crate::btsieve::ethereum::Event;

    let event = Event {
        address: deployed.location,
        topics: vec![Some(Topic(*REDEEM_LOG_MSG))],
    };

    let (transaction, log) = watch_for_event(connector, start_of_swap, event)
        .instrument(tracing::info_span!("watch_redeem"))
        .await?;

    let log_data = log.data.0.as_ref();
    let secret =
        Secret::from_vec(log_data).expect("Must be able to construct secret from log data");

    Ok(Redeemed {
        transaction,
        secret,
    })
}

async fn watch_for_refunded<C>(
    connector: &C,
    start_of_swap: NaiveDateTime,
    deployed: Deployed,
) -> anyhow::Result<Refunded>
where
    C: LatestBlock<Block = Block> + BlockByHash<Block = Block, BlockHash = Hash> + ReceiptByHash,
{
    use crate::btsieve::ethereum::Event;

    let event = Event {
        address: deployed.location,
        topics: vec![Some(Topic(*REFUND_LOG_MSG))],
    };

    let (transaction, _) = watch_for_event(connector, start_of_swap, event)
        .instrument(tracing::info_span!("watch_refund"))
        .await?;

    Ok(Refunded { transaction })
}

#[derive(Clone, Debug)]
pub struct Params {
    pub asset: asset::Erc20,
    pub redeem_identity: identity::Ethereum,
    pub refund_identity: identity::Ethereum,
    pub expiry: Timestamp,
    pub secret_hash: SecretHash,
}

impl Params {
    pub fn bytecode(&self) -> Bytes {
        Erc20Htlc::from(self.clone()).into()
    }
}

impl From<Params> for Erc20Htlc {
    fn from(params: Params) -> Self {
        let refund_address = blockchain_contracts::ethereum::Address(params.refund_identity.into());
        let redeem_address = blockchain_contracts::ethereum::Address(params.redeem_identity.into());
        let token_contract_address =
            blockchain_contracts::ethereum::Address(params.asset.token_contract.into());

        Erc20Htlc::new(
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
) -> Erc20Htlc {
    let refund_address = blockchain_contracts::ethereum::Address(refund_identity.into());
    let redeem_address = blockchain_contracts::ethereum::Address(redeem_identity.into());
    let token_contract_address =
        blockchain_contracts::ethereum::Address(asset.token_contract.into());

    Erc20Htlc::new(
        expiry.into(),
        refund_address,
        redeem_address,
        secret_hash.into(),
        token_contract_address,
        asset.quantity.into(),
    )
}
