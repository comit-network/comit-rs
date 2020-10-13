use crate::network::oneshot_protocol;
use libp2p::{
    core::{connection::ConnectionId, Multiaddr, PeerId},
    swarm::{
        NetworkBehaviour, NetworkBehaviourAction, NotifyHandler, OneShotHandler, PollParameters,
        ProtocolsHandler,
    },
};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    collections::VecDeque,
    fmt::Debug,
    task::{Context, Poll},
};
use tracing::trace;

/// Generic network behaviour for handling oneshot protocols.
#[derive(Debug)]
pub struct Behaviour<M> {
    /// Events that need to be yielded to the outside when polling.
    events: VecDeque<NetworkBehaviourAction<oneshot_protocol::OutboundConfig<M>, OutEvent<M>>>,
}

impl<M> Behaviour<M> {
    pub fn send(&mut self, peer_id: PeerId, message: M) {
        self.events
            .push_back(NetworkBehaviourAction::NotifyHandler {
                peer_id,
                handler: NotifyHandler::Any,
                event: oneshot_protocol::OutboundConfig::new(message),
            })
    }
}

impl<M> Default for Behaviour<M> {
    fn default() -> Self {
        Behaviour {
            events: VecDeque::new(),
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

    fn addresses_of_peer(&mut self, _: &PeerId) -> Vec<Multiaddr> {
        Vec::new()
    }

    fn inject_connected(&mut self, _: &PeerId) {
        // We assume a connection has been established.
    }

    fn inject_disconnected(&mut self, _: &PeerId) {
        // We assume a connection has been established.
    }

    fn inject_event(
        &mut self,
        peer: PeerId,
        _: ConnectionId,
        event: oneshot_protocol::OutEvent<M>,
    ) {
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
