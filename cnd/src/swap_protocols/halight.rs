use crate::swap_protocols::{
    rfc003::{Secret, SecretHash},
    state,
    state::{Insert as _, Update as _},
    NodeLocalSwapId, SwapId,
};
use chrono::NaiveDateTime;
use futures::future::{self, Either};
use genawaiter::{
    sync::{Co, Gen},
    GeneratorState,
};
use std::{collections::HashMap, marker::PhantomData, sync::Arc};
use tokio::sync::Mutex;

/// Resolves when said event has occured.
#[async_trait::async_trait]
pub trait Opened<L, A, I> {
    async fn opened(&self, params: Params<L, A, I>) -> anyhow::Result<data::Opened>;
}

#[async_trait::async_trait]
pub trait Accepted<L, A, I> {
    async fn accepted(&self, params: Params<L, A, I>) -> anyhow::Result<data::Accepted>;
}

#[async_trait::async_trait]
pub trait Settled<L, A, I> {
    async fn settled(&self, params: Params<L, A, I>) -> anyhow::Result<data::Settled>;
}

#[async_trait::async_trait]
pub trait Cancelled<L, A, I> {
    async fn cancelled(&self, params: Params<L, A, I>) -> anyhow::Result<data::Cancelled>;
}

/// Represents states that an invoice can be in.
#[derive(Debug, Clone, Copy)]
pub enum State {
    // TODO: Think about this name - None, Unknown, NotDeployed
    Unknown,
    Opened(data::Opened),
    Accepted(data::Accepted),
    Settled(data::Settled),
    Cancelled(data::Cancelled),
}

/// Represents events that have occurred, transitioning to said state.
#[derive(Debug, Clone, Copy, PartialEq, strum_macros::Display)]
pub enum Event {
    Opened(data::Opened),
    Accepted(data::Accepted),
    Settled(data::Settled),
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
impl state::Insert<State> for InvoiceStates {
    async fn insert(&self, key: SwapId, value: State) {
        let mut states = self.states.lock().await;
        states.insert(key, value);
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
        let state = match states.get_mut(key) {
            Some(state) => state,
            None => {
                tracing::warn!("Value not found for key {}", key);
                return;
            }
        };

        match event {
            Event::Opened(opened) => state.transition_to_opened(opened),
            Event::Accepted(accepted) => state.transition_to_accepted(accepted),
            Event::Settled(settled) => state.transition_to_settled(settled),
            Event::Cancelled(cancelled) => state.transition_to_cancelled(cancelled),
        }
    }
}

pub async fn create_watcher<C, L, A, I>(
    lnd_connector: &C,
    invoice_states: Arc<InvoiceStates>,
    local_id: NodeLocalSwapId,
    params: Params<L, A, I>,
    finalized_at: NaiveDateTime,
) where
    // TODO: add FailedInsertSwap
    C: Opened<L, A, I> + Accepted<L, A, I> + Settled<L, A, I> + Cancelled<L, A, I>,
    L: Clone,
    A: Ord + Clone,
    I: Clone,
{
    let id = SwapId(local_id.0); // FIXME: Resolve this abuse.

    invoice_states.insert(id, State::Unknown).await;

    // construct a generator that watches alpha and beta ledger concurrently
    let mut generator = Gen::new({
        |co| async { watch_ledger::<C, L, A, I>(&lnd_connector, co, params, finalized_at).await }
    });

    loop {
        // wait for events to be emitted as the generator executes
        match generator.async_resume().await {
            // every event that is yielded is passed on
            GeneratorState::Yielded(event) => {
                tracing::info!("swap {} yielded event {}", id, event);
                invoice_states.update(&id, event).await;
            }
            // the generator stopped executing, this means there are no more events that can be
            // watched.
            GeneratorState::Complete(Ok(_)) => {
                tracing::info!("swap {} finished", id);
                return;
            }
            GeneratorState::Complete(Err(e)) => {
                tracing::error!("swap {} failed with {:?}", id, e);
                // TODO: Replace unimplemented with line below
                // facade.insert_failed_swap(&id);
            }
        }
    }
}

/// Returns a future that waits for events to happen on a ledger.
///
/// Each event is yielded through the controller handle (co) of the coroutine.
async fn watch_ledger<C, L, A, I>(
    lnd_connector: &C,
    co: Co<Event>,
    htlc_params: Params<L, A, I>,
    _start_of_swap: NaiveDateTime,
) -> anyhow::Result<()>
where
    C: Opened<L, A, I> + Accepted<L, A, I> + Settled<L, A, I> + Cancelled<L, A, I>,
    Params<L, A, I>: Clone,
{
    let opened = lnd_connector.opened(htlc_params.clone()).await?;
    co.yield_(Event::Opened(opened)).await;

    let accepted = lnd_connector.accepted(htlc_params.clone()).await?;
    co.yield_(Event::Accepted(accepted)).await;

    let settled = lnd_connector.settled(htlc_params.clone());

    let cancelled = lnd_connector.cancelled(htlc_params);

    match future::try_select(settled, cancelled).await {
        Ok(Either::Left((settled, _))) => {
            co.yield_(Event::Settled(settled)).await;
        }
        Ok(Either::Right((cancelled, _))) => {
            co.yield_(Event::Cancelled(cancelled)).await;
        }
        Err(either) => {
            let (error, _other_future) = either.factor_first();

            return Err(error);
        }
    }

    Ok(())
}

#[derive(Clone, Copy, Debug)]
pub struct Params<L, A, I> {
    pub phantom_data: PhantomData<(L, A, I)>,
    // TODO: these are all unused
    // pub asset: A,
    // pub ledger: L,
    // pub to_identity: I,
    // pub self_identity: I,
    // pub cltv_expiry: Timestamp,
    pub secret_hash: SecretHash,
}
