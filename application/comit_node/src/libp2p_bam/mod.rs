mod handler;
mod protocol;

pub use self::{handler::*, protocol::*};

use bam::json::Frame;
use libp2p::{
    core::swarm::{ConnectedPoint, NetworkBehaviour, NetworkBehaviourAction, PollParameters},
    Multiaddr, PeerId,
};
use std::{collections::vec_deque::VecDeque, marker::PhantomData};
use tokio::prelude::{Async, AsyncRead, AsyncWrite};

#[derive(Debug)]
pub struct Bam<TSubstream> {
    marker: PhantomData<TSubstream>,
    events: VecDeque<NetworkBehaviourAction<Frame, (PeerId, Frame)>>,
}

impl<TSubstream> Bam<TSubstream> {
    pub fn send_frame(&mut self, peer_id: PeerId, frame: bam::json::Frame) {
        self.events.push_back(NetworkBehaviourAction::SendEvent {
            peer_id,
            event: frame,
        });
    }
}

impl<TSubstream> NetworkBehaviour for Bam<TSubstream>
where
    TSubstream: AsyncRead + AsyncWrite,
{
    type ProtocolsHandler = BamHandler<TSubstream>;
    type OutEvent = (PeerId, bam::json::Frame);

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        BamHandler::new()
    }

    fn addresses_of_peer(&mut self, _peer_id: &PeerId) -> Vec<Multiaddr> {
        vec![]
    }

    fn inject_connected(&mut self, _peer_id: PeerId, _endpoint: ConnectedPoint) {}

    fn inject_disconnected(&mut self, _peer_id: &PeerId, _endpoint: ConnectedPoint) {}

    fn inject_node_event(&mut self, peer_id: PeerId, event: bam::json::Frame) {
        self.events
            .push_back(NetworkBehaviourAction::GenerateEvent((peer_id, event)));
    }

    fn poll(
        &mut self,
        _params: &mut PollParameters<'_>,
    ) -> Async<NetworkBehaviourAction<bam::json::Frame, Self::OutEvent>> {
        self.events
            .pop_front()
            .map(Async::Ready)
            .unwrap_or(Async::NotReady)
    }
}
