use crate::{
    network::protocols::announce::{
        handler::{self, Handler, HandlerEvent},
        protocol::{OutboundConfig, ReplySubstream},
        SwapDigest,
    },
    swap_protocols::SwapId,
};
use libp2p::{
    core::{ConnectedPoint, Multiaddr, PeerId},
    swarm::{
        NegotiatedSubstream, NetworkBehaviour, NetworkBehaviourAction,
        NetworkBehaviourEventProcess, PollParameters, ProtocolsHandler,
    },
};
use std::{
    collections::VecDeque,
    task::{Context, Poll},
};

/// Network behaviour that announces a swap to peer by sending a `swap_digest`
/// and receives the `swap_id` back.
pub struct Announce {
    /// Pending events to be emitted when polled.
    events: VecDeque<NetworkBehaviourAction<OutboundConfig, BehaviourEvent>>,
}

impl Announce {
    /// Creates a new `Announce` network behaviour.
    pub fn new() -> Self {
        Announce {
            events: VecDeque::new(),
        }
    }

    // This is where data flows into the network behaviour. Begin the announce
    // protocol here.
    pub fn start_announce_protocol(&mut self, outbound_config: OutboundConfig, peer_id: PeerId) {
        self.events.push_back(NetworkBehaviourAction::SendEvent {
            peer_id: peer_id.clone(),
            event: outbound_config,
        });
    }
}

impl NetworkBehaviour for Announce {
    type ProtocolsHandler = Handler;
    type OutEvent = BehaviourEvent;

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        Handler::new()
    }

    fn addresses_of_peer(&mut self, _: &PeerId) -> Vec<Multiaddr> {
        Vec::new()
    }

    fn inject_connected(&mut self, _peer_id: PeerId, _endpoint: ConnectedPoint) {
        // No need to do anything, both this node and connected peer now have a
        // handler (as spawned by `new_handler`) running in the background.
    }

    fn inject_disconnected(&mut self, _peer_id: &PeerId, _: ConnectedPoint) {}

    fn inject_node_event(&mut self, peer_id: PeerId, event: HandlerEvent) {
        match event {
            HandlerEvent::ReceivedConfirmation(confirmed) => {
                self.events.push_back(NetworkBehaviourAction::GenerateEvent(
                    BehaviourEvent::ReceivedConfirmation {
                        peer: peer_id,
                        swap_id: confirmed.swap_id,
                        swap_digest: confirmed.swap_digest,
                    },
                ));
            }
            HandlerEvent::AwaitingConfirmation(sender) => {
                self.events.push_back(NetworkBehaviourAction::GenerateEvent(
                    BehaviourEvent::AwaitingConfirmation {
                        peer: peer_id,
                        io: sender,
                    },
                ));
            }
            HandlerEvent::Error(error) => {
                self.events.push_back(NetworkBehaviourAction::GenerateEvent(
                    BehaviourEvent::Error {
                        peer: peer_id,
                        error,
                    },
                ));
            }
        }
    }

    fn poll(
        &mut self,
        _cx: &mut Context<'_>,
        _params: &mut impl PollParameters,
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

/// Event emitted  by the `Announce` behaviour.
#[derive(Debug)]
pub enum BehaviourEvent {
    /// Node (Alice) has announced the swap and the `swap_id` has been received
    /// from a peer (acting as Bob).
    ReceivedConfirmation {
        /// The peer (Bob) that the swap has been announced to.
        peer: PeerId,
        /// The swap_id returned by the peer (Bob).
        swap_id: SwapId,
        /// The swap_digest
        swap_digest: SwapDigest,
    },

    /// Node (Bob) has received the announced swap (inc. swap_digest) from a
    /// peer (acting as Alice).
    AwaitingConfirmation {
        /// The peer (Alice) that the reply substream is connected to.
        peer: PeerId,
        /// The substream (inc. swap_digest) to reply on (i.e., send `swap_id`).
        io: ReplySubstream<NegotiatedSubstream>,
    },

    /// Error while attempting to announce swap to the remote.
    Error {
        /// The peer with whom the error originated.
        peer: PeerId,
        /// The error that occurred.
        error: handler::Error,
    },
}

impl NetworkBehaviourEventProcess<BehaviourEvent> for Announce {
    // Called when announce behaviour produces an event.
    fn inject_event(&mut self, _: BehaviourEvent) {
        unreachable!("Did you compose Announce behaviour with another behaviour?")
    }
}

#[cfg(test)]
mod tests {
    use crate::{Identify, IdentifyEvent};
    use futures::{pin_mut, prelude::*};
    use libp2p_core::{identity, muxing::StreamMuxer, upgrade, PeerId, Transport};
    use libp2p_mplex::MplexConfig;
    use libp2p_secio::SecioConfig;
    use libp2p_swarm::{Swarm, SwarmEvent};
    use libp2p_tcp::TcpConfig;
    use std::{fmt, io};

    fn transport() -> (
        identity::PublicKey,
        impl Transport<
                Output = (
                    PeerId,
                    impl StreamMuxer<
                        Substream = impl Send,
                        OutboundSubstream = impl Send,
                        Error = impl Into<io::Error>,
                    >,
                ),
                Listener = impl Send,
                ListenerUpgrade = impl Send,
                Dial = impl Send,
                Error = impl fmt::Debug,
            > + Clone,
    ) {
        let id_keys = identity::Keypair::generate_ed25519();
        let pubkey = id_keys.public();
        let transport = TcpConfig::new()
            .nodelay(true)
            .upgrade(upgrade::Version::V1)
            .authenticate(SecioConfig::new(id_keys))
            .multiplex(MplexConfig::new());
        (pubkey, transport)
    }

    #[test]
    fn periodic_id_works() {
        let (mut swarm1, pubkey1) = {
            let (pubkey, transport) = transport();
            let protocol = Identify::new("a".to_string(), "b".to_string(), pubkey.clone());
            let swarm = Swarm::new(transport, protocol, pubkey.clone().into_peer_id());
            (swarm, pubkey)
        };

        let (mut swarm2, pubkey2) = {
            let (pubkey, transport) = transport();
            let protocol = Identify::new("c".to_string(), "d".to_string(), pubkey.clone());
            let swarm = Swarm::new(transport, protocol, pubkey.clone().into_peer_id());
            (swarm, pubkey)
        };

        Swarm::listen_on(&mut swarm1, "/ip4/127.0.0.1/tcp/0".parse().unwrap()).unwrap();

        let listen_addr = async_std::task::block_on(async {
            loop {
                let swarm1_fut = swarm1.next_event();
                pin_mut!(swarm1_fut);
                match swarm1_fut.await {
                    SwarmEvent::NewListenAddr(addr) => return addr,
                    _ => {}
                }
            }
        });
        Swarm::dial_addr(&mut swarm2, listen_addr).unwrap();

        // nb. Either swarm may receive the `Identified` event first, upon which
        // it will permit the connection to be closed, as defined by
        // `IdentifyHandler::connection_keep_alive`. Hence the test succeeds if
        // either `Identified` event arrives correctly.
        async_std::task::block_on(async move {
            loop {
                let swarm1_fut = swarm1.next();
                pin_mut!(swarm1_fut);
                let swarm2_fut = swarm2.next();
                pin_mut!(swarm2_fut);

                match future::select(swarm1_fut, swarm2_fut)
                    .await
                    .factor_second()
                    .0
                {
                    future::Either::Left(IdentifyEvent::Received { info, .. }) => {
                        assert_eq!(info.public_key, pubkey2);
                        assert_eq!(info.protocol_version, "c");
                        assert_eq!(info.agent_version, "d");
                        assert!(!info.protocols.is_empty());
                        assert!(info.listen_addrs.is_empty());
                        return;
                    }
                    future::Either::Right(IdentifyEvent::Received { info, .. }) => {
                        assert_eq!(info.public_key, pubkey1);
                        assert_eq!(info.protocol_version, "a");
                        assert_eq!(info.agent_version, "b");
                        assert!(!info.protocols.is_empty());
                        assert_eq!(info.listen_addrs.len(), 1);
                        return;
                    }
                    _ => {}
                }
            }
        })
    }
}
