use crate::{
    local_swap_id::LocalSwapId,
    network::peer_tracker::PeerTracker,
    spawn,
    storage::{
        BtcDaiOrder, ForSwap, InsertableOrderSwap, InsertableSecretHash, Load, Order,
        OrderHbitParams, RootSeed, Save, Storage, SwapContext,
    },
    ProtocolSpawner,
};
use chrono::NaiveDateTime;
use comit::{
    lightning,
    network::{
        comit::{Comit, LocalData, RemoteData},
        orderbook,
        orderbook::Orderbook,
        protocols::{announce, announce::Announce, setup_swap::SetupSwap},
        setup_swap,
        swap_digest::SwapDigest,
        Identities, SharedSwapId, WhatAliceLearnedFromBob, WhatBobLearnedFromAlice,
    },
    orderpool, LockProtocol, Never, OrderId, Quantity, Role, SecretHash, Side,
};
use futures::{channel::mpsc, SinkExt, TryFutureExt};
use libp2p::{identity::Keypair, NetworkBehaviour, PeerId};
use std::collections::HashMap;
use time::OffsetDateTime;
use tokio::runtime::Handle;

/// A `NetworkBehaviour` that represents a COMIT node.
#[derive(NetworkBehaviour)]
#[allow(missing_debug_implementations)]
pub struct ComitNode {
    pub announce: Announce<LocalSwapId>,
    pub setup_swap: SetupSwap<SetupSwapContext>,
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
    storage: Storage,
    #[behaviour(ignore)]
    protocol_spawner: ProtocolSpawner,
    #[behaviour(ignore)]
    matches_sender: mpsc::Sender<orderpool::Match>,
}

impl ComitNode {
    pub fn new(
        seed: RootSeed,
        task_executor: Handle,
        storage: Storage,
        protocol_spawner: ProtocolSpawner,
        peer_id: PeerId,
        key: Keypair,
        matches_sender: mpsc::Sender<orderpool::Match>,
    ) -> Self {
        Self {
            announce: Announce::default(),
            setup_swap: Default::default(),
            orderbook: Orderbook::new(peer_id, key),
            comit: Comit::default(),
            peer_tracker: PeerTracker::default(),
            seed,
            task_executor,
            local_data: HashMap::default(),
            local_swap_ids: HashMap::default(),
            storage,
            protocol_spawner,
            matches_sender,
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
}

/// The context we are passing to [`SetupSwap`] for each invocation.
#[derive(Debug, Clone, Copy)]
pub struct SetupSwapContext {
    pub swap: LocalSwapId,
    pub order: OrderId,
    pub match_reference_point: OffsetDateTime,
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
            orderbook::BehaviourOutEvent::OrderMatch(new_match) => {
                tracing::info!(
                    "order {} matched against order {} from {}",
                    new_match.ours,
                    new_match.theirs,
                    new_match.peer
                );
                let mut sender = self.matches_sender.clone();

                self.task_executor.spawn(async move {
                    if sender.send(new_match).await.is_err() {
                        tracing::error!("failed to dispatch new order match");
                    }
                });
            }
        }
    }
}

impl libp2p::swarm::NetworkBehaviourEventProcess<setup_swap::BehaviourOutEvent<SetupSwapContext>>
    for ComitNode
{
    fn inject_event(&mut self, event: setup_swap::BehaviourOutEvent<SetupSwapContext>) {
        match event {
            setup_swap::BehaviourOutEvent::ExecutableSwap(exec_swap) => {
                use crate::storage::{InsertableHbit, InsertableHerc20, InsertableSwap};
                use setup_swap::SwapProtocol::*;

                let SetupSwapContext {
                    swap: swap_id,
                    order: order_id,
                    match_reference_point,
                } = exec_swap.context;
                let protocol = exec_swap.swap_protocol;
                let role = exec_swap.our_role;
                let secret_hash = exec_swap.herc20.secret_hash;
                let start_of_swap =
                    NaiveDateTime::from_timestamp(match_reference_point.timestamp(), 0);

                let insertable_swap =
                    InsertableSwap::new(swap_id, exec_swap.peer_id, role, start_of_swap);

                let hbit_params = exec_swap.hbit;
                let insertable_hbit = move |swap_fk, our_final_address| {
                    InsertableHbit::new(
                        swap_fk,
                        hbit_params.asset,
                        hbit_params.network,
                        hbit_params.expiry.into(),
                        our_final_address,
                        match (role, protocol) {
                            (Role::Alice, HbitHerc20) => hbit_params.redeem_identity,
                            (Role::Bob, HbitHerc20) => hbit_params.refund_identity,
                            (Role::Alice, Herc20Hbit) => hbit_params.refund_identity,
                            (Role::Bob, Herc20Hbit) => hbit_params.redeem_identity,
                        },
                        match protocol {
                            HbitHerc20 => Side::Alpha,
                            Herc20Hbit => Side::Beta,
                        },
                    )
                };

                let herc20_params = exec_swap.herc20;
                let insertable_herc20 = move |swap_fk| {
                    InsertableHerc20::new(
                        swap_fk,
                        herc20_params.asset,
                        herc20_params.chain_id,
                        herc20_params.expiry.into(),
                        herc20_params.redeem_identity,
                        herc20_params.refund_identity,
                        match protocol {
                            Herc20Hbit => Side::Alpha,
                            HbitHerc20 => Side::Beta,
                        },
                    )
                };
                let insertable_secret_hash =
                    move |swap_fk| InsertableSecretHash::new(swap_fk, secret_hash);

                let save_data_and_start_swap = {
                    let spawner = self.protocol_spawner.clone();
                    let storage = self.storage.clone();
                    async move {
                        storage
                            .db
                            .do_in_transaction(|conn| {
                                let swap_pk = insertable_swap.insert(conn)?;

                                insertable_secret_hash(swap_pk).insert(conn)?;
                                insertable_herc20(swap_pk).insert(conn)?;

                                let order = Order::by_order_id(conn, order_id)?;
                                BtcDaiOrder::by_order(conn, &order)?.set_to_settling(conn)?;

                                InsertableOrderSwap::new(swap_pk, order.id).insert(conn)?;
                                let order_hbit_params = OrderHbitParams::by_order(conn, &order)?;
                                insertable_hbit(
                                    swap_pk,
                                    order_hbit_params.our_final_address.into(),
                                )
                                .insert(conn)?;

                                Ok(())
                            })
                            .await?;
                        spawn::spawn(&spawner, &storage, SwapContext {
                            id: swap_id,
                            role,
                            alpha: match protocol {
                                Herc20Hbit => LockProtocol::Herc20,
                                HbitHerc20 => LockProtocol::Hbit,
                            },
                            beta: match protocol {
                                Herc20Hbit => LockProtocol::Hbit,
                                HbitHerc20 => LockProtocol::Herc20,
                            },
                        })
                        .await?;

                        Ok(())
                    }
                };
                self.task_executor
                    .spawn(save_data_and_start_swap.map_err(|e: anyhow::Error| {
                        tracing::error!("{}", e);
                    }));
                if let Err(e) = self
                    .orderbook
                    .orderpool_mut()
                    .notify_swap_setup_successful(order_id, Quantity::new(hbit_params.asset))
                {
                    tracing::error!(
                        "failed to notify orderpool about successful swap setup: {:#}",
                        e
                    );
                }
            }
            setup_swap::BehaviourOutEvent::AlreadyHaveRoleParams { peer, .. } => tracing::error!(
                "Already have role dependent parameters from this peer: {}",
                peer
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
