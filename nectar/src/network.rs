use crate::{
    bitcoin, ethereum,
    order::BtcDaiOrderForm,
    swap::{Database, SwapKind, SwapParams},
    SwapId,
};
use ::bitcoin::hashes::{sha256, Hash, HashEngine};
use chrono::{NaiveDateTime, Utc};
use comit::{
    identity,
    network::{
        orderbook,
        protocols::setup_swap::{BobParams, RoleDependentParams},
        setup_swap,
        setup_swap::CommonParams,
    },
    order::SwapProtocol,
    orderpool::Match,
    Position, Role,
};
use futures::Future;
use libp2p::{
    identity::{ed25519, Keypair},
    swarm::{NetworkBehaviourAction, PollParameters},
    NetworkBehaviour, PeerId,
};
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};
use std::{
    collections::VecDeque,
    pin::Pin,
    str::FromStr,
    sync::Arc,
    task::{Context, Poll},
};
use time::{Duration, OffsetDateTime};

pub type Swarm = libp2p::Swarm<Nectar>;

pub const SEED_LENGTH: usize = 32;

pub fn new_swarm(
    seed: Seed,
    settings: &crate::config::Settings,
    bitcoin_wallet: Arc<bitcoin::Wallet>,
    ethereum_wallet: Arc<ethereum::Wallet>,
    database: Arc<Database>,
) -> anyhow::Result<Swarm> {
    use anyhow::Context as _;

    let behaviour = Nectar::new(
        seed,
        settings.ethereum.chain.dai_contract_address(),
        bitcoin_wallet,
        ethereum_wallet,
        database,
    );

    let local_key_pair = behaviour.identity();
    let local_peer_id = behaviour.peer_id();

    let transport = transport::build_transport(local_key_pair)?;

    let mut swarm = libp2p::swarm::SwarmBuilder::new(transport, behaviour, local_peer_id)
        .executor(Box::new(TokioExecutor {
            handle: tokio::runtime::Handle::current(),
        }))
        .build();
    for addr in settings.network.listen.clone() {
        Swarm::listen_on(&mut swarm, addr.clone())
            .with_context(|| format!("Address is not supported: {:?}", addr))?;
    }

    Ok(swarm)
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum Event {
    SpawnSwap(SwapKind),
    OrderMatch {
        form: BtcDaiOrderForm,
        to: PeerId,
        to_send: RoleDependentParams,
        common: CommonParams,
        swap_protocol: comit::network::setup_swap::SwapProtocol,
        swap_id: SwapId,
        match_ref_point: OffsetDateTime,
        bitcoin_transient_key_index: u32,
    },
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
#[behaviour(out_event = "Event", poll_method = "poll")]
#[allow(missing_debug_implementations)]
pub struct Nectar {
    pub orderbook: orderbook::Orderbook,
    pub setup_swap: setup_swap::SetupSwap<SetupSwapContext>,
    #[behaviour(ignore)]
    identity: Keypair,
    #[behaviour(ignore)]
    events: VecDeque<Event>,
    #[behaviour(ignore)]
    database: Arc<Database>,
    /// The address of the DAI ERC20 token contract on the current Ethereum
    /// network.
    #[behaviour(ignore)]
    dai_contract_address: ethereum::Address,
    #[behaviour(ignore)]
    bitcoin_wallet: Arc<bitcoin::Wallet>,
    #[behaviour(ignore)]
    ethereum_wallet: Arc<ethereum::Wallet>,
}

impl Nectar {
    fn new(
        seed: Seed,
        dai_contract_address: ethereum::Address,
        bitcoin_wallet: Arc<bitcoin::Wallet>,
        ethereum_wallet: Arc<ethereum::Wallet>,
        database: Arc<Database>,
    ) -> Self {
        let identity = seed.derive_libp2p_identity();
        let peer_id = PeerId::from(identity.public());

        Self {
            orderbook: comit::network::Orderbook::new(peer_id, identity.clone()),
            identity,
            setup_swap: Default::default(),
            events: VecDeque::new(),
            dai_contract_address,
            bitcoin_wallet,
            ethereum_wallet,
            database,
        }
    }

    pub fn identity(&self) -> Keypair {
        self.identity.clone()
    }

    pub fn peer_id(&self) -> PeerId {
        PeerId::from(self.identity.public())
    }

    fn ethereum_chain_id(&self) -> ethereum::ChainId {
        self.ethereum_wallet.chain_id()
    }

    fn bitcoin_network(&self) -> bitcoin::Network {
        self.bitcoin_wallet.network
    }

    fn poll<BIE>(
        &mut self,
        _cx: &mut Context<'_>,
        _params: &mut impl PollParameters,
    ) -> Poll<NetworkBehaviourAction<BIE, Event>> {
        if let Some(event) = self.events.pop_front() {
            return Poll::Ready(NetworkBehaviourAction::GenerateEvent(event));
        }

        // We trust in libp2p to poll us.
        Poll::Pending
    }
}

impl libp2p::swarm::NetworkBehaviourEventProcess<::comit::network::orderbook::BehaviourOutEvent>
    for Nectar
{
    fn inject_event(&mut self, event: ::comit::network::orderbook::BehaviourOutEvent) {
        match event {
            orderbook::BehaviourOutEvent::OrderMatch(Match {
                peer,
                price,
                quantity,
                our_position,
                swap_protocol,
                match_reference_point,
                ours,
                ..
            }) => {
                // TODO: Just push this to the stream and process it in `trade.rs`.
                let taker = ActivePeer {
                    peer_id: peer.clone(),
                };

                let ongoing_trade_with_taker_exists = match self
                    .database
                    .contains_active_peer(&taker)
                {
                    Ok(res) => res,
                    Err(e) => {
                        tracing::error!(
                            "could not determine if taker has ongoing trade: {}; taker: {}, order: {}",
                            e,
                            taker.peer_id(),
                            ours,
                        );
                        return;
                    }
                };

                if ongoing_trade_with_taker_exists {
                    tracing::warn!(
                        "ignoring take order request from taker with ongoing trade, taker: {:?}, order: {}",
                        taker.peer_id(),
                        ours,
                    );
                    return;
                }

                let token_contract = self.dai_contract_address;
                let swap_id = SwapId::default();
                let index = match self.database.fetch_inc_bitcoin_transient_key_index() {
                    Err(err) => {
                        tracing::error!(
                            "Could not fetch the index for the Bitcoin transient key: {:#}",
                            err
                        );
                        return;
                    }
                    Ok(index) => index,
                };

                let ethereum_identity = self.ethereum_wallet.account();
                let bitcoin_transient_sk = match self.bitcoin_wallet.derive_transient_sk(index) {
                    Ok(sk) => sk,
                    Err(err) => {
                        tracing::error!("Could not derive Bitcoin transient key: {:?}", err);
                        return;
                    }
                };

                let bitcoin_identity =
                    identity::Bitcoin::from_secret_key(&crate::SECP, &bitcoin_transient_sk);

                let erc20_quantity = quantity * price.clone();

                let form = BtcDaiOrderForm {
                    position: our_position,
                    quantity,
                    price,
                };

                let (role_dependant_params, common_params, swap_protocol) = match swap_protocol {
                    SwapProtocol::HbitHerc20 {
                        hbit_expiry_offset,
                        herc20_expiry_offset,
                    } => {
                        // todo: do checked addition
                        #[allow(clippy::cast_sign_loss)]
                        #[allow(clippy::cast_possible_truncation)]
                        let ethereum_absolute_expiry = (match_reference_point
                            + Duration::from(herc20_expiry_offset))
                        .timestamp() as u32;
                        #[allow(clippy::cast_sign_loss)]
                        #[allow(clippy::cast_possible_truncation)]
                        let bitcoin_absolute_expiry = (match_reference_point
                            + Duration::from(hbit_expiry_offset))
                        .timestamp() as u32;

                        match our_position {
                            Position::Buy => (
                                RoleDependentParams::Bob(BobParams {
                                    bitcoin_identity,
                                    ethereum_identity,
                                }),
                                CommonParams {
                                    erc20: comit::asset::Erc20 {
                                        token_contract,
                                        quantity: erc20_quantity,
                                    },
                                    bitcoin: quantity.to_inner(),
                                    ethereum_absolute_expiry,
                                    bitcoin_absolute_expiry,
                                    ethereum_chain_id: self.ethereum_chain_id(),
                                    bitcoin_network: self.bitcoin_network().into(),
                                },
                                comit::network::setup_swap::SwapProtocol::HbitHerc20,
                            ),
                            Position::Sell => (
                                RoleDependentParams::Bob(BobParams {
                                    bitcoin_identity,
                                    ethereum_identity,
                                }),
                                CommonParams {
                                    erc20: comit::asset::Erc20 {
                                        token_contract,
                                        quantity: erc20_quantity,
                                    },
                                    bitcoin: quantity.to_inner(),
                                    ethereum_absolute_expiry,
                                    bitcoin_absolute_expiry,
                                    ethereum_chain_id: self.ethereum_chain_id(),
                                    bitcoin_network: self.bitcoin_network().into(),
                                },
                                comit::network::setup_swap::SwapProtocol::HbitHerc20,
                            ),
                        }
                    }
                    SwapProtocol::Herc20Hbit {
                        hbit_expiry_offset,
                        herc20_expiry_offset,
                    } => {
                        // todo: do checked addition
                        #[allow(clippy::cast_sign_loss)]
                        #[allow(clippy::cast_possible_truncation)]
                        let ethereum_absolute_expiry = (match_reference_point
                            + Duration::from(herc20_expiry_offset))
                        .timestamp() as u32;
                        #[allow(clippy::cast_sign_loss)]
                        #[allow(clippy::cast_possible_truncation)]
                        let bitcoin_absolute_expiry = (match_reference_point
                            + Duration::from(hbit_expiry_offset))
                        .timestamp() as u32;

                        match our_position {
                            Position::Buy => (
                                RoleDependentParams::Bob(BobParams {
                                    bitcoin_identity,
                                    ethereum_identity,
                                }),
                                CommonParams {
                                    erc20: comit::asset::Erc20 {
                                        token_contract,
                                        quantity: erc20_quantity,
                                    },
                                    bitcoin: quantity.to_inner(),
                                    ethereum_absolute_expiry,
                                    bitcoin_absolute_expiry,
                                    ethereum_chain_id: self.ethereum_chain_id(),
                                    bitcoin_network: self.bitcoin_network().into(),
                                },
                                comit::network::setup_swap::SwapProtocol::Herc20Hbit,
                            ),
                            Position::Sell => (
                                RoleDependentParams::Bob(BobParams {
                                    bitcoin_identity,
                                    ethereum_identity,
                                }),
                                CommonParams {
                                    erc20: comit::asset::Erc20 {
                                        token_contract,
                                        quantity: erc20_quantity,
                                    },
                                    bitcoin: quantity.to_inner(),
                                    ethereum_absolute_expiry,
                                    bitcoin_absolute_expiry,
                                    ethereum_chain_id: self.ethereum_chain_id(),
                                    bitcoin_network: self.bitcoin_network().into(),
                                },
                                comit::network::setup_swap::SwapProtocol::HbitHerc20,
                            ),
                        }
                    }
                };

                self.events.push_back(Event::OrderMatch {
                    form,
                    to: peer,
                    to_send: role_dependant_params,
                    common: common_params,
                    swap_protocol,
                    swap_id,
                    match_ref_point: match_reference_point,
                    bitcoin_transient_key_index: index,
                });
            }
        }
    }
}

impl
    libp2p::swarm::NetworkBehaviourEventProcess<
        ::comit::network::setup_swap::BehaviourOutEvent<SetupSwapContext>,
    > for Nectar
{
    fn inject_event(
        &mut self,
        event: ::comit::network::setup_swap::BehaviourOutEvent<SetupSwapContext>,
    ) {
        match event {
            ::comit::network::setup_swap::BehaviourOutEvent::ExecutableSwap(exec_swap) => {
                let swap_id = exec_swap.context.swap_id;

                let start_of_swap = chrono::DateTime::from_utc(
                    NaiveDateTime::from_timestamp(exec_swap.context.match_ref_point.timestamp(), 0),
                    Utc,
                );

                let bitcoin_transient_sk = match self
                    .bitcoin_wallet
                    .derive_transient_sk(exec_swap.context.bitcoin_transient_key_index)
                {
                    Ok(sk) => sk,
                    Err(err) => {
                        tracing::error!("Could not derive Bitcoin transient key: {:?}", err);
                        return;
                    }
                };

                let swap_kind = match (exec_swap.our_role, exec_swap.swap_protocol) {
                    // Sell
                    (Role::Alice, setup_swap::SwapProtocol::HbitHerc20) => {
                        SwapKind::HbitHerc20(SwapParams {
                            hbit_params: crate::swap::hbit::Params::new(
                                exec_swap.hbit,
                                bitcoin_transient_sk,
                            ),
                            herc20_params: crate::swap::herc20::Params {
                                asset: exec_swap.herc20.asset.clone(),
                                redeem_identity: exec_swap.herc20.refund_identity,
                                refund_identity: exec_swap.herc20.redeem_identity,
                                expiry: exec_swap.herc20.expiry,
                                secret_hash: exec_swap.herc20.secret_hash,
                                chain_id: exec_swap.herc20.chain_id,
                            },
                            secret_hash: exec_swap.hbit.secret_hash,
                            start_of_swap,
                            swap_id,
                            taker: ActivePeer {
                                peer_id: exec_swap.peer_id,
                            },
                        })
                    }
                    // Buy
                    (Role::Bob, setup_swap::SwapProtocol::HbitHerc20) => {
                        SwapKind::HbitHerc20(SwapParams {
                            hbit_params: crate::swap::hbit::Params::new(
                                exec_swap.hbit,
                                bitcoin_transient_sk,
                            ),
                            herc20_params: crate::swap::herc20::Params {
                                asset: exec_swap.herc20.asset.clone(),
                                redeem_identity: exec_swap.herc20.redeem_identity,
                                refund_identity: exec_swap.herc20.refund_identity,
                                expiry: exec_swap.herc20.expiry,
                                secret_hash: exec_swap.herc20.secret_hash,
                                chain_id: exec_swap.herc20.chain_id,
                            },
                            secret_hash: exec_swap.hbit.secret_hash,
                            start_of_swap,
                            swap_id,
                            taker: ActivePeer {
                                peer_id: exec_swap.peer_id,
                            },
                        })
                    }
                    // Buy
                    (Role::Alice, setup_swap::SwapProtocol::Herc20Hbit) => {
                        SwapKind::Herc20Hbit(SwapParams {
                            hbit_params: crate::swap::hbit::Params::new(
                                exec_swap.hbit,
                                bitcoin_transient_sk,
                            ),
                            herc20_params: crate::swap::herc20::Params {
                                asset: exec_swap.herc20.asset.clone(),
                                redeem_identity: exec_swap.herc20.redeem_identity,
                                refund_identity: exec_swap.herc20.refund_identity,
                                expiry: exec_swap.herc20.expiry,
                                secret_hash: exec_swap.herc20.secret_hash,
                                chain_id: exec_swap.herc20.chain_id,
                            },
                            secret_hash: exec_swap.hbit.secret_hash,
                            start_of_swap,
                            swap_id,
                            taker: ActivePeer {
                                peer_id: exec_swap.peer_id,
                            },
                        })
                    }
                    // Sell
                    (Role::Bob, setup_swap::SwapProtocol::Herc20Hbit) => {
                        SwapKind::Herc20Hbit(SwapParams {
                            hbit_params: crate::swap::hbit::Params::new(
                                exec_swap.hbit,
                                bitcoin_transient_sk,
                            ),
                            herc20_params: crate::swap::herc20::Params {
                                asset: exec_swap.herc20.asset.clone(),
                                redeem_identity: exec_swap.herc20.redeem_identity,
                                refund_identity: exec_swap.herc20.refund_identity,
                                expiry: exec_swap.herc20.expiry,
                                secret_hash: exec_swap.herc20.secret_hash,
                                chain_id: exec_swap.herc20.chain_id,
                            },
                            secret_hash: exec_swap.hbit.secret_hash,
                            start_of_swap,
                            swap_id,
                            taker: ActivePeer {
                                peer_id: exec_swap.peer_id,
                            },
                        })
                    }
                };
                self.events.push_back(Event::SpawnSwap(swap_kind));
            }
            ::comit::network::setup_swap::BehaviourOutEvent::AlreadyHaveRoleParams {
                peer, ..
            } => tracing::error!("already received role params from {}", peer),
        }
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
        secio::{SecioConfig, SecioError},
        tcp::TokioTcpConfig,
        yamux, PeerId,
    };
    use std::time::Duration;

    pub type NectarTransport = Boxed<
        (PeerId, StreamMuxerBox),
        TransportTimeoutError<
            EitherError<
                EitherError<DnsErr<std::io::Error>, UpgradeError<SecioError>>,
                UpgradeError<EitherError<std::io::Error, std::io::Error>>,
            >,
        >,
    >;

    /// Builds a libp2p transport with the following features:
    /// - TcpConnection
    /// - DNS name resolution
    /// - authentication via secio
    /// - multiplexing via yamux or mplex
    pub fn build_transport(keypair: libp2p::identity::Keypair) -> anyhow::Result<NectarTransport> {
        let transport = TokioTcpConfig::new().nodelay(true);
        let transport = DnsConfig::new(transport)?;

        let transport = transport
            .upgrade(Version::V1)
            .authenticate(SecioConfig::new(keypair))
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
