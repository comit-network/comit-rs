use anyhow::Context;
use comit::{
    asset::{ethereum::FromWei, Erc20Quantity},
    Position, Price,
};
use num::{BigUint, Integer, ToPrimitive};
use rust_decimal::prelude::*;
use serde::{Deserialize, Serialize};
use std::{convert::TryFrom, fmt};

/// Represent a rate. Note this is designed to support Bitcoin/Dai buy and sell
/// rates (Bitcoin being in the range of 10k-100kDai) A rate has a maximum
/// precision of 9 digits after the decimal rate = self.0 * 10e-9
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, PartialOrd)]
pub struct Rate(u64);

impl fmt::Display for Rate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut decimal = Decimal::from(self.0);
        decimal
            .set_scale(Self::PRECISION as u32)
            .expect("Self::PRECISION < Decimal::MAX_PRECISION");

        decimal.fmt(f)
    }
}

impl Rate {
    pub const PRECISION: u16 = 10;

    /// integer = rate * 10ePRECISION
    pub fn new(integer: u64) -> Self {
        Rate(integer)
    }

    /// integer = rate * 10ePRECISION
    pub fn integer(self) -> BigUint {
        BigUint::from(self.0)
    }
}

impl TryFrom<Decimal> for Rate {
    type Error = anyhow::Error;

    fn try_from(value: Decimal) -> Result<Self, Self::Error> {
        let value = value
            .to_u64()
            .context("Rate is too big, cannot be expressed within given precision")?;

        Ok(Rate(value))
    }
}

impl TryFrom<f64> for Rate {
    type Error = anyhow::Error;

    fn try_from(rate: f64) -> Result<Self, Self::Error> {
        let decimal = Decimal::from_f64(rate).context("Rate exeeds precision of decimal")?;
        let rate_precision = Decimal::from(10_000_000_000u64);
        let adjusted_rate = decimal
            .checked_mul(rate_precision)
            .context("scaling rate to precision of 10 overflowed")?;

        let rate = Rate::try_from(adjusted_rate)?;

        Ok(rate)
    }
}

impl Into<Price<comit::asset::Bitcoin, comit::asset::Erc20Quantity>> for Rate {
    fn into(self) -> Price<comit::asset::Bitcoin, comit::asset::Erc20Quantity> {
        let btc_to_dai = Erc20Quantity::from_wei(self.0);
        Price::from_wei_per_sat(btc_to_dai)
    }
}

/// Spread: percentage to be added on top of a rate or amount with
/// a maximum precision of 2 decimals
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Spread(u16);

impl Spread {
    /// Input is the spread in permyriad (per ten thousand):
    /// 5% => 500 permyriad
    /// 23.14% => 2314 permyriad
    /// 0.001% => Not allowed
    /// 200% => Not allowed
    pub fn new(permyriad: u16) -> anyhow::Result<Spread> {
        if permyriad > 10000 {
            anyhow::bail!("Spread must be between 0% and 100%");
        }

        Ok(Spread(permyriad))
    }

    pub fn apply(self, rate: Rate, position: Position) -> anyhow::Result<Rate> {
        let ten_thousand = BigUint::from(10_000u16);

        let spread = match position {
            Position::Sell => ten_thousand.clone() + self.0,
            Position::Buy => ten_thousand.clone() - self.0,
        };

        let integer = rate.integer() * (spread);

        // Now divide by 10e4 because of the spread
        let (rate, _remainder) = integer.div_rem(&ten_thousand);
        let rate = rate
            .to_u64()
            .ok_or_else(|| anyhow::anyhow!("Result is unexpectedly large"))?;
        Ok(Rate::new(rate))
    }
}

#[cfg(test)]
pub fn rate(rate: f64) -> Rate {
    Rate::try_from(rate).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn from_f64_and_new_matches_1() {
        let rate_from_f64 = Rate::try_from(123.456).unwrap();
        let rate_new = Rate::new(1_234_560_000_000);
        assert_eq!(rate_from_f64, rate_new);
    }

    #[test]
    fn from_f64_and_new_matches_2() {
        let rate_from_f64 = Rate::try_from(10.0).unwrap();
        let rate_new = Rate::new(100_000_000_000);
        assert_eq!(rate_from_f64, rate_new);
    }

    #[test]
    fn rate_error_on_negative_rate() {
        let rate = Rate::try_from(-1.0);
        assert!(rate.is_err());
    }

    #[test]
    fn spread_error_on_above_hundred() {
        let spread = Spread::new(10100);
        assert!(spread.is_err());
    }

    #[test]
    fn spread_no_error_on_hundred() {
        let spread = Spread::new(10000);
        assert!(spread.is_ok());
    }

    #[test]
    fn apply_spread_20() {
        let spread = Spread::new(2000).unwrap();
        let rate = Rate::try_from(25.0).unwrap();

        let new_rate = spread.apply(rate, Position::Sell).unwrap();
        assert_eq!(new_rate, Rate::try_from(30.0).unwrap());

        let new_rate = spread.apply(rate, Position::Buy).unwrap();
        assert_eq!(new_rate, Rate::try_from(20.0).unwrap());
    }

    #[test]
    fn apply_spread_3() {
        let spread = Spread::new(300).unwrap();
        let rate = Rate::try_from(10.0).unwrap();

        let sell_rate = spread.apply(rate, Position::Sell).unwrap();
        assert_eq!(sell_rate, Rate::try_from(10.3).unwrap());

        let buy_rate = spread.apply(rate, Position::Buy).unwrap();
        assert_eq!(buy_rate, Rate::try_from(9.7).unwrap());
    }

    #[test]
    fn apply_spread_zero_doesnt_change_rate() {
        let spread = Spread::new(0).unwrap();
        let rate = Rate::try_from(123_456.789).unwrap();

        let res = spread.apply(rate, Position::Sell).unwrap();
        assert_eq!(rate, res);

        let res = spread.apply(rate, Position::Buy).unwrap();
        assert_eq!(rate, res);
    }

    proptest! {
        #[test]
        fn spread_new_doesnt_panic(s in any::<u16>()) {
            let _ = Spread::new(s);
        }
    }

    proptest! {
        #[test]
        fn rate_new_doesnt_panic(i in any::<u64>()) {
            let _ = Rate::new(i);
        }
    }

    proptest! {
        #[test]
        fn rate_from_f64_doesnt_panic(f in any::<f64>()) {
            let _ = Rate::try_from(f);
        }
    }

    prop_compose! {
        fn new_spread()(s in any::<u16>()) -> anyhow::Result<Spread> {
            Spread::new(s)
        }
    }

    prop_compose! {
        fn new_rate()(f in any::<f64>()) -> anyhow::Result<Rate> {
            Rate::try_from(f)
        }
    }

    proptest! {
        #[test]
        fn spread_apply_doesnt_panic(rate in new_rate(), spread in new_spread()) {
            if let (Ok(rate), Ok(spread)) = (rate, spread) {
                let _ = spread.apply(rate, Position::Sell);
                let _ = spread.apply(rate, Position::Buy);
            }
        }
    }

    proptest! {
        #[test]
        fn spread_zero_doesnt_change_rate(rate in new_rate()) {
            let spread = Spread::new(0).unwrap();
            if let Ok(rate) = rate {
                let res = spread.apply(rate, Position::Sell).unwrap();
                assert_eq!(res, rate);

                let res = spread.apply(rate, Position::Buy).unwrap();
                assert_eq!(res, rate);
            }
        }
    }
}
