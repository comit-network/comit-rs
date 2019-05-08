use crate::libp2p_bam::{
    handler::{AutomaticallyGeneratedErrorResponse, InnerEvent, PendingIncomingResponse},
    BamHandler, PendingIncomingRequest, PendingOutgoingRequest,
};
use bam::json::{OutgoingRequest, Response};
use futures::{
    stream::Stream,
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    Async, Future,
};
use libp2p::{
    core::swarm::{ConnectedPoint, NetworkBehaviour, NetworkBehaviourAction, PollParameters},
    Multiaddr, PeerId,
};
use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    marker::PhantomData,
};
use tokio::prelude::{AsyncRead, AsyncWrite};

#[derive(Debug)]
enum ConnectionState {
    Connected {
        addresses: HashSet<Multiaddr>,
    },
    Connecting {
        pending_requests: Vec<PendingOutgoingRequest>,
    },
}

#[derive(Debug)]
pub struct BamBehaviour<TSubstream> {
    marker: PhantomData<TSubstream>,

    events_sender:
        UnboundedSender<NetworkBehaviourAction<PendingOutgoingRequest, PendingIncomingRequest>>,
    events:
        UnboundedReceiver<NetworkBehaviourAction<PendingOutgoingRequest, PendingIncomingRequest>>,

    known_request_headers: HashMap<String, HashSet<String>>,
    connections: HashMap<PeerId, ConnectionState>,
}

impl<TSubstream> BamBehaviour<TSubstream> {
    pub fn new(known_request_headers: HashMap<String, HashSet<String>>) -> Self {
        let (sender, receiver) = mpsc::unbounded();

        Self {
            marker: PhantomData,
            events_sender: sender,
            events: receiver,
            known_request_headers,
            connections: HashMap::new(),
        }
    }

    pub fn send_request(
        &mut self,
        peer_id: PeerId,
        request: OutgoingRequest,
    ) -> Box<dyn Future<Item = Response, Error = ()> + Send> {
        let (sender, receiver) = futures::oneshot();

        let request = PendingOutgoingRequest {
            request,
            channel: sender,
        };

        match self.connections.entry(peer_id.clone()) {
            Entry::Vacant(entry) => {
                self.events_sender
                    .unbounded_send(NetworkBehaviourAction::DialPeer { peer_id })
                    .expect("we own the receiver");
                entry.insert(ConnectionState::Connecting {
                    pending_requests: vec![request],
                });
            }
            Entry::Occupied(mut entry) => {
                let connection_state = entry.get_mut();

                match connection_state {
                    ConnectionState::Connecting { pending_requests } => {
                        pending_requests.push(request);
                    }
                    ConnectionState::Connected { .. } => {
                        self.events_sender
                            .unbounded_send(NetworkBehaviourAction::SendEvent {
                                peer_id,
                                event: request,
                            })
                            .expect("we own the receiver");
                    }
                }
            }
        }

        Box::new(receiver.map_err(|_| {
            log::warn!(
                "Sender of response future was unexpectedly dropped before response was received."
            )
        }))
    }

    pub fn addresses(&mut self) -> impl Iterator<Item = (PeerId, Vec<Multiaddr>)> {
        let addresses = self
            .connections
            .iter()
            .map(|(peer, connection_state)| match connection_state {
                ConnectionState::Connecting { .. } => (peer.clone(), vec![]),
                ConnectionState::Connected { addresses } => {
                    (peer.clone(), addresses.clone().into_iter().collect())
                }
            })
            .collect::<Vec<_>>();

        addresses.into_iter()
    }
}

impl<TSubstream> NetworkBehaviour for BamBehaviour<TSubstream>
where
    TSubstream: AsyncRead + AsyncWrite,
{
    type ProtocolsHandler = BamHandler<TSubstream>;
    type OutEvent = PendingIncomingRequest;

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        BamHandler::new(self.known_request_headers.clone())
    }

    fn addresses_of_peer(&mut self, peer_id: &PeerId) -> Vec<Multiaddr> {
        self.addresses()
            .find_map(|(candidate, addresses)| {
                if &candidate == peer_id {
                    Some(addresses)
                } else {
                    None
                }
            })
            .unwrap_or_else(Vec::new)
    }

    fn inject_connected(&mut self, peer_id: PeerId, endpoint: ConnectedPoint) {
        log::debug!(target: "sub-libp2p", "connected to {} at {:?}", peer_id, endpoint);

        let address = match endpoint {
            ConnectedPoint::Dialer { address } => address,
            ConnectedPoint::Listener { send_back_addr, .. } => send_back_addr,
        };

        match self.connections.entry(peer_id.clone()) {
            Entry::Occupied(entry) => {
                let connection_state = entry.remove();

                match connection_state {
                    ConnectionState::Connected { mut addresses } => {
                        addresses.insert(address);
                        self.connections
                            .insert(peer_id.clone(), ConnectionState::Connected { addresses });
                    }
                    ConnectionState::Connecting { pending_requests } => {
                        for request in pending_requests {
                            self.events_sender
                                .unbounded_send(NetworkBehaviourAction::SendEvent {
                                    peer_id: peer_id.clone(),
                                    event: request,
                                })
                                .expect("we own the receiver");
                        }
                        let mut addresses = HashSet::new();
                        addresses.insert(address);

                        self.connections
                            .insert(peer_id, ConnectionState::Connected { addresses });
                    }
                }
            }
            Entry::Vacant(entry) => {
                let mut addresses = HashSet::new();
                addresses.insert(address);

                entry.insert(ConnectionState::Connected { addresses });
            }
        }
    }

    fn inject_disconnected(&mut self, peer_id: &PeerId, endpoint: ConnectedPoint) {
        log::debug!(target: "sub-libp2p", "disconnected from {} at {:?}", peer_id, endpoint);

        let address = match endpoint {
            ConnectedPoint::Dialer { address } => address,
            ConnectedPoint::Listener { send_back_addr, .. } => send_back_addr,
        };

        if let Some(ConnectionState::Connected { mut addresses }) = self.connections.remove(peer_id)
        {
            addresses.remove(&address);

            if !addresses.is_empty() {
                self.connections
                    .insert(peer_id.clone(), ConnectionState::Connected { addresses });
            }
        }
    }

    fn inject_node_event(&mut self, peer: PeerId, event: InnerEvent) {
        match event {
            InnerEvent::IncomingRequest(pending_incoming_request) => {
                self.events_sender
                    .unbounded_send(NetworkBehaviourAction::GenerateEvent(
                        pending_incoming_request,
                    ))
                    .expect("we own the receiver");
            }
            InnerEvent::IncomingResponse(PendingIncomingResponse { response, channel }) => {
                let _ = channel.send(response);
            }
            InnerEvent::BadIncomingRequest(AutomaticallyGeneratedErrorResponse {
                response,
                channel,
            }) => {
                let _ = channel.send(response);
            }
            InnerEvent::Error => {
                log::error!(target: "sub-libp2p", "error in communication with {:?}", peer);
            }
            InnerEvent::BadIncomingResponse => {
                log::error!(target: "sub-libp2p", "badly formatted response from {:?}", peer);
            }
        }
    }

    fn poll(
        &mut self,
        _params: &mut PollParameters<'_>,
    ) -> Async<NetworkBehaviourAction<PendingOutgoingRequest, PendingIncomingRequest>> {
        self.events
            .poll()
            .expect("unbounded channel can never fail")
            .map(|item| item.expect("unbounded channel never ends"))
    }
}
