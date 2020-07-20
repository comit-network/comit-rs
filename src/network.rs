mod active_takers;

use crate::{
    bitcoin,
    ethereum::dai::DaiContractAddress,
    order::{BtcDaiOrder, Position},
    swap::{hbit, herc20, SwapKind, SwapParams},
    Seed, SwapId,
};
use bimap::BiMap;
use chrono::Utc;
use comit::{
    asset,
    ethereum::ChainId,
    identity,
    network::{
        self,
        orderbook::{self, take_order, OrderId},
        Comit, Identities, LocalData, Orderbook, RemoteData,
    },
    SharedSwapId, Timestamp,
};
use futures::Future;
use libp2p::{
    identity::ed25519,
    request_response::ResponseChannel,
    swarm::{NetworkBehaviourAction, NetworkBehaviourEventProcess, PollParameters},
    NetworkBehaviour, PeerId,
};
use network::{SwapType, TradingPair};
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, VecDeque},
    pin::Pin,
    str::FromStr,
    task::{Context, Poll},
};

pub use active_takers::ActiveTakers;

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
        task_executor: tokio::runtime::Handle,
    ) -> anyhow::Result<Self> {
        use anyhow::Context as _;

        let local_key_pair = derive_key_pair(seed);
        let local_peer_id = PeerId::from(local_key_pair.public());

        let transport = transport::build_transport(local_key_pair)?;

        #[cfg(not(test))]
        let active_takers = ActiveTakers::new(&settings.data.dir.join("active_takers"))?;
        #[cfg(test)]
        let active_takers = ActiveTakers::new_test()?;

        let behaviour = Nectar::new(local_peer_id.clone(), active_takers);

        let mut swarm =
            libp2p::swarm::SwarmBuilder::new(transport, behaviour, local_peer_id.clone())
                .executor(Box::new(TokioExecutor {
                    handle: task_executor,
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

    pub fn as_inner(&self) -> &libp2p::Swarm<Nectar> {
        &self.inner
    }

    pub fn announce_btc_dai_trading_pair(&mut self) -> anyhow::Result<()> {
        // The comit::network::Orderbook requires announcing the
        // trading pair in both directions

        self.inner.announce_trading_pair(TradingPair {
            buy: SwapType::Hbit,
            sell: SwapType::Herc20,
        })?;

        self.inner.announce_trading_pair(TradingPair {
            buy: SwapType::Herc20,
            sell: SwapType::Hbit,
        })?;

        Ok(())
    }

    pub fn publish(&mut self, order: PublishOrder) -> anyhow::Result<()> {
        self.inner.make(order)
    }

    pub fn confirm(&mut self, order: TakenOrder) -> anyhow::Result<()> {
        self.inner.confirm(order)
    }

    pub fn deny(&mut self, order: TakenOrder) -> anyhow::Result<()> {
        self.inner.deny(order)
    }

    /// Save the swap identities and send them to the taker
    pub fn set_swap_identities(
        &mut self,
        swap_metadata: SwapMetadata,
        bitcoin_transient_sk: ::bitcoin::secp256k1::SecretKey,
        ethereum_identity: identity::Ethereum,
    ) {
        self.inner
            .set_swap_identities(swap_metadata, bitcoin_transient_sk, ethereum_identity)
    }

    pub fn remove_from_active_takers(&mut self, taker: &Taker) -> anyhow::Result<()> {
        self.inner.remove_from_active_takers(&taker)
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum Event {
    TakeOrderRequest(TakenOrder),
    SetSwapIdentities(SwapMetadata),
    SpawnSwap(SwapKind),
}

#[derive(Debug)]
pub struct SwapMetadata {
    shared_swap_id: SharedSwapId,
    taker_peer_id: PeerId,
}

/// A `NetworkBehaviour` that delegates to the `Orderbook` and `Comit` behaviours.
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "Event", poll_method = "poll")]
#[allow(missing_debug_implementations)]
pub struct Nectar {
    comit: Comit,
    orderbook: Orderbook,
    #[behaviour(ignore)]
    events: VecDeque<Event>,
    #[behaviour(ignore)]
    local_peer_id: PeerId,
    #[behaviour(ignore)]
    takers: HashMap<OrderId, PeerId>,
    #[behaviour(ignore)]
    active_takers: ActiveTakers,
    #[behaviour(ignore)]
    order_swap_ids: BiMap<OrderId, SharedSwapId>,
    #[behaviour(ignore)]
    bitcoin_transient_sks: HashMap<SharedSwapId, ::bitcoin::secp256k1::SecretKey>,
    #[behaviour(ignore)]
    local_data: HashMap<SharedSwapId, LocalData>,
}

impl Nectar {
    fn new(local_peer_id: PeerId, active_takers: ActiveTakers) -> Self {
        Self {
            comit: Comit::default(),
            orderbook: Orderbook::new(local_peer_id.clone()),
            events: VecDeque::new(),
            local_peer_id,
            takers: HashMap::new(),
            active_takers,
            order_swap_ids: BiMap::new(),
            bitcoin_transient_sks: HashMap::new(),
            local_data: HashMap::new(),
        }
    }

    fn announce_trading_pair(&mut self, trading_pair: TradingPair) -> anyhow::Result<()> {
        self.orderbook.announce_trading_pair(trading_pair)
    }

    fn make(&mut self, order: PublishOrder) -> anyhow::Result<()> {
        let order = orderbook::Order::new(self.local_peer_id.clone(), order.into());
        let _order_id = self.orderbook.make(order)?;

        Ok(())
    }

    fn confirm(&mut self, order: TakenOrder) -> anyhow::Result<()> {
        self.active_takers.insert(order.taker)?;

        self.orderbook.confirm(order.id, order.confirmation_channel);

        Ok(())
    }

    fn deny(&mut self, order: TakenOrder) -> anyhow::Result<()> {
        self.active_takers.remove(&order.taker)?;

        self.orderbook
            .deny(order.taker.peer_id(), order.id, order.confirmation_channel);

        Ok(())
    }

    /// Save the swap identities and send them to the taker
    fn set_swap_identities(
        &mut self,
        SwapMetadata {
            shared_swap_id,
            taker_peer_id,
        }: SwapMetadata,
        bitcoin_transient_sk: ::bitcoin::secp256k1::SecretKey,
        ethereum_identity: identity::Ethereum,
    ) {
        // TODO: Saving this and the bitcoin identity inside
        // `LocalData` is redundant. It may be better to just save the
        // bitcoin transient key and the ethereum identity
        self.bitcoin_transient_sks
            .insert(shared_swap_id, bitcoin_transient_sk);

        let bitcoin_identity =
            identity::Bitcoin::from_secret_key(&crate::SECP, &bitcoin_transient_sk);

        let identities = Identities {
            bitcoin_identity: Some(bitcoin_identity),
            ethereum_identity: Some(ethereum_identity),
            lightning_identity: None,
        };
        let local_data = LocalData::for_bob(identities);

        self.local_data.insert(shared_swap_id, local_data);

        self.comit
            .communicate(taker_peer_id, shared_swap_id, local_data);
    }

    pub fn remove_from_active_takers(&mut self, taker: &Taker) -> anyhow::Result<()> {
        self.active_takers.remove(taker)
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

impl NetworkBehaviourEventProcess<orderbook::BehaviourOutEvent> for Nectar {
    fn inject_event(&mut self, event: orderbook::BehaviourOutEvent) {
        match event {
            orderbook::BehaviourOutEvent::TakeOrderRequest {
                peer_id: taker_peer_id,
                response_channel,
                order_id,
            } => {
                let taker = Taker::new(taker_peer_id);
                let ongoing_trade_with_taker_exists = match self.active_takers.contains(&taker) {
                    Ok(res) => res,
                    Err(e) => {
                        tracing::error!(
                            "could not determine if taker has ongoing trade: {}; taker: {}, order: {}",
                            e,
                            taker.peer_id(),
                            order_id,
                        );
                        return;
                    }
                };

                if ongoing_trade_with_taker_exists {
                    tracing::warn!(
                        "ignoring take order request from taker with ongoing trade, taker: {:?}, order: {}",
                        taker.peer_id(),
                        order_id,
                    );
                    return;
                }

                let order = match self.orderbook.get_order(&order_id) {
                    Some(order) => order,
                    None => {
                        tracing::warn!(
                            "unexpected take order request, taker: {}, order: {}",
                            taker.peer_id(),
                            order_id,
                        );
                        return;
                    }
                };

                self.takers.insert(order_id, taker.peer_id.clone());

                let taken_order = TakenOrder::new(order, taker, response_channel);
                self.events.push_back(Event::TakeOrderRequest(taken_order))
            }
            orderbook::BehaviourOutEvent::TakeOrderConfirmation {
                order_id,
                shared_swap_id,
            } => {
                let taker_peer_id = match self.takers.get(&order_id) {
                    Some(taker) => taker,
                    None => {
                        tracing::warn!("unknown taker for order: {}", order_id,);
                        return;
                    }
                };

                self.order_swap_ids.insert(order_id, shared_swap_id);

                self.events
                    .push_back(Event::SetSwapIdentities(SwapMetadata {
                        shared_swap_id,
                        taker_peer_id: taker_peer_id.clone(),
                    }))
            }
            orderbook::BehaviourOutEvent::Failed { peer_id, order_id } => tracing::warn!(
                "take order request failed, peer: {}, order: {}",
                peer_id,
                order_id,
            ),
        }
    }
}

impl NetworkBehaviourEventProcess<network::comit::BehaviourOutEvent> for Nectar {
    fn inject_event(&mut self, event: network::comit::BehaviourOutEvent) {
        match event {
            network::comit::BehaviourOutEvent::SwapFinalized {
                shared_swap_id,
                remote_data,
            } => {
                let (secret_hash, taker_bitcoin_identity, taker_ethereum_identity) =
                    match remote_data {
                        RemoteData {
                            secret_hash: Some(secret_hash),
                            bitcoin_identity: Some(bitcoin_identity),
                            ethereum_identity: Some(ethereum_identity),
                            ..
                        } => (secret_hash, bitcoin_identity, ethereum_identity),
                        _ => {
                            tracing::warn!(
                                "incorrect remote data received from taker, shared_swap_id: {}, remote_data: {:?}",
                                shared_swap_id,
                                remote_data
                            );
                            return;
                        }
                    };

                let (maker_bitcoin_identity, maker_ethereum_identity) =
                    match self.local_data.remove(&shared_swap_id) {
                        Some(LocalData {
                            bitcoin_identity: Some(bitcoin_identity),
                            ethereum_identity: Some(ethereum_identity),
                            ..
                        }) => (bitcoin_identity, ethereum_identity),
                        _ => {
                            tracing::warn!(
                                "could not find identities for shared_swap_id: {}",
                                shared_swap_id,
                            );
                            return;
                        }
                    };

                let bitcoin_transient_sk = match self.bitcoin_transient_sks.remove(&shared_swap_id)
                {
                    Some(bitcoin_transient_sk) => bitcoin_transient_sk,
                    None => {
                        tracing::warn!(
                            "could not find bitcoin transient sk for shared_swap_id: {}",
                            shared_swap_id,
                        );
                        return;
                    }
                };

                // TODO: Handle sell orders when orderbook supports them
                let (
                    network::Order {
                        buy: satoshi_amount,
                        sell: erc20_asset,
                        absolute_expiry,
                        ..
                    },
                    taker,
                ) = match self.order_swap_ids.get_by_right(&shared_swap_id) {
                    Some(order_id) => {
                        let order = match self.orderbook.get_order(order_id) {
                            Some(order) => order,
                            None => {
                                tracing::warn!("could not find order with id, id: {}", order_id);
                                return;
                            }
                        };

                        let taker = match self.takers.get(order_id) {
                            Some(taker_peer_id) => Taker::new(taker_peer_id.clone()),
                            None => {
                                tracing::warn!("unknown taker for order: {}", order_id,);
                                return;
                            }
                        };

                        (order, taker)
                    }
                    None => {
                        tracing::warn!(
                            "could order_id corresponding to shared_swap_id: {}",
                            shared_swap_id
                        );
                        return;
                    }
                };

                // TODO: Handle expiries properly when orderbook does so

                let hbit_params = hbit::Params::new(
                    hbit::SharedParams {
                        asset: asset::Bitcoin::from_sat(satoshi_amount),
                        redeem_identity: maker_bitcoin_identity,
                        refund_identity: taker_bitcoin_identity,
                        expiry: Timestamp::from(absolute_expiry),
                        secret_hash,
                        // TODO: Make it dynamic when orderbook handles networks
                        network: ::bitcoin::Network::Regtest,
                    },
                    bitcoin_transient_sk,
                );

                let herc20_params = herc20::Params {
                    asset: erc20_asset,
                    redeem_identity: taker_ethereum_identity,
                    refund_identity: maker_ethereum_identity,
                    expiry: Timestamp::from(absolute_expiry),
                    secret_hash,
                    // TODO: Make it dynamic once orderbook handles networks
                    chain_id: ChainId::regtest(),
                };

                let swap = SwapKind::HbitHerc20(SwapParams {
                    hbit_params,
                    herc20_params,
                    secret_hash,
                    start_of_swap: Utc::now().naive_local(),
                    swap_id: SwapId::default(),
                    taker,
                });

                self.events.push_back(Event::SpawnSwap(swap));
            }
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

#[derive(Debug)]
pub struct TakenOrder {
    pub id: OrderId,
    pub inner: BtcDaiOrder,
    pub taker: Taker,
    confirmation_channel: ResponseChannel<take_order::Response>,
}

impl TakenOrder {
    fn new(
        order: comit::network::orderbook::Order,
        taker: Taker,
        confirmation_channel: ResponseChannel<take_order::Response>,
    ) -> Self {
        // TODO: comit::network::orderbook does not yet support selling Bitcoin
        let inner = BtcDaiOrder {
            position: Position::Buy,
            base: bitcoin::Amount::from_sat(order.buy),
            quote: order.sell.into(),
        };

        Self {
            id: order.id,
            inner,
            taker,
            confirmation_channel,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Taker {
    peer_id: PeerId,
}

impl Taker {
    pub fn new(peer_id: PeerId) -> Taker {
        Taker { peer_id }
    }

    pub fn peer_id(&self) -> PeerId {
        self.peer_id.clone()
    }
}

impl From<TakenOrder> for BtcDaiOrder {
    fn from(order: TakenOrder) -> Self {
        order.inner
    }
}

#[cfg(test)]
impl Default for Taker {
    fn default() -> Self {
        Self {
            peer_id: PeerId::random(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PublishOrder(BtcDaiOrder);

impl From<BtcDaiOrder> for PublishOrder {
    fn from(order: BtcDaiOrder) -> Self {
        Self(order)
    }
}

// TODO: The comit::network::orderbook expects a token contract but we
// assume DAI. Maybe we should use a type that combines dai::Amount
// and DaiContractAddress.

// The unwrap both cannot fail and should go away when we fix the TODO above
#[allow(clippy::fallible_impl_from)]
impl From<PublishOrder> for comit::network::orderbook::NewOrder {
    fn from(from: PublishOrder) -> Self {
        match from.0 {
            BtcDaiOrder {
                position: Position::Buy,
                base,
                quote,
            } => Self {
                buy: base.into(),
                sell: asset::Erc20::new(
                    // TODO: Handle other networks
                    DaiContractAddress::from_public_chain_id(ChainId::regtest())
                        .unwrap()
                        .into(),
                    quote.into(),
                ),
                // TODO: comit::network::orderbook currently only
                // supports defining one expiry. In cnd, this is used
                // for both Alpha and Beta. Eventually we will need to
                // handle this properly, but for now, let's set it to
                // 24 hours in the future
                absolute_expiry: Timestamp::now().plus(60 * 60 * 24).into(),
            },
            BtcDaiOrder {
                position: Position::Sell,
                ..
            } => todo!("comit::network::orderbook does not yet support selling Bitcoin"),
        }
    }
}

impl Serialize for Taker {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let string = self.peer_id.to_string();
        serializer.serialize_str(&string)
    }
}

impl<'de> Deserialize<'de> for Taker {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        let string = String::deserialize(deserializer)?;
        let peer_id = PeerId::from_str(&string).map_err(D::Error::custom)?;

        Ok(Taker { peer_id })
    }
}

fn derive_key_pair(seed: &Seed) -> libp2p::identity::Keypair {
    let mut sha = Sha256::new();
    sha.update(seed.bytes());
    sha.update(b"LIBP2P_KEYPAIR");

    let bytes = sha.finalize();
    let key = ed25519::SecretKey::from_bytes(bytes).expect("we always pass 32 bytes");
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
