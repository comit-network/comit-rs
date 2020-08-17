use crate::{
    config::Settings,
    ethereum,
    local_swap_id::LocalSwapId,
    network::peer_tracker::PeerTracker,
    spawn,
    storage::{ForSwap, Load, RootSeed, Save, Storage, SwapContext},
    ProtocolSpawner,
};
use comit::{
    lightning,
    network::{
        comit::{Comit, LocalData, RemoteData},
        orderbook,
        orderbook::Orderbook,
        protocols::{announce, announce::Announce},
        swap_digest::SwapDigest,
        Identities, SharedSwapId, WhatAliceLearnedFromBob, WhatBobLearnedFromAlice,
    },
    orderpool, BtcDaiOrderForm, LockProtocol, Never, OrderId, Role, SecretHash,
};
use futures::TryFutureExt;
use libp2p::{NetworkBehaviour, PeerId};
use std::collections::HashMap;
use tokio::runtime::Handle;

/// A `NetworkBehaviour` that represents a COMIT node.
#[derive(NetworkBehaviour)]
#[allow(missing_debug_implementations)]
pub struct ComitNode {
    pub announce: Announce<LocalSwapId>,
    pub orderbook: Orderbook,
    pub comit: Comit,
    pub peer_tracker: PeerTracker,

    #[behaviour(ignore)]
    seed: RootSeed,
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
    order_addresses: HashMap<OrderId, BtcDaiOrderAddresses>,
    #[behaviour(ignore)]
    storage: Storage,
    #[behaviour(ignore)]
    protocol_spawner: ProtocolSpawner,
    /// The Bitcoin network we are currently connected to.
    #[behaviour(ignore)]
    _configured_bitcoin_network: bitcoin::Network,
    /// The Ethereum network we are currently connected to.
    #[behaviour(ignore)]
    _configured_ethereum_network: ethereum::ChainId,
    /// The address of the DAI ERC20 token contract on the current Ethereum
    /// network.
    #[behaviour(ignore)]
    _dai_contract_address: ethereum::Address,
}

/// A container used for temporarily storing two addresses provided by a user.
///
/// These addresses are used to set up a swap once the order they are associated
/// with yields a match. This allows us to avoid an interaction with the user at
/// the time of an order match.
#[derive(Debug)]
pub struct BtcDaiOrderAddresses {
    pub bitcoin: bitcoin::Address,
    pub ethereum: ethereum::Address,
}

impl ComitNode {
    pub fn new(
        seed: RootSeed,
        task_executor: Handle,
        storage: Storage,
        protocol_spawner: ProtocolSpawner,
        peer_id: PeerId,
        settings: &Settings,
    ) -> Self {
        Self {
            announce: Announce::default(),
            orderbook: Orderbook::new(peer_id),
            comit: Comit::default(),
            peer_tracker: PeerTracker::default(),
            seed,
            task_executor,
            local_data: HashMap::default(),
            local_swap_ids: HashMap::default(),
            order_addresses: Default::default(),
            storage,
            protocol_spawner,
            _configured_bitcoin_network: settings.bitcoin.network,
            _configured_ethereum_network: settings.ethereum.chain_id,
            _dai_contract_address: settings.ethereum.tokens.dai,
        }
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

    pub fn publish_order(
        &mut self,
        form: BtcDaiOrderForm,
        addresses: BtcDaiOrderAddresses,
    ) -> OrderId {
        let id = self.orderbook.publish(form);
        self.order_addresses.insert(id, addresses);

        id
    }

    fn assert_have_lnd_if_needed(
        &self,
        identity: Option<lightning::PublicKey>,
    ) -> anyhow::Result<()> {
        if identity.is_some() {
            return self.protocol_spawner.supports_halbit();
        }
        Ok(())
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
            orderbook::BehaviourOutEvent::OrderMatch(orderpool::Match {
                ours,
                theirs,
                peer,
                ..
            }) => {
                // TODO send setup-swap message here

                tracing::info!(
                    "order {} matched against order {} from {}",
                    ours,
                    theirs,
                    peer
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
                alpha: LockProtocol::Herc20,
                beta: LockProtocol::Halbit,
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
                alpha: LockProtocol::Herc20,
                beta: LockProtocol::Halbit,
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
                alpha: LockProtocol::Halbit,
                beta: LockProtocol::Herc20,
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
                alpha: LockProtocol::Halbit,
                beta: LockProtocol::Herc20,
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
                alpha: LockProtocol::Herc20,
                beta: LockProtocol::Hbit,
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
                alpha: LockProtocol::Herc20,
                beta: LockProtocol::Hbit,
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
                alpha: LockProtocol::Hbit,
                beta: LockProtocol::Herc20,
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
                alpha: LockProtocol::Hbit,
                beta: LockProtocol::Herc20,
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
