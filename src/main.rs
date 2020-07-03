#![allow(unreachable_code, unused_variables, clippy::unit_arg)]

use nectar::{
    bitcoin, bitcoin_wallet, dai,
    maker::Reaction,
    mid_market_rate::get_btc_dai_mid_market_rate,
    network::{self, Nectar, Orderbook},
    Maker, Spread,
};
use std::time::Duration;

async fn init_maker(bitcoin_wallet: bitcoin_wallet::Wallet) -> Maker {
    let initial_btc_balance = bitcoin_wallet.balance().await;

    // TODO ethereum wallet (passed in)
    let initial_dai_balance: anyhow::Result<dai::Amount> = unimplemented!();

    // TODO: from config
    let btc_max_sell: anyhow::Result<bitcoin::Amount> = unimplemented!();
    let dai_max_sell: anyhow::Result<dai::Amount> = unimplemented!();
    let btc_fee_reserve: anyhow::Result<bitcoin::Amount> = unimplemented!();
    let dai_fee_reserve: anyhow::Result<dai::Amount> = unimplemented!();

    let initial_rate = get_btc_dai_mid_market_rate().await;

    // TODO from config
    let spread = Spread::default();

    match (
        initial_btc_balance,
        initial_dai_balance,
        btc_fee_reserve,
        dai_fee_reserve,
        btc_max_sell,
        dai_max_sell,
        initial_rate,
    ) {
        // TODO better error handling
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
                initial_dai_balance,
                btc_fee_reserve,
                dai_fee_reserve,
                btc_max_sell,
                dai_max_sell,
                initial_rate,
                spread,
            ),
        _ => panic!("Maker initialisation failed!"),
    }
}

#[tokio::main]
async fn main() {
    let bitcoin_wallet =
        bitcoin_wallet::Wallet::new(unimplemented!(), unimplemented!(), unimplemented!()).unwrap();

    let maker = init_maker(bitcoin_wallet).await;

    let orderbook = Orderbook;
    let nectar = Nectar::new(orderbook);

    let mut swarm: libp2p::Swarm<Nectar> = unimplemented!();

    let initial_sell_order = maker.next_sell_order();
    let initial_buy_order = maker.next_buy_order();

    match (initial_sell_order, initial_buy_order) {
        (Ok(sell_order), Ok(buy_order)) => {
            swarm.orderbook.publish(sell_order.into());
            swarm.orderbook.publish(buy_order.into());
        }
        _ => panic!("Unable to publish initial orders!"),
    }

    loop {
        let rate = get_btc_dai_mid_market_rate().await;
        match rate {
            Ok(new_rate) => maker.update_rate(unimplemented!()), // maker should record timestamp of this
            Err(e) => maker.track_failed_rate(e),
        }

        let bitcoin_balance = bitcoin_wallet.balance().await;
        match bitcoin_balance {
            Ok(new_balance) => maker.update_bitcoin_balance(new_balance), // maker should record timestamp of this
            Err(e) => maker.track_failed_balance_update(e),
        }

        // if nothing happens on the network for 15 seconds, loop
        // again

        // ASSUMPTION: a BtcDaiOrder Sell order and a BtcDaiOrder Buy
        // order were published before we enter the loop (during
        // initialization)
        #[allow(clippy::single_match)]
        match tokio::time::timeout(Duration::from_secs(15), swarm.next()).await {
            Ok(event) => {
                match event {
                    network::Event::OrderTakeRequest(order) => {
                        // decide & take & reserve
                        let reaction = maker.react_to_taken_order(order.clone());

                        match reaction {
                            Ok(Reaction::Confirmed { next_order }) => {
                                swarm.orderbook.take(order);
                                orderbook.publish(next_order.into());
                            }
                            Ok(Reaction::RateSucks)
                            | Ok(Reaction::InsufficientFunds)
                            | Ok(Reaction::CannotTradeWithTaker)
                            | Err(_) => {
                                swarm.orderbook.ignore(order);
                            }
                        }
                    }
                    network::Event::SwapFinalized(local_swap_id, remote_data) => {
                        let order = maker.get_order_for_local_swap_id(local_swap_id);

                        // TODO: Add remote_data learned from the other party to the swap and persist the swap
                        // TODO: Spawn swap execution
                    }
                }
            }
            _ => (),
        }
    }
}
