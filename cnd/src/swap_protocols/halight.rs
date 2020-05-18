use crate::{
    asset, identity,
    swap_protocols::{ledger, state, state::Update, Ledger, LocalSwapId},
    timestamp::Timestamp,
};
use futures::TryStreamExt;
use std::{
    collections::{hash_map::Entry, HashMap},
    sync::Arc,
};
use tokio::sync::Mutex;

pub use comit::halight::*;

/// Htlc Lightning Bitcoin atomic swap protocol.

/// Data required to create a swap that involves bitcoin on the lightning
/// network.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CreatedSwap {
    pub amount: asset::Bitcoin,
    pub identity: identity::Lightning,
    pub network: ledger::Lightning,
    pub cltv_expiry: u32,
}

/// Halight specific data for an in progress swap.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InProgressSwap {
    pub ledger: Ledger,
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
    params: Params,
    state_store: Arc<States>,
    connector: C,
) where
    C: WaitForOpened + WaitForAccepted + WaitForSettled + WaitForCancelled,
{
    let mut events = new(&connector, params)
        .inspect_ok(|event| tracing::info!("yielded event {}", event))
        .inspect_err(|error| tracing::error!("swap failed with {:?}", error));

    while let Ok(Some(event)) = events.try_next().await {
        state_store.update(&id, event).await;
    }

    tracing::info!("swap finished");
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
