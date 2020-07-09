#![allow(unreachable_code, unused_variables, clippy::unit_arg)]

use anyhow::Context;
use nectar::config::settings;
use nectar::maker::PublishOrders;
use nectar::{
    bitcoin_wallet, config,
    config::Settings,
    ethereum_wallet,
    maker::TakeRequestDecision,
    mid_market_rate::get_btc_dai_mid_market_rate,
    network::{self, Nectar, Orderbook},
    options::{self, Options},
    Maker, Spread,
};
use std::time::Duration;
use structopt::StructOpt;

async fn init_maker(
    bitcoin_wallet: bitcoin_wallet::Wallet,
    ethereum_wallet: ethereum_wallet::Wallet,
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

#[tokio::main]
async fn main() {
    let options = options::Options::from_args();

    let settings = read_config(&options)
        .and_then(Settings::from_config_file_and_defaults)
        .expect("Could not initialize configuration");

    let dai_contract_addr: comit::ethereum::Address = settings.ethereum.dai_contract_address;

    // TODO: Proper wallet initialisation from config
    let bitcoin_wallet = bitcoin_wallet::Wallet::new(
        unimplemented!(),
        settings.bitcoin.bitcoind.node_url,
        settings.bitcoin.network,
    )
    .unwrap();
    let ethereum_wallet =
        ethereum_wallet::Wallet::new(unimplemented!(), settings.ethereum.node_url).unwrap();

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

    let network_event_timeout_secs: u64 = todo!("from config");
    loop {
        let rate = get_btc_dai_mid_market_rate().await;
        match rate {
            Ok(new_rate) => {
                let reaction = maker.update_rate(new_rate); // maker should record timestamp of this
                match reaction {
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

        let bitcoin_balance = bitcoin_wallet.balance().await;
        match bitcoin_balance {
            Ok(new_balance) => maker.update_bitcoin_balance(new_balance),
            Err(e) => unimplemented!(),
        }
        let dai_balance = ethereum_wallet.dai_balance().await;
        match dai_balance {
            Ok(new_balance) => maker.update_dai_balance(new_balance.into()),
            Err(e) => unimplemented!(),
        }

        // if nothing happens on the network for 15 seconds, loop
        // again

        // ASSUMPTION: a BtcDaiOrder Sell order and a BtcDaiOrder Buy
        // order were published before we enter the loop (during
        // initialization)
        #[allow(clippy::single_match)]
        match tokio::time::timeout(
            Duration::from_secs(network_event_timeout_secs),
            swarm.next(),
        )
        .await
        {
            Ok(event) => {
                match event {
                    network::Event::TakeRequest(order) => {
                        // decide & take & reserve
                        let result = maker.process_taken_order(order.clone());

                        match result {
                            Ok(TakeRequestDecision::GoForSwap { next_order }) => {
                                swarm.orderbook.take(order);
                                orderbook.publish(next_order.into());
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
                        // TODO: Add remote_data learned from the other party to the swap and persist the swap
                        // TODO: Spawn swap execution
                    }
                }
            }
            _ => (),
        }
    }
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
