use crate::bitcoin;
use crate::dai;
use crate::{Rate, Spread};
use std::cmp::min;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    Buy,
    Sell,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BtcDaiOrder {
    pub position: Position,
    pub base: bitcoin::Amount,
    pub quote: dai::Amount,
}

impl BtcDaiOrder {
    pub fn new_sell(
        base_balance: bitcoin::Amount,
        base_fees: bitcoin::Amount,
        base_reserved_funds: bitcoin::Amount,
        max_amount: bitcoin::Amount,
        mid_market_rate: Rate,
        spread: Spread,
    ) -> anyhow::Result<BtcDaiOrder> {
        let base = min(base_balance - base_reserved_funds, max_amount) - base_fees;

        let rate = spread.apply(mid_market_rate, Position::Sell)?;
        let quote = base.worth_in(rate);

        Ok(BtcDaiOrder {
            position: Position::Sell,
            base,
            quote,
        })
    }

    pub fn new_buy(
        quote_balance: dai::Amount,
        quote_fees: dai::Amount,
        quote_reserved_funds: dai::Amount,
        max_amount: dai::Amount,
        mid_market_rate: Rate,
        spread: Spread,
    ) -> anyhow::Result<BtcDaiOrder> {
        let quote = min(quote_balance - quote_reserved_funds, max_amount) - quote_fees;

        let rate = spread.apply(mid_market_rate, Position::Buy)?;
        let base = quote.worth_in(rate)?;

        Ok(BtcDaiOrder {
            position: Position::Buy,
            base,
            quote,
        })
    }
}

pub trait LockedFunds {
    type Amount;
    fn locked_funds(&self) -> Self::Amount;
}

pub trait Balance {
    type Amount;
    fn balance(&self) -> Self::Amount;
}

pub trait Fees {
    type Amount;
    fn fees(&self) -> Self::Amount;
}

#[cfg(test)]
impl Default for BtcDaiOrder {
    fn default() -> Self {
        Self {
            position: Position::Buy,
            base: bitcoin::Amount::from_sat(1),
            quote: dai::Amount::from_atto(num::BigUint::from(1u8)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Rate;
    use std::convert::TryFrom;

    fn btc(btc: f64) -> bitcoin::Amount {
        bitcoin::Amount::from_btc(btc).unwrap()
    }

    fn dai(dai: f64) -> dai::Amount {
        dai::Amount::from_dai_trunc(dai).unwrap()
    }

    #[test]
    fn given_a_balance_return_order_selling_full_balance() {
        let rate = Rate::try_from(1.0).unwrap();
        let order = BtcDaiOrder::new_sell(
            btc(10.0),
            btc(0.0),
            btc(0.0),
            btc(100.0),
            rate,
            Spread::new(0).unwrap(),
        )
        .unwrap();

        assert_eq!(order.base, btc(10.0));

        let order = BtcDaiOrder::new_buy(
            dai(10.0),
            dai(0.0),
            dai(0.0),
            dai(100.0),
            rate,
            Spread::new(0).unwrap(),
        )
        .unwrap();

        assert_eq!(order.quote, dai(10.0));
    }

    #[test]
    fn given_a_balance_and_locked_funds_return_order_selling_available_balance() {
        let rate = Rate::try_from(1.0).unwrap();
        let order = BtcDaiOrder::new_sell(
            btc(10.0),
            btc(0.0),
            btc(2.0),
            btc(100.0),
            rate,
            Spread::new(0).unwrap(),
        )
        .unwrap();

        assert_eq!(order.base, btc(8.0));

        let order = BtcDaiOrder::new_buy(
            dai(10.0),
            dai(0.0),
            dai(2.0),
            dai(100.0),
            rate,
            Spread::new(0).unwrap(),
        )
        .unwrap();

        assert_eq!(order.quote, dai(8.0));
    }

    #[test]
    fn given_an_available_balance_and_a_max_amount_sell_min_of_either() {
        let rate = Rate::try_from(1.0).unwrap();
        let order = BtcDaiOrder::new_sell(
            btc(10.0),
            btc(0.0),
            btc(2.0),
            btc(2.0),
            rate,
            Spread::new(0).unwrap(),
        )
        .unwrap();

        assert_eq!(order.base, btc(2.0));

        let order = BtcDaiOrder::new_buy(
            dai(10.0),
            dai(0.0),
            dai(2.0),
            dai(2.0),
            rate,
            Spread::new(0).unwrap(),
        )
        .unwrap();

        assert_eq!(order.quote, dai(2.0));
    }

    #[test]
    fn given_an_available_balance_and_fees_sell_balance_minus_fees() {
        let rate = Rate::try_from(1.0).unwrap();
        let order = BtcDaiOrder::new_sell(
            btc(10.0),
            btc(1.0),
            btc(2.0),
            btc(2.0),
            rate,
            Spread::new(0).unwrap(),
        )
        .unwrap();

        assert_eq!(order.base, btc(1.0));

        let order = BtcDaiOrder::new_buy(
            dai(10.0),
            dai(1.0),
            dai(2.0),
            dai(2.0),
            rate,
            Spread::new(0).unwrap(),
        )
        .unwrap();

        assert_eq!(order.quote, dai(1.0));
    }

    #[test]
    fn given_a_rate_return_order_with_both_amounts() {
        let spread = Spread::new(0).unwrap();

        let rate = Rate::try_from(0.1).unwrap();
        let order =
            BtcDaiOrder::new_sell(btc(1051.0), btc(1.0), btc(50.0), btc(9999.0), rate, spread)
                .unwrap();

        // 1 Sell => 0.1 Buy
        // 1000 Sell => 100 Buy
        assert_eq!(order.base, btc(1000.0));
        assert_eq!(order.quote, dai(100.0));

        let rate = Rate::try_from(10.0).unwrap();
        let order =
            BtcDaiOrder::new_sell(btc(1051.0), btc(1.0), btc(50.0), btc(9999.0), rate, spread)
                .unwrap();

        assert_eq!(order.base, btc(1000.0));
        assert_eq!(order.quote, dai(10_000.0));

        let rate = Rate::try_from(0.1).unwrap();
        let order =
            BtcDaiOrder::new_buy(dai(1051.0), dai(1.0), dai(50.0), dai(9999.0), rate, spread)
                .unwrap();

        // 1 Sell => 0.1 Buy
        // 1000 Sell => 100 Buy
        assert_eq!(order.base, btc(100.0));
        assert_eq!(order.quote, dai(1000.0));

        let rate = Rate::try_from(10.0).unwrap();
        let order =
            BtcDaiOrder::new_buy(dai(1051.0), dai(1.0), dai(50.0), dai(9999.0), rate, spread)
                .unwrap();

        assert_eq!(order.base, btc(10_000.0));
        assert_eq!(order.quote, dai(1000.0));
    }

    #[test]
    fn given_a_rate_and_spread_return_order_with_both_amounts() {
        let rate = Rate::try_from(0.1).unwrap();
        let spread = Spread::new(300).unwrap();

        let order =
            BtcDaiOrder::new_sell(btc(1051.0), btc(1.0), btc(50.0), btc(9999.0), rate, spread)
                .unwrap();

        assert_eq!(order.base, btc(1000.0));
        assert_eq!(order.quote, dai(103.0));

        let order =
            BtcDaiOrder::new_buy(dai(1051.0), dai(1.0), dai(50.0), dai(9999.0), rate, spread)
                .unwrap();

        assert_eq!(order.base, btc(97.0));
        assert_eq!(order.quote, dai(1000.0));
    }
}
