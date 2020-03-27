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
use std::{any::Any, collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

#[async_trait::async_trait]
pub trait InvoiceOpened<L, A, I> {
    async fn invoice_opened(&self, params: Params<L, A, I>) -> anyhow::Result<Opened>;
}

#[async_trait::async_trait]
pub trait InvoiceAccepted<L, A, I> {
    async fn invoice_accepted(&self, params: Params<L, A, I>) -> anyhow::Result<Accepted>;
}

#[async_trait::async_trait]
pub trait InvoiceSettled<L, A, I> {
    async fn invoice_settled(&self, params: Params<L, A, I>) -> anyhow::Result<Settled>;
}

#[async_trait::async_trait]
pub trait InvoiceCancelled<L, A, I> {
    async fn invoice_cancelled(&self, params: Params<L, A, I>) -> anyhow::Result<Cancelled>;
}

#[derive(Debug, Clone, Copy)]
pub enum InvoiceState {
    None,
    Opened,
    Accepted,
    Settled,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, strum_macros::Display)]
pub enum Event {
    Opened(Opened),
    Accepted(Accepted),
    Settled(Settled),
    Cancelled(Cancelled),
}

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
pub struct InvoiceStates {
    states: Mutex<HashMap<SwapId, Box<dyn Any + Send>>>,
}

impl InvoiceState {
    pub fn transition_to_opened(&mut self, _opened: Opened) {
        unimplemented!()
    }
    pub fn transition_to_accepted(&mut self, _accepted: Accepted) {
        unimplemented!()
    }
    pub fn transition_to_settled(&mut self, _settled: Settled) {
        unimplemented!()
    }
    pub fn transition_to_cancelled(&mut self, _cancelled: Cancelled) {
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl<S> state::Insert<S> for InvoiceStates
where
    S: Send + 'static,
{
    async fn insert(&self, key: SwapId, value: S) {
        let mut states = self.states.lock().await;
        states.insert(key, Box::new(value));
    }
}

#[async_trait::async_trait]
impl state::Get<InvoiceState> for InvoiceStates {
    async fn get(&self, _key: &SwapId) -> anyhow::Result<Option<InvoiceState>> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl state::Update<Event> for InvoiceStates {
    async fn update(&self, key: &SwapId, event: Event) {
        let mut states = self.states.lock().await;
        let state = match states
            .get_mut(key)
            .and_then(|state| state.downcast_mut::<InvoiceState>())
        {
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
    id: SwapId,
    params: Params<L, A, I>,
    finalized_at: NaiveDateTime,
) where
    // TODO: add FailedInsertSwap
    C: InvoiceOpened<L, A, I>
        + InvoiceAccepted<L, A, I>
        + InvoiceSettled<L, A, I>
        + InvoiceCancelled<L, A, I>,
    L: Clone,
    A: Ord + Clone,
    I: Clone,
{
    invoice_states.insert(id, InvoiceState::Opened).await;

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
    C: InvoiceOpened<L, A, I>
        + InvoiceAccepted<L, A, I>
        + InvoiceSettled<L, A, I>
        + InvoiceCancelled<L, A, I>,
    Params<L, A, I>: Clone,
{
    let opened = lnd_connector.invoice_opened(htlc_params.clone()).await?;
    co.yield_(Event::Opened(opened.clone())).await;

    let accepted = lnd_connector.invoice_accepted(htlc_params.clone()).await?;
    co.yield_(Event::Accepted(accepted)).await;

    let settled = lnd_connector.invoice_settled(htlc_params.clone());

    let cancelled = lnd_connector.invoice_cancelled(htlc_params);

    match future::try_select(settled, cancelled).await {
        Ok(Either::Left((settled, _))) => {
            co.yield_(Event::Settled(settled.clone())).await;
        }
        Ok(Either::Right((cancelled, _))) => {
            co.yield_(Event::Cancelled(cancelled.clone())).await;
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
