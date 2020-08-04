use crate::{
    bitcoin,
    ethereum::{self, dai},
    order::{BtcDaiOrder, Position},
    swap::{hbit, herc20, SwapKind, SwapParams},
    Seed,
};
use ::bitcoin::hashes::{sha256, Hash, HashEngine};
use chrono::Utc;
use comit::{
    asset, identity,
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
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};
use std::{
    collections::{HashMap, VecDeque},
    pin::Pin,
    str::FromStr,
    task::{Context, Poll},
};

use crate::swap::Database;
use std::sync::Arc;

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
        database: Arc<Database>,
    ) -> anyhow::Result<Self> {
        use anyhow::Context as _;

        let local_key_pair = derive_key_pair(seed);
        let local_peer_id = PeerId::from(local_key_pair.public());

        let transport = transport::build_transport(local_key_pair)?;

        let behaviour = Nectar::new(local_peer_id.clone(), database);

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

    pub fn publish(&mut self, order: PublishOrder) -> anyhow::Result<()> {
        tracing::info!("Publishing new order: {}", order.0);
        self.inner.make(order)
    }

    pub fn confirm(&mut self, order: TakenOrder) -> anyhow::Result<()> {
        self.inner.confirm(order)
    }

    pub fn deny(&mut self, order: TakenOrder) {
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
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum Event {
    TakeRequest(TakenOrder),
    SetSwapIdentities(SwapMetadata),
    SpawnSwap(SwapKind),
}

#[derive(Debug)]
pub struct SwapMetadata {
    swap_id: SwapId,
    taker_peer_id: PeerId,
}

impl SwapMetadata {
    pub fn swap_id(&self) -> SwapId {
        self.swap_id
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SwapId {
    inner: crate::SwapId,
    shared: SharedSwapId,
}

impl SwapId {
    pub fn new(shared: SharedSwapId) -> Self {
        Self {
            inner: crate::SwapId::default(),
            shared,
        }
    }
}

impl From<SwapId> for crate::SwapId {
    fn from(from: SwapId) -> Self {
        from.inner
    }
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
    database: Arc<Database>,
    #[behaviour(ignore)]
    order_ids: HashMap<SharedSwapId, OrderId>,
    #[behaviour(ignore)]
    local_identities: HashMap<SharedSwapId, LocalIdentities>,
}

impl Nectar {
    fn new(local_peer_id: PeerId, database: Arc<Database>) -> Self {
        Self {
            comit: Comit::default(),
            orderbook: Orderbook::new(local_peer_id.clone()),
            events: VecDeque::new(),
            local_peer_id,
            takers: HashMap::new(),
            database,
            order_ids: HashMap::new(),
            local_identities: HashMap::new(),
        }
    }

    fn make(&mut self, order: PublishOrder) -> anyhow::Result<()> {
        let order_id = OrderId::random();
        let order = order.into_orderbook_order(order_id, self.local_peer_id.clone());
        let _ = self.orderbook.make(order)?;

        Ok(())
    }

    fn confirm(&mut self, order: TakenOrder) -> anyhow::Result<()> {
        self.database.insert_active_taker(order.taker.clone())?;

        self.orderbook
            .confirm(order.id, order.confirmation_channel, order.taker.peer_id());

        Ok(())
    }

    fn deny(&mut self, order: TakenOrder) {
        self.orderbook
            .deny(order.taker.peer_id(), order.id, order.confirmation_channel)
    }

    /// Save the swap identities and send them to the taker
    fn set_swap_identities(
        &mut self,
        SwapMetadata {
            swap_id,
            taker_peer_id,
        }: SwapMetadata,
        bitcoin_transient_sk: ::bitcoin::secp256k1::SecretKey,
        ethereum_identity: identity::Ethereum,
    ) {
        self.local_identities.insert(
            swap_id.shared,
            LocalIdentities::new(bitcoin_transient_sk, ethereum_identity),
        );

        let bitcoin_identity =
            identity::Bitcoin::from_secret_key(&crate::SECP, &bitcoin_transient_sk);

        let identities = Identities {
            bitcoin_identity: Some(bitcoin_identity),
            ethereum_identity: Some(ethereum_identity),
            lightning_identity: None,
        };
        let local_data = LocalData::for_bob(identities);

        self.comit
            .communicate(taker_peer_id, swap_id.shared, local_data);
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

struct LocalIdentities {
    bitcoin: ::bitcoin::secp256k1::SecretKey,
    ethereum: identity::Ethereum,
}

impl LocalIdentities {
    fn new(bitcoin: ::bitcoin::secp256k1::SecretKey, ethereum: identity::Ethereum) -> Self {
        Self { bitcoin, ethereum }
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
                let ongoing_trade_with_taker_exists = match self
                    .database
                    .contains_active_taker(&taker)
                {
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
                self.events.push_back(Event::TakeRequest(taken_order))
            }
            orderbook::BehaviourOutEvent::TakeOrderConfirmation {
                order_id,
                shared_swap_id,
                peer_id: taker_peer_id,
            } => {
                let swap_id = SwapId::new(shared_swap_id);

                self.order_ids.insert(swap_id.shared, order_id);

                self.events
                    .push_back(Event::SetSwapIdentities(SwapMetadata {
                        swap_id,
                        taker_peer_id,
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

                let (maker_bitcoin_transient_sk, maker_ethereum_identity) =
                    match self.local_identities.remove(&shared_swap_id) {
                        Some(LocalIdentities { bitcoin, ethereum }) => (bitcoin, ethereum),
                        _ => {
                            tracing::warn!(
                                "could not find identities for shared_swap_id: {}",
                                shared_swap_id,
                            );
                            return;
                        }
                    };

                let order = match self.order_ids.get(&shared_swap_id) {
                    Some(order_id) => match self.orderbook.get_order(order_id) {
                        Some(order) => order,
                        None => {
                            tracing::warn!("could not find order with id: {}", order_id);
                            return;
                        }
                    },
                    None => {
                        tracing::warn!(
                            "could not find order_id corresponding to shared_swap_id: {}",
                            shared_swap_id
                        );
                        return;
                    }
                };

                let taker = match self.takers.get(&order.id) {
                    Some(taker_peer_id) => Taker::new(taker_peer_id.clone()),
                    None => {
                        tracing::warn!("unknown taker for order: {}", order.id,);
                        return;
                    }
                };

                let redeem_identity =
                    identity::Bitcoin::from_secret_key(&crate::SECP, &maker_bitcoin_transient_sk);
                let hbit_params = hbit::Params::new(
                    hbit::SharedParams {
                        asset: order.bitcoin_amount,
                        redeem_identity,
                        refund_identity: taker_bitcoin_identity,
                        expiry: order.bitcoin_absolute_expiry.into(),
                        secret_hash,
                        network: order.bitcoin_ledger.into(),
                    },
                    maker_bitcoin_transient_sk,
                );

                let herc20_params = herc20::Params {
                    asset: asset::Erc20::new(order.token_contract, order.ethereum_amount),
                    redeem_identity: taker_ethereum_identity,
                    refund_identity: maker_ethereum_identity,
                    expiry: order.ethereum_absolute_expiry.into(),
                    secret_hash,
                    chain_id: order.ethereum_ledger.chain_id,
                };

                let swap = SwapKind::HbitHerc20(SwapParams {
                    hbit_params,
                    herc20_params,
                    secret_hash,
                    start_of_swap: Utc::now().naive_local(),
                    swap_id: crate::SwapId::default(),
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
        let base = bitcoin::Asset {
            amount: order.bitcoin_amount.into(),
            network: order.bitcoin_ledger.into(),
        };
        let chain = ethereum::Chain::new(order.ethereum_ledger.chain_id, order.token_contract);
        let quote = dai::Asset {
            amount: order.ethereum_amount.into(),
            chain,
        };
        let inner = BtcDaiOrder {
            position: order.position.into(),
            base,
            quote,
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
impl crate::StaticStub for Taker {
    fn static_stub() -> Self {
        Self {
            peer_id: PeerId::random(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PublishOrder(BtcDaiOrder);

impl PublishOrder {
    fn into_orderbook_order(self, id: OrderId, maker: PeerId) -> comit::network::orderbook::Order {
        // No special logic for expiry generation, other than setting
        // the asset that will be alpha to be longer than the asset
        // that will be beta
        let twelve_hours = 12 * 60 * 60;
        let beta_expiry = Timestamp::now().plus(twelve_hours);
        let alpha_expiry = beta_expiry.plus(twelve_hours);

        let (bitcoin_absolute_expiry, ethereum_absolute_expiry) = match self.0.position {
            Position::Buy => (alpha_expiry, beta_expiry),
            Position::Sell => (beta_expiry, alpha_expiry),
        };

        comit::network::orderbook::Order {
            id,
            maker: maker.into(),
            position: self.0.position.into(),
            bitcoin_amount: self.0.base.amount.into(),
            bitcoin_ledger: self.0.base.network.into(),
            bitcoin_absolute_expiry: bitcoin_absolute_expiry.into(),
            ethereum_amount: self.0.quote.amount.into(),
            token_contract: self.0.quote.chain.dai_contract_address(),
            ethereum_ledger: comit::ledger::Ethereum::new(self.0.quote.chain.chain_id()),
            ethereum_absolute_expiry: ethereum_absolute_expiry.into(),
        }
    }
}

impl From<BtcDaiOrder> for PublishOrder {
    fn from(order: BtcDaiOrder) -> Self {
        Self(order)
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
