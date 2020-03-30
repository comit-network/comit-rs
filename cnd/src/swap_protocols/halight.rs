use crate::{
    swap_protocols::{
        rfc003::{Secret, SecretHash},
        state,
        state::{Insert as _, Update as _},
        SwapId,
    },
    timestamp::Timestamp,
};
use chrono::NaiveDateTime;
use futures::future::{self, Either};
use genawaiter::{
    sync::{Co, Gen},
    GeneratorState,
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

#[async_trait::async_trait]
pub trait InvoiceAdded<L, A, I> {
    async fn invoice_added(&self, params: Params<L, A, I>) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
pub trait InvoiceAccepted<L, A, I> {
    async fn invoice_accepted(&self, params: Params<L, A, I>) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
pub trait InvoiceSettled<L, A, I> {
    async fn invoice_settled(&self, params: Params<L, A, I>) -> anyhow::Result<Settled>;
}

#[async_trait::async_trait]
pub trait InvoiceCancelled<L, A, I> {
    async fn invoice_cancelled(&self, params: Params<L, A, I>) -> anyhow::Result<()>;
}

/// Represents states that an invoice can be in.
#[derive(Debug, Clone, Copy)]
pub enum InvoiceState {
    None,
    Added,
    Accepted,
    Settled(Settled),
    Cancelled,
}

/// Represents events that have occurred, transitioning the state.
#[derive(Debug, Clone, Copy, PartialEq, strum_macros::Display)]
pub enum Event {
    Added,
    Accepted,
    Settled(Settled),
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settled {
    pub secret: Secret,
}

#[derive(Default, Debug)]
pub struct InvoiceStates {
    states: Mutex<HashMap<SwapId, InvoiceState>>,
}

impl InvoiceState {
    pub fn transition_to_opened(&mut self) {
        match std::mem::replace(self, InvoiceState::None) {
            InvoiceState::None => *self = InvoiceState::Added,
            other => panic!("expected state None, got {:?}", other),
        }
    }

    pub fn transition_to_accepted(&mut self) {
        match std::mem::replace(self, InvoiceState::None) {
            InvoiceState::Added => *self = InvoiceState::Accepted,
            other => panic!("expected state Added, got {:?}", other),
        }
    }

    pub fn transition_to_settled(&mut self, settled: Settled) {
        match std::mem::replace(self, InvoiceState::None) {
            InvoiceState::Accepted => *self = InvoiceState::Settled(settled),
            other => panic!("expected state Accepted, got {:?}", other),
        }
    }

    pub fn transition_to_cancelled(&mut self) {
        match std::mem::replace(self, InvoiceState::None) {
            // Alice cancels invoice before Bob has accepted it.
            InvoiceState::Added => *self = InvoiceState::Accepted,
            // Alice cancels invoice after Bob has accepted it.
            InvoiceState::Accepted => *self = InvoiceState::Cancelled,
            other => panic!("expected state Added or Accepted, got {:?}", other),
        }
    }
}

#[async_trait::async_trait]
impl state::Insert<InvoiceState> for InvoiceStates {
    async fn insert(&self, key: SwapId, value: InvoiceState) {
        let mut states = self.states.lock().await;
        states.insert(key, value);
    }
}

#[async_trait::async_trait]
impl state::Get<InvoiceState> for InvoiceStates {
    async fn get(&self, key: &SwapId) -> anyhow::Result<Option<InvoiceState>> {
        let states = self.states.lock().await;
        let state = states.get(key).map(|s| *s);
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
            Event::Added => state.transition_to_opened(),
            Event::Accepted => state.transition_to_accepted(),
            Event::Settled(settled) => state.transition_to_settled(settled),
            Event::Cancelled => state.transition_to_cancelled(),
        }
    }
}

pub async fn create_watcher<C, L, A, I>(
    lnd_connector: &C,
    invoice_states: Arc<InvoiceStates>,
    id: SwapId,
    params: Params<L, A, I>,
    finalized_at: NaiveDateTime,
) where
    // TODO: add FailedInsertSwap
    C: InvoiceAdded<L, A, I>
        + InvoiceAccepted<L, A, I>
        + InvoiceSettled<L, A, I>
        + InvoiceCancelled<L, A, I>,
    L: Clone,
    A: Ord + Clone,
    I: Clone,
{
    invoice_states.insert(id, InvoiceState::Added).await;

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
                unimplemented!();
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
    C: InvoiceAdded<L, A, I>
        + InvoiceAccepted<L, A, I>
        + InvoiceSettled<L, A, I>
        + InvoiceCancelled<L, A, I>,
    Params<L, A, I>: Clone,
{
    lnd_connector.invoice_added(htlc_params.clone()).await?;
    co.yield_(Event::Added).await;

    lnd_connector.invoice_accepted(htlc_params.clone()).await?;
    co.yield_(Event::Accepted).await;

    let settled = lnd_connector.invoice_settled(htlc_params.clone());

    let cancelled = lnd_connector.invoice_cancelled(htlc_params);

    match future::try_select(settled, cancelled).await {
        Ok(Either::Left((settled, _))) => {
            co.yield_(Event::Settled(settled.clone())).await;
        }
        Ok(Either::Right((_cancelled, _))) => {
            co.yield_(Event::Cancelled).await;
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
    pub asset: A,
    pub ledger: L,
    pub redeem_identity: I,
    pub refund_identity: I,
    pub expiry: Timestamp,
    pub secret_hash: SecretHash,
}
