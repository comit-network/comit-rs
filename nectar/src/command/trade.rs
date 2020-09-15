use crate::{
    bitcoin,
    command::{into_history_trade, FinishedSwap},
    config::Settings,
    ethereum::{self, dai},
    history::History,
    maker::{PublishOrders, TakeRequestDecision},
    mid_market_rate::get_btc_dai_mid_market_rate,
    network::{self, new_swarm, ActivePeer, SetupSwapContext, Swarm},
    order::BtcDaiOrderForm,
    swap::{Database, SwapKind, SwapParams},
    Maker, MidMarketRate, Seed, Spread, SwapId,
};
use anyhow::{anyhow, bail, Context};
use chrono::{NaiveDateTime, Utc};
use comit::{
    btsieve::{bitcoin::BitcoindConnector, ethereum::Web3Connector},
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
    Future, FutureExt, SinkExt, StreamExt, TryFutureExt,
};
use futures_timer::Delay;
use std::{borrow::Borrow, sync::Arc, time::Duration};

const ENSURED_CONSUME_ZERO_BUFFER: usize = 0;

pub async fn trade(
    seed: &Seed,
    settings: Settings,
    bitcoin_wallet: bitcoin::Wallet,
    ethereum_wallet: ethereum::Wallet,
) -> anyhow::Result<()> {
    let bitcoin_wallet = Arc::new(bitcoin_wallet);
    let ethereum_wallet = Arc::new(ethereum_wallet);

    let mut maker = init_maker(
        Arc::clone(&bitcoin_wallet),
        Arc::clone(&ethereum_wallet),
        settings.clone(),
    )
    .await
    .context("Could not initialise Maker")?;

    #[cfg(not(test))]
    let db = Arc::new(Database::new(&settings.data.dir.join("database"))?);
    #[cfg(test)]
    let db = Arc::new(Database::new_test()?);

    let mut swarm = new_swarm(network::Seed::new(seed.bytes()), &settings)?;

    let initial_sell_order = maker
        .new_sell_order()
        .context("Could not generate sell order")?;

    let initial_buy_order = maker
        .new_buy_order()
        .context("Could not generate buy order")?;

    swarm
        .orderbook
        .publish(initial_sell_order.to_comit_order(maker.swap_protocol(Position::Buy)));
    swarm
        .orderbook
        .publish(initial_buy_order.to_comit_order(maker.swap_protocol(Position::Sell)));

    let update_interval = Duration::from_secs(15u64);

    let (rate_future, mut rate_update_receiver) = init_rate_updates(update_interval);
    let (btc_balance_future, mut btc_balance_update_receiver) =
        init_bitcoin_balance_updates(update_interval, Arc::clone(&bitcoin_wallet));
    let (dai_balance_future, mut dai_balance_update_receiver) =
        init_dai_balance_updates(update_interval, Arc::clone(&ethereum_wallet));

    tokio::spawn(rate_future);
    tokio::spawn(btc_balance_future);
    tokio::spawn(dai_balance_future);

    let (swap_execution_finished_sender, mut swap_execution_finished_receiver) =
        futures::channel::mpsc::channel::<FinishedSwap>(ENSURED_CONSUME_ZERO_BUFFER);

    let mut history = History::new(settings.data.dir.join("history.csv").as_path())?;

    let bitcoin_connector = Arc::new(BitcoindConnector::new(settings.bitcoin.bitcoind.node_url)?);
    let ethereum_connector = Arc::new(Web3Connector::new(settings.ethereum.node_url));

    respawn_swaps(
        Arc::clone(&db),
        &mut maker,
        Arc::clone(&bitcoin_wallet),
        Arc::clone(&ethereum_wallet),
        Arc::clone(&bitcoin_connector),
        Arc::clone(&ethereum_connector),
        swap_execution_finished_sender.clone(),
    )
    .context("Could not respawn swaps")?;

    loop {
        futures::select! {
            finished_swap = swap_execution_finished_receiver.next().fuse() => {
                if let Some(finished_swap) = finished_swap {
                    handle_finished_swap(finished_swap, &mut maker, &db, &mut history, &mut swarm).await;
                }
            },
            orderbook_event = swarm.next().fuse() => {
                handle_network_event(
                        orderbook_event,
                        &mut maker,
                        &mut swarm,
                        Arc::clone(&db),
                        Arc::clone(&bitcoin_wallet),
                        Arc::clone(&ethereum_wallet),
                        Arc::clone(&bitcoin_connector),
                        Arc::clone(&ethereum_connector),
                        swap_execution_finished_sender.clone()
                    ).await?;
            },
            rate_update = rate_update_receiver.next().fuse() => {
                handle_rate_update(rate_update.unwrap(), &mut maker, &mut swarm);
            },
            btc_balance_update = btc_balance_update_receiver.next().fuse() => {
                handle_btc_balance_update(btc_balance_update.unwrap(), &mut maker, &mut swarm);
            },
            dai_balance_update = dai_balance_update_receiver.next().fuse() => {
                handle_dai_balance_update(dai_balance_update.unwrap(), &mut maker, &mut swarm);
            }
        }
    }
}

async fn init_maker(
    bitcoin_wallet: Arc<bitcoin::Wallet>,
    ethereum_wallet: Arc<ethereum::Wallet>,
    settings: Settings,
) -> anyhow::Result<Maker> {
    let initial_btc_balance = bitcoin_wallet
        .balance()
        .await
        .context("Could not get Bitcoin balance")?;

    let initial_dai_balance = ethereum_wallet
        .dai_balance()
        .await
        .context("Could not get Dai balance")?;

    let btc_max_sell = settings.maker.max_sell.bitcoin;
    let dai_max_sell = settings.maker.max_sell.dai.clone();
    let btc_fee_reserve = settings.maker.maximum_possible_fee.bitcoin;

    let initial_rate = get_btc_dai_mid_market_rate()
        .await
        .context("Could not get rate")?;

    let spread: Spread = settings.maker.spread;

    Ok(Maker::new(
        initial_btc_balance,
        initial_dai_balance,
        btc_fee_reserve,
        btc_max_sell,
        dai_max_sell,
        initial_rate,
        spread,
        settings.bitcoin.network,
        settings.ethereum.chain,
        // todo: get from config
        Role::Bob,
    ))
}

fn init_rate_updates(
    update_interval: Duration,
) -> (
    impl Future<Output = comit::Never> + Send,
    Receiver<anyhow::Result<MidMarketRate>>,
) {
    let (mut sender, receiver) = futures::channel::mpsc::channel::<anyhow::Result<MidMarketRate>>(
        ENSURED_CONSUME_ZERO_BUFFER,
    );

    let future = async move {
        loop {
            let rate = get_btc_dai_mid_market_rate().await;

            let _ = sender.send(rate).await.map_err(|e| {
                tracing::trace!(
                    "Error when sending rate update from sender to receiver: {}",
                    e
                )
            });

            Delay::new(update_interval).await;
        }
    };

    (future, receiver)
}

fn init_bitcoin_balance_updates(
    update_interval: Duration,
    wallet: Arc<bitcoin::Wallet>,
) -> (
    impl Future<Output = comit::Never> + Send,
    Receiver<anyhow::Result<bitcoin::Amount>>,
) {
    let (mut sender, receiver) = futures::channel::mpsc::channel::<anyhow::Result<bitcoin::Amount>>(
        ENSURED_CONSUME_ZERO_BUFFER,
    );

    let future = async move {
        loop {
            let balance = wallet.balance().await;

            let _ = sender.send(balance).await.map_err(|e| {
                tracing::trace!(
                    "Error when sending balance update from sender to receiver: {}",
                    e
                )
            });

            Delay::new(update_interval).await;
        }
    };

    (future, receiver)
}

fn init_dai_balance_updates(
    update_interval: Duration,
    wallet: Arc<ethereum::Wallet>,
) -> (
    impl Future<Output = comit::Never> + Send,
    Receiver<anyhow::Result<dai::Amount>>,
) {
    let (mut sender, receiver) =
        futures::channel::mpsc::channel::<anyhow::Result<dai::Amount>>(ENSURED_CONSUME_ZERO_BUFFER);

    let future = async move {
        loop {
            let balance = wallet.dai_balance().await;

            let _ = sender.send(balance).await.map_err(|e| {
                tracing::trace!(
                    "Error when sending rate balance from sender to receiver: {}",
                    e
                )
            });

            Delay::new(update_interval).await;
        }
    };

    (future, receiver)
}

#[allow(clippy::too_many_arguments)]
async fn execute_swap(
    db: Arc<Database>,
    bitcoin_wallet: Arc<bitcoin::Wallet>,
    ethereum_wallet: Arc<ethereum::Wallet>,
    bitcoin_connector: Arc<comit::btsieve::bitcoin::BitcoindConnector>,
    ethereum_connector: Arc<comit::btsieve::ethereum::Web3Connector>,
    mut finished_swap_sender: Sender<FinishedSwap>,
    swap: SwapKind,
) -> anyhow::Result<()> {
    db.insert_swap(swap.clone()).await?;

    swap.execute(
        db,
        bitcoin_wallet,
        ethereum_wallet,
        bitcoin_connector,
        ethereum_connector,
    )
    .await?;

    let _ = finished_swap_sender
        .send(FinishedSwap::new(
            swap.clone(),
            swap.params().taker,
            chrono::Utc::now(),
        ))
        .await
        .map_err(|_| {
            tracing::trace!("Error when sending execution finished from sender to receiver.")
        });

    Ok(())
}

fn respawn_swaps(
    db: Arc<Database>,
    maker: &mut Maker,
    bitcoin_wallet: Arc<bitcoin::Wallet>,
    ethereum_wallet: Arc<ethereum::Wallet>,
    bitcoin_connector: Arc<comit::btsieve::bitcoin::BitcoindConnector>,
    ethereum_connector: Arc<comit::btsieve::ethereum::Web3Connector>,
    finished_swap_sender: Sender<FinishedSwap>,
) -> anyhow::Result<()> {
    for swap in db.all_swaps()?.into_iter() {
        // Reserve funds
        match swap {
            SwapKind::HbitHerc20(SwapParams {
                ref herc20_params, ..
            }) => {
                let fund_amount = herc20_params.asset.clone().into();
                maker.dai_reserved_funds = maker.dai_reserved_funds.clone() + fund_amount;
            }
            SwapKind::Herc20Hbit(SwapParams { hbit_params, .. }) => {
                let fund_amount = hbit_params.shared.asset.into();
                maker.btc_reserved_funds = maker.btc_reserved_funds + fund_amount + maker.btc_fee;
            }
        };

        tokio::spawn(execute_swap(
            Arc::clone(&db),
            Arc::clone(&bitcoin_wallet),
            Arc::clone(&ethereum_wallet),
            Arc::clone(&bitcoin_connector),
            Arc::clone(&ethereum_connector),
            finished_swap_sender.clone(),
            swap,
        ));
    }

    Ok(())
}

fn handle_rate_update(
    rate_update: anyhow::Result<MidMarketRate>,
    maker: &mut Maker,
    swarm: &mut Swarm,
) {
    match rate_update {
        Ok(new_rate) => {
            let result = maker.update_rate(new_rate);
            match result {
                Ok(Some(PublishOrders {
                    new_sell_order,
                    new_buy_order,
                })) => {
                    swarm.orderbook.publish(
                        new_sell_order.to_comit_order(maker.swap_protocol(Position::Sell)),
                    );
                    swarm
                        .orderbook
                        .publish(new_buy_order.to_comit_order(maker.swap_protocol(Position::Buy)));
                    swarm.orderbook.clear_own_orders();
                }

                Ok(None) => (),
                Err(e) => tracing::warn!("Rate update yielded error: {}", e),
            }
        }
        Err(e) => {
            maker.invalidate_rate();
            tracing::error!(
                "Unable to fetch latest rate! Fetching rate yielded error: {}",
                e
            );
        }
    }
}

fn handle_btc_balance_update(
    btc_balance_update: anyhow::Result<bitcoin::Amount>,
    maker: &mut Maker,
    swarm: &mut Swarm,
) {
    match btc_balance_update {
        Ok(btc_balance) => match maker.update_bitcoin_balance(btc_balance) {
            Ok(Some(new_sell_order)) => {
                let order = new_sell_order.to_comit_order(maker.swap_protocol(Position::Sell));
                swarm.orderbook.clear_own_orders();
                swarm.orderbook.publish(order);
            }
            Ok(None) => (),
            Err(e) => tracing::warn!("Bitcoin balance update yielded error: {}", e),
        },
        Err(e) => {
            maker.invalidate_bitcoin_balance();
            tracing::error!(
                "Unable to fetch bitcoin balance! Fetching balance yielded error: {}",
                e
            );
        }
    }
}

fn handle_dai_balance_update(
    dai_balance_update: anyhow::Result<dai::Amount>,
    maker: &mut Maker,
    swarm: &mut Swarm,
) {
    match dai_balance_update {
        Ok(dai_balance) => match maker.update_dai_balance(dai_balance) {
            Ok(Some(new_buy_order)) => {
                let order = new_buy_order.to_comit_order(maker.swap_protocol(Position::Buy));
                swarm.orderbook.clear_own_orders();
                swarm.orderbook.publish(order);
            }
            Ok(None) => (),
            Err(e) => tracing::warn!("Dai balance update yielded error: {}", e),
        },
        Err(e) => {
            maker.invalidate_dai_balance();
            tracing::error!(
                "Unable to fetch dai balance! Fetching balance yielded error: {}",
                e
            );
        }
    }
}

async fn handle_finished_swap(
    finished_swap: FinishedSwap,
    maker: &mut Maker,
    db: &Database,
    history: &mut History,
    _swarm: &mut Swarm,
) {
    {
        let trade = into_history_trade(
            finished_swap.peer.peer_id(),
            finished_swap.swap.clone(),
            #[cfg(not(test))]
            finished_swap.final_timestamp,
        );

        let _ = history.write(trade).map_err(|error| {
            tracing::error!(
                "Unable to register history entry: {}; {:?}",
                error,
                finished_swap
            )
        });
    }

    let (dai, btc, swap_id) = match finished_swap.swap {
        SwapKind::HbitHerc20(swap) => (Some(swap.herc20_params.asset.into()), None, swap.swap_id),
        SwapKind::Herc20Hbit(swap) => (
            None,
            Some(swap.hbit_params.shared.asset.into()),
            swap.swap_id,
        ),
    };

    maker.free_funds(dai, btc);

    let _ = db
        .remove_active_peer(&finished_swap.peer)
        .await
        .map_err(|error| tracing::error!("Unable to remove from active takers: {}", error));

    let _ = db
        .remove_swap(&swap_id)
        .await
        .map_err(|error| tracing::error!("Unable to delete swap from db: {}", error));
}

#[allow(clippy::too_many_arguments)]
async fn handle_network_event(
    event: network::BehaviourOutEvent,
    maker: &mut Maker,
    swarm: &mut Swarm,
    database: Arc<Database>,
    bitcoin_wallet: Arc<bitcoin::Wallet>,
    ethereum_wallet: Arc<ethereum::Wallet>,
    bitcoin_connector: Arc<comit::btsieve::bitcoin::BitcoindConnector>,
    ethereum_connector: Arc<comit::btsieve::ethereum::Web3Connector>,
    finished_swap_sender: Sender<FinishedSwap>,
) -> anyhow::Result<()> {
    match event {
        network::BehaviourOutEvent::Orderbook(event) => {
            handle_orderbook_event(
                event,
                maker,
                swarm,
                database,
                bitcoin_wallet,
                ethereum_wallet,
            )
            .await
        }
        network::BehaviourOutEvent::SetupSwap(event) => {
            handle_setup_swap_event(
                event,
                database,
                bitcoin_wallet,
                ethereum_wallet,
                bitcoin_connector,
                ethereum_connector,
                finished_swap_sender,
            )
            .await
        }
    }
}

async fn handle_setup_swap_event(
    event: setup_swap::BehaviourOutEvent<SetupSwapContext>,
    database: Arc<Database>,
    bitcoin_wallet: Arc<bitcoin::Wallet>,
    ethereum_wallet: Arc<ethereum::Wallet>,
    bitcoin_connector: Arc<comit::btsieve::bitcoin::BitcoindConnector>,
    ethereum_connector: Arc<comit::btsieve::ethereum::Web3Connector>,
    finished_swap_sender: Sender<FinishedSwap>,
) -> anyhow::Result<()> {
    match event {
        setup_swap::BehaviourOutEvent::ExecutableSwap(exec_swap) => {
            let swap_id = exec_swap.context.swap_id;

            let start_of_swap = chrono::DateTime::from_utc(
                NaiveDateTime::from_timestamp(exec_swap.context.match_ref_point.timestamp(), 0),
                Utc,
            );

            let bitcoin_transient_sk = bitcoin_wallet
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

            let res = database
                .insert_swap(swap_kind.clone())
                .map_err(|e| tracing::error!("Could not insert swap {}: {:?}", swap_id, e))
                .await;

            if res.is_ok() {
                let _ = tokio::spawn(execute_swap(
                    database,
                    bitcoin_wallet,
                    ethereum_wallet,
                    bitcoin_connector,
                    ethereum_connector,
                    finished_swap_sender,
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

async fn handle_orderbook_event(
    event: orderbook::BehaviourOutEvent,
    maker: &mut Maker,
    swarm: &mut Swarm,
    database: impl Borrow<Database>,
    bitcoin_wallet: impl Borrow<bitcoin::Wallet>,
    ethereum_wallet: impl Borrow<ethereum::Wallet>,
) -> anyhow::Result<()> {
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

            let ongoing_trade_with_taker_exists = database
                .borrow()
                .contains_active_peer(&taker)
                .context(format!(
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
            let index = database
                .borrow()
                .fetch_inc_bitcoin_transient_key_index()
                .await
                .map_err(|err| {
                    anyhow!(
                        "Could not fetch the index for the Bitcoin transient key: {:#}",
                        err
                    )
                })?;

            let token_contract = ethereum_wallet.borrow().dai_contract_address();
            let ethereum_identity = ethereum_wallet.borrow().account();
            let bitcoin_transient_sk = bitcoin_wallet
                .borrow()
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

            let ethereum_chain_id = ethereum_wallet.borrow().chain_id();
            let bitcoin_network = bitcoin_wallet.borrow().network.into();

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

            let result = maker.process_taken_order(form);

            match result {
                Ok(TakeRequestDecision::GoForSwap) => {
                    if let Err(e) = swarm.setup_swap.send(
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

                    let _ = database
                        .borrow()
                        .insert_active_peer(ActivePeer { peer_id: peer })
                        .await
                        .map_err(|e| tracing::error!("Failed to confirm order: {}", e));

                    // todo: publish new order here?
                    // What if i publish a new order here and the does go
                    // through?
                }
                Ok(TakeRequestDecision::InsufficientFunds) => tracing::info!("Insufficient funds"),
                Ok(TakeRequestDecision::RateNotProfitable) => tracing::info!("Rate not profitable"),
                Err(e) => tracing::error!("Processing taken order yielded error: {}", e),
            };
        }
    }

    Ok(())
}

#[cfg(all(test, feature = "test-docker"))]
mod tests {
    use super::*;
    use crate::{
        config::{settings, Data, Logging, MaxSell, Network},
        swap::herc20::asset::ethereum::FromWei,
        test_harness, Seed,
    };
    use comit::{asset, asset::Erc20Quantity, ethereum::ChainId};
    use ethereum::ether;
    use log::LevelFilter;

    // Run cargo test with `--ignored --nocapture` to see the `println output`
    #[ignore]
    #[tokio::test]
    async fn trade_command() {
        let client = testcontainers::clients::Cli::default();
        let seed = Seed::random().unwrap();

        let bitcoin_blockchain = test_harness::bitcoin::Blockchain::new(&client).unwrap();
        bitcoin_blockchain.init().await.unwrap();

        let mut ethereum_blockchain = test_harness::ethereum::Blockchain::new(&client).unwrap();
        ethereum_blockchain.init().await.unwrap();

        let settings = Settings {
            maker: settings::Maker {
                max_sell: MaxSell {
                    bitcoin: None,
                    dai: None,
                },
                spread: Default::default(),
                maximum_possible_fee: Default::default(),
            },
            network: Network {
                listen: vec!["/ip4/98.97.96.95/tcp/20500"
                    .parse()
                    .expect("invalid multiaddr")],
            },
            data: Data {
                dir: Default::default(),
            },
            logging: Logging {
                level: LevelFilter::Trace,
            },
            bitcoin: Default::default(),
            ethereum: settings::Ethereum {
                node_url: ethereum_blockchain.node_url.clone(),
                chain: ethereum::Chain::new(
                    ChainId::GETH_DEV,
                    ethereum_blockchain.token_contract(),
                ),
            },
        };

        let bitcoin_wallet = bitcoin::Wallet::new(
            seed,
            bitcoin_blockchain.node_url.clone(),
            ::bitcoin::Network::Regtest,
        )
        .await
        .unwrap();

        let ethereum_wallet = crate::ethereum::Wallet::new(
            seed,
            ethereum_blockchain.node_url.clone(),
            settings.ethereum.chain,
        )
        .await
        .unwrap();

        bitcoin_blockchain
            .mint(
                bitcoin_wallet.new_address().await.unwrap(),
                asset::Bitcoin::from_sat(1_000_000_000).into(),
            )
            .await
            .unwrap();

        ethereum_blockchain
            .mint_ether(
                ethereum_wallet.account(),
                ether::Amount::from(1_000_000_000_000_000_000u64),
                settings.ethereum.chain.chain_id(),
            )
            .await
            .unwrap();
        ethereum_blockchain
            .mint_erc20_token(
                ethereum_wallet.account(),
                asset::Erc20::new(
                    settings.ethereum.chain.dai_contract_address(),
                    Erc20Quantity::from_wei(1_000_000_000_000_000_000u64),
                ),
                settings.ethereum.chain.chain_id(),
            )
            .await
            .unwrap();

        let _ = trade(&seed, settings, bitcoin_wallet, ethereum_wallet)
            .await
            .unwrap();
    }
}
