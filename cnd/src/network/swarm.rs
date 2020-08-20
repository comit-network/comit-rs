use crate::{
    config::Settings,
    local_swap_id::LocalSwapId,
    network::{
        comit_node::{BtcDaiOrderAddresses, ComitNode},
        transport,
    },
    protocol_spawner::ProtocolSpawner,
    storage::{RootSeed, Storage},
};
use anyhow::Context as _;
use comit::{
    network::{swap_digest::SwapDigest, Identities},
    BtcDaiOrderForm, OrderId, Role,
};
use futures::stream::StreamExt;
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

        let transport = transport::build(local_key_pair.clone(), settings.network.listen.clone())?;

        let behaviour = ComitNode::new(
            seed,
            task_executor.clone(),
            storage,
            protocol_spawner,
            local_peer_id.clone(),
            local_key_pair,
            &settings,
        );

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

        if let Some(address_hint) = address_hint {
            if let Some(addr) = guard
                .peer_tracker
                .add_address_hint(peer.clone(), address_hint.clone())
            {
                tracing::warn!(
                    "clobbered old address hint, old: {}, new: {}",
                    addr,
                    address_hint,
                );
            }
            let existing_connection_to_peer =
                libp2p::Swarm::connection_info(&mut guard, &peer).is_some();

            if !existing_connection_to_peer {
                tracing::debug!("dialing ...");
                let _ = libp2p::Swarm::dial(&mut guard, &peer)?;
            }
        }

        guard.initiate_communication(id, peer, role, digest, identities)
    }

    pub async fn publish_order(
        &self,
        form: BtcDaiOrderForm,
        addresses: BtcDaiOrderAddresses,
    ) -> OrderId {
        self.inner.lock().await.publish_order(form, addresses)
    }

    pub async fn dial_addr(&mut self, addr: Multiaddr) -> anyhow::Result<()> {
        let mut guard = self.inner.lock().await;
        let _ = libp2p::Swarm::dial_addr(&mut *guard, addr)?;
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
