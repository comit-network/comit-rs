#![allow(unreachable_code, unused_variables, clippy::unit_arg)]
#![recursion_limit = "256"]

use anyhow::Context;
use chrono::{DateTime, Local, Utc};
use futures::{
    channel::mpsc::{Receiver, Sender},
    Future, FutureExt, SinkExt, StreamExt,
};
use futures_timer::Delay;
use libp2p::PeerId;
use nectar::options::Command;
use nectar::{
    bitcoin, config,
    config::{settings, Settings},
    ethereum,
    ethereum::dai,
    history,
    history::History,
    maker::{PublishOrders, TakeRequestDecision},
    mid_market_rate::get_btc_dai_mid_market_rate,
    network::{self, Nectar, Orderbook, Taker},
    options::{self, Options},
    order::Position,
    swap::{self, herc20, Database, SwapKind, SwapParams},
    Maker, MidMarketRate, Spread, SwapId,
};
use num::BigUint;
use std::str::FromStr;
use std::sync::Mutex;
use std::{sync::Arc, time::Duration};
use structopt::StructOpt;

const ENSURED_CONSUME_ZERO_BUFFER: usize = 0;

async fn init_maker(
    bitcoin_wallet: Arc<bitcoin::Wallet>,
    ethereum_wallet: Arc<ethereum::Wallet>,
    maker_settings: settings::Maker,
) -> Maker {
    let initial_btc_balance = bitcoin_wallet.balance().await;

    let initial_dai_balance = ethereum_wallet.dai_balance().await;

    let btc_max_sell = maker_settings.max_sell.bitcoin;
    let dai_max_sell = maker_settings.max_sell.dai;
    let btc_fee_reserve = maker_settings.maximum_possible_fee.bitcoin;

    let initial_rate = get_btc_dai_mid_market_rate().await;

    let spread: Spread = maker_settings.spread;

    // TODO: This match is weird. If the settings does not give you want you want then it should fail earlier.
    match (initial_btc_balance, initial_dai_balance, initial_rate) {
        (Ok(initial_btc_balance), Ok(initial_dai_balance), Ok(initial_rate)) => Maker::new(
            initial_btc_balance,
            initial_dai_balance.into(),
            btc_fee_reserve,
            btc_max_sell,
            dai_max_sell,
            initial_rate,
            spread,
        ),
        // TODO better error handling
        _ => panic!("Maker initialisation failed!"),
    }
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

            if let Err(e) = sender.send(rate).await {
                tracing::trace!("Error when sending rate update from sender to receiver.")
            }

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

            if let Err(e) = sender.send(balance).await {
                tracing::trace!("Error when sending balance update from sender to receiver.")
            }

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

            if let Err(e) = sender.send(balance.map(|balance| balance.into())).await {
                tracing::trace!("Error when sending rate balance from sender to receiver.")
            }

            Delay::new(update_interval).await;
        }
    };

    (future, receiver)
}

async fn execute_swap(
    db: Arc<Database>,
    bitcoin_wallet: Arc<bitcoin::Wallet>,
    ethereum_wallet: Arc<ethereum::Wallet>,
    bitcoin_connector: Arc<comit::btsieve::bitcoin::BitcoindConnector>,
    ethereum_connector: Arc<comit::btsieve::ethereum::Web3Connector>,
    swap_execution_finished_sender: Sender<FinishedSwap>,
) -> anyhow::Result<()> {
    let swap_id = SwapId::default();
    let position: Position =
        todo!("decision what kind of what swap it is hbit->herc20 or herc20->hbit");

    let taker: Taker = todo!("Taker has to be available after execution, e.g. load from db");

    match position {
        Position::Sell => {
            todo!("handle this match arm like below");
        }
        Position::Buy => {
            let herc20_params: herc20::Params = unimplemented!();

            let swap = SwapParams {
                hbit_params: todo!("from arguments"),
                herc20_params,
                secret_hash: todo!("from arguments"),
                start_of_swap: Utc::now().naive_local(), // Is this the correct start time?
                swap_id,
            };

            let swap_kind = SwapKind::HbitHerc20(swap);
            db.insert(swap_kind)?;

            swap::nectar_hbit_herc20(
                Arc::clone(&db),
                Arc::clone(&bitcoin_wallet),
                Arc::clone(&ethereum_wallet),
                Arc::clone(&bitcoin_connector),
                Arc::clone(&ethereum_connector),
                swap,
            )
            .await?;

            // TODO: use map_err
            if let Err(e) = swap_execution_finished_sender
                .send(FinishedSwap::new(swap_kind, taker, Local::now()))
                .await
            {
                tracing::trace!("Error when sending execution finished from sender to receiver.")
            }
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn handle_network_event(
    network_event: network::Event,
    maker: &mut Maker,
    swarm: &mut libp2p::Swarm<Nectar>,
    db: Arc<Database>,
    bitcoin_wallet: Arc<bitcoin::Wallet>,
    ethereum_wallet: Arc<ethereum::Wallet>,
    bitcoin_connector: Arc<comit::btsieve::bitcoin::BitcoindConnector>,
    ethereum_connector: Arc<comit::btsieve::ethereum::Web3Connector>,
    sender: Sender<FinishedSwap>,
) {
    match network_event {
        network::Event::TakeRequest(order) => {
            // decide & take & reserve
            let result = maker.process_taken_order(order.clone());

            match result {
                Ok(TakeRequestDecision::GoForSwap) => {
                    swarm.orderbook.take(order.clone());

                    match maker.new_order(order.inner.position) {
                        Ok(new_order) => {
                            swarm.orderbook.publish(new_order.into());
                        }
                        Err(e) => tracing::error!("Error when trying to create new order: {}", e),
                    }
                }
                Ok(TakeRequestDecision::RateNotProfitable)
                | Ok(TakeRequestDecision::InsufficientFunds)
                | Ok(TakeRequestDecision::CannotTradeWithTaker) => {
                    swarm.orderbook.ignore(order);
                }
                Err(e) => {
                    swarm.orderbook.ignore(order);
                    tracing::error!("Processing taken order yielded error: {}", e)
                }
            }
        }
        network::Event::SwapFinalized(local_swap_id, remote_data) => {
            tokio::spawn(execute_swap(
                Arc::clone(&db),
                Arc::clone(&bitcoin_wallet),
                Arc::clone(&ethereum_wallet),
                todo!("bitcoin_connector"),
                todo!("ethereum_connector"),
                sender,
            ));
        }
    }
}

fn handle_rate_update(
    rate_update: anyhow::Result<MidMarketRate>,
    maker: &mut Maker,
    swarm: &mut libp2p::Swarm<Nectar>,
) {
    match rate_update {
        Ok(new_rate) => {
            let result = maker.update_rate(new_rate);
            match result {
                Ok(Some(PublishOrders {
                    new_sell_order,
                    new_buy_order,
                })) => {
                    swarm.orderbook.publish(new_sell_order.into());
                    swarm.orderbook.publish(new_buy_order.into());
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
    swarm: &mut libp2p::Swarm<Nectar>,
) {
    match btc_balance_update {
        Ok(btc_balance) => match maker.update_bitcoin_balance(btc_balance) {
            Ok(Some(new_sell_order)) => {
                swarm.orderbook.publish(new_sell_order.into());
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
    swarm: &mut libp2p::Swarm<Nectar>,
) {
    match dai_balance_update {
        Ok(dai_balance) => match maker.update_dai_balance(dai_balance) {
            Ok(Some(new_buy_order)) => {
                swarm.orderbook.publish(new_buy_order.into());
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

// TODO: I don't think `finished_swap` should be an Option
fn handle_finished_swap(
    finished_swap: Option<FinishedSwap>,
    maker: &mut Maker,
    db: &Database,
    history: Arc<Mutex<History>>,
) {
    if let Some(finished_swap) = finished_swap {
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
            SwapKind::HbitHerc20(swap) => {
                (Some(swap.herc20_params.asset.into()), None, swap.swap_id)
            }
            SwapKind::Herc20Hbit(swap) => (
                None,
                Some(swap.hbit_params.shared.asset.into()),
                swap.swap_id,
            ),
        };

        maker.process_finished_swap(dai, btc, finished_swap.taker);

        let _ = db
            .remove(&swap_id)
            .map_err(|error| tracing::error!("Unable to delete swap from db: {}", error));
    }
}

// TODO: Move all this stuff in trade.rs

fn into_trade(peer_id: PeerId, swap: SwapKind, final_timestamp: DateTime<Local>) -> history::Trade {
    use history::*;

    let (swap, position) = match swap {
        SwapKind::HbitHerc20(swap) => (swap, Position::Sell),
        SwapKind::Herc20Hbit(swap) => (swap, Position::Buy),
    };

    Trade {
        start_timestamp: history::LocalDateTime::from_utc_naive(&swap.start_of_swap),
        final_timestamp: final_timestamp.into(),
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

#[tokio::main]
async fn main() {
    let options = options::Options::from_args();

    let settings = read_config(&options)
        .and_then(Settings::from_config_file_and_defaults)
        .expect("Could not initialize configuration");

    let seed = config::Seed::from_file_or_generate(&settings.data.dir)
        .expect("Could not retrieve/initialize seed")
        .into();

    let dai_contract_addr: comit::ethereum::Address = settings.ethereum.dai_contract_address;

    let bitcoin_wallet = bitcoin::Wallet::new(
        seed,
        settings.bitcoin.bitcoind.node_url.clone(),
        settings.bitcoin.network,
    )
    .await
    .expect("can initialise bitcoin wallet");
    let bitcoin_wallet = Arc::new(bitcoin_wallet);
    let ethereum_wallet =
        ethereum::Wallet::new(seed, settings.ethereum.node_url.clone(), dai_contract_addr)
            .expect("can initialise ethereum wallet");
    let ethereum_wallet = Arc::new(ethereum_wallet);

    match options.cmd {
        Command::Trade => trade(settings, bitcoin_wallet, ethereum_wallet).await,
    }
}

async fn trade(
    settings: Settings,
    bitcoin_wallet: Arc<bitcoin::Wallet>,
    ethereum_wallet: Arc<ethereum::Wallet>,
) {
    let maker = init_maker(bitcoin_wallet, ethereum_wallet, settings.maker).await;

    let orderbook = Orderbook;
    let nectar = Nectar::new(orderbook);

    let mut swarm: libp2p::Swarm<Nectar> = unimplemented!();

    let initial_sell_order = maker.new_sell_order();
    let initial_buy_order = maker.new_buy_order();

    match (initial_sell_order, initial_buy_order) {
        (Ok(sell_order), Ok(buy_order)) => {
            swarm.orderbook.publish(sell_order.into());
            swarm.orderbook.publish(buy_order.into());
        }
        _ => panic!("Unable to publish initial orders!"),
    }

    let update_interval = Duration::from_secs(15u64);

    let (rate_future, rate_update_receiver) = init_rate_updates(update_interval);
    let (btc_balance_future, btc_balance_update_receiver) =
        init_bitcoin_balance_updates(update_interval, bitcoin_wallet);
    let (dai_balance_future, dai_balance_update_receiver) =
        init_dai_balance_updates(update_interval, ethereum_wallet);

    tokio::spawn(rate_future);
    tokio::spawn(btc_balance_future);
    tokio::spawn(dai_balance_future);

    let (swap_execution_finished_sender, swap_execution_finished_receiver) =
        futures::channel::mpsc::channel::<FinishedSwap>(ENSURED_CONSUME_ZERO_BUFFER);

    let db = Arc::new(Database::new(&settings.data.dir.join("database")).unwrap());

    let history = Arc::new(Mutex::new(
        History::new(settings.data.dir.join("history.csv").as_path()).unwrap(),
    ));

    respawn_swaps(
        Arc::clone(&db),
        Arc::clone(&bitcoin_wallet),
        Arc::clone(&ethereum_wallet),
        todo!("bitcoin_connector"),
        todo!("ethereum_connector"),
        swap_execution_finished_sender.clone(),
    )
    .unwrap();

    loop {
        futures::select! {
            // TODO: I don't think we need to handle the Option
            finished_swap = swap_execution_finished_receiver.next().fuse() => {
                handle_finished_swap(finished_swap, &mut maker, &db, history);
            },
            network_event = swarm.next().fuse() => {
                handle_network_event(
                    network_event,
                    &mut maker,
                    &mut swarm,
                    Arc::clone(&db),
                    Arc::clone(&bitcoin_wallet),
                    Arc::clone(&ethereum_wallet),
                    todo!("bitcoin_connector"),
                    todo!("ethereum_connector"),
                    swap_execution_finished_sender.clone()
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
}

fn respawn_swaps(
    db: Arc<Database>,
    bitcoin_wallet: Arc<bitcoin::Wallet>,
    ethereum_wallet: Arc<ethereum::Wallet>,
    bitcoin_connector: Arc<comit::btsieve::bitcoin::BitcoindConnector>,
    ethereum_connector: Arc<comit::btsieve::ethereum::Web3Connector>,
    swap_execution_finished_sender: Sender<FinishedSwap>,
) -> anyhow::Result<()> {
    for swap in db.load_all()?.into_iter() {
        // TODO: Reserve funds

        match swap {
            SwapKind::HbitHerc20(swap) => {
                tokio::spawn(swap::nectar_hbit_herc20(
                    Arc::clone(&db),
                    Arc::clone(&bitcoin_wallet),
                    Arc::clone(&ethereum_wallet),
                    Arc::clone(&bitcoin_connector),
                    Arc::clone(&ethereum_connector),
                    swap,
                ));
            }
            SwapKind::Herc20Hbit(_) => todo!("implement swap::nectar_herc20_hbit"),
        }
    }

    Ok(())
}

fn read_config(options: &Options) -> anyhow::Result<config::File> {
    // if the user specifies a config path, use it
    if let Some(path) = &options.config_file {
        eprintln!("Using config file {}", path.display());

        return config::File::read(&path)
            .with_context(|| format!("failed to read config file {}", path.display()));
    }

    // try to load default config
    let default_path = nectar::default_config_path()?;

    if !default_path.exists() {
        return Ok(config::File::default());
    }

    eprintln!(
        "Using config file at default path: {}",
        default_path.display()
    );

    config::File::read(&default_path)
        .with_context(|| format!("failed to read config file {}", default_path.display()))
}
