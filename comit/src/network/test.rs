use libp2p::{
    core::{muxing::StreamMuxerBox, upgrade::Version},
    secio::SecioConfig,
    swarm::{IntoProtocolsHandler, NetworkBehaviour, ProtocolsHandler, SwarmBuilder, SwarmEvent},
    yamux, Multiaddr, PeerId, Transport,
};
use std::{fmt::Debug, future::Future, pin::Pin, time::Duration};

/// An adaptor struct for libp2p that spawns futures into the current
/// thread-local runtime.
struct GlobalSpawnTokioExecutor;

impl libp2p::core::Executor for GlobalSpawnTokioExecutor {
    fn exec(&self, future: Pin<Box<dyn Future<Output = ()> + Send>>) {
        let _ = tokio::spawn(future);
    }
}

pub fn new_swarm<B: NetworkBehaviour, F: Fn(PeerId) -> B>(behaviour_fn: F) -> (libp2p::Swarm<B>, Multiaddr, PeerId) where <<<B as NetworkBehaviour>::ProtocolsHandler as IntoProtocolsHandler>::Handler as ProtocolsHandler>::InEvent: Clone{
    let keypair = libp2p::identity::Keypair::generate_ed25519();
    let peer_id = PeerId::from(keypair.public());

    let transport = libp2p::core::transport::memory::MemoryTransport::default()
        .upgrade(Version::V1)
        .authenticate(SecioConfig::new(keypair))
        .multiplex(yamux::Config::default())
        .map(|(peer, muxer), _| (peer, StreamMuxerBox::new(muxer)))
        .timeout(Duration::from_secs(5))
        .boxed();

    let mut swarm: libp2p::Swarm<B> =
        SwarmBuilder::new(transport, behaviour_fn(peer_id.clone()), peer_id.clone())
            .executor(Box::new(GlobalSpawnTokioExecutor))
            .build();

    let address_port = rand::random::<u64>();
    let addr = format!("/memory/{}", address_port)
        .parse::<Multiaddr>()
        .unwrap();

    libp2p::Swarm::listen_on(&mut swarm, addr.clone()).unwrap();

    (swarm, addr, peer_id)
}

pub async fn await_events_or_timeout<A, B>(
    alice_event: impl Future<Output = A>,
    bob_event: impl Future<Output = B>,
) -> (A, B) {
    tokio::time::timeout(
        Duration::from_secs(10),
        futures::future::join(alice_event, bob_event),
    )
    .await
    .expect("network behaviours to emit an event within 10 seconds")
}

/// Connects two swarms with each other.
///
/// This assumes the transport that is in use can be used by Alice to connect to
/// the listen address that is emitted by Bob. In other words, they have to be
/// on the same network. The memory transport used by the above `new_swarm`
/// function fulfills this.
///
/// We also assume that the swarms don't emit any behaviour events during the
/// connection phase. Any event emitted is considered a bug from this functions
/// PoV because they would be lost.
pub async fn connect<B>(alice: &mut libp2p::Swarm<B>, bob: &mut libp2p::Swarm<B>)
    where
        B: NetworkBehaviour,
        <<<B as NetworkBehaviour>::ProtocolsHandler as IntoProtocolsHandler>::Handler as ProtocolsHandler>::InEvent: Clone,
<B as NetworkBehaviour>::OutEvent: Debug{
    let mut alice_connected = false;
    let mut bob_connected = false;

    while !alice_connected && !bob_connected {
        let (alice_event, bob_event) =
            futures::future::join(alice.next_event(), bob.next_event()).await;

        match alice_event {
            SwarmEvent::ConnectionEstablished { .. } => {
                alice_connected = true;
            }
            SwarmEvent::Behaviour(event) => {
                panic!(
                    "alice unexpectedly emitted a behaviour event during connection: {:?}",
                    event
                );
            }
            _ => {}
        }
        match bob_event {
            SwarmEvent::ConnectionEstablished { .. } => {
                bob_connected = true;
            }
            SwarmEvent::NewListenAddr(addr) => {
                libp2p::Swarm::dial_addr(alice, addr).unwrap();
            }
            SwarmEvent::Behaviour(event) => {
                panic!(
                    "bob unexpectedly emitted a behaviour event during connection: {:?}",
                    event
                );
            }
            _ => {}
        }
    }
}
