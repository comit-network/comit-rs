use crate::{
    hbit, herc20,
    local_swap_id::LocalSwapId,
    network::peer_tracker::PeerTracker,
    spawn,
    storage::{CreatedSwap, ForSwap, Load, RootSeed, Save, Storage, SwapContext},
    ProtocolSpawner,
};
use chrono::offset::Utc;
use comit::{
    asset::Erc20,
    bitcoin, ethereum, lightning,
    network::{
        comit::{Comit, LocalData, RemoteData},
        orderbook,
        orderbook::Orderbook,
        protocols::{announce, announce::Announce},
        swap_digest::SwapDigest,
        Identities, SharedSwapId, WhatAliceLearnedFromBob, WhatBobLearnedFromAlice,
    },
    Never, NewOrder, Order, OrderId, Position, Protocol, Role, SecretHash, Side,
};
use futures::TryFutureExt;
use libp2p::{core::Multiaddr, NetworkBehaviour, PeerId};
use std::collections::HashMap;
use tokio::runtime::Handle;

/// A `NetworkBehaviour` that represents a COMIT node.
#[derive(NetworkBehaviour)]
#[allow(missing_debug_implementations)]
pub struct ComitNode {
    announce: Announce<LocalSwapId>,
    orderbook: Orderbook,
    comit: Comit,
    peer_tracker: PeerTracker,

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
    storage: Storage,
    #[behaviour(ignore)]
    protocol_spawner: ProtocolSpawner,
    #[behaviour(ignore)]
    bitcoin_addresses: HashMap<bitcoin::PublicKey, comit::bitcoin::Address>,
    #[behaviour(ignore)]
    order_swap_ids: HashMap<OrderId, LocalSwapId>,
}

impl ComitNode {
    pub fn new(
        seed: RootSeed,
        task_executor: Handle,
        storage: Storage,
        protocol_spawner: ProtocolSpawner,
        peer_id: PeerId,
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
            storage,
            protocol_spawner,
            bitcoin_addresses: HashMap::default(),
            order_swap_ids: Default::default(),
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

    fn assert_have_lnd_if_needed(
        &self,
        identity: Option<lightning::PublicKey>,
    ) -> anyhow::Result<()> {
        if identity.is_some() {
            return self.protocol_spawner.supports_halbit();
        }
        Ok(())
    }

    /// The taker plays the role of Alice.
    pub fn take_order(
        &mut self,
        order_id: OrderId,
        swap_id: LocalSwapId,
        bitcoin_identity: comit::bitcoin::Address,
        ethereum_identity: ethereum::Address,
    ) -> anyhow::Result<()> {
        let order = self.orderbook.take(order_id)?;

        let transient = match order.position {
            Position::Buy => {
                self.storage
                    .derive_transient_identity(swap_id, Role::Alice, Side::Alpha)
            }
            Position::Sell => {
                self.storage
                    .derive_transient_identity(swap_id, Role::Alice, Side::Beta)
            }
        };

        self.bitcoin_addresses.insert(transient, bitcoin_identity);

        let data = LocalData {
            secret_hash: Some(SecretHash::new(
                self.seed.derive_swap_seed(swap_id).derive_secret(),
            )),
            ethereum_identity: Some(ethereum_identity),
            bitcoin_identity: Some(transient),
            lightning_identity: None,
        };
        self.local_data.insert(swap_id, data);
        self.order_swap_ids.insert(order_id, swap_id);

        Ok(())
    }

    /// The maker plays the role of Bob.
    pub fn make_order(
        &mut self,
        new_order: NewOrder,
        swap_id: LocalSwapId,
        ethereum_identity: ethereum::Address,
        bitcoin_identity: comit::bitcoin::Address,
    ) -> anyhow::Result<OrderId> {
        let transient = match new_order.position {
            Position::Buy => {
                self.storage
                    .derive_transient_identity(swap_id, Role::Bob, Side::Alpha)
            }
            Position::Sell => {
                self.storage
                    .derive_transient_identity(swap_id, Role::Bob, Side::Beta)
            }
        };

        self.bitcoin_addresses.insert(transient, bitcoin_identity);

        let data = LocalData {
            secret_hash: None,
            ethereum_identity: Some(ethereum_identity),
            bitcoin_identity: Some(transient),
            lightning_identity: None,
        };
        self.local_data.insert(swap_id, data);

        let order_id = self.orderbook.make(new_order);
        self.order_swap_ids.insert(order_id, swap_id);

        Ok(order_id)
    }

    pub fn get_order(&self, order_id: OrderId) -> Option<Order> {
        self.orderbook
            .orders()
            .all()
            .find(|order| order.id == order_id)
            .cloned()
    }

    pub fn get_orders(&self) -> Vec<Order> {
        self.orderbook.orders().all().cloned().collect()
    }

    pub fn connected_peers(&self) -> impl Iterator<Item = (PeerId, Vec<Multiaddr>)> {
        self.peer_tracker.connected_peers()
    }

    pub fn add_address_hint(&mut self, id: PeerId, addr: Multiaddr) -> Option<Multiaddr> {
        self.peer_tracker.add_address_hint(id, addr)
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
                let order = match self.orderbook.orders_mut().remove_ours(order_id) {
                    Some(order) => order,
                    None => {
                        return;
                    }
                };

                let &local_swap_id = match self.order_swap_ids.get(&order_id) {
                    Some(id) => id,
                    None => {
                        tracing::warn!(
                            "inconsistent state, non-existent order_id->local_swap_id mapping"
                        );
                        return;
                    }
                };

                let data = match self.local_data.get(&local_swap_id) {
                    Some(data) => data,
                    None => {
                        tracing::error!(
                            "inconsistent state, local data missing: {}",
                            local_swap_id
                        );
                        return;
                    }
                };
                let (bitcoin_identity, ethereum_identity) =
                    match (data.bitcoin_identity, data.ethereum_identity) {
                        (Some(bitcoin), Some(ethereum)) => (bitcoin, ethereum),
                        _ => {
                            tracing::error!(
                                "inconsistent state, identities] missing: {}",
                                local_swap_id
                            );
                            return;
                        }
                    };
                let final_identity = match self.bitcoin_addresses.get(&bitcoin_identity) {
                    Some(identity) => identity.clone(),
                    None => {
                        tracing::error!(
                            "inconsistent state, bitcoin address missing: {}",
                            local_swap_id
                        );
                        return;
                    }
                };
                let start_of_swap = Utc::now().naive_local();

                let storage = self.storage.clone();
                let order_id = order.id;

                match order.position {
                    Position::Buy => {
                        let swap = CreatedSwap {
                            swap_id: local_swap_id,

                            alpha: hbit::CreatedSwap {
                                amount: order.bitcoin_amount,
                                final_identity: final_identity.into(),
                                network: order.bitcoin_ledger,
                                absolute_expiry: order.bitcoin_absolute_expiry,
                            },
                            beta: herc20::CreatedSwap {
                                asset: Erc20 {
                                    token_contract: order.token_contract,
                                    quantity: order.ethereum_amount,
                                },
                                identity: ethereum_identity,
                                chain_id: order.ethereum_ledger.chain_id,
                                absolute_expiry: order.ethereum_absolute_expiry,
                            },
                            peer: peer_id.clone(),
                            address_hint: None,
                            role: Role::Bob,
                            start_of_swap,
                        };
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
                    }
                    Position::Sell => {
                        let swap = CreatedSwap {
                            swap_id: local_swap_id,
                            alpha: herc20::CreatedSwap {
                                asset: Erc20 {
                                    token_contract: order.token_contract,
                                    quantity: order.ethereum_amount,
                                },
                                identity: ethereum_identity,
                                chain_id: order.ethereum_ledger.chain_id,
                                absolute_expiry: order.ethereum_absolute_expiry,
                            },
                            beta: hbit::CreatedSwap {
                                amount: order.bitcoin_amount,
                                final_identity: final_identity.into(),
                                network: order.bitcoin_ledger,
                                absolute_expiry: order.bitcoin_absolute_expiry,
                            },
                            peer: peer_id.clone(),
                            address_hint: None,
                            role: Role::Bob,
                            start_of_swap,
                        };
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
                    }
                };

                // No other validation, just take the order. This
                // implies that an order can be taken multiple times.
                self.orderbook.confirm(order_id, response_channel, peer_id);
            }
            orderbook::BehaviourOutEvent::TakeOrderConfirmation {
                peer_id,
                order_id,
                shared_swap_id,
            } => {
                let local_swap_id = match self.order_swap_ids.get(&order_id) {
                    Some(id) => id,
                    None => {
                        tracing::error!(
                            "inconsistent swaps state, no local swap id found for order id: {}",
                            shared_swap_id
                        );
                        return;
                    }
                };
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
                self.local_swap_ids.insert(shared_swap_id, *local_swap_id);
                self.comit.communicate(peer_id, shared_swap_id, data);
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
