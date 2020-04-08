use crate::swap_protocols::{
    rfc003::{Secret, SecretHash},
    state, SwapId,
};
use chrono::NaiveDateTime;
use futures::{
    future::{self, Either},
    Stream, TryFutureExt,
};
use genawaiter::sync::Gen;
use std::collections::{hash_map::Entry, HashMap};
use tokio::sync::Mutex;

/// Resolves when said event has occured.
#[async_trait::async_trait]
pub trait Opened {
    async fn opened(&self, params: Params) -> anyhow::Result<data::Opened>;
}

#[async_trait::async_trait]
pub trait Accepted {
    async fn accepted(&self, params: Params) -> anyhow::Result<data::Accepted>;
}

#[async_trait::async_trait]
pub trait Settled {
    async fn settled(&self, params: Params) -> anyhow::Result<data::Settled>;
}

#[async_trait::async_trait]
pub trait Cancelled {
    async fn cancelled(&self, params: Params) -> anyhow::Result<data::Cancelled>;
}

/// Represents states that an invoice can be in.
#[derive(Debug, Clone, Copy)]
pub enum State {
    Unknown,
    Opened(data::Opened),
    Accepted(data::Accepted),
    Settled(data::Settled),
    Cancelled(data::Cancelled),
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
    Opened(data::Opened),

    /// The payment to the invoice was accepted but the preimage has not yet
    /// been revealed.
    ///
    /// On the recipient side, this means the hold invoice moved to the
    /// `Accepted` state. On the (payment) sender side, we assume that once
    /// the payment is `InFlight`, it also reached the recipient.
    Accepted(data::Accepted),

    /// The payment is settled and therefore the preimage was revealed.
    Settled(data::Settled),

    /// The payment was cancelled.
    Cancelled(data::Cancelled),
}

/// Represents the data available at said state.
pub mod data {
    // These empty types are useful because they give us additional type safety.
    use super::*;

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
}

#[derive(Default, Debug)]
pub struct InvoiceStates {
    states: Mutex<HashMap<SwapId, State>>,
}

impl State {
    pub fn transition_to_opened(&mut self, opened: data::Opened) {
        match std::mem::replace(self, State::Unknown) {
            State::Unknown => *self = State::Opened(opened),
            other => panic!("expected state Unknown, got {:?}", other),
        }
    }

    pub fn transition_to_accepted(&mut self, accepted: data::Accepted) {
        match std::mem::replace(self, State::Unknown) {
            State::Opened(_) => *self = State::Accepted(accepted),
            other => panic!("expected state Opened, got {:?}", other),
        }
    }

    pub fn transition_to_settled(&mut self, settled: data::Settled) {
        match std::mem::replace(self, State::Unknown) {
            State::Accepted(_) => *self = State::Settled(settled),
            other => panic!("expected state Accepted, got {:?}", other),
        }
    }

    pub fn transition_to_cancelled(&mut self, cancelled: data::Cancelled) {
        match std::mem::replace(self, State::Unknown) {
            // Alice cancels invoice before Bob has accepted it.
            State::Opened(_) => *self = State::Cancelled(cancelled),
            // Alice cancels invoice after Bob has accepted it.
            State::Accepted(_) => *self = State::Cancelled(cancelled),
            other => panic!("expected state Opened or Accepted, got {:?}", other),
        }
    }
}

#[async_trait::async_trait]
impl state::Get<State> for InvoiceStates {
    async fn get(&self, key: &SwapId) -> anyhow::Result<Option<State>> {
        let states = self.states.lock().await;
        let state = states.get(key).copied();

        Ok(state)
    }
}

#[async_trait::async_trait]
impl state::Update<Event> for InvoiceStates {
    async fn update(&self, key: &SwapId, event: Event) {
        let mut states = self.states.lock().await;
        let entry = states.entry(*key);

        match (event, entry) {
            (Event::Started, Entry::Vacant(vacant)) => {
                vacant.insert(State::Unknown);
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
pub fn new<'a, C>(
    lnd_connector: &'a C,
    params: Params,
    _finalized_at: NaiveDateTime,
) -> impl Stream<Item = anyhow::Result<Event>> + 'a
where
    C: Opened + Accepted + Settled + Cancelled,
{
    Gen::new({
        |co| async move {
            co.yield_(Ok(Event::Started)).await;

            let opened_or_error = lnd_connector
                .opened(params.clone())
                .map_ok(Event::Opened)
                .await;
            co.yield_(opened_or_error).await;

            let accepted_or_error = lnd_connector
                .accepted(params.clone())
                .map_ok(Event::Accepted)
                .await;
            co.yield_(accepted_or_error).await;

            let settled = lnd_connector.settled(params.clone());
            let cancelled = lnd_connector.cancelled(params);

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
