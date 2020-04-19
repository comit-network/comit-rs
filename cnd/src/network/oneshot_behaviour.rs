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
        self.events
            .push_back(NetworkBehaviourAction::NotifyHandler {
                peer_id,
                handler: NotifyHandler::Any,
                event: oneshot_protocol::OutboundConfig::new(message),
            })
    }

    // If we decide to keep these different network behaviour (and not use a
    // multi-protocols handler or something) then we should do our own
    // connection handling in these as-well by extracting the one from the
    // announce protocol in a reusable-manner.
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

    fn inject_connected(&mut self, _: &PeerId) {
        // Do nothing, announce protocol is going to take care of connections.
    }

    fn inject_disconnected(&mut self, _: &PeerId) {
        // Do nothing, announce protocol is going to take care of connections.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        network::{derive_key_pair, protocols::announce::SwapDigest, transport, TokioExecutor},
        seed::RootSeed,
    };
    use anyhow::Context;

    use futures::pin_mut;
    use libp2p::{
        multihash::Sha3_256,
        swarm::{Swarm, SwarmBuilder, SwarmEvent},
        Multiaddr, PeerId,
    };
    use rand::thread_rng;
    use serde::{Deserialize, Serialize};

    use tokio::runtime;

    fn random_swap_digest() -> SwapDigest {
        SwapDigest::new(Sha3_256::digest(b"hello world"))
    }

    /// The message for the Bitcoin identity sharing protocol.
    #[derive(Clone, Copy, Deserialize, Serialize, Debug)]
    pub struct Message;

    impl Message {
        pub fn new() -> Self {
            Self
        }
    }

    impl oneshot_protocol::Message for Message {
        const INFO: &'static str = "/comit/swap/identity/bitcoin/1.0.0";
    }

    #[test]
    fn oneshot_integration_test() {
        let (alice_key_pair, alice_peer_id) = {
            let seed = RootSeed::new_random(thread_rng()).unwrap();
            let key_pair = derive_key_pair(&seed);
            let peer_id = PeerId::from(key_pair.clone().public());
            (key_pair, peer_id)
        };

        let (bob_key_pair, bob_peer_id) = {
            let seed = RootSeed::new_random(thread_rng()).unwrap();
            let key_pair = derive_key_pair(&seed);
            let peer_id = PeerId::from(key_pair.clone().public());
            (key_pair, peer_id)
        };

        let mut alice_runtime = runtime::Builder::new()
            .enable_all()
            .threaded_scheduler()
            .thread_stack_size(1024 * 1024 * 8) // the default is 2MB but that causes a segfault for some reason
            .build()
            .unwrap();

        let dummy_oneshot: Behaviour<Message> = Behaviour::default();

        let mut alice_swarm = SwarmBuilder::new(
            transport::build_comit_transport(alice_key_pair).unwrap(),
            dummy_oneshot,
            alice_peer_id.clone(),
        )
        .executor(Box::new(TokioExecutor {
            handle: alice_runtime.handle().clone(),
        }))
        .build();

        let mut bob_runtime = runtime::Builder::new()
            .enable_all()
            .threaded_scheduler()
            .thread_stack_size(1024 * 1024 * 8) // the default is 2MB but that causes a segfault for some reason
            .build()
            .unwrap();

        let dummy_oneshot: Behaviour<Message> = Behaviour::default();

        let mut bob_swarm = SwarmBuilder::new(
            transport::build_comit_transport(bob_key_pair).unwrap(),
            dummy_oneshot,
            bob_peer_id.clone(),
        )
        .executor(Box::new(TokioExecutor {
            handle: bob_runtime.handle().clone(),
        }))
        .build();

        let bob_addr: Multiaddr = "/ip4/127.0.0.1/tcp/0".parse().unwrap();
        Swarm::listen_on(&mut bob_swarm, bob_addr.clone())
            .with_context(|| format!("Address is not supported: {:?}", bob_addr))
            .unwrap();

        let bob_addr: libp2p::core::Multiaddr = bob_runtime.block_on(async {
            loop {
                let bob_swarm_fut = bob_swarm.next_event();
                pin_mut!(bob_swarm_fut);
                match bob_swarm_fut.await {
                    SwarmEvent::NewListenAddr(addr) => return addr,
                    _ => {}
                }
            }
        });

        alice_swarm.register_addresses(bob_peer_id.clone(), vec![bob_addr]);

        alice_swarm.send(bob_peer_id, Message);

        bob_runtime.block_on(async {
            loop {
                let bob_swarm_fut = bob_swarm.next_event();
                pin_mut!(bob_swarm_fut);
                match bob_swarm_fut.await {
                    SwarmEvent::IncomingConnection { .. } => return,
                    _ => {}
                }
            }
        });

        // // trying to check if swap finalized or other events occur on bob
        // // doing something wrong here causing the test to hang
        // bob_runtime.block_on(async move {
        //     loop {
        //         let bob_swarm_fut = bob_swarm.next_event();
        //         pin_mut!(bob_swarm_fut);
        //         match bob_swarm_fut.await {
        //
        //             SwarmEvent::Behaviour(behavior_event) => {
        //                 // never enters this block causing the test to hang
        //                 // if let BehaviourOutEvent::SwapFinalized {..} =
        // behavior_event {                 //
        // //assert_eq!(io.swap_digest, send_swap_digest);
        // //     // assert_eq!(peer, peer)                 //
        //                 //     return;
        //                 // }
        //
        //                 return;
        //             }
        //             _ => {}
        //         }
        //     }
        // })
    }
}
