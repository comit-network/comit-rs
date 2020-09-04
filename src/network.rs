use crate::{
    bitcoin, ethereum, ethereum::dai, order::BtcDaiOrderForm, swap::SwapKind, Seed, SwapId,
};
use ::bitcoin::hashes::{sha256, Hash, HashEngine};

use comit::{
    identity,
    network::{
        orderbook,
        protocols::setup_swap::{BobParams, RoleDependentParams},
        setup_swap,
        setup_swap::{AliceParams, CommonParams},
    },
    order::SwapProtocol,
    orderpool::Match,
    BtcDaiOrder, Position, Role, Secret, SecretHash,
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

use crate::swap::{Database, SwapParams};
use chrono::{NaiveDateTime, Utc};
use time::{Duration, OffsetDateTime};

#[derive(derivative::Derivative)]
#[derivative(Debug)]
pub struct Swarm {
    #[derivative(Debug = "ignore")]
    inner: libp2p::Swarm<Nectar>,
    local_peer_id: PeerId,
}

impl Swarm {
    pub fn new(
        seed: &Seed,
        settings: &crate::config::Settings,
        bitcoin_wallet: Arc<bitcoin::Wallet>,
        ethereum_wallet: Arc<ethereum::Wallet>,
        database: Arc<Database>,
    ) -> anyhow::Result<Self> {
        use anyhow::Context as _;

        let local_key_pair = derive_key_pair(seed);
        let local_peer_id = PeerId::from(local_key_pair.public());

        let transport = transport::build_transport(local_key_pair.clone())?;

        let behaviour = Nectar::new(
            local_peer_id.clone(),
            local_key_pair,
            settings.ethereum.chain.dai_contract_address(),
            bitcoin_wallet,
            ethereum_wallet,
            database,
        );

        let mut swarm =
            libp2p::swarm::SwarmBuilder::new(transport, behaviour, local_peer_id.clone())
                .executor(Box::new(TokioExecutor {
                    handle: tokio::runtime::Handle::current(),
                }))
                .build();
        for addr in settings.network.listen.clone() {
            libp2p::Swarm::listen_on(&mut swarm, addr.clone())
                .with_context(|| format!("Address is not supported: {:?}", addr))?;
        }

        Ok(Self {
            inner: swarm,
            local_peer_id,
        })
    }

    pub fn as_inner(&mut self) -> &mut libp2p::Swarm<Nectar> {
        &mut self.inner
    }

    pub fn publish(&mut self, order: BtcDaiOrder) {
        tracing::info!("Publishing new order");
        self.inner.publish(order);
    }

    pub fn setup_swap(
        &mut self,
        to: &PeerId,
        to_send: RoleDependentParams,
        common: CommonParams,
        swap_protocol: comit::network::setup_swap::SwapProtocol,
        swap_id: SwapId,
        match_ref_point: OffsetDateTime,
    ) -> anyhow::Result<()> {
        tracing::info!("Sending setup swap message");
        self.inner.setup_swap.send(
            to,
            to_send,
            common,
            swap_protocol,
            SetupSwapContext {
                swap_id,
                match_ref_point,
            },
        )?;
        Ok(())
    }

    pub fn clear_own_orders(&mut self) {
        tracing::info!("Cancelling all current orders");
        self.inner.orderbook.clear_own_orders();
    }
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
    },
}

#[derive(Debug, Copy, Clone)]
pub struct SetupSwapContext {
    swap_id: SwapId,
    match_ref_point: OffsetDateTime,
}

/// A `NetworkBehaviour` that delegates to the `Orderbook` and `SetupSwap`
/// behaviours.
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "Event", poll_method = "poll")]
#[allow(missing_debug_implementations)]
pub struct Nectar {
    orderbook: orderbook::Orderbook,
    setup_swap: setup_swap::SetupSwap<SetupSwapContext>,
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
        local_peer_id: PeerId,
        local_key_pair: Keypair,
        dai_contract_address: ethereum::Address,
        bitcoin_wallet: Arc<bitcoin::Wallet>,
        ethereum_wallet: Arc<ethereum::Wallet>,
        database: Arc<Database>,
    ) -> Self {
        Self {
            orderbook: comit::network::Orderbook::new(local_peer_id, local_key_pair),
            setup_swap: Default::default(),
            events: VecDeque::new(),
            dai_contract_address,
            bitcoin_wallet,
            ethereum_wallet,
            database,
        }
    }

    fn publish(&mut self, order: BtcDaiOrder) {
        self.orderbook.publish(order);
    }

    fn seed(&self) -> Seed {
        // todo: see if there is a better place to store the seed so it doesnt have to
        // be pulled from bitcoin wallet
        self.bitcoin_wallet.seed()
    }

    fn derive_secret_hash(&self, swap_id: SwapId) -> SecretHash {
        let secret: Secret =
            Self::sha256_with_seed(self.seed(), &[b"SECRET", swap_id.as_bytes()]).into();
        SecretHash::new(secret)
    }

    fn sha256_with_seed(seed: Seed, slices: &[&[u8]]) -> [u8; 32] {
        let mut engine = sha256::HashEngine::default();

        engine.input(&seed.bytes());
        engine.input(b"TRANSIENT_KEY");
        for slice in slices {
            engine.input(slice);
        }

        let hash = sha256::Hash::from_engine(engine);

        hash.into_inner()
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
                let secret_hash = self.derive_secret_hash(swap_id);

                let ethereum_identity = self.ethereum_wallet.account();
                let bitcoin_identity = identity::Bitcoin::from_secret_key(
                    &crate::SECP,
                    &self.bitcoin_wallet.derive_transient_sk(swap_id),
                );

                let erc20_quantity = quantity.as_sat() * price;

                let form = BtcDaiOrderForm {
                    position: our_position,
                    base: bitcoin::Asset {
                        amount: quantity.into(),
                        network: self.bitcoin_network(),
                    },
                    quote: dai::Asset {
                        amount: dai::Amount::from(erc20_quantity.clone()),
                        chain: ethereum::Chain::Local {
                            chain_id: u32::from(self.ethereum_chain_id()),
                            dai_contract_address: token_contract,
                        },
                    },
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
                                    bitcoin: quantity,
                                    ethereum_absolute_expiry,
                                    bitcoin_absolute_expiry,
                                    ethereum_chain_id: self.ethereum_chain_id(),
                                    bitcoin_network: self.bitcoin_network(),
                                },
                                comit::network::setup_swap::SwapProtocol::HbitHerc20,
                            ),
                            Position::Sell => (
                                RoleDependentParams::Alice(AliceParams {
                                    bitcoin_identity,
                                    ethereum_identity,
                                    secret_hash,
                                }),
                                CommonParams {
                                    erc20: comit::asset::Erc20 {
                                        token_contract,
                                        quantity: erc20_quantity,
                                    },
                                    bitcoin: quantity,
                                    ethereum_absolute_expiry,
                                    bitcoin_absolute_expiry,
                                    ethereum_chain_id: self.ethereum_chain_id(),
                                    bitcoin_network: self.bitcoin_network(),
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
                                RoleDependentParams::Alice(AliceParams {
                                    bitcoin_identity,
                                    ethereum_identity,
                                    secret_hash,
                                }),
                                CommonParams {
                                    erc20: comit::asset::Erc20 {
                                        token_contract,
                                        quantity: erc20_quantity,
                                    },
                                    bitcoin: quantity,
                                    ethereum_absolute_expiry,
                                    bitcoin_absolute_expiry,
                                    ethereum_chain_id: self.ethereum_chain_id(),
                                    bitcoin_network: self.bitcoin_network(),
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
                                    bitcoin: quantity,
                                    ethereum_absolute_expiry,
                                    bitcoin_absolute_expiry,
                                    ethereum_chain_id: self.ethereum_chain_id(),
                                    bitcoin_network: self.bitcoin_network(),
                                },
                                comit::network::setup_swap::SwapProtocol::Herc20Hbit,
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

                let transient_sk = self.bitcoin_wallet.derive_transient_sk(swap_id);
                let swap_kind = match (exec_swap.our_role, exec_swap.swap_protocol) {
                    // Sell
                    (Role::Alice, setup_swap::SwapProtocol::HbitHerc20) => {
                        SwapKind::HbitHerc20(SwapParams {
                            hbit_params: crate::swap::hbit::Params::new(
                                exec_swap.hbit,
                                transient_sk,
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
                                transient_sk,
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
                                transient_sk,
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
                                transient_sk,
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

fn derive_key_pair(seed: &Seed) -> libp2p::identity::Keypair {
    let mut engine = sha256::HashEngine::default();

    engine.input(&seed.bytes());
    engine.input(b"LIBP2P_KEYPAIR");

    let hash = sha256::Hash::from_engine(engine);
    let key = ed25519::SecretKey::from_bytes(hash.into_inner()).expect("we always pass 32 bytes");
    libp2p::identity::Keypair::Ed25519(key.into())
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
