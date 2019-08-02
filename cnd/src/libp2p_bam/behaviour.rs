use crate::{
    libp2p_bam::{
        handler::{
            self, AutomaticallyGeneratedErrorResponse, PendingIncomingResponse, ProtocolInEvent,
            ProtocolOutEvent,
        },
        BamHandler, PendingIncomingRequest, PendingOutgoingRequest,
    },
    network::DialInformation,
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
        pending_events: Vec<ProtocolInEvent>,
        address_hints: Vec<Multiaddr>,
    },
}

/// Events that are caused 'out'-side of this node and emitted by the
/// `Behaviour` to the application.
#[derive(Debug)]
pub enum BehaviourOutEvent {
    PendingIncomingRequest {
        request: PendingIncomingRequest,
        peer_id: PeerId,
    },
}

#[derive(Debug)]
pub struct BamBehaviour<TSubstream> {
    marker: PhantomData<TSubstream>,

    events_sender: UnboundedSender<NetworkBehaviourAction<ProtocolInEvent, BehaviourOutEvent>>,
    events: UnboundedReceiver<NetworkBehaviourAction<ProtocolInEvent, BehaviourOutEvent>>,

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
        dial_information: DialInformation,
        request: OutgoingRequest,
    ) -> Box<dyn Future<Item = Response, Error = ()> + Send> {
        let (sender, receiver) = futures::oneshot();

        let request = PendingOutgoingRequest {
            request,
            channel: sender,
        };

        match self.connections.entry(dial_information.clone().peer_id) {
            Entry::Vacant(entry) => {
                self.events_sender
                    .unbounded_send(NetworkBehaviourAction::DialPeer {
                        peer_id: dial_information.peer_id,
                    })
                    .expect("we own the receiver");

                let address_hints = dial_information
                    .address_hint
                    .map(|address| vec![address])
                    .unwrap_or_else(Vec::new);

                entry.insert(ConnectionState::Connecting {
                    pending_events: vec![ProtocolInEvent::PendingOutgoingRequest { request }],
                    address_hints,
                });
            }
            Entry::Occupied(mut entry) => {
                let connection_state = entry.get_mut();

                match connection_state {
                    ConnectionState::Connecting {
                        pending_events,
                        address_hints,
                    } => {
                        pending_events.push(ProtocolInEvent::PendingOutgoingRequest { request });

                        if let Some(address) = dial_information.address_hint {
                            // We insert at the front because we consider the new address to be the
                            // most likely one to succeed. The order of this vector is important
                            // when returning it from `addresses_of_peer` because it will be tried
                            // by libp2p in the returned order.
                            address_hints.insert(0, address);
                        }
                    }
                    ConnectionState::Connected { .. } => {
                        self.events_sender
                            .unbounded_send(NetworkBehaviourAction::SendEvent {
                                peer_id: dial_information.peer_id,
                                event: ProtocolInEvent::PendingOutgoingRequest { request },
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

    pub fn connected_peers(&mut self) -> impl Iterator<Item = (PeerId, Vec<Multiaddr>)> {
        let addresses = self
            .connections
            .iter()
            .filter_map(|(peer, connection_state)| match connection_state {
                ConnectionState::Connecting { .. } => None,
                ConnectionState::Connected { addresses } => {
                    Some((peer.clone(), addresses.clone().into_iter().collect()))
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
    type OutEvent = BehaviourOutEvent;

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        BamHandler::new(self.known_request_headers.clone())
    }

    fn addresses_of_peer(&mut self, peer_id: &PeerId) -> Vec<Multiaddr> {
        self.connections
            .iter()
            .find_map(|(candidate, addresses)| {
                if candidate == peer_id {
                    Some(addresses)
                } else {
                    None
                }
            })
            .map(|connection_state| match connection_state {
                ConnectionState::Connecting { address_hints, .. } => address_hints.clone(),
                ConnectionState::Connected { addresses } => addresses.iter().cloned().collect(),
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
                    ConnectionState::Connecting {
                        pending_events,
                        address_hints: _we_no_longer_care_at_this_stage,
                    } => {
                        for event in pending_events {
                            self.events_sender
                                .unbounded_send(NetworkBehaviourAction::SendEvent {
                                    peer_id: peer_id.clone(),
                                    event,
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

    fn inject_node_event(&mut self, peer: PeerId, event: ProtocolOutEvent) {
        match event {
            ProtocolOutEvent::IncomingRequest(pending_incoming_request) => {
                self.events_sender
                    .unbounded_send(NetworkBehaviourAction::GenerateEvent(
                        BehaviourOutEvent::PendingIncomingRequest {
                            request: pending_incoming_request,
                            peer_id: peer,
                        },
                    ))
                    .expect("we own the receiver");
            }
            ProtocolOutEvent::IncomingResponse(PendingIncomingResponse { response, channel }) => {
                let _ = channel.send(response);
            }
            ProtocolOutEvent::BadIncomingRequest(AutomaticallyGeneratedErrorResponse {
                response,
                channel,
            }) => {
                let _ = channel.send(response);
            }
            ProtocolOutEvent::Error {
                error: handler::Error::Stream(error),
            } => {
                log::error!(target: "sub-libp2p", "failure in communication with {:?}: {:?}", peer, error);
            }
            ProtocolOutEvent::Error {
                error: handler::Error::DroppedResponseSender(_),
            } => {
                // The `oneshot::Sender` is the only way to send a RESPONSE as an answer to the
                // SWAP REQUEST. A dropped `Sender` therefore is either a bug in
                // the application or the application consciously does not want to answer the
                // SWAP REQUEST. In either way, we should signal this to the remote peer by
                // closing the substream.
                log::error!(target: "sub-libp2p", "user dropped `oneshot::Sender` for response, closing substream with peer {:?}", peer);
            }
            ProtocolOutEvent::BadIncomingResponse => {
                log::error!(target: "sub-libp2p", "badly formatted response from {:?}", peer);
            }
            ProtocolOutEvent::UnexpectedFrameType {
                bad_frame,
                expected_type,
            } => {
                log::error!(target: "sub-libp2p", "{:?} sent the frame {:?} even though a {:?} was expected", peer, bad_frame, expected_type);
            }
            ProtocolOutEvent::UnexpectedEOF => {
                log::error!(target: "sub-libp2p", "substream with {:?} unexpectedly ended while waiting for messages", peer);
            }
        }
    }

    fn poll(
        &mut self,
        _params: &mut impl PollParameters,
    ) -> Async<NetworkBehaviourAction<ProtocolInEvent, BehaviourOutEvent>> {
        self.events
            .poll()
            .expect("unbounded channel can never fail")
            .map(|item| item.expect("unbounded channel never ends"))
    }
}
