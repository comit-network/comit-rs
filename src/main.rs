use chrono::Utc;
use nectar::dai;
use nectar::mid_market_rate::get_btc_dai_mid_market_rate;
use nectar::network::{Nectar, Orderbook};
use nectar::rate::Spread;
use nectar::{bitcoin, bitcoin_wallet, network};
use reqwest::get;
use std::time::Duration;

mod maker {
    use super::*;
    use comit::LocalSwapId;
    use nectar::order::{BtcDaiOrder, Position};
    use nectar::MidMarketRate;

    #[derive(Debug, PartialEq)]
    pub enum NewOrder {
        Created(BtcDaiOrder),
    }

    // Bundles the state of the application
    pub struct Maker {
        btc_balance: bitcoin::Amount,
        dai_balance: dai::Amount,
        btc_fee: bitcoin::Amount,
        dai_fee: dai::Amount,
        btc_locked_funds: bitcoin::Amount,
        dai_locked_funds: dai::Amount,
        btc_max_sell_amount: bitcoin::Amount,
        dai_max_sell_amount: dai::Amount,
        rate: MidMarketRate,
        spread: Spread,
    }

    impl Maker {
        // TODO: Proper constructor
        pub fn new(initial_bitcoin_balance: bitcoin::Amount, initial_rate: MidMarketRate) -> Self {
            //TODO: Get function to return zero
            let zero_dai = dai::Amount::from_dai_trunc(0.0).unwrap();
            let zero_btc = bitcoin::Amount::from_btc(0.0).unwrap();

            Maker {
                btc_balance: initial_bitcoin_balance,
                dai_balance: zero_dai.clone(),
                btc_fee: zero_btc,
                dai_fee: zero_dai.clone(),
                btc_locked_funds: zero_btc,
                dai_locked_funds: zero_dai.clone(),
                btc_max_sell_amount: zero_btc,
                dai_max_sell_amount: zero_dai,
                rate: initial_rate,
                // 300 ^= 3.00%
                spread: Spread::new(300).expect("default spread works"),
            }
        }

        pub fn expire_order(&mut self, order: BtcDaiOrder) -> anyhow::Result<maker::NewOrder> {
            // TODO: Bookkeeping, decision making if new order

            // TODO: creating a new order should not take the wallet / book
            // let new_order = new_dai_bitcoin_order();

            let new_order = match order.position {
                Position::Sell => BtcDaiOrder::new_sell(
                    self.btc_balance,
                    self.btc_fee,
                    self.btc_locked_funds,
                    self.btc_max_sell_amount,
                    self.rate.value,
                    self.spread,
                ),
                Position::Buy => BtcDaiOrder::new_buy(),
            }?;

            // Why would we need an Event here anyway?
            Ok(maker::NewOrder::Created(new_order))
        }

        pub fn lock_funds(&mut self, order: BtcDaiOrder) -> anyhow::Result<()> {
            // TODO: Bookkeeping, lock up funds

            Ok(())
        }

        //
        pub fn update_rate(&mut self, new_rate: ()) {}

        // the balance is to be updated once the trade was actually setteled, i.e. the swap execution is finished
        pub fn update_bitcoin_balance(&mut self, balance: crate::bitcoin::Amount) {
            self.btc_balance = balance;
        }

        pub fn track_failed_rate(&mut self, error: anyhow::Error) {}

        pub fn track_failed_balance_update(&mut self, error: anyhow::Error) {}

        pub fn get_order_for_local_swap_id(&self, local_swap_id: LocalSwapId) -> BtcDaiOrder {
            unimplemented!()
        }
    }
}

#[tokio::main]
async fn main() {
    let mut maker = maker::Maker::new(unimplemented!(), unimplemented!());
    let mut orderbook = Orderbook;

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
            Ok(new_balance) => maker.update_bitcoin_balance(new_balance.into()), // maker should record timestamp of this
            Err(e) => maker.track_failed_balance_update(e),
        }

        // if nothing happens on the network for 15 seconds, loop again
        match tokio::time::timeout(Duration::from_secs(15), swarm.next()).await {
            Ok(event) => {
                match event {
                    network::Event::OrderExpired(order) => {
                        let new_order = maker.expire_order(order.into());
                    }
                    network::Event::OrderTakeRequest(order) => unimplemented!(),
                    network::Event::SwapFinalized(local_swap_id, remote_data) => {
                        let order = maker.get_order_for_local_swap_id(local_swap_id);

                        match maker.lock_funds(order) {
                            Ok(()) => {
                                // TODO: Add remote_data learned from the other party to the swap and persist the swap

                                // TODO: Spawn swap execution
                            }
                            Err(error) => unimplemented!(),
                        }
                    }
                }
            }
            _ => (),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use comit::network::RemoteData;
    use comit::LocalSwapId;
    use nectar::order::{BtcDaiOrder, Position};
    use nectar::{MidMarketRate, Rate};

    #[test]
    fn given_that_an_order_expired_then_new_order_is_created_for_same_position() {
        let zero_dai = dai::Amount::from_dai_trunc(0.0).unwrap();
        let zero_btc = bitcoin::Amount::from_btc(0.0).unwrap();

        // Given a maker in a certain state
        let mut maker = maker::Maker::new(
            bitcoin::Amount::from_btc(1.0).unwrap(),
            MidMarketRate {
                value: Rate::new(9000),
                timestamp: Utc::now(),
            },
        );

        let expired_order = BtcDaiOrder {
            position: Position::Sell,
            base: zero_btc,
            quote: zero_dai,
        };

        // When Event happens
        let event = maker.expire_order(expired_order).unwrap();

        assert!(matches!(
            event,
            maker::NewOrder::Created(BtcDaiOrder {
                position: Position::Sell,
                ..
            })
        ));
    }

    #[test]
    fn given_swap_finalized_than_start_execution_action() {
        let zero_dai = dai::Amount::from_dai_trunc(0.0).unwrap();
        let zero_btc = crate::bitcoin::Amount::from_btc(0.0).unwrap();

        let mut maker = maker::Maker::new(
            zero_btc,
            MidMarketRate {
                value: Rate::new(0),
                timestamp: Utc::now(),
            },
        );

        let local_swap_id = LocalSwapId::random();
        let remote_data = RemoteData {
            secret_hash: None,
            ethereum_identity: None,
            lightning_identity: None,
            bitcoin_identity: None,
        };

        // TODO: Additionally assert on the state of the maker (correct bookkeeping)
    }

    #[test]
    fn given_that_an_order_is_taken_and_acceptable_then_take_action() {
        // TODO: implement
    }

    #[test]
    fn given_that_an_order_is_taken_and_unacceptable_then_decline_action() {
        // TODO: implement
    }
}
