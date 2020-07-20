#![allow(unreachable_code, unused_variables)]

use crate::{
    bitcoin,
    config::Settings,
    ethereum::{self, dai},
    history::{self, History},
    maker::{PublishOrders, TakeRequestDecision},
    mid_market_rate::get_btc_dai_mid_market_rate,
    network::{self, Swarm, Taker},
    swap::{self, Database, SwapKind, SwapParams},
    Maker, MidMarketRate, Seed, Spread,
};
use anyhow::Context;
use chrono::{DateTime, Local};
use comit::btsieve::bitcoin::BitcoindConnector;
use comit::btsieve::ethereum::Web3Connector;
use futures::{
    channel::mpsc::{Receiver, Sender},
    Future, FutureExt, SinkExt, StreamExt,
};
use futures_timer::Delay;
use libp2p::PeerId;
use num::BigUint;
use std::{
    str::FromStr,
    sync::{Arc, Mutex},
    time::Duration,
};

const ENSURED_CONSUME_ZERO_BUFFER: usize = 0;

pub async fn trade(
    runtime_handle: tokio::runtime::Handle,
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

    let mut swarm = Swarm::new(&seed, &settings, runtime_handle.clone())?;

    swarm.announce_btc_dai_trading_pair()?;

    let initial_sell_order = maker
        .new_sell_order()
        .context("Could not generate sell order")?;
    let initial_buy_order = maker
        .new_buy_order()
        .context("Could not generate buy order")?;

    swarm
        .publish(initial_sell_order.into())
        .context("Could not publish initial sell order")?;
    swarm
        .publish(initial_buy_order.into())
        .context("Could not publish initial buy order")?;

    let update_interval = Duration::from_secs(15u64);

    let (rate_future, mut rate_update_receiver) = init_rate_updates(update_interval);
    let (btc_balance_future, mut btc_balance_update_receiver) =
        init_bitcoin_balance_updates(update_interval, Arc::clone(&bitcoin_wallet));
    let (dai_balance_future, mut dai_balance_update_receiver) =
        init_dai_balance_updates(update_interval, Arc::clone(&ethereum_wallet));

    runtime_handle.clone().spawn(rate_future);
    runtime_handle.clone().spawn(btc_balance_future);
    runtime_handle.spawn(dai_balance_future);

    let (swap_execution_finished_sender, mut swap_execution_finished_receiver) =
        futures::channel::mpsc::channel::<FinishedSwap>(ENSURED_CONSUME_ZERO_BUFFER);

    #[cfg(not(test))]
    let db = Arc::new(Database::new(&settings.data.dir.join("database"))?);
    #[cfg(test)]
    let db = Arc::new(Database::new_test()?);

    let history = Arc::new(Mutex::new(History::new(
        settings.data.dir.join("history.csv").as_path(),
    )?));

    let bitcoin_connector = Arc::new(BitcoindConnector::new(
        settings.bitcoin.bitcoind.node_url,
        settings.bitcoin.network,
    )?);
    let ethereum_connector = Arc::new(Web3Connector::new(settings.ethereum.node_url));

    respawn_swaps(
        Arc::clone(&db),
        &mut maker,
        Arc::clone(&bitcoin_wallet),
        Arc::clone(&ethereum_wallet),
        Arc::clone(&bitcoin_connector),
        Arc::clone(&ethereum_connector),
        swap_execution_finished_sender.clone(),
        runtime_handle.clone(),
    )
    .context("Could not respawn swaps")?;

    loop {
        futures::select! {
            finished_swap = swap_execution_finished_receiver.next().fuse() => {
                if let Some(finished_swap) = finished_swap {
                    handle_finished_swap(finished_swap, &mut maker, &db, Arc::clone(&history), &mut swarm);
                }
            },
            network_event = swarm.as_inner().next().fuse() => {
                handle_network_event(
                    network_event,
                    &mut maker,
                    &mut swarm,
                    Arc::clone(&db),
                    Arc::clone(&bitcoin_wallet),
                    Arc::clone(&ethereum_wallet),
                    Arc::clone(&bitcoin_connector),
        Arc::clone(&ethereum_connector),
                    swap_execution_finished_sender.clone(),
                    runtime_handle.clone(),
                );
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

    Ok(())
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
    match swap {
        SwapKind::Herc20Hbit(params) => {
            unimplemented!("comit::orderbook does not yet handle sell orders");

            let _ = finished_swap_sender
                .send(FinishedSwap::new(
                    swap.clone(),
                    params.taker.clone(),
                    Local::now(),
                ))
                .await
                .map_err(|e| {
                    tracing::trace!(
                        "Error when sending execution finished from sender to receiver: {}",
                        e
                    )
                });
        }
        SwapKind::HbitHerc20(ref params) => {
            db.insert(swap.clone())?;

            swap::nectar_hbit_herc20(
                Arc::clone(&db),
                Arc::clone(&bitcoin_wallet),
                Arc::clone(&ethereum_wallet),
                Arc::clone(&bitcoin_connector),
                Arc::clone(&ethereum_connector),
                params.clone(),
            )
            .await?;

            let _ = finished_swap_sender
                .send(FinishedSwap::new(
                    swap.clone(),
                    params.taker.clone(),
                    Local::now(),
                ))
                .await
                .map_err(|e| {
                    tracing::trace!(
                        "Error when sending execution finished from sender to receiver."
                    )
                });
        }
    }

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
    runtime_handle: tokio::runtime::Handle,
) -> anyhow::Result<()> {
    for swap in db.load_all()?.into_iter() {
        // Reserve funds
        match swap {
            SwapKind::HbitHerc20(SwapParams {
                swap_id,
                ref herc20_params,
                ..
            }) => {
                let fund_amount = herc20_params.asset.clone().into();
                maker.dai_reserved_funds = maker.dai_reserved_funds.clone() + fund_amount;
            }
            SwapKind::Herc20Hbit(SwapParams {
                swap_id,
                hbit_params,
                ..
            }) => {
                let fund_amount = hbit_params.shared.asset.into();
                maker.btc_reserved_funds = maker.btc_reserved_funds + fund_amount + maker.btc_fee;
            }
        };

        runtime_handle.spawn(execute_swap(
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

#[allow(clippy::too_many_arguments)]
fn handle_network_event(
    network_event: network::Event,
    maker: &mut Maker,
    swarm: &mut Swarm,
    db: Arc<Database>,
    bitcoin_wallet: Arc<bitcoin::Wallet>,
    ethereum_wallet: Arc<ethereum::Wallet>,
    bitcoin_connector: Arc<comit::btsieve::bitcoin::BitcoindConnector>,
    ethereum_connector: Arc<comit::btsieve::ethereum::Web3Connector>,
    finished_swap_sender: Sender<FinishedSwap>,
    runtime_handle: tokio::runtime::Handle
) {
    match network_event {
        network::Event::TakeOrderRequest(order) => {
            let order_ref = &order;
            let result = maker.process_taken_order(order_ref.into());

            match result {
                Ok(TakeRequestDecision::GoForSwap) => {
                    let position = order_ref.inner.position;
                    let _ = swarm
                        .confirm(order)
                        .map_err(|e| tracing::error!("Failed to confirm order: {}", e));

                    match maker.new_order(position) {
                        Ok(new_order) => {
                            let _ = swarm
                                .publish(new_order.into())
                                .map_err(|e| tracing::error!("Failed to publish order: {}", e));
                        }
                        Err(e) => tracing::error!("Error when trying to create order: {}", e),
                    }
                }
                Ok(TakeRequestDecision::RateNotProfitable)
                | Ok(TakeRequestDecision::InsufficientFunds) => swarm.deny(order),
                Err(e) => {
                    tracing::error!("Processing taken order yielded error: {}", e);
                    swarm.deny(order)
                }
            }
        }
        network::Event::SetSwapIdentities(swap_metadata) => {
            let bitcoin_identity = match bitcoin_wallet.random_transient_sk() {
                Ok(bitcoin_identity) => bitcoin_identity,
                Err(e) => {
                    tracing::error!("Generating transient sk yielded error: {}", e);
                    return;
                }
            };
            let ethereum_identity = ethereum_wallet.account();

            swarm.set_swap_identities(swap_metadata, bitcoin_identity, ethereum_identity)
        }
        network::Event::SpawnSwap(swap) => {
            runtime_handle.spawn(execute_swap(
                Arc::clone(&db),
                Arc::clone(&bitcoin_wallet),
                Arc::clone(&ethereum_wallet),
                todo!("bitcoin_connector"),
                todo!("ethereum_connector"),
                finished_swap_sender,
                swap,
            ));
        }
    }
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
                    let _ = swarm
                        .publish(new_sell_order.into())
                        .map_err(|e| tracing::error!("Failed to publish new sell order: {}", e));
                    let _ = swarm
                        .publish(new_buy_order.into())
                        .map_err(|e| tracing::error!("Failed to publish new buy order: {}", e));
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
                let _ = swarm
                    .publish(new_sell_order.into())
                    .map_err(|e| tracing::error!("Failed to publish new order: {}", e));
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
                let _ = swarm
                    .publish(new_buy_order.into())
                    .map_err(|e| tracing::error!("Failed to publish order: {}", e));
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

fn handle_finished_swap(
    finished_swap: FinishedSwap,
    maker: &mut Maker,
    db: &Database,
    history: Arc<Mutex<History>>,
    swarm: &mut Swarm,
) {
    {
        let trade = into_trade(
            finished_swap.taker.peer_id(),
            finished_swap.swap.clone(),
            finished_swap.final_timestamp,
        );

        let mut history = history
            .lock()
            .expect("No thread panicked while holding the lock");
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

    let _ = swarm
        .remove_from_active_takers(&finished_swap.taker)
        .map_err(|error| tracing::error!("Unable to remove from active takers: {}", error));

    let _ = db
        .remove(&swap_id)
        .map_err(|error| tracing::error!("Unable to delete swap from db: {}", error));
}

fn into_trade(peer_id: PeerId, swap: SwapKind, final_timestamp: DateTime<Local>) -> history::Trade {
    use history::*;

    let (swap, position) = match swap {
        SwapKind::HbitHerc20(swap) => (swap, history::Position::Sell),
        SwapKind::Herc20Hbit(swap) => (swap, history::Position::Buy),
    };

    #[cfg(not(test))]
    let final_timestamp = final_timestamp.into();

    #[cfg(test)]
    let final_timestamp = DateTime::from_str("2020-07-10T17:48:26.123+10:00")
        .unwrap()
        .into();

    Trade {
        start_timestamp: history::LocalDateTime::from_utc_naive(&swap.start_of_swap),
        final_timestamp,
        base_symbol: Symbol::Btc,
        quote_symbol: Symbol::Dai,
        position,
        base_precise_amount: swap.hbit_params.shared.asset.as_sat().into(),
        quote_precise_amount: BigUint::from_str(&swap.herc20_params.asset.quantity.to_wei_dec())
            .expect("number to number conversion")
            .into(),
        peer: peer_id.into(),
    }
}

#[derive(Debug, Clone)]
struct FinishedSwap {
    pub swap: SwapKind,
    pub taker: Taker,
    pub final_timestamp: DateTime<Local>,
}

impl FinishedSwap {
    pub fn new(swap: SwapKind, taker: Taker, final_timestamp: DateTime<Local>) -> Self {
        Self {
            swap,
            taker,
            final_timestamp,
        }
    }
}

#[cfg(all(test, feature = "test-docker"))]
mod tests {
    use super::*;
    use crate::{test_harness, Seed};
    use crate::config::{Logging, Data, Network, settings, MaxSell};
    use log::LevelFilter;

    // Run cargo test with `--ignored --nocapture` to see the `println output`
    #[ignore]
    #[tokio::test]
    async fn trade_command() {
        let runtime = tokio::runtime::Runtime::new().unwrap();

        let client = testcontainers::clients::Cli::default();
        let seed = Seed::random().unwrap();

        let bitcoin_blockchain = test_harness::bitcoin::Blockchain::new(&client).unwrap();
        bitcoin_blockchain.init(runtime.handle().clone()).await.unwrap();

        let bitcoin_wallet = bitcoin::Wallet::new(
            seed,
            bitcoin_blockchain.node_url,
            ::bitcoin::Network::Regtest,
        )
            .await
            .unwrap();

        let mut ethereum_blockchain = test_harness::ethereum::Blockchain::new(&client).unwrap();
        ethereum_blockchain.init().await.unwrap();

        let ethereum_wallet = crate::ethereum::Wallet::new(
            seed,
            ethereum_blockchain.node_url.clone(),
            ethereum_blockchain.token_contract().unwrap(),
        )
            .unwrap();

        let settings = Settings {
            maker:  settings::Maker {
                max_sell: MaxSell { bitcoin: None, dai: None },
                spread: Default::default(),
                maximum_possible_fee: Default::default()
            },
            network: Network { listen: vec!["/ip4/98.97.96.95/tcp/20500".parse().expect("invalid multiaddr")] },
            data: Data { dir: Default::default() },
            logging: Logging { level: LevelFilter::Trace },
            bitcoin: Default::default(),
            ethereum: Default::default()
        };

        let _ = trade(runtime.handle().clone(), &seed, settings, bitcoin_wallet, ethereum_wallet);
    }
}
