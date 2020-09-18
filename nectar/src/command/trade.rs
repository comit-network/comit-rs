mod event_loop;

use crate::{
    bitcoin,
    command::{trade::event_loop::EventLoop, FinishedSwap},
    config::Settings,
    ethereum::{self, dai},
    history::History,
    mid_market_rate::get_btc_dai_mid_market_rate,
    network::{self, new_swarm},
    swap::{Database, SwapKind, SwapParams},
    Maker, MidMarketRate, Seed, Spread,
};
use anyhow::Context;
use comit::{
    btsieve::{bitcoin::BitcoindConnector, ethereum::Web3Connector},
    Position, Role,
};
use futures::{
    channel::mpsc::{Receiver, Sender},
    Future, SinkExt,
};
use futures_timer::Delay;
use std::{sync::Arc, time::Duration};

const ENSURED_CONSUME_ZERO_BUFFER: usize = 0;

pub async fn trade(
    seed: &Seed,
    settings: Settings,
    bitcoin_wallet: bitcoin::Wallet,
    ethereum_wallet: ethereum::Wallet,
    network: comit::Network,
) -> anyhow::Result<()> {
    let bitcoin_wallet = Arc::new(bitcoin_wallet);
    let ethereum_wallet = Arc::new(ethereum_wallet);

    let mut maker = init_maker(
        Arc::clone(&bitcoin_wallet),
        Arc::clone(&ethereum_wallet),
        settings.clone(),
        network,
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

    let (rate_future, rate_update_receiver) = init_rate_updates(update_interval);
    let (btc_balance_future, btc_balance_update_receiver) =
        init_bitcoin_balance_updates(update_interval, Arc::clone(&bitcoin_wallet));
    let (dai_balance_future, dai_balance_update_receiver) =
        init_dai_balance_updates(update_interval, Arc::clone(&ethereum_wallet));

    tokio::spawn(rate_future);
    tokio::spawn(btc_balance_future);
    tokio::spawn(dai_balance_future);

    let bitcoin_connector = Arc::new(BitcoindConnector::new(settings.bitcoin.bitcoind.node_url)?);
    let ethereum_connector = Arc::new(Web3Connector::new(settings.ethereum.node_url));

    let (swap_executor, swap_execution_finished_receiver) = SwapExecutor::new(
        Arc::clone(&db),
        Arc::clone(&bitcoin_wallet),
        Arc::clone(&ethereum_wallet),
        bitcoin_connector,
        ethereum_connector,
    );

    respawn_swaps(Arc::clone(&db), &mut maker, swap_executor.clone())
        .context("Could not respawn swaps")?;

    let history = History::new(settings.data.dir.join("history.csv").as_path())?;

    let event_loop = EventLoop::new(
        maker,
        swarm,
        history,
        db,
        bitcoin_wallet,
        ethereum_wallet,
        swap_executor,
    );

    event_loop
        .run(
            swap_execution_finished_receiver,
            rate_update_receiver,
            btc_balance_update_receiver,
            dai_balance_update_receiver,
        )
        .await
}

async fn init_maker(
    bitcoin_wallet: Arc<bitcoin::Wallet>,
    ethereum_wallet: Arc<ethereum::Wallet>,
    settings: Settings,
    network: comit::Network,
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
        network,
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

#[derive(Debug, Clone)]
struct SwapExecutor {
    db: Arc<Database>,
    bitcoin_wallet: Arc<bitcoin::Wallet>,
    ethereum_wallet: Arc<ethereum::Wallet>,
    finished_swap_sender: Sender<FinishedSwap>,
    bitcoin_connector: Arc<comit::btsieve::bitcoin::BitcoindConnector>,
    ethereum_connector: Arc<comit::btsieve::ethereum::Web3Connector>,
}

impl SwapExecutor {
    pub fn new(
        db: Arc<Database>,
        bitcoin_wallet: Arc<bitcoin::Wallet>,
        ethereum_wallet: Arc<ethereum::Wallet>,
        bitcoin_connector: Arc<comit::btsieve::bitcoin::BitcoindConnector>,
        ethereum_connector: Arc<comit::btsieve::ethereum::Web3Connector>,
    ) -> (Self, Receiver<FinishedSwap>) {
        let (finished_swap_sender, finished_swap_receiver) =
            futures::channel::mpsc::channel::<FinishedSwap>(ENSURED_CONSUME_ZERO_BUFFER);

        (
            Self {
                db,
                bitcoin_wallet,
                ethereum_wallet,
                finished_swap_sender,
                bitcoin_connector,
                ethereum_connector,
            },
            finished_swap_receiver,
        )
    }
}

impl SwapExecutor {
    async fn run(mut self, swap: SwapKind) -> anyhow::Result<()> {
        self.db.insert_swap(swap.clone()).await?;

        swap.execute(
            self.db,
            self.bitcoin_wallet,
            self.ethereum_wallet,
            self.bitcoin_connector,
            self.ethereum_connector,
        )
        .await?;

        let _ = self
            .finished_swap_sender
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
}

fn respawn_swaps(
    db: Arc<Database>,
    maker: &mut Maker,
    swap_executor: SwapExecutor,
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

        tokio::spawn(swap_executor.clone().run(swap));
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
    use comit::{asset, asset::Erc20Quantity, ethereum::ChainId, ledger};
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
            bitcoin: settings::Bitcoin::new(ledger::Bitcoin::Regtest),
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

        let _ = trade(
            &seed,
            settings,
            bitcoin_wallet,
            ethereum_wallet,
            comit::Network::Dev,
        )
        .await
        .unwrap();
    }
}
