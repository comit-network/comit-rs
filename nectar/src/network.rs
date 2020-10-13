use crate::SwapId;
use ::bitcoin::hashes::{sha256, Hash, HashEngine};
use comit::network::{orderbook, setup_swap};
use futures::Future;
use libp2p::{
    identity::{ed25519, Keypair},
    NetworkBehaviour, PeerId,
};
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};
use std::{pin::Pin, str::FromStr};
use time::OffsetDateTime;

pub type Swarm = libp2p::Swarm<Nectar>;

pub const SEED_LENGTH: usize = 32;

pub fn new_swarm(seed: Seed, settings: &crate::config::Settings) -> anyhow::Result<Swarm> {
    use anyhow::Context as _;

    let behaviour = Nectar::new(seed);

    let local_key_pair = behaviour.identity();
    let local_peer_id = behaviour.peer_id();

    let transport = transport::build_transport(local_key_pair)?;

    let mut swarm = libp2p::swarm::SwarmBuilder::new(transport, behaviour, local_peer_id.clone())
        .executor(Box::new(TokioExecutor {
            handle: tokio::runtime::Handle::current(),
        }))
        .build();
    for addr in settings.network.listen.clone() {
        Swarm::listen_on(&mut swarm, addr.clone())
            .with_context(|| format!("Address is not supported: {:#}", addr))?;
    }

    tracing::info!("Initialized swarm with identity {}", local_peer_id);

    Ok(swarm)
}

#[allow(clippy::large_enum_variant)]
pub enum BehaviourOutEvent {
    Orderbook(orderbook::BehaviourOutEvent),
    SetupSwap(setup_swap::BehaviourOutEvent<SetupSwapContext>),
}

impl From<orderbook::BehaviourOutEvent> for BehaviourOutEvent {
    fn from(event: orderbook::BehaviourOutEvent) -> Self {
        BehaviourOutEvent::Orderbook(event)
    }
}

impl From<setup_swap::BehaviourOutEvent<SetupSwapContext>> for BehaviourOutEvent {
    fn from(event: setup_swap::BehaviourOutEvent<SetupSwapContext>) -> Self {
        BehaviourOutEvent::SetupSwap(event)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct SetupSwapContext {
    pub swap_id: SwapId,
    pub bitcoin_transient_key_index: u32,
    pub match_ref_point: OffsetDateTime,
}

/// A `NetworkBehaviour` that delegates to the `Orderbook` and `SetupSwap`
/// behaviours.
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "BehaviourOutEvent", event_process = false)]
#[allow(missing_debug_implementations)]
pub struct Nectar {
    pub orderbook: orderbook::Orderbook,
    pub setup_swap: setup_swap::SetupSwap<SetupSwapContext>,
    #[behaviour(ignore)]
    identity: Keypair,
}

impl Nectar {
    fn new(seed: Seed) -> Self {
        let identity = seed.derive_libp2p_identity();
        let peer_id = PeerId::from(identity.public());

        Self {
            orderbook: comit::network::Orderbook::new(peer_id, identity.clone()),
            identity,
            setup_swap: Default::default(),
        }
    }

    pub fn identity(&self) -> Keypair {
        self.identity.clone()
    }

    pub fn peer_id(&self) -> PeerId {
        PeerId::from(self.identity.public())
    }
}

struct TokioExecutor {
    handle: tokio::runtime::Handle,
}

impl libp2p::core::Executor for TokioExecutor {
    fn exec(&self, future: Pin<Box<dyn Future<Output = ()> + Send>>) {
        let _ = self.handle.spawn(future);
    }
}

/// This type is used to track peers that have a swap ongoing
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ActivePeer {
    pub(crate) peer_id: PeerId,
}

impl ActivePeer {
    pub fn peer_id(&self) -> PeerId {
        self.peer_id.clone()
    }
}

#[cfg(test)]
impl crate::StaticStub for ActivePeer {
    fn static_stub() -> Self {
        Self {
            peer_id: PeerId::random(),
        }
    }
}

impl Serialize for ActivePeer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let string = self.peer_id.to_string();
        serializer.serialize_str(&string)
    }
}

impl<'de> Deserialize<'de> for ActivePeer {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        let string = String::deserialize(deserializer)?;
        let peer_id = PeerId::from_str(&string).map_err(D::Error::custom)?;

        Ok(ActivePeer { peer_id })
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Seed([u8; SEED_LENGTH]);

impl Seed {
    /// prefix "NETWORK" to the provided seed and apply sha256
    pub fn new(seed: [u8; crate::seed::SEED_LENGTH]) -> Self {
        let mut engine = sha256::HashEngine::default();

        engine.input(&seed);
        engine.input(b"NETWORK");

        let hash = sha256::Hash::from_engine(engine);
        Self(hash.into_inner())
    }

    pub fn bytes(&self) -> [u8; SEED_LENGTH] {
        self.0
    }

    pub fn derive_libp2p_identity(&self) -> libp2p::identity::Keypair {
        let mut engine = sha256::HashEngine::default();

        engine.input(&self.bytes());
        engine.input(b"LIBP2P_IDENTITY");

        let hash = sha256::Hash::from_engine(engine);
        let key =
            ed25519::SecretKey::from_bytes(hash.into_inner()).expect("we always pass 32 bytes");
        libp2p::identity::Keypair::Ed25519(key.into())
    }
}

mod transport {
    use libp2p::{
        core::{
            either::EitherError,
            muxing::StreamMuxerBox,
            transport::{boxed::Boxed, timeout::TransportTimeoutError, Transport},
            upgrade::{SelectUpgrade, Version},
            UpgradeError,
        },
        dns::{DnsConfig, DnsErr},
        mplex::MplexConfig,
        noise,
        noise::{NoiseConfig, X25519Spec},
        tcp::TokioTcpConfig,
        yamux, PeerId,
    };
    use std::time::Duration;

    pub type NectarTransport = Boxed<
        (PeerId, StreamMuxerBox),
        TransportTimeoutError<
            EitherError<
                EitherError<DnsErr<std::io::Error>, UpgradeError<noise::NoiseError>>,
                UpgradeError<EitherError<std::io::Error, std::io::Error>>,
            >,
        >,
    >;

    /// Builds a libp2p transport with the following features:
    /// - TcpConnection
    /// - DNS name resolution
    /// - authentication via noise
    /// - multiplexing via yamux or mplex
    pub fn build_transport(keypair: libp2p::identity::Keypair) -> anyhow::Result<NectarTransport> {
        let dh_keys = noise::Keypair::<X25519Spec>::new().into_authentic(&keypair)?;
        let noise = NoiseConfig::xx(dh_keys).into_authenticated();

        let transport = TokioTcpConfig::new().nodelay(true);
        let transport = DnsConfig::new(transport)?;

        let transport = transport
            .upgrade(Version::V1)
            .authenticate(noise)
            .multiplex(SelectUpgrade::new(
                yamux::Config::default(),
                MplexConfig::new(),
            ))
            .map(|(peer, muxer), _| (peer, StreamMuxerBox::new(muxer)))
            .timeout(Duration::from_secs(20))
            .boxed();

        Ok(transport)
    }
}

#[cfg(test)]
mod arbitrary {
    use super::*;
    use libp2p::multihash;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for ActivePeer {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let mut bytes = [0u8; 32];
            for byte in bytes.iter_mut() {
                *byte = u8::arbitrary(g);
            }
            let peer_id =
                PeerId::from_multihash(multihash::wrap(multihash::Code::Sha2_256, &bytes)).unwrap();
            ActivePeer { peer_id }
        }
    }
}
