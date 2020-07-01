#![allow(unreachable_code, unused_variables, clippy::unit_arg)]

use nectar::{
    bitcoin_wallet,
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

        // if nothing happens on the network for 15 seconds, loop again
        #[allow(clippy::single_match)]
        match tokio::time::timeout(Duration::from_secs(15), swarm.next()).await {
            Ok(event) => {
                match event {
                    network::Event::OrderExpired(order) => {
                        let new_order = maker.expire_order(order.into());
                    }
                    network::Event::OrderTakeRequest(order) => {
                        // decide & take & reserve
                        let res = maker.confirm_order(order.clone());
                        if res.is_ok() {
                            swarm.orderbook.take(order);
                        } else {
                            swarm.orderbook.ignore(order);
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
