use crate::{
    connectors::Connectors,
    local_swap_id::LocalSwapId,
    network::peer_tracker::PeerTracker,
    spawn,
    storage::{
        commands, InsertableOrderSwap, InsertableSecretHash, Order, OrderHbitParams, Storage,
        SwapContext,
    },
};
use comit::{
    network::{orderbook, orderbook::Orderbook, protocols::setup_swap::SetupSwap, setup_swap},
    orderpool, LockProtocol, Never, OrderId, Quantity, Role, Side,
};
use futures::{channel::mpsc, SinkExt, TryFutureExt};
use libp2p::{identity::Keypair, NetworkBehaviour, PeerId};
use time::OffsetDateTime;
use tokio::runtime::Handle;

/// A `NetworkBehaviour` that represents a COMIT node.
#[derive(NetworkBehaviour)]
#[allow(missing_debug_implementations)]
pub struct ComitNode {
    pub setup_swap: SetupSwap<SetupSwapContext>,
    pub orderbook: Orderbook,
    pub peer_tracker: PeerTracker,

    #[behaviour(ignore)]
    task_executor: Handle,
    #[behaviour(ignore)]
    storage: Storage,
    #[behaviour(ignore)]
    connectors: Connectors,
    #[behaviour(ignore)]
    matches_sender: mpsc::Sender<orderpool::Match>,
}

impl ComitNode {
    pub fn new(
        task_executor: Handle,
        storage: Storage,
        connectors: Connectors,
        peer_id: PeerId,
        key: Keypair,
        matches_sender: mpsc::Sender<orderpool::Match>,
    ) -> Self {
        Self {
            setup_swap: Default::default(),
            orderbook: Orderbook::new(peer_id, key),
            peer_tracker: PeerTracker::default(),
            task_executor,
            storage,
            connectors,
            matches_sender,
        }
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
                    match_reference_point: start_of_swap,
                } = exec_swap.context;
                let protocol = exec_swap.swap_protocol;
                let role = exec_swap.our_role;
                let secret_hash = exec_swap.herc20.secret_hash;

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
                    let connectors = self.connectors.clone();
                    let storage = self.storage.clone();
                    let handle = self.task_executor.clone();

                    async move {
                        storage
                            .db
                            .do_in_transaction(|conn| {
                                let swap_pk = insertable_swap.insert(conn)?;

                                insertable_secret_hash(swap_pk).insert(conn)?;
                                insertable_herc20(swap_pk).insert(conn)?;

                                commands::update_btc_dai_order_to_settling(conn, order_id)?;

                                let order = Order::by_order_id(conn, order_id)?;
                                InsertableOrderSwap::new(swap_pk, order.id).insert(conn)?;
                                let order_hbit_params = OrderHbitParams::by_order(conn, &order)?;
                                insertable_hbit(swap_pk, order_hbit_params.our_final_address)
                                    .insert(conn)?;

                                Ok(())
                            })
                            .await?;
                        spawn::spawn(connectors, storage, handle, SwapContext {
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
