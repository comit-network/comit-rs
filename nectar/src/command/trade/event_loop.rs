use crate::{
    bitcoin,
    command::{into_history_trade, FinishedSwap},
    ethereum::{self, dai},
    history::History,
    maker::{PublishOrders, TakeRequestDecision},
    network::{self, ActivePeer, SetupSwapContext, Swarm},
    order::BtcDaiOrderForm,
    swap::{hbit, Database, SwapExecutor, SwapKind, SwapParams},
    Maker, MidMarketRate, SwapId,
};
use anyhow::{bail, Context, Result};
use chrono::{NaiveDateTime, Utc};
use comit::{
    identity,
    network::{
        orderbook,
        setup_swap::{self, BobParams, CommonParams, RoleDependentParams},
    },
    order::SwapProtocol,
    orderpool::Match,
    Position,
};
use futures::{channel::mpsc::Receiver, FutureExt, StreamExt};
use std::sync::Arc;

pub(super) struct EventLoop {
    maker: Maker,
    swarm: Swarm,
    history: History,
    database: Arc<Database>,
    bitcoin_wallet: Arc<bitcoin::Wallet>,
    ethereum_wallet: Arc<ethereum::Wallet>,
    swap_executor: SwapExecutor,
}

impl EventLoop {
    pub fn new(
        maker: Maker,
        swarm: Swarm,
        history: History,
        database: Arc<Database>,
        bitcoin_wallet: Arc<bitcoin::Wallet>,
        ethereum_wallet: Arc<ethereum::Wallet>,
        swap_executor: SwapExecutor,
    ) -> Self {
        Self {
            maker,
            swarm,
            history,
            database,
            bitcoin_wallet,
            ethereum_wallet,
            swap_executor,
        }
    }

    pub async fn run(
        mut self,
        mut finished_swap_receiver: Receiver<FinishedSwap>,
        mut rate_update_receiver: Receiver<Result<MidMarketRate>>,
        mut btc_balance_update_receiver: Receiver<Result<bitcoin::Amount>>,
        mut dai_balance_update_receiver: Receiver<Result<dai::Amount>>,
    ) -> anyhow::Result<()> {
        loop {
            futures::select! {
                finished_swap = finished_swap_receiver.next().fuse() => {
                    if let Some(finished_swap) = finished_swap {
                        if let Err(err) = self.handle_finished_swap(finished_swap).await {
                            tracing::error!("Could handle finished swap: {:#}", err);
                        }
                    }
                },
                event = self.swarm.next().fuse() => {
                    if let Err(err) = self.handle_network_event(event).await {
                        tracing::error!("Network event handling failed: {:#}", err);
                    }
                },
                new_rate = rate_update_receiver.next().fuse() => {
                    if let Some(Ok(new_rate)) = new_rate {
                        if let Err(err) = self.handle_rate_update(new_rate) {
                            tracing::error!("Rate update handling failed: {:#}", err);
                        }
                    } else if let Some(Err(err)) = new_rate {
                        tracing::error!("Rate retrieval failed: {:#}", err);
                    }
                },
                new_btc_balance = btc_balance_update_receiver.next().fuse() => {
                    if let Some(new_btc_balance) = new_btc_balance {
                        match new_btc_balance {
                            Ok(new_btc_balance) => {
                                if let Err(err) = self.handle_btc_balance_update(new_btc_balance) {
                                    tracing::error!("BTC balance update handing failed: {:#}", err);
                                }
                            }
                            Err(err) => tracing::error!("BTC balance update failed: {:#}", err),
                        }
                    }
                },
                new_dai_balance = dai_balance_update_receiver.next().fuse() => {
                    if let Some(new_dai_balance) = new_dai_balance {
                        match new_dai_balance {
                            Ok(new_dai_balance) => {
                                if let Err(err) = self.handle_dai_balance_update(new_dai_balance) {
                                    tracing::error!("Dai balance update handing failed: {:#}", err);
                                }
                            }
                            Err(err) => tracing::error!("Dai balance update failed: {:#}", err),
                        }
                    }
                }
            }
        }
    }

    fn handle_rate_update(&mut self, new_rate: MidMarketRate) -> Result<()> {
        let publish_order = self.maker.update_rate(new_rate)?;
        if let Some(PublishOrders {
            new_sell_order,
            new_buy_order,
        }) = publish_order
        {
            self.swarm
                .orderbook
                .publish(new_sell_order.to_comit_order(self.maker.swap_protocol(Position::Sell)));
            self.swarm
                .orderbook
                .publish(new_buy_order.to_comit_order(self.maker.swap_protocol(Position::Buy)));
            self.swarm.orderbook.clear_own_orders();
        }

        Ok(())
    }

    fn handle_btc_balance_update(&mut self, new_btc_balance: bitcoin::Amount) -> Result<()> {
        if let Some(new_sell_order) = self.maker.update_bitcoin_balance(new_btc_balance)? {
            let order = new_sell_order.to_comit_order(self.maker.swap_protocol(Position::Sell));
            self.swarm.orderbook.clear_own_orders();
            self.swarm.orderbook.publish(order);
        }

        Ok(())
    }

    fn handle_dai_balance_update(&mut self, new_dai_balance: dai::Amount) -> Result<()> {
        if let Some(new_buy_order) = self.maker.update_dai_balance(new_dai_balance)? {
            let order = new_buy_order.to_comit_order(self.maker.swap_protocol(Position::Buy));
            self.swarm.orderbook.clear_own_orders();
            self.swarm.orderbook.publish(order);
        }

        Ok(())
    }

    async fn handle_finished_swap(&mut self, finished_swap: FinishedSwap) -> Result<()> {
        let peer_db_res = self
            .database
            .remove_active_peer(&finished_swap.peer)
            .await
            .context("Unable to remove from active takers");

        let trade = into_history_trade(
            finished_swap.peer.peer_id(),
            finished_swap.swap.clone(),
            #[cfg(not(test))]
            finished_swap.final_timestamp,
        );

        self.history
            .write(trade)
            .with_context(|| format!("Unable to register history entry: {:?}", finished_swap))?;

        let (dai, btc, swap_id) = match finished_swap.swap {
            SwapKind::HbitHerc20(swap) => {
                (Some(swap.herc20_params.asset.into()), None, swap.swap_id)
            }
            SwapKind::Herc20Hbit(swap) => (None, Some(swap.hbit_params.shared.asset), swap.swap_id),
        };

        self.database
            .remove_swap(&swap_id)
            .await
            .context("Unable to delete swap from db")?;

        // Only free funds if the swap was removed from the db
        self.maker.free_funds(dai, btc);

        peer_db_res
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
                let hbit_params = hbit::Params::new(exec_swap.hbit, bitcoin_transient_sk);

                let params = SwapParams {
                    swap_id,
                    start_of_swap,
                    hbit_params,
                    herc20_params: exec_swap.herc20,
                    secret_hash: exec_swap.hbit.secret_hash,
                    taker: ActivePeer {
                        peer_id: exec_swap.peer_id,
                    },
                };
                let swap_kind = match exec_swap.swap_protocol {
                    setup_swap::SwapProtocol::HbitHerc20 => SwapKind::HbitHerc20(params),
                    setup_swap::SwapProtocol::Herc20Hbit => SwapKind::Herc20Hbit(params),
                };

                self.database
                    .insert_swap(swap_kind.clone())
                    .await
                    .with_context(|| format!("Could not insert swap {}", swap_id))?;

                self.swap_executor.execute(swap_kind);
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

                let ongoing_trade_with_taker_exists = self
                    .database
                    .contains_active_peer(&taker)
                    .with_context(|| {
                        format!(
                            "could not determine if taker has ongoing trade; taker: {}, order: {}",
                            taker.peer_id(),
                            ours,
                        )
                    })?;

                if ongoing_trade_with_taker_exists {
                    bail!(
                        "ignoring take order request from taker with ongoing trade, taker: {:#}, order: {}",
                        taker.peer_id(),
                        ours,
                    );
                }

                let swap_id = SwapId::default();
                let index = self
                    .database
                    .fetch_inc_bitcoin_transient_key_index()
                    .await
                    .context("Could not fetch the index for the Bitcoin transient key")?;

                let token_contract = self.ethereum_wallet.dai_contract_address();
                let ethereum_identity = self.ethereum_wallet.account();
                let bitcoin_transient_sk = self
                    .bitcoin_wallet
                    .derive_transient_sk(index)
                    .context("Could not derive Bitcoin transient key")?;

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

                let (ethereum_absolute_expiry, bitcoin_absolute_expiry, swap_protocol) =
                    match swap_protocol {
                        SwapProtocol::HbitHerc20 {
                            hbit_expiry_offset,
                            herc20_expiry_offset,
                        } => {
                            // todo: do checked addition
                            #[allow(clippy::cast_sign_loss)]
                            #[allow(clippy::cast_possible_truncation)]
                            let ethereum_absolute_expiry =
                                (match_ref_point + time::Duration::from(herc20_expiry_offset))
                                    .timestamp() as u32;
                            #[allow(clippy::cast_sign_loss)]
                            #[allow(clippy::cast_possible_truncation)]
                            let bitcoin_absolute_expiry =
                                (match_ref_point + time::Duration::from(hbit_expiry_offset))
                                    .timestamp() as u32;

                            (
                                ethereum_absolute_expiry,
                                bitcoin_absolute_expiry,
                                setup_swap::SwapProtocol::HbitHerc20,
                            )
                        }
                        SwapProtocol::Herc20Hbit {
                            hbit_expiry_offset,
                            herc20_expiry_offset,
                        } => {
                            // todo: do checked addition
                            #[allow(clippy::cast_sign_loss)]
                            #[allow(clippy::cast_possible_truncation)]
                            let ethereum_absolute_expiry =
                                (match_ref_point + time::Duration::from(herc20_expiry_offset))
                                    .timestamp() as u32;
                            #[allow(clippy::cast_sign_loss)]
                            #[allow(clippy::cast_possible_truncation)]
                            let bitcoin_absolute_expiry =
                                (match_ref_point + time::Duration::from(hbit_expiry_offset))
                                    .timestamp() as u32;

                            (
                                ethereum_absolute_expiry,
                                bitcoin_absolute_expiry,
                                setup_swap::SwapProtocol::Herc20Hbit,
                            )
                        }
                    };

                let decision = self
                    .maker
                    .process_taken_order(form)
                    .context("Processing taken order yielded error")?;

                match decision {
                    TakeRequestDecision::GoForSwap => {
                        self.swarm
                            .setup_swap
                            .send(
                                &peer,
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
                                swap_protocol,
                                SetupSwapContext {
                                    swap_id,
                                    match_ref_point,
                                    bitcoin_transient_key_index: index,
                                },
                            )
                            .context("Sending setup swap message yielded error")?;

                        let _ = self
                            .database
                            .insert_active_peer(ActivePeer { peer_id: peer })
                            .await
                            .context("Failed to confirm order")?;

                        // todo: publish new order here?
                        // What if i publish a new order here and the does go
                        // through?
                    }
                    TakeRequestDecision::InsufficientFunds => bail!("Insufficient funds"),
                    TakeRequestDecision::RateNotProfitable => bail!("Rate not profitable"),
                };
            }
        }

        Ok(())
    }
}
