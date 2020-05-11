mod connector_impls;

pub use connector_impls::*;

use crate::{
    asset,
    ethereum::Bytes,
    htlc_location, identity,
    swap_protocols::{
        rfc003::{Secret, SecretHash},
        state, Ledger, LocalSwapId,
    },
    timestamp::Timestamp,
    transaction,
};
use chrono::NaiveDateTime;
use futures::{
    future::{self, Either},
    Stream,
};
use genawaiter::sync::{Co, Gen};
use std::collections::{hash_map::Entry, HashMap};
use tokio::sync::Mutex;

use blockchain_contracts::ethereum::rfc003::Erc20Htlc;

/// Htlc ERC20 Token atomic swap protocol.

/// Data required to create a swap that involves an ERC20 token.
#[derive(Clone, Debug, PartialEq)]
pub struct CreatedSwap {
    pub amount: asset::Erc20Quantity,
    pub identity: identity::Ethereum,
    pub chain_id: u32,
    pub token_contract: identity::Ethereum,
    pub absolute_expiry: u32,
}

/// Herc20 specific data for an in progress swap.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InProgressSwap {
    pub ledger: Ledger,
    pub refund_identity: identity::Ethereum,
    pub redeem_identity: identity::Ethereum,
    pub expiry: Timestamp, // This is the absolute_expiry for now.
}

/// Resolves when said event has occurred.
#[async_trait::async_trait]
pub trait WaitForDeployed {
    async fn wait_for_deployed(&self, params: Params) -> anyhow::Result<Deployed>;
}

#[async_trait::async_trait]
pub trait WaitForFunded {
    async fn wait_for_funded(&self, params: Params, deployed: Deployed) -> anyhow::Result<Funded>;
}

#[async_trait::async_trait]
pub trait WaitForRedeemed {
    async fn wait_for_redeemed(
        &self,
        params: Params,
        deployed: Deployed,
    ) -> anyhow::Result<Redeemed>;
}

#[async_trait::async_trait]
pub trait WaitForRefunded {
    async fn wait_for_refunded(
        &self,
        params: Params,
        deployed: Deployed,
    ) -> anyhow::Result<Refunded>;
}

/// Represents states that an ERC20 HTLC can be in.
#[derive(Debug, Clone)]
pub enum State {
    None,
    Deployed(Deployed),
    Funded(Funded),
    Redeemed(Redeemed),
    Refunded(Refunded),
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

#[derive(Default, Debug)]
pub struct States(Mutex<HashMap<LocalSwapId, State>>);

impl State {
    pub fn transition_to_deployed(&mut self, deployed: Deployed) {
        match std::mem::replace(self, State::None) {
            State::None => *self = State::Deployed(deployed),
            other => panic!("expected state None, got {:?}", other),
        }
    }

    pub fn transition_to_funded(&mut self, funded: Funded) {
        match std::mem::replace(self, State::None) {
            State::Deployed(_) => *self = State::Funded(funded),
            other => panic!("expected state Deployed, got {:?}", other),
        }
    }

    pub fn transition_to_redeemed(&mut self, redeemed: Redeemed) {
        match std::mem::replace(self, State::None) {
            State::Funded(_) => *self = State::Redeemed(redeemed),
            other => panic!("expected state Funded, got {:?}", other),
        }
    }

    pub fn transition_to_refunded(&mut self, refunded: Refunded) {
        match std::mem::replace(self, State::None) {
            State::Funded(_) => *self = State::Refunded(refunded),
            other => panic!("expected state Funded, got {:?}", other),
        }
    }
}

#[async_trait::async_trait]
impl state::Get<State> for States {
    async fn get(&self, key: &LocalSwapId) -> anyhow::Result<Option<State>> {
        let states = self.0.lock().await;
        let state = states.get(key).cloned();

        Ok(state)
    }
}

#[async_trait::async_trait]
impl state::Update<Event> for States {
    async fn update(&self, key: &LocalSwapId, event: Event) {
        let mut states = self.0.lock().await;
        let entry = states.entry(*key);

        match (event, entry) {
            (Event::Started, Entry::Vacant(vacant)) => {
                vacant.insert(State::None);
            }
            (Event::Deployed(deployed), Entry::Occupied(mut state)) => {
                state.get_mut().transition_to_deployed(deployed)
            }
            (Event::Funded(funded), Entry::Occupied(mut state)) => {
                state.get_mut().transition_to_funded(funded)
            }
            (Event::Redeemed(redeemed), Entry::Occupied(mut state)) => {
                state.get_mut().transition_to_redeemed(redeemed)
            }
            (Event::Refunded(refunded), Entry::Occupied(mut state)) => {
                state.get_mut().transition_to_refunded(refunded)
            }
            (Event::Started, Entry::Occupied(_)) => {
                tracing::warn!(
                    "Received Started event for {} although state is already present",
                    key
                );
            }
            (_, Entry::Vacant(_)) => {
                tracing::warn!("State not found for {}", key);
            }
        }
    }
}

/// Creates a new instance of the herc20 protocol.
///
/// Returns a stream of events happening during the execution.
pub fn new<'a, C>(
    connector: &'a C,
    params: Params,
) -> impl Stream<Item = anyhow::Result<Event>> + 'a
where
    C: WaitForDeployed + WaitForFunded + WaitForRedeemed + WaitForRefunded,
{
    Gen::new({
        |co| async move {
            if let Err(error) = watch_ledger(connector, params, &co).await {
                co.yield_(Err(error)).await;
            }
        }
    })
}

async fn watch_ledger<C, R>(
    connector: &C,
    params: Params,
    co: &Co<anyhow::Result<Event>, R>,
) -> anyhow::Result<()>
where
    C: WaitForDeployed + WaitForFunded + WaitForRedeemed + WaitForRefunded,
{
    co.yield_(Ok(Event::Started)).await;

    let deployed = connector.wait_for_deployed(params.clone()).await?;

    co.yield_(Ok(Event::Deployed(deployed.clone()))).await;

    let funded = connector
        .wait_for_funded(params.clone(), deployed.clone())
        .await?;
    co.yield_(Ok(Event::Funded(funded))).await;

    let redeemed = connector.wait_for_redeemed(params.clone(), deployed.clone());
    let refunded = connector.wait_for_refunded(params, deployed);

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

#[derive(Clone, Debug)]
pub struct Params {
    pub asset: asset::Erc20,
    pub redeem_identity: identity::Ethereum,
    pub refund_identity: identity::Ethereum,
    pub expiry: Timestamp,
    pub start_of_swap: NaiveDateTime,
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
