use libp2p::{
    core::{muxing::StreamMuxerBox, upgrade::Version},
    secio::SecioConfig,
    swarm::{IntoProtocolsHandler, NetworkBehaviour, ProtocolsHandler, SwarmBuilder},
    yamux, Multiaddr, PeerId, Transport,
};
use std::{future::Future, pin::Pin, time::Duration};

/// An adaptor struct for libp2p that spawns futures into the current
/// thread-local runtime.
struct GlobalSpawnTokioExecutor;

impl libp2p::core::Executor for GlobalSpawnTokioExecutor {
    fn exec(&self, future: Pin<Box<dyn Future<Output = ()> + Send>>) {
        let _ = tokio::spawn(future);
    }
}

pub fn new<B: NetworkBehaviour>(behaviour: B) -> (libp2p::Swarm<B>, Multiaddr, PeerId) where <<<B as NetworkBehaviour>::ProtocolsHandler as IntoProtocolsHandler>::Handler as ProtocolsHandler>::InEvent: Clone{
    let keypair = libp2p::identity::Keypair::generate_ed25519();
    let peer_id = PeerId::from(keypair.public());

    let transport = libp2p::core::transport::memory::MemoryTransport::default()
        .upgrade(Version::V1)
        .authenticate(SecioConfig::new(keypair))
        .multiplex(yamux::Config::default())
        .map(|(peer, muxer), _| (peer, StreamMuxerBox::new(muxer)))
        .timeout(Duration::from_secs(5))
        .boxed();

    let mut swarm: libp2p::Swarm<B> = SwarmBuilder::new(transport, behaviour, peer_id.clone())
        .executor(Box::new(GlobalSpawnTokioExecutor))
        .build();

    let address_port = rand::random::<u64>();
    let addr = format!("/memory/{}", address_port)
        .parse::<Multiaddr>()
        .unwrap();

    libp2p::Swarm::listen_on(&mut swarm, addr.clone()).unwrap();

    (swarm, addr, peer_id)
}
