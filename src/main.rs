#![allow(unreachable_code, unused_variables, clippy::unit_arg)]

use nectar::maker::PublishOrders;
use nectar::{
    bitcoin, bitcoin_wallet, dai, ethereum_wallet,
    maker::TakeRequestDecision,
    mid_market_rate::get_btc_dai_mid_market_rate,
    network::{self, Nectar, Orderbook},
    Maker, Spread,
};
use std::time::Duration;

async fn init_maker(
    bitcoin_wallet: bitcoin_wallet::Wallet,
    ethereum_wallet: ethereum_wallet::Wallet,
) -> Maker {
    let initial_btc_balance = bitcoin_wallet.balance().await;

    let initial_dai_balance = ethereum_wallet.dai_balance().await;

    let btc_max_sell: anyhow::Result<bitcoin::Amount> = todo!("from config");
    let dai_max_sell: anyhow::Result<dai::Amount> = todo!("from config");
    let btc_fee_reserve: anyhow::Result<bitcoin::Amount> = todo!("from config");
    let dai_fee_reserve: anyhow::Result<dai::Amount> = todo!("from config");

    let initial_rate = get_btc_dai_mid_market_rate().await;

    let spread: Spread = todo!("from config");

    match (
        initial_btc_balance,
        initial_dai_balance,
        btc_fee_reserve,
        dai_fee_reserve,
        btc_max_sell,
        dai_max_sell,
        initial_rate,
    ) {
        (
            Ok(initial_btc_balance),
            Ok(initial_dai_balance),
            Ok(btc_fee_reserve),
            Ok(dai_fee_reserve),
            Ok(btc_max_sell),
            Ok(dai_max_sell),
            Ok(initial_rate),
        ) => Maker::new(
            initial_btc_balance,
            initial_dai_balance.into(),
            btc_fee_reserve,
            dai_fee_reserve,
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
    let bitcoin_wallet = bitcoin_wallet::Wallet::new(
        todo!("from config"),
        todo!("from config"),
        todo!("from config"),
    )
    .unwrap();
    let ethereum_wallet =
        ethereum_wallet::Wallet::new(todo!("from config"), todo!("from config")).unwrap();

    let maker = init_maker(bitcoin_wallet, ethereum_wallet).await;

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
