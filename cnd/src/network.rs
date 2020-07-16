mod peer_tracker;
mod tor;
mod transport;

// Export comit network types while maintaining the module abstraction.
pub use self::tor::TorTokioTcpConfig;
pub use ::comit::{asset, ledger, network::*};
pub use transport::ComitTransport;

use crate::{
    config::Settings,
    hbit, herc20, identity,
    network::{peer_tracker::PeerTracker, Comit, LocalData, MakerId},
    spawn,
    storage::{CreatedSwap, ForSwap, Load, Save, SwapContext},
    LocalSwapId, Never, Protocol, ProtocolSpawner, Role, RootSeed, SecretHash, SharedSwapId, Side,
    Storage,
};
use anyhow::Context;
use async_trait::async_trait;
use chrono::Utc;
use futures::{stream::StreamExt, Future, TryFutureExt};
use libp2p::{
    core::connection::ConnectionLimit,
    identity::{ed25519, Keypair},
    swarm::SwarmBuilder,
    Multiaddr, NetworkBehaviour, PeerId,
};
use std::{
    collections::HashMap,
    fmt::Debug,
    io,
    pin::Pin,
    sync::Arc,
    task::{self, Poll},
};
use tokio::{runtime::Handle, sync::Mutex};

#[derive(Clone, derivative::Derivative)]
#[derivative(Debug)]
pub struct Swarm {
    #[derivative(Debug = "ignore")]
    inner: Arc<Mutex<libp2p::Swarm<ComitNode>>>,
    local_peer_id: PeerId,
}

impl Swarm {
    pub fn new(
        settings: &Settings,
        seed: RootSeed,
        task_executor: tokio::runtime::Handle,
        storage: Storage,
        protocol_spawner: ProtocolSpawner,
    ) -> anyhow::Result<Self> {
        let local_key_pair = derive_key_pair(&seed);
        let local_peer_id = PeerId::from(local_key_pair.public());
        tracing::info!("Starting with peer_id: {}", local_peer_id);

        let transport = transport::build(local_key_pair, settings.network.listen.clone())?;

        let behaviour = ComitNode::new(
            seed,
            task_executor.clone(),
            storage,
            protocol_spawner,
            local_peer_id.clone(),
        )?;

        let mut swarm = SwarmBuilder::new(transport, behaviour, local_peer_id.clone())
            .executor(Box::new(TokioExecutor {
                handle: task_executor,
            }))
            .build();

        for addr in settings.network.listen.clone() {
            libp2p::Swarm::listen_on(&mut swarm, addr.clone())
                .with_context(|| format!("Address is not supported: {:?}", addr))?;
        }

        let swarm = Arc::new(Mutex::new(swarm));

        Ok(Self {
            inner: swarm,
            local_peer_id,
        })
    }

    pub async fn initiate_communication(
        &self,
        id: LocalSwapId,
        role: Role,
        digest: SwapDigest,
        identities: Identities,
        peer: PeerId,
        address_hint: Option<Multiaddr>,
    ) -> anyhow::Result<()> {
        let mut guard = self.inner.lock().await;

        // best effort attempt to establish a connection to the other party
        if let Some(address_hint) = address_hint {
            let existing_connection_to_peer =
                libp2p::Swarm::connection_info(&mut guard, &peer).is_some();

            if !existing_connection_to_peer {
                match libp2p::Swarm::dial_addr(&mut guard, address_hint) {
                    Ok(()) => {}
                    // How did we hit the connection limit if we are not connected?
                    // Match on the error directly so our assumption of only hitting the connection
                    // limit here is not violated by future API changes.
                    Err(ConnectionLimit { .. }) => {}
                }
            }
        }

        guard.initiate_communication(id, peer, role, digest, identities)
    }

    /// The taker plays the role of Alice.
    pub async fn take_herc20_hbit_order(
        &self,
        order_id: OrderId,
        swap_id: LocalSwapId,
        redeem_identity: crate::bitcoin::Address,
        refund_identity: identity::Ethereum,
    ) -> anyhow::Result<()> {
        let mut guard = self.inner.lock().await;
        guard.take_herc20_hbit_order(order_id, swap_id, redeem_identity, refund_identity)
    }

    /// The maker plays the role of Bob.
    pub async fn make_herc20_hbit_order(
        &self,
        order: NewOrder,
        swap_id: LocalSwapId,
        redeem_identity: identity::Ethereum,
        refund_identity: crate::bitcoin::Address,
    ) -> anyhow::Result<OrderId> {
        let mut guard = self.inner.lock().await;
        guard.make_herc20_hbit_order(order, swap_id, redeem_identity, refund_identity)
    }

    pub async fn get_orders(&self) -> Vec<Order> {
        let guard = self.inner.lock().await;
        guard.get_orders()
    }

    pub async fn get_order(&self, order_id: OrderId) -> Option<Order> {
        let guard = self.inner.lock().await;
        guard.get_order(order_id)
    }

    pub async fn dial_addr(&mut self, addr: Multiaddr) -> anyhow::Result<()> {
        let mut guard = self.inner.lock().await;
        // todo: log error
        libp2p::Swarm::dial_addr(&mut *guard, addr).unwrap();
        // guard.dial_addr(addr);
        Ok(())
    }

    pub async fn announce_trading_pair(&mut self, tp: TradingPair) -> anyhow::Result<()> {
        let mut guard = self.inner.lock().await;
        guard.announce_trading_pair(tp)
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

/// The SwarmWorker, when spawned into a runtime, continuously polls the
/// underlying swarm for events.
///
/// This is the main driver of the networking code.
/// Note that the inner swarm is wrapped in an `Arc<Mutex>` and we only hold the
/// lock for a short period of time, giving other parts of the code also the
/// opportunity to acquire the lock and interact with the network.
#[derive(Debug)]
pub struct SwarmWorker {
    pub swarm: Swarm,
}

impl futures::Future for SwarmWorker {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Self::Output> {
        loop {
            let mutex = self.swarm.inner.lock();
            futures::pin_mut!(mutex);

            let mut guard = futures::ready!(mutex.poll(cx));
            futures::ready!(guard.poll_next_unpin(cx));
        }
    }
}

fn derive_key_pair(seed: &RootSeed) -> Keypair {
    let bytes = seed.sha256_with_seed(&[b"NODE_ID"]);
    let key = ed25519::SecretKey::from_bytes(bytes).expect("we always pass 32 bytes");
    Keypair::Ed25519(key.into())
}

/// A `NetworkBehaviour` that represents a COMIT node.
#[derive(NetworkBehaviour)]
#[allow(missing_debug_implementations)]
pub struct ComitNode {
    announce: Announce<LocalSwapId>,
    orderbook: Orderbook,
    comit: Comit,
    peer_tracker: PeerTracker,

    #[behaviour(ignore)]
    pub seed: RootSeed,
    #[behaviour(ignore)]
    task_executor: Handle,
    #[behaviour(ignore)]
    pub peer_id: PeerId,
    /// We receive the LocalData for the execution parameter exchange at the
    /// same time as we announce the swap. We save `LocalData` here until the
    /// swap is confirmed.
    #[behaviour(ignore)]
    local_data: HashMap<LocalSwapId, LocalData>,
    /// The execution parameter exchange only knows about `SharedSwapId`s, so we
    /// need to map this back to a `LocalSwapId` to save the data correctly to
    /// the database.
    #[behaviour(ignore)]
    local_swap_ids: HashMap<SharedSwapId, LocalSwapId>,
    #[behaviour(ignore)]
    pub storage: Storage,
    #[behaviour(ignore)]
    pub protocol_spawner: ProtocolSpawner,
    #[behaviour(ignore)]
    bitcoin_addresses: HashMap<identity::Bitcoin, crate::bitcoin::Address>,
    #[behaviour(ignore)]
    order_swap_ids: HashMap<OrderId, LocalSwapId>,
    #[behaviour(ignore)]
    confirmed_order_peers: HashMap<OrderId, PeerId>,
}

impl ComitNode {
    pub fn new(
        seed: RootSeed,
        task_executor: Handle,
        storage: Storage,
        protocol_spawner: ProtocolSpawner,
        peer_id: PeerId,
    ) -> Result<Self, io::Error> {
        Ok(Self {
            announce: Announce::default(),
            orderbook: Orderbook::new(peer_id.clone()),
            comit: Comit::default(),
            peer_tracker: PeerTracker::default(),
            seed,
            task_executor,
            peer_id,
            local_data: HashMap::default(),
            local_swap_ids: HashMap::default(),
            storage,
            protocol_spawner,
            bitcoin_addresses: HashMap::default(),
            order_swap_ids: Default::default(),
            confirmed_order_peers: Default::default(),
        })
    }

    pub fn initiate_communication(
        &mut self,
        local_swap_id: LocalSwapId,
        peer_id: PeerId,
        role: Role,
        digest: SwapDigest,
        identities: Identities,
    ) -> anyhow::Result<()> {
        // At this stage we do not know if the arguments passed to us make up a
        // valid swap, we just trust the controller to pass in something
        // valid. Do _some_ form of validation here so that we can early return
        // errors and they do not get lost in the asynchronous call chain that
        // kicks off here.
        self.assert_have_lnd_if_needed(identities.lightning_identity)?;

        let local_data = match role {
            Role::Alice => {
                self.announce.announce_swap(digest, peer_id, local_swap_id);

                let swap_seed = self.seed.derive_swap_seed(local_swap_id);
                let secret = swap_seed.derive_secret();
                let secret_hash = SecretHash::new(secret);

                LocalData::for_alice(secret_hash, identities)
            }
            Role::Bob => {
                self.announce
                    .await_announcement(digest, peer_id, local_swap_id);

                LocalData::for_bob(identities)
            }
        };

        self.local_data.insert(local_swap_id, local_data);

        Ok(())
    }

    fn assert_have_lnd_if_needed(
        &self,
        identity: Option<identity::Lightning>,
    ) -> anyhow::Result<()> {
        if identity.is_some() {
            return self.protocol_spawner.supports_halbit();
        }
        Ok(())
    }

    /// The taker plays the role of Alice.
    pub fn take_herc20_hbit_order(
        &mut self,
        order_id: OrderId,
        swap_id: LocalSwapId,
        redeem_identity: crate::bitcoin::Address,
        refund_identity: identity::Ethereum,
    ) -> anyhow::Result<()> {
        let transient = self
            .storage
            .derive_transient_identity(swap_id, Role::Alice, Side::Beta);

        self.bitcoin_addresses.insert(transient, redeem_identity);

        let data = LocalData {
            secret_hash: Some(SecretHash::new(
                self.seed.derive_swap_seed(swap_id).derive_secret(),
            )),
            ethereum_identity: Some(refund_identity),
            bitcoin_identity: Some(transient),
            lightning_identity: None,
        };
        self.local_data.insert(swap_id, data);

        self.order_swap_ids.insert(order_id, swap_id);
        self.orderbook.take(order_id)?;

        Ok(())
    }

    /// The maker plays the role of Bob.
    pub fn make_herc20_hbit_order(
        &mut self,
        new_order: NewOrder,
        swap_id: LocalSwapId,
        redeem_identity: identity::Ethereum,
        refund_identity: crate::bitcoin::Address,
    ) -> anyhow::Result<OrderId> {
        let transient = self
            .storage
            .derive_transient_identity(swap_id, Role::Bob, Side::Beta);

        self.bitcoin_addresses.insert(transient, refund_identity);

        let data = LocalData {
            secret_hash: None,
            ethereum_identity: Some(redeem_identity),
            bitcoin_identity: Some(transient),
            lightning_identity: None,
        };
        self.local_data.insert(swap_id, data);

        let order = Order {
            id: OrderId::random(),
            maker: MakerId::from(self.peer_id.clone()),
            position: new_order.position,
            bitcoin_amount: new_order.bitcoin_amount,
            bitcoin_ledger: new_order.bitcoin_ledger,
            ethereum_amount: new_order.ethereum_amount,
            token_contract: new_order.token_contract,
            ethereum_ledger: new_order.ethereum_ledger,
            absolute_expiry: new_order.absolute_expiry,
        };
        let order_id = self.orderbook.make(order)?;
        self.order_swap_ids.insert(order_id, swap_id);

        Ok(order_id)
    }

    pub fn get_order(&self, order_id: OrderId) -> Option<Order> {
        self.orderbook.get_order(&order_id)
    }

    pub fn get_orders(&self) -> Vec<Order> {
        self.orderbook.get_orders()
    }

    pub fn announce_trading_pair(&mut self, tp: TradingPair) -> anyhow::Result<()> {
        self.orderbook.announce_trading_pair(tp)
    }
}

/// Get the `PeerId` of this node.
#[ambassador::delegatable_trait]
pub trait LocalPeerId {
    fn local_peer_id(&self) -> PeerId;
}

impl LocalPeerId for Swarm {
    fn local_peer_id(&self) -> PeerId {
        self.local_peer_id.clone()
    }
}

/// Get `PeerId`s of connected nodes.
#[async_trait]
#[ambassador::delegatable_trait]
#[allow(clippy::type_complexity)]
pub trait ComitPeers {
    async fn comit_peers(
        &self,
    ) -> Box<dyn Iterator<Item = (PeerId, Vec<Multiaddr>)> + Send + 'static>;
}

#[async_trait]
impl ComitPeers for Swarm {
    async fn comit_peers(
        &self,
    ) -> Box<dyn Iterator<Item = (PeerId, Vec<Multiaddr>)> + Send + 'static> {
        let swarm = self.inner.lock().await;
        Box::new(swarm.peer_tracker.connected_peers())
    }
}

/// IP addresses local node is listening on.
#[async_trait]
#[ambassador::delegatable_trait]
pub trait ListenAddresses {
    async fn listen_addresses(&self) -> Vec<Multiaddr>;
}

#[async_trait]
impl ListenAddresses for Swarm {
    async fn listen_addresses(&self) -> Vec<Multiaddr> {
        let swarm = self.inner.lock().await;

        libp2p::Swarm::listeners(&swarm)
            .chain(libp2p::Swarm::external_addresses(&swarm))
            .cloned()
            .collect()
    }
}

/// Used by the controller to pass in parameters for a new order.
#[derive(Debug)]
pub struct NewOrder {
    pub position: Position,
    pub bitcoin_amount: asset::Bitcoin,
    pub bitcoin_ledger: ledger::Bitcoin,
    pub ethereum_amount: asset::Erc20Quantity,
    pub token_contract: identity::Ethereum,
    pub ethereum_ledger: ledger::Ethereum,
    // TODO: Add both expiries
    pub absolute_expiry: u32,
}

impl NewOrder {
    pub fn assert_valid_ledger_pair(&self) -> anyhow::Result<()> {
        let a = self.bitcoin_ledger;
        let b = self.ethereum_ledger;

        if ledger::is_valid_ledger_pair(a, b) {
            return Ok(());
        }
        Err(anyhow::anyhow!("invalid ledger pair {}/{}", a, b))
    }
}

impl libp2p::swarm::NetworkBehaviourEventProcess<()> for ComitNode {
    fn inject_event(&mut self, _event: ()) {}
}

impl libp2p::swarm::NetworkBehaviourEventProcess<Never> for ComitNode {
    fn inject_event(&mut self, _: Never) {}
}

impl libp2p::swarm::NetworkBehaviourEventProcess<::comit::network::comit::BehaviourOutEvent>
    for ComitNode
{
    fn inject_event(&mut self, event: ::comit::network::comit::BehaviourOutEvent) {
        match event {
            ::comit::network::comit::BehaviourOutEvent::SwapFinalized {
                shared_swap_id,
                remote_data,
            } => {
                let storage = self.storage.clone();
                let spawner = self.protocol_spawner.clone();

                let local_swap_id = match self.local_swap_ids.remove(&shared_swap_id) {
                    Some(local_swap_id) => local_swap_id,
                    None => {
                        tracing::warn!("inconsistent data, missing local_swap_id mapping");
                        return;
                    }
                };

                let save_and_start_swap = async move {
                    let swap = storage.load(local_swap_id).await?;
                    save_swap_remote_data(&storage, swap, remote_data).await?;
                    spawn::spawn(&spawner, &storage, swap).await?;

                    Ok::<(), anyhow::Error>(())
                };

                self.task_executor
                    .spawn(save_and_start_swap.map_err(|e: anyhow::Error| {
                        tracing::error!("{}", e);
                    }));
            }
        }
    }
}

impl libp2p::swarm::NetworkBehaviourEventProcess<announce::BehaviourOutEvent<LocalSwapId>>
    for ComitNode
{
    fn inject_event(&mut self, event: announce::BehaviourOutEvent<LocalSwapId>) {
        match event {
            announce::BehaviourOutEvent::Confirmed {
                peer,
                shared_swap_id,
                context: local_swap_id,
            } => {
                let data = match self.local_data.remove(&local_swap_id) {
                    Some(local_data) => local_data,
                    None => {
                        tracing::warn!("inconsistent data, missing local-data mapping");
                        return;
                    }
                };

                self.comit.communicate(peer, shared_swap_id, data);
                self.local_swap_ids.insert(shared_swap_id, local_swap_id);
            }
            announce::BehaviourOutEvent::Failed {
                peer,
                context: local_swap_id,
            } => {
                tracing::warn!(
                    "failed to complete announce protocol for swap {} with {}",
                    local_swap_id,
                    peer,
                );
            }
        }
    }
}

impl libp2p::swarm::NetworkBehaviourEventProcess<orderbook::BehaviourOutEvent> for ComitNode {
    fn inject_event(&mut self, event: orderbook::BehaviourOutEvent) {
        match event {
            orderbook::BehaviourOutEvent::TakeOrderRequest {
                peer_id,
                response_channel,
                order_id,
            } => {
                let order = self
                    .orderbook
                    .get_order(&order_id)
                    .expect("orderbook only bubbles up existing orders");
                let &local_swap_id = match self.order_swap_ids.get(&order_id) {
                    Some(id) => id,
                    None => {
                        tracing::warn!(
                            "inconsistent state, non-existent order_id->local_swap_id mapping"
                        );
                        return;
                    }
                };
                // TODO: Unwraps
                let data = self.local_data.get(&local_swap_id).unwrap();
                let refund_identity = data.bitcoin_identity.unwrap();
                let redeem_identity = data.ethereum_identity.unwrap();
                let start_of_swap = Utc::now().naive_local();

                // TODO: Remove unwrap.
                let final_identity = self
                    .bitcoin_addresses
                    .get(&refund_identity)
                    .unwrap()
                    .clone();

                let swap = CreatedSwap {
                    swap_id: local_swap_id,
                    alpha: herc20::CreatedSwap {
                        asset: asset::Erc20 {
                            token_contract: order.token_contract,
                            quantity: order.ethereum_amount,
                        },
                        identity: redeem_identity,
                        chain_id: order.ethereum_ledger.chain_id,
                        absolute_expiry: order.absolute_expiry,
                    },
                    beta: hbit::CreatedSwap {
                        amount: order.bitcoin_amount,
                        final_identity: final_identity.into(),
                        network: order.bitcoin_ledger,
                        absolute_expiry: order.absolute_expiry,
                    },
                    peer: peer_id.clone(),
                    address_hint: None,
                    role: Role::Bob,
                    start_of_swap,
                };

                let storage = self.storage.clone();
                let order_id = order.id;

                // Saving can fail but subsequent communication steps will continue.
                self.task_executor.spawn(async move {
                    storage
                        .associate_swap_with_order(order_id, local_swap_id)
                        .await;
                    match storage.save(swap).await {
                        Ok(()) => (),
                        Err(e) => tracing::error!("{}", e),
                    }
                });

                self.confirmed_order_peers.insert(order_id, peer_id);

                // No other validation, just take the order. This
                // implies that an order can be taken multiple times.
                self.orderbook.confirm(order_id, response_channel);
            }
            orderbook::BehaviourOutEvent::TakeOrderConfirmation {
                order_id,
                shared_swap_id,
            } => {
                // TODO: Re-evaluate all the hashmap access
                let local_swap_id = self.local_swap_ids.get(&shared_swap_id).unwrap();
                let &data = match self.local_data.get(local_swap_id) {
                    Some(data) => data,
                    None => {
                        tracing::warn!(
                            "inconsistent state, no local data found for swap id: {}",
                            shared_swap_id
                        );
                        return;
                    }
                };

                // TODO: Consider creating/saving the swap here.

                let peer_id = self
                    .confirmed_order_peers
                    .get(&order_id)
                    .expect("peer id to be inserted during confirmation");
                self.comit
                    .communicate(peer_id.clone(), shared_swap_id, data); //
            }

            orderbook::BehaviourOutEvent::Failed { peer_id, order_id } => tracing::warn!(
                "take order request failed, peer: {}, order: {}",
                peer_id,
                order_id,
            ),
        }
    }
}

#[derive(Copy, Clone, Debug, thiserror::Error)]
#[error(
"unable to save swap with id {local_swap_id} in database because the protocol combination is not supported"
)]
struct SaveUnsupportedSwap {
    local_swap_id: LocalSwapId,
}

async fn save_swap_remote_data(
    storage: &Storage,
    swap: SwapContext,
    data: RemoteData,
) -> anyhow::Result<()> {
    match (&swap, data) {
        (
            SwapContext {
                alpha: Protocol::Herc20,
                beta: Protocol::Halbit,
                role: Role::Alice,
                ..
            },
            RemoteData {
                ethereum_identity: Some(ethereum_identity),
                lightning_identity: Some(lightning_identity),
                ..
            },
        ) => {
            storage
                .save(ForSwap {
                    local_swap_id: swap.id,
                    data: WhatAliceLearnedFromBob {
                        alpha_redeem_identity: ethereum_identity,
                        beta_refund_identity: lightning_identity,
                    },
                })
                .await?;
        }
        (
            SwapContext {
                alpha: Protocol::Herc20,
                beta: Protocol::Halbit,
                role: Role::Bob,
                ..
            },
            RemoteData {
                ethereum_identity: Some(ethereum_identity),
                lightning_identity: Some(lightning_identity),
                secret_hash: Some(secret_hash),
                ..
            },
        ) => {
            storage
                .save(ForSwap {
                    local_swap_id: swap.id,
                    data: WhatBobLearnedFromAlice {
                        secret_hash,
                        alpha_refund_identity: ethereum_identity,
                        beta_redeem_identity: lightning_identity,
                    },
                })
                .await?;
        }
        (
            SwapContext {
                alpha: Protocol::Halbit,
                beta: Protocol::Herc20,
                role: Role::Alice,
                ..
            },
            RemoteData {
                ethereum_identity: Some(ethereum_identity),
                lightning_identity: Some(lightning_identity),
                ..
            },
        ) => {
            storage
                .save(ForSwap {
                    local_swap_id: swap.id,
                    data: WhatAliceLearnedFromBob {
                        alpha_redeem_identity: lightning_identity,
                        beta_refund_identity: ethereum_identity,
                    },
                })
                .await?;
        }
        (
            SwapContext {
                alpha: Protocol::Halbit,
                beta: Protocol::Herc20,
                role: Role::Bob,
                ..
            },
            RemoteData {
                ethereum_identity: Some(ethereum_identity),
                lightning_identity: Some(lightning_identity),
                secret_hash: Some(secret_hash),
                ..
            },
        ) => {
            storage
                .save(ForSwap {
                    local_swap_id: swap.id,
                    data: WhatBobLearnedFromAlice {
                        secret_hash,
                        alpha_refund_identity: lightning_identity,
                        beta_redeem_identity: ethereum_identity,
                    },
                })
                .await?;
        }
        (
            SwapContext {
                alpha: Protocol::Herc20,
                beta: Protocol::Hbit,
                role: Role::Alice,
                ..
            },
            RemoteData {
                ethereum_identity: Some(ethereum_identity),
                bitcoin_identity: Some(bitcoin_identity),
                ..
            },
        ) => {
            storage
                .save(ForSwap {
                    local_swap_id: swap.id,
                    data: WhatAliceLearnedFromBob {
                        alpha_redeem_identity: ethereum_identity,
                        beta_refund_identity: bitcoin_identity,
                    },
                })
                .await?;
        }
        (
            SwapContext {
                alpha: Protocol::Herc20,
                beta: Protocol::Hbit,
                role: Role::Bob,
                ..
            },
            RemoteData {
                ethereum_identity: Some(ethereum_identity),
                bitcoin_identity: Some(bitcoin_identity),
                secret_hash: Some(secret_hash),
                ..
            },
        ) => {
            storage
                .save(ForSwap {
                    local_swap_id: swap.id,
                    data: WhatBobLearnedFromAlice {
                        secret_hash,
                        alpha_refund_identity: ethereum_identity,
                        beta_redeem_identity: bitcoin_identity,
                    },
                })
                .await?;
        }
        (
            SwapContext {
                alpha: Protocol::Hbit,
                beta: Protocol::Herc20,
                role: Role::Alice,
                ..
            },
            RemoteData {
                bitcoin_identity: Some(bitcoin_identity),
                ethereum_identity: Some(ethereum_identity),
                ..
            },
        ) => {
            storage
                .save(ForSwap {
                    local_swap_id: swap.id,
                    data: WhatAliceLearnedFromBob {
                        alpha_redeem_identity: bitcoin_identity,
                        beta_refund_identity: ethereum_identity,
                    },
                })
                .await?;
        }
        (
            SwapContext {
                alpha: Protocol::Hbit,
                beta: Protocol::Herc20,
                role: Role::Bob,
                ..
            },
            RemoteData {
                ethereum_identity: Some(ethereum_identity),
                bitcoin_identity: Some(bitcoin_identity),
                secret_hash: Some(secret_hash),
                ..
            },
        ) => {
            storage
                .save(ForSwap {
                    local_swap_id: swap.id,
                    data: WhatBobLearnedFromAlice {
                        secret_hash,
                        alpha_refund_identity: bitcoin_identity,
                        beta_redeem_identity: ethereum_identity,
                    },
                })
                .await?;
        }
        _ => anyhow::bail!(SaveUnsupportedSwap {
            local_swap_id: swap.id,
        }),
    };

    Ok(())
}
