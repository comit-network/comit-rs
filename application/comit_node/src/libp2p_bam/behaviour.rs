use crate::libp2p_bam::{BamHandler, PendingIncomingRequest, PendingOutgoingRequest};
use bam::json::{OutgoingRequest, Response};
use futures::{task, Async, Future};
use libp2p::{
    core::swarm::{ConnectedPoint, NetworkBehaviour, NetworkBehaviourAction, PollParameters},
    Multiaddr, PeerId,
};
use std::{
    collections::{hash_map::Entry, vec_deque::VecDeque, HashMap, HashSet},
    marker::PhantomData,
};
use tokio::prelude::{task::Task, AsyncRead, AsyncWrite};

#[derive(Debug)]
pub struct BamBehaviour<TSubstream> {
    marker: PhantomData<TSubstream>,

    events: VecDeque<NetworkBehaviourAction<PendingOutgoingRequest, PendingIncomingRequest>>,
    known_request_headers: HashMap<String, HashSet<String>>,
    current_task: Option<Task>,
    addresses: HashMap<PeerId, Vec<Multiaddr>>,
}

impl<TSubstream> BamBehaviour<TSubstream> {
    pub fn new(known_request_headers: HashMap<String, HashSet<String>>) -> Self {
        Self {
            marker: PhantomData,
            events: VecDeque::new(),
            known_request_headers,
            current_task: None,
            addresses: HashMap::new(),
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

        self.events.push_back(NetworkBehaviourAction::DialPeer {
            peer_id: peer_id.clone(),
        });
        self.events.push_back(NetworkBehaviourAction::SendEvent {
            peer_id,
            event: request,
        });

        if let Some(task) = &self.current_task {
            task.notify();
        }

        Box::new(receiver.map_err(|_| {
            log::warn!(
                "Sender of response future was unexpectedly dropped before response was received."
            )
        }))
    }

    pub fn addresses(&mut self) -> impl Iterator<Item = (PeerId, Vec<Multiaddr>)> {
        self.addresses
            .clone()
            .into_iter()
            .map(|(peer, addresses)| (peer.clone(), addresses.clone()))
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
        self.addresses
            .get(peer_id)
            .map(|addresses| addresses.clone())
            .unwrap_or_else(Vec::new)
    }

    fn inject_connected(&mut self, peer_id: PeerId, endpoint: ConnectedPoint) {
        log::debug!(target: "bam", "connected to {} at {:?}", peer_id, endpoint);

        let address = match endpoint {
            ConnectedPoint::Dialer { address } => address,
            ConnectedPoint::Listener { send_back_addr, .. } => send_back_addr,
        };

        match self.addresses.entry(peer_id) {
            Entry::Occupied(mut entry) => {
                let addresses = entry.get_mut();
                addresses.push(address)
            }
            Entry::Vacant(entry) => {
                let addresses = vec![address];
                entry.insert(addresses);
            }
        }

        if let Some(task) = &self.current_task {
            task.notify()
        }
    }

    fn inject_disconnected(&mut self, peer_id: &PeerId, endpoint: ConnectedPoint) {
        log::debug!(target: "bam", "disconnected from {} at {:?}", peer_id, endpoint);

        if let Some(task) = &self.current_task {
            task.notify()
        }
    }

    fn inject_node_event(&mut self, _: PeerId, event: PendingIncomingRequest) {
        self.events
            .push_back(NetworkBehaviourAction::GenerateEvent(event));

        if let Some(task) = &self.current_task {
            task.notify()
        }
    }

    fn poll(
        &mut self,
        _params: &mut PollParameters<'_>,
    ) -> Async<NetworkBehaviourAction<PendingOutgoingRequest, PendingIncomingRequest>> {
        match self.events.pop_front() {
            Some(event) => {
                log::debug!(target: "bam", "emitting {:?}", event);

                if let NetworkBehaviourAction::SendEvent { peer_id, .. } = &event {
                    if !self.addresses.contains_key(peer_id) {
                        log::info!(
                            target: "bam",
                            "not yet connected to {}, cannot send message",
                            peer_id.clone()
                        );

                        self.events.push_back(event);

                        self.current_task = Some(task::current());
                        return Async::NotReady;
                    }
                }

                return Async::Ready(event);
            }
            None => {
                log::debug!(target: "bam", "Currently no events, storing current task");

                self.current_task = Some(task::current());
                return Async::NotReady;
            }
        }
    }
}
