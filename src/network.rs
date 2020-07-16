use crate::order::BtcDaiOrder;
use comit::network::{Comit, RemoteData};
use comit::LocalSwapId;
use libp2p::swarm::{NetworkBehaviourAction, PollParameters};
use libp2p::{NetworkBehaviour, PeerId};
use std::collections::VecDeque;
use std::task::{Context, Poll};

#[derive(Debug)]
pub enum Event {
    // The orderbook registered that somebody wants to take a specific order.
    // We assume that this event is emitted by the orderbook.
    TakeRequest(Order), // Emitted by orderbook

    // Message from the network that is emitted at the end of the announce protocol when the
    // swap is finalized.
    SwapFinalized(LocalSwapId, RemoteData),
}

// TODO: Replace this placeholder with something meaningful
// Representation of an Order - implement mapping/replace with orderbook order
// once we integrate the orderbook.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Orderbook;

impl Orderbook {
    pub fn publish(&mut self, _order: PublishOrder) {
        // TODO
    }

    pub fn take(&self, _order: Order) {
        // TODO
    }

    pub fn ignore(&self, _order: Order) {
        // TODO: does not take the order, close the channel with the taker
    }
}

// TODO: Change this, currently placeholder ot the Order as represented in the orderbook
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Order {
    pub inner: BtcDaiOrder,
    pub taker: Taker,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Taker {
    peer_id: PeerId,
}

impl Taker {
    pub fn new(peer_id: PeerId) -> Taker {
        Taker { peer_id }
    }

    pub fn peer_id(&self) -> PeerId {
        self.peer_id.clone()
    }
}

impl From<Order> for BtcDaiOrder {
    fn from(order: Order) -> Self {
        order.inner
    }
}

#[cfg(test)]
impl Default for Taker {
    fn default() -> Self {
        Taker {
            peer_id: PeerId::random(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PublishOrder(BtcDaiOrder);

impl From<BtcDaiOrder> for PublishOrder {
    fn from(order: BtcDaiOrder) -> Self {
        Self(order)
    }
}

#[cfg(test)]
impl Default for Order {
    fn default() -> Self {
        Self {
            inner: BtcDaiOrder::default(),
            taker: Taker::default(),
        }
    }
}

/// A `NetworkBehaviour` that delegates to the `Announce`, `Orderbook`, `Take` and `Comit` (execution) behaviours.
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "Event", poll_method = "poll")]
#[allow(missing_debug_implementations)]
pub struct Nectar {
    pub comit: Comit,

    // TODO: `Orderbook` and `Take` behaviours to be tied in
    #[behaviour(ignore)]
    pub orderbook: Orderbook,

    #[behaviour(ignore)]
    events: VecDeque<Event>,
}

impl Nectar {
    pub fn new(orderbook: Orderbook) -> Self {
        Self {
            orderbook,
            comit: Comit::default(),
            events: VecDeque::new(),
        }
    }

    fn poll<BIE>(
        &mut self,
        _cx: &mut Context<'_>,
        _params: &mut impl PollParameters,
    ) -> Poll<NetworkBehaviourAction<BIE, Event>> {
        if let Some(event) = self.events.pop_front() {
            return Poll::Ready(NetworkBehaviourAction::GenerateEvent(event));
        }

        // We trust in libp2p to poll us.
        Poll::Pending
    }
}

/// Handle events from the network layer.
///
/// These are:
/// - order taken (once orderbook is added)
/// - swap finalized
impl libp2p::swarm::NetworkBehaviourEventProcess<comit::network::comit::BehaviourOutEvent>
    for Nectar
{
    fn inject_event(&mut self, event: comit::network::comit::BehaviourOutEvent) {
        self.events
            .push_back(Event::SwapFinalized(event.local_swap_id, event.remote_data))
    }
}
