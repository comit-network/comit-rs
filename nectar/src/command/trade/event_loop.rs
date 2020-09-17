use crate::{
    bitcoin,
    command::{into_history_trade, trade::execute_swap, FinishedSwap},
    ethereum::{self, dai},
    history::History,
    maker::{PublishOrders, TakeRequestDecision},
    network::{self, ActivePeer, SetupSwapContext, Swarm},
    order::BtcDaiOrderForm,
    swap::{Database, SwapKind, SwapParams},
    Maker, MidMarketRate, SwapId,
};
use anyhow::{anyhow, bail, Context, Result};
use chrono::{NaiveDateTime, Utc};
use comit::{
    identity,
    network::{
        orderbook,
        setup_swap::{self, BobParams, CommonParams, RoleDependentParams},
    },
    order::SwapProtocol,
    orderpool::Match,
    Position, Role,
};
use futures::{
    channel::mpsc::{Receiver, Sender},
    FutureExt, StreamExt, TryFutureExt,
};
use std::sync::Arc;

pub struct EventLoop {
    maker: Maker,
    swarm: Swarm,
    history: History,
    database: Arc<Database>,
    bitcoin_wallet: Arc<bitcoin::Wallet>,
    ethereum_wallet: Arc<ethereum::Wallet>,
    bitcoin_connector: Arc<comit::btsieve::bitcoin::BitcoindConnector>,
    ethereum_connector: Arc<comit::btsieve::ethereum::Web3Connector>,
    finished_swap_sender: Sender<FinishedSwap>,
    finished_swap_receiver: Receiver<FinishedSwap>,
    rate_update_receiver: Receiver<Result<MidMarketRate>>,
    btc_balance_update_receiver: Receiver<Result<bitcoin::Amount>>,
    dai_balance_update_receiver: Receiver<Result<dai::Amount>>,
}

impl EventLoop {
    // TODO: Improve interface, especially regarding the receivers/senders
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        maker: Maker,
        swarm: Swarm,
        history: History,
        database: Arc<Database>,
        bitcoin_wallet: Arc<bitcoin::Wallet>,
        ethereum_wallet: Arc<ethereum::Wallet>,
        bitcoin_connector: Arc<comit::btsieve::bitcoin::BitcoindConnector>,
        ethereum_connector: Arc<comit::btsieve::ethereum::Web3Connector>,
        finished_swap_sender: Sender<FinishedSwap>,
        finished_swap_receiver: Receiver<FinishedSwap>,
        rate_update_receiver: Receiver<Result<MidMarketRate>>,
        btc_balance_update_receiver: Receiver<Result<bitcoin::Amount>>,
        dai_balance_update_receiver: Receiver<Result<dai::Amount>>,
    ) -> Self {
        Self {
            maker,
            swarm,
            history,
            database,
            bitcoin_wallet,
            ethereum_wallet,
            bitcoin_connector,
            ethereum_connector,
            finished_swap_sender,
            finished_swap_receiver,
            rate_update_receiver,
            btc_balance_update_receiver,
            dai_balance_update_receiver,
        }
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        loop {
            futures::select! {
                finished_swap = self.finished_swap_receiver.next().fuse() => {
                    if let Some(finished_swap) = finished_swap {
                        self.handle_finished_swap(finished_swap).await;
                    }
                },
                event = self.swarm.next().fuse() => {
                    self.handle_network_event(
                            event,
                        ).await?;
                },
                rate_update = self.rate_update_receiver.next().fuse() => {
                    self.handle_rate_update(rate_update.unwrap());
                },
                btc_balance_update = self.btc_balance_update_receiver.next().fuse() => {
                    self.handle_btc_balance_update(btc_balance_update.unwrap());
                },
                dai_balance_update = self.dai_balance_update_receiver.next().fuse() => {
                    self.handle_dai_balance_update(dai_balance_update.unwrap());
                }
            }
        }
    }

    fn handle_rate_update(&mut self, rate_update: Result<MidMarketRate>) {
        match rate_update {
            Ok(new_rate) => {
                let result = self.maker.update_rate(new_rate);
                match result {
                    Ok(Some(PublishOrders {
                        new_sell_order,
                        new_buy_order,
                    })) => {
                        self.swarm.orderbook.publish(
                            new_sell_order.to_comit_order(self.maker.swap_protocol(Position::Sell)),
                        );
                        self.swarm.orderbook.publish(
                            new_buy_order.to_comit_order(self.maker.swap_protocol(Position::Buy)),
                        );
                        self.swarm.orderbook.clear_own_orders();
                    }

                    Ok(None) => (),
                    Err(e) => tracing::warn!("Rate update yielded error: {}", e),
                }
            }
            Err(e) => {
                self.maker.invalidate_rate();
                tracing::error!(
                    "Unable to fetch latest rate! Fetching rate yielded error: {}",
                    e
                );
            }
        }
    }

    fn handle_btc_balance_update(&mut self, btc_balance_update: Result<bitcoin::Amount>) {
        match btc_balance_update {
            Ok(btc_balance) => match self.maker.update_bitcoin_balance(btc_balance) {
                Ok(Some(new_sell_order)) => {
                    let order =
                        new_sell_order.to_comit_order(self.maker.swap_protocol(Position::Sell));
                    self.swarm.orderbook.clear_own_orders();
                    self.swarm.orderbook.publish(order);
                }
                Ok(None) => (),
                Err(e) => tracing::warn!("Bitcoin balance update yielded error: {}", e),
            },
            Err(e) => {
                self.maker.invalidate_bitcoin_balance();
                tracing::error!(
                    "Unable to fetch bitcoin balance! Fetching balance yielded error: {}",
                    e
                );
            }
        }
    }

    fn handle_dai_balance_update(&mut self, dai_balance_update: Result<dai::Amount>) {
        match dai_balance_update {
            Ok(dai_balance) => match self.maker.update_dai_balance(dai_balance) {
                Ok(Some(new_buy_order)) => {
                    let order =
                        new_buy_order.to_comit_order(self.maker.swap_protocol(Position::Buy));
                    self.swarm.orderbook.clear_own_orders();
                    self.swarm.orderbook.publish(order);
                }
                Ok(None) => (),
                Err(e) => tracing::warn!("Dai balance update yielded error: {}", e),
            },
            Err(e) => {
                self.maker.invalidate_dai_balance();
                tracing::error!(
                    "Unable to fetch dai balance! Fetching balance yielded error: {}",
                    e
                );
            }
        }
    }

    async fn handle_finished_swap(&mut self, finished_swap: FinishedSwap) {
        {
            let trade = into_history_trade(
                finished_swap.peer.peer_id(),
                finished_swap.swap.clone(),
                #[cfg(not(test))]
                finished_swap.final_timestamp,
            );

            let _ = self.history.write(trade).map_err(|error| {
                tracing::error!(
                    "Unable to register history entry: {}; {:?}",
                    error,
                    finished_swap
                )
            });
        }

        let (dai, btc, swap_id) = match finished_swap.swap {
            SwapKind::HbitHerc20(swap) => {
                (Some(swap.herc20_params.asset.into()), None, swap.swap_id)
            }
            SwapKind::Herc20Hbit(swap) => (
                None,
                Some(swap.hbit_params.shared.asset.into()),
                swap.swap_id,
            ),
        };

        self.maker.free_funds(dai, btc);

        let _ = self
            .database
            .remove_active_peer(&finished_swap.peer)
            .await
            .map_err(|error| tracing::error!("Unable to remove from active takers: {}", error));

        let _ = self
            .database
            .remove_swap(&swap_id)
            .await
            .map_err(|error| tracing::error!("Unable to delete swap from db: {}", error));
    }

    #[allow(clippy::too_many_arguments)]
    async fn handle_network_event(&mut self, event: network::BehaviourOutEvent) -> Result<()> {
        match event {
            network::BehaviourOutEvent::Orderbook(event) => {
                self.handle_orderbook_event(event).await
            }
            network::BehaviourOutEvent::SetupSwap(event) => {
                self.handle_setup_swap_event(event).await
            }
        }
    }

    async fn handle_setup_swap_event(
        &mut self,
        event: setup_swap::BehaviourOutEvent<SetupSwapContext>,
    ) -> Result<()> {
        match event {
            setup_swap::BehaviourOutEvent::ExecutableSwap(exec_swap) => {
                let swap_id = exec_swap.context.swap_id;

                let start_of_swap = chrono::DateTime::from_utc(
                    NaiveDateTime::from_timestamp(exec_swap.context.match_ref_point.timestamp(), 0),
                    Utc,
                );

                let bitcoin_transient_sk = self
                    .bitcoin_wallet
                    .derive_transient_sk(exec_swap.context.bitcoin_transient_key_index)
                    .context("Could not derive Bitcoin transient key")?;

                let swap_kind = match (exec_swap.our_role, exec_swap.swap_protocol) {
                    // Sell
                    (Role::Alice, setup_swap::SwapProtocol::HbitHerc20) => {
                        SwapKind::HbitHerc20(SwapParams {
                            hbit_params: crate::swap::hbit::Params::new(
                                exec_swap.hbit,
                                bitcoin_transient_sk,
                            ),
                            herc20_params: crate::swap::herc20::Params {
                                asset: exec_swap.herc20.asset.clone(),
                                redeem_identity: exec_swap.herc20.refund_identity,
                                refund_identity: exec_swap.herc20.redeem_identity,
                                expiry: exec_swap.herc20.expiry,
                                secret_hash: exec_swap.herc20.secret_hash,
                                chain_id: exec_swap.herc20.chain_id,
                            },
                            secret_hash: exec_swap.hbit.secret_hash,
                            start_of_swap,
                            swap_id,
                            taker: ActivePeer {
                                peer_id: exec_swap.peer_id,
                            },
                        })
                    }
                    // Buy
                    (Role::Bob, setup_swap::SwapProtocol::HbitHerc20) => {
                        SwapKind::HbitHerc20(SwapParams {
                            hbit_params: crate::swap::hbit::Params::new(
                                exec_swap.hbit,
                                bitcoin_transient_sk,
                            ),
                            herc20_params: crate::swap::herc20::Params {
                                asset: exec_swap.herc20.asset.clone(),
                                redeem_identity: exec_swap.herc20.redeem_identity,
                                refund_identity: exec_swap.herc20.refund_identity,
                                expiry: exec_swap.herc20.expiry,
                                secret_hash: exec_swap.herc20.secret_hash,
                                chain_id: exec_swap.herc20.chain_id,
                            },
                            secret_hash: exec_swap.hbit.secret_hash,
                            start_of_swap,
                            swap_id,
                            taker: ActivePeer {
                                peer_id: exec_swap.peer_id,
                            },
                        })
                    }
                    // Buy
                    (Role::Alice, setup_swap::SwapProtocol::Herc20Hbit) => {
                        SwapKind::Herc20Hbit(SwapParams {
                            hbit_params: crate::swap::hbit::Params::new(
                                exec_swap.hbit,
                                bitcoin_transient_sk,
                            ),
                            herc20_params: crate::swap::herc20::Params {
                                asset: exec_swap.herc20.asset.clone(),
                                redeem_identity: exec_swap.herc20.redeem_identity,
                                refund_identity: exec_swap.herc20.refund_identity,
                                expiry: exec_swap.herc20.expiry,
                                secret_hash: exec_swap.herc20.secret_hash,
                                chain_id: exec_swap.herc20.chain_id,
                            },
                            secret_hash: exec_swap.hbit.secret_hash,
                            start_of_swap,
                            swap_id,
                            taker: ActivePeer {
                                peer_id: exec_swap.peer_id,
                            },
                        })
                    }
                    // Sell
                    (Role::Bob, setup_swap::SwapProtocol::Herc20Hbit) => {
                        SwapKind::Herc20Hbit(SwapParams {
                            hbit_params: crate::swap::hbit::Params::new(
                                exec_swap.hbit,
                                bitcoin_transient_sk,
                            ),
                            herc20_params: crate::swap::herc20::Params {
                                asset: exec_swap.herc20.asset.clone(),
                                redeem_identity: exec_swap.herc20.redeem_identity,
                                refund_identity: exec_swap.herc20.refund_identity,
                                expiry: exec_swap.herc20.expiry,
                                secret_hash: exec_swap.herc20.secret_hash,
                                chain_id: exec_swap.herc20.chain_id,
                            },
                            secret_hash: exec_swap.hbit.secret_hash,
                            start_of_swap,
                            swap_id,
                            taker: ActivePeer {
                                peer_id: exec_swap.peer_id,
                            },
                        })
                    }
                };
                let swap_id = swap_kind.swap_id();

                let res = self
                    .database
                    .insert_swap(swap_kind.clone())
                    .map_err(|e| tracing::error!("Could not insert swap {}: {:?}", swap_id, e))
                    .await;

                if res.is_ok() {
                    let _ = tokio::spawn(execute_swap(
                        Arc::clone(&self.database),
                        Arc::clone(&self.bitcoin_wallet),
                        Arc::clone(&self.ethereum_wallet),
                        Arc::clone(&self.bitcoin_connector),
                        Arc::clone(&self.ethereum_connector),
                        self.finished_swap_sender.clone(),
                        swap_kind,
                    ))
                    .await
                    .map_err(|e| {
                        tracing::error!("Execution failed for swap swap {}: {:?}", swap_id, e)
                    });
                }
            }
            setup_swap::BehaviourOutEvent::AlreadyHaveRoleParams { peer, .. } => {
                bail!("already received role params from {}", peer)
            }
        }

        Ok(())
    }

    async fn handle_orderbook_event(&mut self, event: orderbook::BehaviourOutEvent) -> Result<()> {
        match event {
            orderbook::BehaviourOutEvent::OrderMatch(Match {
                peer,
                price,
                quantity,
                our_position,
                swap_protocol,
                match_reference_point: match_ref_point,
                ours,
                ..
            }) => {
                let taker = ActivePeer {
                    peer_id: peer.clone(),
                };

                let ongoing_trade_with_taker_exists =
                    self.database.contains_active_peer(&taker).context(format!(
                        "could not determine if taker has ongoing trade; taker: {}, order: {}",
                        taker.peer_id(),
                        ours,
                    ))?;

                if ongoing_trade_with_taker_exists {
                    // TODO: We upgraded a warning to an error, think about it.
                    bail!(
                        "ignoring take order request from taker with ongoing trade, taker: {:?}, order: {}",
                        taker.peer_id(),
                        ours,
                    );
                }

                let swap_id = SwapId::default();
                let index = self
                    .database
                    .fetch_inc_bitcoin_transient_key_index()
                    .await
                    .map_err(|err| {
                        anyhow!(
                            "Could not fetch the index for the Bitcoin transient key: {:#}",
                            err
                        )
                    })?;

                let token_contract = self.ethereum_wallet.dai_contract_address();
                let ethereum_identity = self.ethereum_wallet.account();
                let bitcoin_transient_sk = self
                    .bitcoin_wallet
                    .derive_transient_sk(index)
                    .map_err(|err| anyhow!("Could not derive Bitcoin transient key: {:?}", err))?;

                let bitcoin_identity =
                    identity::Bitcoin::from_secret_key(&crate::SECP, &bitcoin_transient_sk);

                let erc20_quantity = quantity * price.clone();

                let form = BtcDaiOrderForm {
                    position: our_position,
                    quantity,
                    price,
                };

                let ethereum_chain_id = self.ethereum_wallet.chain_id();
                let bitcoin_network = self.bitcoin_wallet.network.into();

                let (role_dependant_params, common_params, swap_protocol) = match swap_protocol {
                    SwapProtocol::HbitHerc20 {
                        hbit_expiry_offset,
                        herc20_expiry_offset,
                    } => {
                        // todo: do checked addition
                        #[allow(clippy::cast_sign_loss)]
                        #[allow(clippy::cast_possible_truncation)]
                        let ethereum_absolute_expiry = (match_ref_point
                            + time::Duration::from(herc20_expiry_offset))
                        .timestamp() as u32;
                        #[allow(clippy::cast_sign_loss)]
                        #[allow(clippy::cast_possible_truncation)]
                        let bitcoin_absolute_expiry = (match_ref_point
                            + time::Duration::from(hbit_expiry_offset))
                        .timestamp() as u32;

                        match our_position {
                            Position::Buy => (
                                RoleDependentParams::Bob(BobParams {
                                    bitcoin_identity,
                                    ethereum_identity,
                                }),
                                CommonParams {
                                    erc20: comit::asset::Erc20 {
                                        token_contract,
                                        quantity: erc20_quantity,
                                    },
                                    bitcoin: quantity.to_inner(),
                                    ethereum_absolute_expiry,
                                    bitcoin_absolute_expiry,
                                    ethereum_chain_id,
                                    bitcoin_network,
                                },
                                comit::network::setup_swap::SwapProtocol::HbitHerc20,
                            ),
                            Position::Sell => (
                                RoleDependentParams::Bob(BobParams {
                                    bitcoin_identity,
                                    ethereum_identity,
                                }),
                                CommonParams {
                                    erc20: comit::asset::Erc20 {
                                        token_contract,
                                        quantity: erc20_quantity,
                                    },
                                    bitcoin: quantity.to_inner(),
                                    ethereum_absolute_expiry,
                                    bitcoin_absolute_expiry,
                                    ethereum_chain_id,
                                    bitcoin_network,
                                },
                                comit::network::setup_swap::SwapProtocol::HbitHerc20,
                            ),
                        }
                    }
                    SwapProtocol::Herc20Hbit {
                        hbit_expiry_offset,
                        herc20_expiry_offset,
                    } => {
                        // todo: do checked addition
                        #[allow(clippy::cast_sign_loss)]
                        #[allow(clippy::cast_possible_truncation)]
                        let ethereum_absolute_expiry = (match_ref_point
                            + time::Duration::from(herc20_expiry_offset))
                        .timestamp() as u32;
                        #[allow(clippy::cast_sign_loss)]
                        #[allow(clippy::cast_possible_truncation)]
                        let bitcoin_absolute_expiry = (match_ref_point
                            + time::Duration::from(hbit_expiry_offset))
                        .timestamp() as u32;

                        match our_position {
                            Position::Buy => (
                                RoleDependentParams::Bob(BobParams {
                                    bitcoin_identity,
                                    ethereum_identity,
                                }),
                                CommonParams {
                                    erc20: comit::asset::Erc20 {
                                        token_contract,
                                        quantity: erc20_quantity,
                                    },
                                    bitcoin: quantity.to_inner(),
                                    ethereum_absolute_expiry,
                                    bitcoin_absolute_expiry,
                                    ethereum_chain_id,
                                    bitcoin_network,
                                },
                                comit::network::setup_swap::SwapProtocol::Herc20Hbit,
                            ),
                            Position::Sell => (
                                RoleDependentParams::Bob(BobParams {
                                    bitcoin_identity,
                                    ethereum_identity,
                                }),
                                CommonParams {
                                    erc20: comit::asset::Erc20 {
                                        token_contract,
                                        quantity: erc20_quantity,
                                    },
                                    bitcoin: quantity.to_inner(),
                                    ethereum_absolute_expiry,
                                    bitcoin_absolute_expiry,
                                    ethereum_chain_id,
                                    bitcoin_network,
                                },
                                comit::network::setup_swap::SwapProtocol::HbitHerc20,
                            ),
                        }
                    }
                };

                let result = self.maker.process_taken_order(form);

                match result {
                    Ok(TakeRequestDecision::GoForSwap) => {
                        if let Err(e) = self.swarm.setup_swap.send(
                            &peer,
                            role_dependant_params,
                            common_params,
                            swap_protocol,
                            SetupSwapContext {
                                swap_id,
                                match_ref_point,
                                bitcoin_transient_key_index: index,
                            },
                        ) {
                            tracing::error!("Sending setup swap message yielded error: {}", e)
                        }

                        let _ = self
                            .database
                            .insert_active_peer(ActivePeer { peer_id: peer })
                            .await
                            .map_err(|e| tracing::error!("Failed to confirm order: {}", e));

                        // todo: publish new order here?
                        // What if i publish a new order here and the does go
                        // through?
                    }
                    Ok(TakeRequestDecision::InsufficientFunds) => {
                        tracing::info!("Insufficient funds")
                    }
                    Ok(TakeRequestDecision::RateNotProfitable) => {
                        tracing::info!("Rate not profitable")
                    }
                    Err(e) => tracing::error!("Processing taken order yielded error: {}", e),
                };
            }
        }

        Ok(())
    }
}
