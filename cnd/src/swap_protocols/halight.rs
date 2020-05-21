use crate::{
    swap_protocols::{state, state::Update, LocalSwapId},
    tracing_ext::InstrumentProtocol,
};
pub use comit::{halight::*, identity};
use comit::{Protocol, Role, Side};
use futures::TryStreamExt;
use std::{
    collections::{hash_map::Entry, HashMap},
    sync::Arc,
};
use tokio::sync::Mutex;

/// HTLC Lightning Bitcoin atomic swap protocol.

/// Creates a new instance of the halight protocol.
///
/// This wrapper functions allows us to reuse code within `cnd` without having
/// to give knowledge about tracing or the state hashmaps to the `comit` crate.
pub async fn new<C>(
    id: LocalSwapId,
    params: Params,
    role: Role,
    side: Side,
    states: Arc<States>,
    connector: C,
) where
    C: WaitForOpened + WaitForAccepted + WaitForSettled + WaitForCancelled,
{
    let mut events = comit::halight::new(&connector, params)
        .instrument_protocol(id, role, side, Protocol::Halight)
        .inspect_ok(|event| tracing::info!("yielded event {}", event))
        .inspect_err(|error| tracing::error!("swap failed with {:?}", error));

    while let Ok(Some(event)) = events.try_next().await {
        states.update(&id, event).await;
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

#[derive(Copy, Clone, Debug)]
pub struct Identities {
    pub redeem_identity: identity::Lightning,
    pub refund_identity: identity::Lightning,
}
