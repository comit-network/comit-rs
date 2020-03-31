use crate::network::oneshot_protocol;
use libp2p::{
    core::{ConnectedPoint, Multiaddr, PeerId},
    swarm::{
        NetworkBehaviour, NetworkBehaviourAction, OneShotHandler, PollParameters, ProtocolsHandler,
    },
};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    fmt::Debug,
    task::{Context, Poll},
};
use tracing::trace;

/// Generic network behaviour for handling oneshot protocols.
#[derive(Debug)]
pub struct Behaviour<M> {
    /// Events that need to be yielded to the outside when polling.
    events: VecDeque<NetworkBehaviourAction<oneshot_protocol::OutboundConfig<M>, OutEvent<M>>>,
    address_book: HashMap<PeerId, Vec<Multiaddr>>,
}

impl<M> Behaviour<M> {
    pub fn send(&mut self, peer_id: PeerId, message: M) {
        self.events.push_back(NetworkBehaviourAction::SendEvent {
            peer_id,
            event: oneshot_protocol::OutboundConfig::new(message),
        })
    }

    // TODO: if we decide to keep these different network behaviour (and not use a
    // multi-protocols handler or something) then we should do our own
    // connection handling in these as-well by extracting the one from the announce
    // protocol in a reusable-manner
    pub fn register_addresses(&mut self, peer_id: PeerId, addresses: Vec<Multiaddr>) {
        self.address_book.insert(peer_id, addresses);
    }
}

impl<M> Default for Behaviour<M> {
    fn default() -> Self {
        Behaviour {
            events: VecDeque::new(),
            address_book: HashMap::default(),
        }
    }
}

/// Events emitted from the NetworkBehaviour up to the swarm.
#[derive(Debug)]
pub enum OutEvent<M> {
    /// We received the message M from the given peer.
    Received { peer: PeerId, message: M },
    /// We sent the message M to given peer.
    Sent { peer: PeerId, message: M },
}

impl<M> NetworkBehaviour for Behaviour<M>
where
    M: oneshot_protocol::Message + Serialize + DeserializeOwned + Clone + Debug + Send + 'static,
{
    type ProtocolsHandler = OneShotHandler<
        oneshot_protocol::InboundConfig<M>,
        oneshot_protocol::OutboundConfig<M>,
        oneshot_protocol::OutEvent<M>,
    >;
    type OutEvent = OutEvent<M>;

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        Default::default()
    }

    fn addresses_of_peer(&mut self, peer: &PeerId) -> Vec<Multiaddr> {
        self.address_book.get(peer).cloned().unwrap_or_default()
    }

    fn inject_connected(&mut self, _: PeerId, _: ConnectedPoint) {
        // Do nothing, announce protocol is going to take care of connections.
    }

    fn inject_disconnected(&mut self, _: &PeerId, _: ConnectedPoint) {
        // Do nothing, announce protocol is going to take care of connections.
    }

    fn inject_node_event(&mut self, peer: PeerId, event: oneshot_protocol::OutEvent<M>) {
        match event {
            oneshot_protocol::OutEvent::Received(message) => {
                trace!(
                    "Received message from {} on protocol {}: {:?}",
                    peer,
                    M::INFO,
                    message
                );

                // Add the message to be dispatched to the user.
                self.events
                    .push_back(NetworkBehaviourAction::GenerateEvent(OutEvent::Received {
                        peer,
                        message,
                    }));
            }
            oneshot_protocol::OutEvent::Sent(message) => {
                trace!(
                    "Sent message {:?} to {} on protocol {}",
                    message,
                    peer,
                    M::INFO
                );

                self.events
                    .push_back(NetworkBehaviourAction::GenerateEvent(OutEvent::Sent {
                        peer,
                        message,
                    }));
            }
        }
    }

    fn poll(
        &mut self,
        _: &mut Context<'_>,
        _: &mut impl PollParameters,
    ) -> Poll<
        NetworkBehaviourAction<
            <Self::ProtocolsHandler as ProtocolsHandler>::InEvent,
            Self::OutEvent,
        >,
    > {
        if let Some(event) = self.events.pop_front() {
            return Poll::Ready(event);
        }

        Poll::Pending
    }
}
