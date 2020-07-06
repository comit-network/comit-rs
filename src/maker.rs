#![allow(unused_variables)]

use crate::{
    bitcoin, dai,
    network::Order,
    order::{BtcDaiOrder, Position},
    rate::Spread,
    MidMarketRate, PeersWithOngoingTrades, Rate,
};
use std::convert::TryFrom;

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
    mid_market_rate: Option<MidMarketRate>,
    spread: Spread,
    ongoing_takers: PeersWithOngoingTrades,
}

impl Maker {
    #![allow(clippy::too_many_arguments)]
    pub fn new(
        btc_balance: bitcoin::Amount,
        dai_balance: dai::Amount,
        btc_fee: bitcoin::Amount,
        dai_fee: dai::Amount,
        btc_max_sell_amount: bitcoin::Amount,
        dai_max_sell_amount: dai::Amount,
        mid_market_rate: MidMarketRate,
        spread: Spread,
    ) -> Self {
        Maker {
            btc_balance,
            dai_balance,
            btc_fee,
            dai_fee,
            btc_reserved_funds: Default::default(),
            dai_reserved_funds: Default::default(),
            btc_max_sell_amount,
            dai_max_sell_amount,
            mid_market_rate: Some(mid_market_rate),
            spread,
            ongoing_takers: Default::default(),
        }
    }

    pub fn update_rate(
        &mut self,
        mid_market_rate: MidMarketRate,
    ) -> anyhow::Result<RateUpdateDecision> {
        if let Some(previous_mid_market_rate) = self.mid_market_rate {
            if previous_mid_market_rate.value == mid_market_rate.value {
                return Ok(RateUpdateDecision::NoRateChange);
            }
        }

        self.mid_market_rate = Some(mid_market_rate);
        Ok(RateUpdateDecision::RateChange {
            new_sell_order: self.new_sell_order()?,
            new_buy_order: self.new_buy_order()?,
        })
    }

    pub fn update_bitcoin_balance(&mut self, balance: bitcoin::Amount) {
        self.btc_balance = balance;
    }

    pub fn update_dai_balance(&mut self, balance: dai::Amount) {
        self.dai_balance = balance;
    }

    pub fn track_failed_rate_update(&mut self) {
        self.mid_market_rate = None;
    }

    pub fn track_failed_balance_update(&mut self, error: anyhow::Error) {}

    pub fn new_sell_order(&self) -> anyhow::Result<BtcDaiOrder> {
        match self.mid_market_rate {
            Some(mid_market_rate) => BtcDaiOrder::new_sell(
                self.btc_balance,
                self.btc_fee,
                self.btc_reserved_funds,
                self.btc_max_sell_amount,
                mid_market_rate.value,
                self.spread,
            ),
            None => anyhow::bail!(RateNotAvailable(Position::Sell)),
        }
    }

    pub fn new_buy_order(&self) -> anyhow::Result<BtcDaiOrder> {
        match self.mid_market_rate {
            Some(mid_market_rate) => BtcDaiOrder::new_buy(
                self.dai_balance.clone(),
                self.dai_fee.clone(),
                self.dai_reserved_funds.clone(),
                self.dai_max_sell_amount.clone(),
                mid_market_rate.value,
                self.spread,
            ),
            None => anyhow::bail!(RateNotAvailable(Position::Buy)),
        }
    }

    /// Decide whether we should proceed with order,
    /// Confirm with the order book
    /// Re & take & reserve
    pub fn process_taken_order(&mut self, order: Order) -> anyhow::Result<TakeRequestDecision> {
        if self.ongoing_takers.has_an_ongoing_trade(&order.taker) {
            return Ok(TakeRequestDecision::CannotTradeWithTaker);
        }

        if self.mid_market_rate.is_none() {
            anyhow::bail!(RateNotAvailable(order.inner.position))
        }

        let current_mid_market_rate = self.mid_market_rate.unwrap();
        let current_profitable_rate = self
            .spread
            .apply(current_mid_market_rate.value, order.inner.position)?;
        let order_rate = Rate::try_from(order.inner.clone())?;
        let next_order = match order.clone().into() {
            order
            @
            BtcDaiOrder {
                position: Position::Buy,
                ..
            } => {
                // We are buying BTC for DAI
                // Given an order rate of: 1:9000
                // It is NOT profitable to buy, if the current rate is greater than the order rate.
                // 1:8800 -> We give less DAI for getting BTC -> Good.
                // 1:9200 -> We have to give more DAI for getting BTC -> Sucks.
                if order_rate > current_profitable_rate {
                    return Ok(TakeRequestDecision::RateSucks);
                }

                let updated_dai_reserved_funds = self.dai_reserved_funds.clone() + order.quote;
                if updated_dai_reserved_funds > self.dai_balance {
                    return Ok(TakeRequestDecision::InsufficientFunds);
                }

                self.dai_reserved_funds = updated_dai_reserved_funds;
                self.new_buy_order()?
            }
            order
            @
            BtcDaiOrder {
                position: Position::Sell,
                ..
            } => {
                // We are selling BTC for DAI
                // Given an order rate of: 1:9000
                // It is NOT profitable to sell, if the current rate is smaller than the order rate.
                // 1:8800 -> We get less DAI for our BTC -> Sucks.
                // 1:9200 -> We get more DAI for our BTC -> Good.
                if order_rate < current_profitable_rate {
                    return Ok(TakeRequestDecision::RateSucks);
                }

                let updated_btc_reserved_funds = self.btc_reserved_funds + order.base;
                if updated_btc_reserved_funds > self.btc_balance {
                    return Ok(TakeRequestDecision::InsufficientFunds);
                }

                self.btc_reserved_funds = updated_btc_reserved_funds;
                self.new_sell_order()?
            }
        };

        self.ongoing_takers
            .insert(order.taker)
            .expect("already checked that we can trade");

        Ok(TakeRequestDecision::GoForSwap { next_order })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TakeRequestDecision {
    GoForSwap { next_order: BtcDaiOrder },
    RateSucks,
    InsufficientFunds,
    CannotTradeWithTaker,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RateUpdateDecision {
    RateChange {
        new_sell_order: BtcDaiOrder,
        new_buy_order: BtcDaiOrder,
    },
    NoRateChange,
}

#[derive(Debug, Copy, Clone, thiserror::Error)]
#[error("Rate not available when trying to create new {0} order.")]
pub struct RateNotAvailable(Position);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        network::Order,
        order::{BtcDaiOrder, Position},
        MidMarketRate, Rate,
    };
    use std::convert::TryFrom;

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
                mid_market_rate: Some(MidMarketRate::default()),
                spread: Spread::default(),
                ongoing_takers: PeersWithOngoingTrades::default(),
            }
        }
    }

    #[test]
    fn btc_funds_reserved_upon_taking_sell_order() {
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

        let event = maker.process_taken_order(order_taken).unwrap();

        assert!(matches!(event, TakeRequestDecision::GoForSwap { .. }));
        assert_eq!(
            maker.btc_reserved_funds,
            bitcoin::Amount::from_btc(1.5).unwrap()
        )
    }

    #[test]
    fn dai_funds_reserved_upon_taking_buy_order() {
        let mut maker = Maker {
            dai_balance: dai::Amount::from_dai_trunc(10000.0).unwrap(),
            mid_market_rate: Some(MidMarketRate {
                value: Rate::try_from(1.5).unwrap(),
            }),
            ..Default::default()
        };

        let order_taken = Order {
            inner: BtcDaiOrder {
                position: Position::Buy,
                base: bitcoin::Amount::from_btc(1.0).unwrap(),
                quote: dai::Amount::from_dai_trunc(1.5).unwrap(),
            },
            ..Default::default()
        };

        let result = maker.process_taken_order(order_taken).unwrap();

        assert!(matches!(result, TakeRequestDecision::GoForSwap { .. }));
        assert_eq!(
            maker.dai_reserved_funds,
            dai::Amount::from_dai_trunc(1.5).unwrap()
        )
    }

    #[test]
    fn not_enough_btc_funds_to_reserve_for_a_sell_order() {
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

        let result = maker.process_taken_order(order_taken).unwrap();

        assert_eq!(result, TakeRequestDecision::InsufficientFunds);
    }

    #[test]
    fn not_enough_btc_funds_to_reserve_for_a_buy_order() {
        let mut maker = Maker {
            dai_balance: dai::Amount::zero(),
            mid_market_rate: Some(MidMarketRate {
                value: Rate::try_from(1.5).unwrap(),
            }),
            ..Default::default()
        };

        let order_taken = Order {
            inner: BtcDaiOrder {
                position: Position::Buy,
                base: bitcoin::Amount::from_btc(1.0).unwrap(),
                quote: dai::Amount::from_dai_trunc(1.5).unwrap(),
            },
            ..Default::default()
        };

        let result = maker.process_taken_order(order_taken).unwrap();

        assert_eq!(result, TakeRequestDecision::InsufficientFunds);
    }

    #[test]
    fn not_enough_btc_funds_to_reserve_for_a_sell_order_2() {
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

        let result = maker.process_taken_order(order_taken).unwrap();

        assert_eq!(result, TakeRequestDecision::InsufficientFunds);
    }

    #[test]
    fn cannot_trade_with_taker_if_ongoing_swap_already_exists() {
        let mut maker = Maker {
            btc_balance: bitcoin::Amount::from_btc(1.0).unwrap(),
            ..Default::default()
        };

        let order_taken = Order {
            inner: BtcDaiOrder {
                position: Position::Sell,
                ..Default::default()
            },
            ..Default::default()
        };

        let result = maker.process_taken_order(order_taken.clone()).unwrap();

        assert!(matches!(result, TakeRequestDecision::GoForSwap { .. }));

        let result = maker.process_taken_order(order_taken).unwrap();

        assert_eq!(result, TakeRequestDecision::CannotTradeWithTaker);
    }

    #[test]
    fn yield_error_if_rate_is_not_available() {
        let mut maker = Maker {
            mid_market_rate: None,
            ..Default::default()
        };

        let order_taken = Order {
            ..Default::default()
        };

        let result = maker.process_taken_order(order_taken);
        assert!(result.is_err());

        let result = maker.new_buy_order();
        assert!(result.is_err());

        let result = maker.new_sell_order();
        assert!(result.is_err());
    }

    #[test]
    fn fail_to_confirm_sell_order_if_sell_rate_is_not_good_enough() {
        let mut maker = Maker {
            mid_market_rate: Some(MidMarketRate {
                value: Rate::try_from(10000.0).unwrap(),
            }),
            ..Default::default()
        };

        let order_taken = Order {
            inner: BtcDaiOrder {
                position: Position::Sell,
                base: bitcoin::Amount::from_btc(1.0).unwrap(),
                quote: dai::Amount::from_dai_trunc(9000.0).unwrap(),
            },
            ..Default::default()
        };

        let result = maker.process_taken_order(order_taken).unwrap();

        assert_eq!(result, TakeRequestDecision::RateSucks);
    }

    #[test]
    fn fail_to_confirm_buy_order_if_buy_rate_is_not_good_enough() {
        let mut maker = Maker {
            mid_market_rate: Some(MidMarketRate {
                value: Rate::try_from(10000.0).unwrap(),
            }),
            ..Default::default()
        };

        let order_taken = Order {
            inner: BtcDaiOrder {
                position: Position::Buy,
                base: bitcoin::Amount::from_btc(1.0).unwrap(),
                quote: dai::Amount::from_dai_trunc(11000.0).unwrap(),
            },
            ..Default::default()
        };

        let result = maker.process_taken_order(order_taken).unwrap();

        assert_eq!(result, TakeRequestDecision::RateSucks);
    }

    #[test]
    fn rate_updated_if_rate_update_with_same_value() {
        let mut maker = Maker {
            mid_market_rate: Some(MidMarketRate {
                value: Rate::try_from(1.0).unwrap(),
            }),
            ..Default::default()
        };

        let new_mid_market_rate = MidMarketRate {
            value: Rate::try_from(1.0).unwrap(),
        };

        let reaction = maker.update_rate(new_mid_market_rate).unwrap();
        assert_eq!(reaction, RateUpdateDecision::NoRateChange);
        assert_eq!(maker.mid_market_rate.unwrap(), new_mid_market_rate)
    }

    #[test]
    fn rate_updated_and_new_orders_if_rate_update_with_new_value() {
        let mut maker = Maker {
            mid_market_rate: Some(MidMarketRate {
                value: Rate::try_from(1.0).unwrap(),
            }),
            ..Default::default()
        };

        let new_mid_market_rate = MidMarketRate {
            value: Rate::try_from(2.0).unwrap(),
        };

        let reaction = maker.update_rate(new_mid_market_rate).unwrap();
        assert!(matches!(reaction, RateUpdateDecision::RateChange {..}));
        assert_eq!(maker.mid_market_rate.unwrap(), new_mid_market_rate)
    }
}
