use crate::{
    asset,
    config::Settings,
    local_swap_id::LocalSwapId,
    network::{
        comit_node::{ComitNode, SetupSwapContext},
        setup_swap,
        setup_swap::{AliceParams, BobParams},
        transport,
    },
    protocol_spawner::ProtocolSpawner,
    storage::{RootSeed, Storage},
};
use anyhow::{Context as _, Result};
use comit::{
    network::{
        protocols::setup_swap::{CommonParams, RoleDependentParams},
        swap_digest::SwapDigest,
        Identities,
    },
    order::SwapProtocol,
    orderpool, BtcDaiOrder, OrderId, Role, SecretHash, Side,
};
use futures::{channel::mpsc, stream::StreamExt};
use libp2p::{
    core::{
        identity::{ed25519, Keypair},
        Multiaddr,
    },
    swarm::SwarmBuilder,
    PeerId,
};
use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::sync::Mutex;

#[derive(Clone, derivative::Derivative)]
#[derivative(Debug)]
pub struct Swarm {
    #[derivative(Debug = "ignore")]
    inner: Arc<Mutex<libp2p::Swarm<ComitNode>>>,
    local_peer_id: PeerId,
}

impl Swarm {
    pub async fn new(
        settings: &Settings,
        seed: RootSeed,
        task_executor: tokio::runtime::Handle,
        storage: Storage,
        protocol_spawner: ProtocolSpawner,
    ) -> anyhow::Result<Self> {
        let local_key_pair = derive_key_pair(&seed);
        let local_peer_id = PeerId::from(local_key_pair.public());
        tracing::info!("Starting with peer_id: {}", local_peer_id);

        let transport = transport::build(local_key_pair.clone(), settings.network.listen.clone())?;

        let (sender, receiver) = mpsc::channel(1);

        let behaviour = ComitNode::new(
            seed,
            task_executor.clone(),
            storage.clone(),
            protocol_spawner,
            local_peer_id.clone(),
            local_key_pair,
            sender,
        );

        let mut swarm = SwarmBuilder::new(transport, behaviour, local_peer_id.clone())
            .executor(Box::new(TokioExecutor {
                handle: task_executor.clone(),
            }))
            .build();

        for addr in settings.network.listen.clone() {
            libp2p::Swarm::listen_on(&mut swarm, addr.clone())
                .with_context(|| format!("Address is not supported: {:?}", addr))?;
        }

        for peer_addr in &settings.network.peer_addresses {
            tracing::info!("Dialing peer address {} from config", peer_addr);
            if let Err(err) = Self::dial_addr_on_swarm(&mut swarm, peer_addr.clone()).await {
                tracing::warn!("Could not dial peer address {}: {}", peer_addr, err)
            }
        }

        let swarm = Arc::new(Mutex::new(swarm));

        task_executor.spawn(new_match_worker(swarm.clone(), receiver, storage, seed));

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

        if let Some(address_hint) = address_hint {
            guard
                .peer_tracker
                .add_recent_address_hint(peer.clone(), address_hint);

            let existing_connection_to_peer =
                libp2p::Swarm::connection_info(&mut guard, &peer).is_some();

            if !existing_connection_to_peer {
                tracing::debug!("dialing ...");
                let _ = libp2p::Swarm::dial(&mut guard, &peer)?;
            }
        }

        guard.initiate_communication(id, peer, role, digest, identities)
    }

    pub async fn publish_order(&self, order: BtcDaiOrder) {
        self.inner.lock().await.orderbook.publish(order);
    }

    pub async fn btc_dai_market(&self) -> Vec<(PeerId, BtcDaiOrder)> {
        self.inner
            .lock()
            .await
            .orderbook
            .orderpool()
            .all()
            .map(|(maker, order)| (maker.clone(), order.clone()))
            .collect()
    }

    pub async fn cancel_order(&self, order_id: OrderId) {
        self.inner.lock().await.orderbook.cancel(order_id);
    }

    pub async fn dial_addr(&self, addr: Multiaddr) -> anyhow::Result<()> {
        let mut guard = self.inner.lock().await;
        let _ = libp2p::Swarm::dial_addr(&mut *guard, addr)?;
        Ok(())
    }

    async fn dial_addr_on_swarm(
        swarm: &mut libp2p::Swarm<ComitNode>,
        address: Multiaddr,
    ) -> anyhow::Result<()> {
        let _ = libp2p::Swarm::dial_addr(swarm, address)?;

        Ok(())
    }

    pub fn local_peer_id(&self) -> PeerId {
        self.local_peer_id.clone()
    }

    pub async fn connected_peers(&self) -> impl Iterator<Item = (PeerId, Vec<Multiaddr>)> {
        let swarm = self.inner.lock().await;
        Box::new(swarm.peer_tracker.connected_peers())
    }

    pub async fn listen_addresses(&self) -> Vec<Multiaddr> {
        let swarm = self.inner.lock().await;

        libp2p::Swarm::listeners(&swarm)
            .chain(libp2p::Swarm::external_addresses(&swarm))
            .cloned()
            .collect()
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

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
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

/// A background worker that handles new matches coming in through the given
/// channel.
///
/// This is a workaround because we cannot do arbitrary async stuff while we
/// process events from libp2p in `NetworkBehaviourEventProcess`. To process a
/// new match, we need to access the database and afterwards call into
/// [`SetupSwap`]. This in turns requires `&mut self` and hence we cannot just
/// spawn away a future like we do it in other occasions.
///
/// A proper fix for this would be to:
/// 1. stop handling events within `NetworkBehaviourEventProcess`.
/// 2. delete the [`SwarmWorker`]
/// 3. bubble all events of the NetworkBehaviours up to the Swarm
/// 4. poll the swarm manually using [`futures::select!`]
///
/// This would allow us to offload arbitrary async computation after an event
/// happens, receive a message once it is down, call into the Swarm again and
/// trigger [`SetupSwap`].
async fn new_match_worker(
    swarm: Arc<Mutex<libp2p::Swarm<ComitNode>>>,
    mut receiver: mpsc::Receiver<orderpool::Match>,
    storage: Storage,
    seed: RootSeed,
) {
    while let Some(new_match) = receiver.next().await {
        let order_id = new_match.ours;
        let peer = new_match.peer.clone();
        let match_reference_point = new_match.match_reference_point;

        let (swap_id, common, role, protocol) =
            match handle_new_match(&seed, &storage, new_match).await {
                Ok(result) => result,
                Err(e) => {
                    tracing::warn!("failed to handle new match: {:?}", e);
                    continue;
                }
            };

        let mut guard = swarm.lock().await;

        if let Err(e) = guard
            .setup_swap
            .send(&peer, role, common, protocol, SetupSwapContext {
                swap: swap_id,
                order: order_id,
                match_reference_point,
            })
        {
            tracing::warn!("failed to setup swap for order {}: {:#}", order_id, e);
        }
    }
}

async fn handle_new_match(
    seed: &RootSeed,
    storage: &Storage,
    new_match: orderpool::Match,
) -> Result<(
    LocalSwapId,
    CommonParams,
    RoleDependentParams,
    setup_swap::SwapProtocol,
)> {
    let swap_id = LocalSwapId::random();

    let protocol = new_match.swap_protocol;
    let order_id = new_match.ours;

    let (order_hbit, order_herc20) = storage
        .db
        .do_in_transaction(|conn| {
            use crate::storage::*;

            let order = Order::by_order_id(conn, order_id)?;
            let hbit_params = OrderHbitParams::by_order(conn, &order)?;
            let herc20_params = OrderHerc20Params::by_order(conn, &order)?;

            Ok((hbit_params, herc20_params))
        })
        .await?;

    let our_role = protocol.role(new_match.our_position);
    let ethereum_absolute_expiry =
        new_match.match_reference_point + protocol.herc20_expiry_offset();
    let bitcoin_absolute_expiry = new_match.match_reference_point + protocol.hbit_expiry_offset();
    let erc20_quantity = new_match.quote();
    let hbit_quantity = new_match.quantity;

    // TODO: Fix these!
    #[allow(clippy::cast_sign_loss)]
    #[allow(clippy::cast_possible_truncation)]
    let common_params = CommonParams {
        erc20: asset::Erc20 {
            token_contract: order_herc20.token_contract,
            quantity: erc20_quantity,
        },
        bitcoin: hbit_quantity.to_inner(),
        ethereum_absolute_expiry: ethereum_absolute_expiry.timestamp() as u32,
        bitcoin_absolute_expiry: bitcoin_absolute_expiry.timestamp() as u32,
        ethereum_chain_id: u32::from(order_herc20.chain_id).into(),
        bitcoin_network: order_hbit.network,
    };
    let role_params = match our_role {
        Role::Alice => {
            let swap_seed = seed.derive_swap_seed(swap_id);
            RoleDependentParams::Alice(AliceParams {
                ethereum_identity: order_herc20.our_htlc_address,
                bitcoin_identity: storage.derive_transient_identity(
                    swap_id,
                    our_role,
                    hbit_side(&new_match),
                ),
                secret_hash: SecretHash::new(swap_seed.derive_secret()),
            })
        }
        Role::Bob => RoleDependentParams::Bob(BobParams {
            ethereum_identity: order_herc20.our_htlc_address,
            bitcoin_identity: storage.derive_transient_identity(
                swap_id,
                our_role,
                hbit_side(&new_match),
            ),
        }),
    };
    let setup_swap_protocol = match protocol {
        SwapProtocol::HbitHerc20 { .. } => setup_swap::SwapProtocol::HbitHerc20,
        SwapProtocol::Herc20Hbit { .. } => setup_swap::SwapProtocol::Herc20Hbit,
    };

    Ok((swap_id, common_params, role_params, setup_swap_protocol))
}

fn hbit_side(new_match: &orderpool::Match) -> Side {
    match new_match.swap_protocol {
        SwapProtocol::HbitHerc20 { .. } => Side::Alpha,
        SwapProtocol::Herc20Hbit { .. } => Side::Beta,
    }
}
