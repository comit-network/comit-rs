use crate::{
    asset, identity,
    swap_protocols::{
        ledger::lightning,
        rfc003::{Secret, SecretHash},
        state,
        state::Update,
        LocalSwapId,
    },
    timestamp::Timestamp,
};
use futures::{
    future::{self, Either},
    Stream, TryFutureExt, TryStreamExt,
};
use genawaiter::sync::Gen;
use std::{
    collections::{hash_map::Entry, HashMap},
    sync::Arc,
};
use tokio::sync::Mutex;

mod connector;

pub use connector::*;

/// Htlc Lightning Bitcoin atomic swap protocol.

/// Data required to create a swap that involves bitcoin on the lightning
/// network.
#[derive(Clone, Debug)]
pub struct CreatedSwap {
    pub amount: asset::Bitcoin,
    pub identity: identity::Lightning,
    pub network: String,
    pub cltv_expiry: u32,
}

/// Halight specific data for an in progress swap.
#[derive(Debug, Clone, Copy)]
pub struct InProgressSwap {
    pub ledger: lightning::Regtest,
    pub asset: asset::Bitcoin,
    pub refund_identity: identity::Lightning,
    pub redeem_identity: identity::Lightning,
    pub expiry: Timestamp, // This is the cltv_expiry for now.
}

/// Creates a new instance of the halight protocol.
///
/// This function delegates to the `new` function for the actual protocol
/// implementation. Its main purpose is to annotate the protocol instance with
/// logging information and store the events yielded by the protocol.
pub async fn new_halight_swap<C>(
    id: LocalSwapId,
    secret_hash: SecretHash,
    state_store: Arc<States>,
    connector: C,
) where
    C: WaitForOpened + WaitForAccepted + WaitForSettled + WaitForCancelled,
{
    let mut events = new(&connector, Params { secret_hash })
        .inspect_ok(|event| tracing::info!("yielded event {}", event))
        .inspect_err(|error| tracing::error!("swap failed with {:?}", error));

    while let Ok(Some(event)) = events.try_next().await {
        state_store.update(&id, event).await;
    }

    tracing::info!("swap finished");
}

/// Resolves when said event has occured.
#[async_trait::async_trait]
pub trait WaitForOpened {
    async fn wait_for_opened(&self, params: Params) -> anyhow::Result<Opened>;
}

#[async_trait::async_trait]
pub trait WaitForAccepted {
    async fn wait_for_accepted(&self, params: Params) -> anyhow::Result<Accepted>;
}

#[async_trait::async_trait]
pub trait WaitForSettled {
    async fn wait_for_settled(&self, params: Params) -> anyhow::Result<Settled>;
}

#[async_trait::async_trait]
pub trait WaitForCancelled {
    async fn wait_for_cancelled(&self, params: Params) -> anyhow::Result<Cancelled>;
}

/// Represents states that an invoice can be in.
#[derive(Debug, Clone, Copy)]
pub enum State {
    None,
    Opened(Opened),
    Accepted(Accepted),
    Settled(Settled),
    Cancelled(Cancelled),
}

/// Represents the events in the halight protocol.
#[derive(Debug, Clone, Copy, PartialEq, strum_macros::Display)]
pub enum Event {
    /// The halight protocol was started.
    Started,

    /// The invoice was opened and is ready to accept a payment.
    ///
    /// On the recipient side, this means the hold invoice has been added to
    /// lnd. On the (payment) sender side, we cannot (yet) know about this
    /// so we just have to assume that this happens.
    Opened(Opened),

    /// The payment to the invoice was accepted but the preimage has not yet
    /// been revealed.
    ///
    /// On the recipient side, this means the hold invoice moved to the
    /// `Accepted` state. On the (payment) sender side, we assume that once
    /// the payment is `InFlight`, it also reached the recipient.
    Accepted(Accepted),

    /// The payment is settled and therefore the preimage was revealed.
    Settled(Settled),

    /// The payment was cancelled.
    Cancelled(Cancelled),
}

/// Represents the data available at said state.
///
/// These empty types are useful because they give us additional type safety.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Opened;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Accepted;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settled {
    pub secret: Secret,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cancelled;

#[derive(Default, Debug)]
pub struct States(Mutex<HashMap<LocalSwapId, State>>);

impl State {
    pub fn transition_to_opened(&mut self, opened: Opened) {
        match std::mem::replace(self, State::None) {
            State::None => *self = State::Opened(opened),
            other => panic!("expected state Unknown, got {:?}", other),
        }
    }

    pub fn transition_to_accepted(&mut self, accepted: Accepted) {
        match std::mem::replace(self, State::None) {
            State::Opened(_) => *self = State::Accepted(accepted),
            other => panic!("expected state Opened, got {:?}", other),
        }
    }

    pub fn transition_to_settled(&mut self, settled: Settled) {
        match std::mem::replace(self, State::None) {
            State::Accepted(_) => *self = State::Settled(settled),
            other => panic!("expected state Accepted, got {:?}", other),
        }
    }

    pub fn transition_to_cancelled(&mut self, cancelled: Cancelled) {
        match std::mem::replace(self, State::None) {
            // Alice cancels invoice before Bob has accepted it.
            State::Opened(_) => *self = State::Cancelled(cancelled),
            // Alice cancels invoice after Bob has accepted it.
            State::Accepted(_) => *self = State::Cancelled(cancelled),
            other => panic!("expected state Opened or Accepted, got {:?}", other),
        }
    }
}

#[async_trait::async_trait]
impl state::Get<State> for States {
    async fn get(&self, key: &LocalSwapId) -> anyhow::Result<Option<State>> {
        let states = self.0.lock().await;
        let state = states.get(key).copied();

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
            (Event::Opened(opened), Entry::Occupied(mut state)) => {
                state.get_mut().transition_to_opened(opened)
            }
            (Event::Accepted(accepted), Entry::Occupied(mut state)) => {
                state.get_mut().transition_to_accepted(accepted)
            }
            (Event::Settled(settled), Entry::Occupied(mut state)) => {
                state.get_mut().transition_to_settled(settled)
            }
            (Event::Cancelled(cancelled), Entry::Occupied(mut state)) => {
                state.get_mut().transition_to_cancelled(cancelled)
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

/// Creates a new instance of the halight protocol.
///
/// Returns a stream of events happening during the execution.
fn new<'a, C>(connector: &'a C, params: Params) -> impl Stream<Item = anyhow::Result<Event>> + 'a
where
    C: WaitForOpened + WaitForAccepted + WaitForSettled + WaitForCancelled,
{
    Gen::new({
        |co| async move {
            co.yield_(Ok(Event::Started)).await;

            let opened_or_error = connector
                .wait_for_opened(params.clone())
                .map_ok(Event::Opened)
                .await;
            co.yield_(opened_or_error).await;

            let accepted_or_error = connector
                .wait_for_accepted(params.clone())
                .map_ok(Event::Accepted)
                .await;
            co.yield_(accepted_or_error).await;

            let settled = connector.wait_for_settled(params.clone());
            let cancelled = connector.wait_for_cancelled(params);

            match future::try_select(settled, cancelled).await {
                Ok(Either::Left((settled, _))) => {
                    co.yield_(Ok(Event::Settled(settled))).await;
                }
                Ok(Either::Right((cancelled, _))) => {
                    co.yield_(Ok(Event::Cancelled(cancelled))).await;
                }
                Err(either) => {
                    let (error, _other_future) = either.factor_first();

                    co.yield_(Err(error)).await;
                }
            }
        }
    })
}

#[derive(Clone, Copy, Debug)]
pub struct Params {
    pub secret_hash: SecretHash,
}
