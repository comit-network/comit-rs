pub mod transport;

// Export comit network types while maintaining the module abstraction.
pub use ::comit::network::*;
pub use transport::ComitTransport;

use crate::{
    config::Settings,
    identity,
    network::{ExecutionParameters, LocalData},
    spawn,
    storage::{ForSwap, Save},
    Load, LocalSwapId, Protocol, ProtocolSpawner, Role, RootSeed, SecretHash, SharedSwapId,
    Storage, SwapContext,
};
use anyhow::Context;
use async_trait::async_trait;
use futures::{stream::StreamExt, Future, TryFutureExt};
use libp2p::{
    identity::{ed25519, Keypair},
    mdns::Mdns,
    swarm::{NetworkBehaviour as _, SwarmBuilder},
    Multiaddr, NetworkBehaviour, PeerId,
};
use std::{
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

        let transport = transport::build_comit_transport(local_key_pair)?;
        let behaviour = ComitNode::new(seed, task_executor.clone(), storage, protocol_spawner)?;

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
        peer: DialInformation,
        role: Role,
        digest: SwapDigest,
        identities: Identities,
    ) -> anyhow::Result<()> {
        let mut guard = self.inner.lock().await;
        guard.initiate_communication(id, peer, role, digest, identities)
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

/// A `NetworkBehaviour` that delegates to the `Comit` and `Mdns` behaviours.
#[derive(NetworkBehaviour)]
#[allow(missing_debug_implementations)]
pub struct ComitNode {
    announce: Announce,
    execution_parameters: ExecutionParameters,
    /// Multicast DNS discovery network behaviour.
    mdns: Mdns,

    #[behaviour(ignore)]
    pub seed: RootSeed,
    #[behaviour(ignore)]
    task_executor: Handle,

    #[behaviour(ignore)]
    pub storage: Storage,
    #[behaviour(ignore)]
    pub protocol_spawner: ProtocolSpawner,
}

#[derive(Debug, Clone, Copy, thiserror::Error)]
pub enum RequestError {
    #[error("peer node had an internal error while processing the request")]
    InternalError,
    #[error("peer node produced an invalid response")]
    InvalidResponse,
    #[error("failed to establish a new connection to make the request")]
    Connecting(io::ErrorKind),
    #[error("unable to send the data on the existing connection")]
    Connection,
}

impl ComitNode {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        seed: RootSeed,
        task_executor: Handle,
        storage: Storage,
        protocol_spawner: ProtocolSpawner,
    ) -> Result<Self, io::Error> {
        Ok(Self {
            announce: Announce::default(),
            execution_parameters: ExecutionParameters::default(),
            mdns: Mdns::new()?,
            seed,
            task_executor,
            storage,
            protocol_spawner,
        })
    }

    pub fn initiate_communication(
        &mut self,
        id: LocalSwapId,
        peer: DialInformation,
        role: Role,
        digest: SwapDigest,
        identities: Identities,
    ) -> anyhow::Result<()> {
        match role {
            Role::Alice => self.initiate_communication_for_alice(id, peer, digest, identities),
            Role::Bob => self.initiate_communication_for_bob(id, peer, digest, identities),
        }
    }

    fn initiate_communication_for_alice(
        &mut self,
        local_swap_id: LocalSwapId,
        dial_info: DialInformation,
        digest: SwapDigest,
        identities: Identities,
    ) -> anyhow::Result<()> {
        // At this stage we do not know if the arguments passed to us make up a
        // valid swap, we just trust the controller to pass in something
        // valid. Do _some_ form of validation here so that we can early return
        // errors and they do not get lost in the asynchronous call chain that
        // kicks off here.
        self.assert_have_lnd_if_needed(identities.lightning_identity)?;

        let swap_seed = self.seed.derive_swap_seed(local_swap_id);
        let secret = swap_seed.derive_secret();
        let secret_hash = SecretHash::new(secret);
        let data = LocalData::for_alice(secret_hash, identities);

        tracing::info!("Starting announcement for swap: {}", digest);
        self.announce.announce_swap(digest.clone(), dial_info);
        self.execution_parameters
            .swaps
            .create_as_pending_confirmation(digest, local_swap_id, data)?;

        Ok(())
    }

    fn initiate_communication_for_bob(
        &mut self,
        local_swap_id: LocalSwapId,
        dial_info: DialInformation,
        digest: SwapDigest,
        identities: Identities,
    ) -> anyhow::Result<()> {
        let shared_swap_id = SharedSwapId::default();
        let data = LocalData::for_bob(shared_swap_id, identities);

        if let Ok((shared_swap_id, peer_id, io)) = self
            .execution_parameters
            .swaps
            .move_pending_creation_to_communicate(
                &digest,
                local_swap_id,
                dial_info.peer_id.clone(),
                data,
            )
        {
            tracing::info!("Confirm & communicate for swap: {}", digest);
            ExecutionParameters::confirm(shared_swap_id, io);
            let addresses = self.announce.addresses_of_peer(&peer_id);
            self.execution_parameters
                .communicate(shared_swap_id, peer_id, data, addresses)
        } else {
            self.execution_parameters
                .swaps
                .create_as_pending_announcement(
                    digest.clone(),
                    local_swap_id,
                    dial_info.peer_id,
                    data,
                )?;
            tracing::debug!("Swap {} waiting for announcement", digest);
        }

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
        let mut swarm = self.inner.lock().await;
        Box::new(swarm.announce.connected_peers())
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

impl libp2p::swarm::NetworkBehaviourEventProcess<libp2p::mdns::MdnsEvent> for ComitNode {
    fn inject_event(&mut self, _event: libp2p::mdns::MdnsEvent) {}
}

impl libp2p::swarm::NetworkBehaviourEventProcess<()> for ComitNode {
    fn inject_event(&mut self, _event: ()) {}
}

impl libp2p::swarm::NetworkBehaviourEventProcess<execution_parameters::BehaviourOutEvent>
    for ComitNode
{
    fn inject_event(&mut self, event: execution_parameters::BehaviourOutEvent) {
        let execution_parameters::BehaviourOutEvent {
            local_swap_id,
            remote_data,
        } = event;

        let storage = self.storage.clone();
        let spawner = self.protocol_spawner.clone();

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

// It is already split in smaller functions
#[allow(clippy::cognitive_complexity)]
impl libp2p::swarm::NetworkBehaviourEventProcess<announce::BehaviourOutEvent> for ComitNode {
    fn inject_event(&mut self, event: announce::BehaviourOutEvent) {
        match event {
            announce::BehaviourOutEvent::ReceivedAnnouncement { peer, io } => {
                tracing::info!("Peer {} announced a swap ({})", peer, io.swap_digest);
                let span =
                    tracing::trace_span!("swap", digest = format_args!("{}", io.swap_digest));
                let _enter = span.enter();
                match self
                    .execution_parameters
                    .swaps
                    .move_pending_announcement_to_communicate(&io.swap_digest, &peer)
                {
                    Ok((shared_swap_id, create_params)) => {
                        tracing::debug!("Swap confirmation and communication has started.");
                        ExecutionParameters::confirm(shared_swap_id, *io);
                        let addresses = self.announce.addresses_of_peer(&peer);
                        self.execution_parameters.communicate(
                            shared_swap_id,
                            peer,
                            create_params,
                            addresses,
                        );
                    }
                    Err(swaps::Error::NotFound) => {
                        tracing::debug!("Swap has not been created yet, parking it.");
                        let _ = self
                            .execution_parameters
                            .swaps
                            .insert_pending_creation((&io.swap_digest).clone(), peer, *io)
                            .map_err(|_| {
                                tracing::error!(
                                    "Swap already known, Alice appeared to have sent it twice."
                                )
                            });
                    }
                    Err(err) => tracing::warn!(
                        "Announcement for {} was not processed due to {}",
                        io.swap_digest,
                        err
                    ),
                }
            }
            announce::BehaviourOutEvent::ReceivedConfirmation {
                peer,
                swap_digest,
                swap_id: shared_swap_id,
            } => {
                if let Some(data) = self
                    .execution_parameters
                    .swaps
                    .move_pending_confirmation_to_communicate(&swap_digest, shared_swap_id)
                {
                    let addresses = self.announce.addresses_of_peer(&peer);
                    self.execution_parameters
                        .communicate(shared_swap_id, peer, data, addresses);
                } else {
                    tracing::warn!(
                        "Confirmation received for unknown swap {} from {}",
                        shared_swap_id,
                        peer
                    );
                }
            }
            announce::BehaviourOutEvent::Error { peer, error } => {
                tracing::warn!(
                    "failed to complete announce protocol with {} because {:?}",
                    peer,
                    error
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
