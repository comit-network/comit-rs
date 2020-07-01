use crate::order::BtcDaiOrder;
use comit::network::{Comit, RemoteData};
use comit::LocalSwapId;
use libp2p::swarm::{NetworkBehaviourAction, PollParameters};
use libp2p::NetworkBehaviour;
use std::collections::VecDeque;
use std::task::{Context, Poll};

#[derive(Debug)]
pub enum Event {
    // When an order expires a new order is published.
    // New orders are only published after the old one has expired.
    // We assume that this event is emitted by the orderbook.
    OrderExpired(Order),

    // The orderbook registered that somebody wants to take a specific order.
    // We assume that this event is emitted by the orderbook.
    OrderTakeRequest(Order), // Emitted by orderbook

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
    pub fn publish(&mut self, _order: Order) {
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
pub struct Order(BtcDaiOrder);

impl From<Order> for BtcDaiOrder {
    fn from(order: Order) -> Self {
        order.0
    }
}

impl From<BtcDaiOrder> for Order {
    fn from(btc_dai_order: BtcDaiOrder) -> Self {
        Self(btc_dai_order)
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
/// - order expired (once orderbook is added)
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
