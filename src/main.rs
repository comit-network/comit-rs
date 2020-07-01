#![allow(unreachable_code, unused_variables, clippy::unit_arg)]

use nectar::{
    bitcoin_wallet,
    maker::Reaction,
    mid_market_rate::get_btc_dai_mid_market_rate,
    network::{self, Nectar, Orderbook},
    Maker,
};
use std::time::Duration;

#[tokio::main]
async fn main() {
    let maker = Maker::new();
    let orderbook = Orderbook;

    let nectar = Nectar::new(orderbook);
    let wallet =
        bitcoin_wallet::Wallet::new(unimplemented!(), unimplemented!(), unimplemented!()).unwrap();

    let mut swarm: libp2p::Swarm<Nectar> = unimplemented!();

    loop {
        let rate = get_btc_dai_mid_market_rate().await;
        match rate {
            Ok(new_rate) => maker.update_rate(unimplemented!()), // maker should record timestamp of this
            Err(e) => maker.track_failed_rate(e),
        }

        let bitcoin_balance = wallet.balance().await;
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
