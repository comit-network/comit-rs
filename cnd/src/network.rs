mod peer_tracker;
mod tor;
mod transport;

// Export comit network types while maintaining the module abstraction.
pub use self::tor::TorTokioTcpConfig;
pub use ::comit::network::*;
pub use transport::ComitTransport;

use crate::{
    config::Settings,
    identity,
    network::{peer_tracker::PeerTracker, Comit, LocalData},
    spawn,
    storage::{ForSwap, Save},
    CreatedSwap, Load, LocalSwapId, Never, Protocol, ProtocolSpawner, Role, RootSeed, SecretHash,
    SharedSwapId, Storage, SwapContext,
};
use ::comit::asset;
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
#[allow(clippy::type_complexity)]
pub struct Swarm {
    #[derivative(Debug = "ignore")]
    inner: Arc<Mutex<libp2p::Swarm<ComitNode>>>,
    local_peer_id: PeerId,
}

impl Swarm {
    #[allow(clippy::too_many_arguments)]
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

    pub async fn take_hbit_herc20_order(
        &self,
        id: OrderId,
        swap_id: LocalSwapId,
        refund_address: crate::bitcoin::Address,
        refund_identity: identity::Bitcoin,
        redeem_identity: identity::Ethereum,
    ) -> anyhow::Result<()> {
        let mut guard = self.inner.lock().await;
        guard.take_hbit_herc20_order(
            id,
            swap_id,
            refund_address,
            refund_identity,
            redeem_identity,
        )
    }

    pub async fn make_hbit_herc20_order(
        &self,
        order: NewOrder,
        refund_identity: crate::bitcoin::Address,
        redeem_identity: identity::Ethereum,
    ) -> anyhow::Result<OrderId> {
        let mut guard = self.inner.lock().await;
        guard.make_hbit_herc20_order(order, refund_identity, redeem_identity)
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

    pub async fn announce_trading_pair(&mut self, trading_pair: TradingPair) {
        let mut guard = self.inner.lock().await;
        guard.announce_trading_pair(trading_pair)
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
    announce: Announce,
    comit: Comit,
    peer_tracker: PeerTracker,

    #[behaviour(ignore)]
    pub seed: RootSeed,
    #[behaviour(ignore)]
    task_executor: Handle,
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
}

impl ComitNode {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        seed: RootSeed,
        task_executor: Handle,
        storage: Storage,
        protocol_spawner: ProtocolSpawner,
        peer_id: PeerId,
    ) -> Result<Self, io::Error> {
        Ok(Self {
            announce: Announce::default(),
            comit: Comit::new(peer_id),
            peer_tracker: PeerTracker::default(),
            seed,
            task_executor,
            local_data: HashMap::default(),
            local_swap_ids: HashMap::default(),
            storage,
            protocol_spawner,
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

    pub fn take_hbit_herc20_order(
        &mut self,
        order_id: OrderId,
        swap_id: LocalSwapId,
        refund_address: crate::bitcoin::Address,
        refund_identity: identity::Bitcoin,
        redeem_identity: identity::Ethereum,
    ) -> anyhow::Result<()> {
        let local_data = LocalData {
            secret_hash: Some(SecretHash::new(
                self.seed.derive_swap_seed(swap_id).derive_secret(),
            )),
            ethereum_identity: Some(redeem_identity),
            lightning_identity: None,
            bitcoin_identity: Some(refund_identity),
        };

        self.local_data.insert(swap_id, local_data);

        self.comit
            .take_order(order_id, refund_address, redeem_identity)
    }

    pub fn make_hbit_herc20_order(
        &mut self,
        order: NewOrder,
        refund_identity: crate::bitcoin::Address,
        redeem_identity: identity::Ethereum,
    ) -> anyhow::Result<OrderId> {
        self.comit
            .make_order(order, refund_identity, redeem_identity)
    }

    pub fn get_order(&self, order_id: OrderId) -> Option<Order> {
        self.comit.get_order(&order_id)
    }

    pub fn get_orders(&self) -> Vec<Order> {
        self.comit.get_orders()
    }

    pub fn announce_trading_pair(&mut self, trading_pair: TradingPair) {
        self.comit.announce_trading_pair(trading_pair)
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
            ::comit::network::comit::BehaviourOutEvent::OrderTaken {
                order,
                peer,
                refund_identity,
                redeem_identity,
                io,
            } => {
                tracing::info!("order taken: {:?}", order.id);
                let local_swap_id = LocalSwapId::default();
                let shared_swap_id = SharedSwapId::default();

                self.local_swap_ids.insert(shared_swap_id, local_swap_id);

                let role = Role::Bob;

                let swap = CreatedSwap {
                    swap_id: local_swap_id,
                    alpha: crate::hbit::CreatedSwap {
                        amount: asset::Bitcoin::from_sat(order.buy),
                        final_identity: refund_identity.into(),
                        network: crate::ledger::Bitcoin::Regtest,
                        absolute_expiry: 0,
                    },
                    beta: crate::herc20::CreatedSwap {
                        asset: order.sell,
                        identity: redeem_identity,
                        chain_id: crate::ethereum::ChainId::regtest(),
                        absolute_expiry: 0,
                    },
                    peer: peer.clone(),
                    address_hint: None,
                    role,
                    start_of_swap: Utc::now().naive_local(),
                };

                let storage = self.storage.clone();
                let order_id = order.id;

                // todo: saving can fail but subsequent communication steps will continue
                self.task_executor.spawn(async move {
                    storage
                        .associate_swap_with_order(order_id, local_swap_id)
                        .await;
                    match storage.save(swap).await {
                        Ok(()) => (),
                        Err(e) => tracing::error!("{}", e),
                    }
                    let _ = io.send(shared_swap_id).await;
                });

                let transient_identity = self.storage.derive_transient_identity(
                    local_swap_id,
                    role,
                    ::comit::Side::Alpha,
                );

                let identities = Identities {
                    bitcoin_identity: Some(transient_identity),
                    ethereum_identity: Some(redeem_identity),
                    lightning_identity: None,
                };

                let local_data = LocalData::for_bob(identities);
                self.comit.communicate(peer, shared_swap_id, local_data);
            }
        }
    }
}

impl libp2p::swarm::NetworkBehaviourEventProcess<announce::BehaviourOutEvent> for ComitNode {
    fn inject_event(&mut self, event: announce::BehaviourOutEvent) {
        match event {
            announce::BehaviourOutEvent::Confirmed {
                peer,
                shared_swap_id,
                local_swap_id,
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
                local_swap_id,
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
