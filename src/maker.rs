#![allow(unused_variables)]

use crate::{
    bitcoin, dai,
    network::Order,
    order::{BtcDaiOrder, Position},
    rate::Spread,
    MidMarketRate, OngoingTakers,
};
use anyhow::Context;
use comit::LocalSwapId;

#[derive(Debug, PartialEq)]
pub enum NewOrder {
    Created(BtcDaiOrder),
}

// Bundles the state of the application
#[derive(Debug)]
pub struct Maker {
    btc_balance: bitcoin::Amount,
    dai_balance: dai::Amount,
    btc_fee: bitcoin::Amount,
    dai_fee: dai::Amount,
    pub btc_reserved_funds: bitcoin::Amount,
    pub dai_reserved_funds: dai::Amount,
    btc_max_sell_amount: bitcoin::Amount,
    dai_max_sell_amount: dai::Amount,
    rate: MidMarketRate,
    spread: Spread,
    ongoing_swaps: OngoingTakers,
}

impl Maker {
    pub fn new() -> Self {
        todo!()
    }

    pub fn expire_order(&mut self, order: BtcDaiOrder) -> anyhow::Result<NewOrder> {
        // TODO: Bookkeeping, decision making if new order

        // TODO: creating a new order should not take the wallet / book
        // let new_order = new_dai_bitcoin_order();

        let new_order = match order.position {
            Position::Sell => BtcDaiOrder::new_sell(
                self.btc_balance,
                self.btc_fee,
                self.btc_reserved_funds,
                self.btc_max_sell_amount,
                self.rate.value,
                self.spread,
            ),
            Position::Buy => BtcDaiOrder::new_buy(),
        }?;

        // Why would we need an Event here anyway?
        Ok(NewOrder::Created(new_order))
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

    /// Decide whether we should proceed with order,
    /// Confirm with the order book
    /// Re & take & reserve
    pub fn confirm_order(&mut self, order: Order) -> anyhow::Result<()> {
        // TODO: Check that rate is still profitable

        self.ongoing_swaps
            .insert(order.taker)
            .context("could not confirm order")?;

        match order.into() {
            BtcDaiOrder {
                position: Position::Buy,
                ..
            } => todo!(),
            BtcDaiOrder {
                position: Position::Sell,
                base,
                ..
            } => {
                let updated_btc_reserved_funds = self.btc_reserved_funds + base;
                if updated_btc_reserved_funds > self.btc_balance {
                    anyhow::bail!("insufficient funds to confirm the order")
                }

                self.btc_reserved_funds = updated_btc_reserved_funds;
            }
        };

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        network::Order,
        order::{BtcDaiOrder, Position},
        MidMarketRate, Rate,
    };
    use chrono::Utc;

    impl Default for Maker {
        fn default() -> Self {
            Self {
                btc_balance: bitcoin::Amount::default(),
                dai_balance: dai::Amount::default(),
                btc_fee: bitcoin::Amount::default(),
                dai_fee: dai::Amount::default(),
                btc_reserved_funds: bitcoin::Amount::default(),
                dai_reserved_funds: dai::Amount::default(),
                btc_max_sell_amount: bitcoin::Amount::default(),
                dai_max_sell_amount: dai::Amount::default(),
                rate: MidMarketRate::default(),
                spread: Spread::default(),
                ongoing_swaps: OngoingTakers::default(),
            }
        }
    }

    #[test]
    fn given_that_an_order_expired_then_new_order_is_created_for_same_position() {
        let zero_dai = dai::Amount::from_dai_trunc(0.0).unwrap();
        let zero_btc = bitcoin::Amount::from_btc(0.0).unwrap();

        // Given a maker in a certain state
        let mut maker = Maker {
            btc_balance: bitcoin::Amount::from_btc(1.0).unwrap(),
            rate: MidMarketRate {
                value: Rate::new(9000),
                timestamp: Utc::now(),
            },
            ..Default::default()
        };

        let expired_order = BtcDaiOrder {
            position: Position::Sell,
            base: zero_btc,
            quote: zero_dai,
        };

        // When Event happens
        let event = maker.expire_order(expired_order).unwrap();

        assert!(matches!(
            event,
            NewOrder::Created(BtcDaiOrder {
                position: Position::Sell,
                ..
            })
        ));
    }

    #[test]
    fn given_order_is_taken_and_confirmed_then_funds_are_marked_reserved() {
        let mut maker = Maker {
            btc_balance: bitcoin::Amount::from_btc(3.0).unwrap(),
            ..Default::default()
        };

        let order_taken = Order {
            inner: BtcDaiOrder {
                position: Position::Sell,
                base: bitcoin::Amount::from_btc(1.5).unwrap(),
                quote: dai::Amount::zero(),
            },
            ..Default::default()
        };

        let res = maker.confirm_order(order_taken).unwrap();

        assert_eq!(
            maker.btc_reserved_funds,
            bitcoin::Amount::from_btc(1.5).unwrap()
        )
    }

    #[test]
    fn fail_if_not_enough_funds_to_reserve_for_an_order() {
        let mut maker = Maker {
            btc_balance: bitcoin::Amount::ZERO,
            ..Default::default()
        };

        let order_taken = Order {
            inner: BtcDaiOrder {
                position: Position::Sell,
                base: bitcoin::Amount::from_btc(1.5).unwrap(),
                quote: dai::Amount::zero(),
            },
            ..Default::default()
        };

        let res = maker.confirm_order(order_taken);

        assert!(res.is_err())
    }

    #[test]
    fn fail_if_not_enough_funds_to_reserve_for_an_order_2() {
        let mut maker = Maker {
            btc_balance: bitcoin::Amount::from_btc(2.0).unwrap(),
            btc_reserved_funds: bitcoin::Amount::from_btc(1.5).unwrap(),
            ..Default::default()
        };

        let order_taken = Order {
            inner: BtcDaiOrder {
                position: Position::Sell,
                base: bitcoin::Amount::from_btc(1.0).unwrap(),
                quote: dai::Amount::zero(),
            },
            ..Default::default()
        };

        let res = maker.confirm_order(order_taken);

        assert!(res.is_err())
    }

    #[test]
    fn fail_to_confirm_order_if_ongoing_swap_with_same_taker_exists() {
        let mut maker = Maker {
            ..Default::default()
        };

        let order_taken = Order {
            inner: BtcDaiOrder {
                position: Position::Sell,
                base: bitcoin::Amount::ZERO,
                quote: dai::Amount::zero(),
            },
            ..Default::default()
        };

        let res = maker.confirm_order(order_taken.clone());

        assert!(res.is_ok());

        let res = maker.confirm_order(order_taken);

        assert!(res.is_err());
    }
}
