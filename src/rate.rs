use anyhow::{bail, Context};
use num::{BigUint, Integer, ToPrimitive};
use std::convert::TryFrom;
use std::iter::FromIterator;
use std::str::FromStr;

/// Represent a rate. Note this is designed to support Bitcoin/Dai buy and sell rates (Bitcoin being in the range of 10k-100kDai)
/// A rate has a maximum precision of 9 digits after the decimal
// rate = self.0 * 10e-9
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rate(u64);

impl Rate {
    pub const PRECISION: u16 = 9;
    const MAX_RATE: u64 = (u64::MAX / 10) ^ 9;

    /// integer = rate * 10ePRECISION
    pub fn new(integer: u64) -> anyhow::Result<Self> {
        if integer > Self::MAX_RATE {
            bail!("Rate is larger than supported");
        }
        Ok(Rate(integer))
    }

    /// integer = rate * 10ePRECISION
    pub fn integer(&self) -> BigUint {
        BigUint::from(self.0)
    }
}

impl TryFrom<f64> for Rate {
    type Error = anyhow::Error;

    fn try_from(rate: f64) -> Result<Self, Self::Error> {
        if rate.is_sign_negative() {
            anyhow::bail!("Rate must be positive");
        }

        if !rate.is_finite() {
            anyhow::bail!("Rate must be finite")
        }

        let mut rate = rate.to_string();
        let decimal_index = rate.find('.');
        let mantissa = match decimal_index {
            None => String::new(),
            Some(decimal_index) => {
                let mantissa = rate.split_off(decimal_index + 1);
                if mantissa.len() > Self::PRECISION as usize {
                    anyhow::bail!(format!(
                        "Precision of the rate is too high (max is {}).",
                        Self::PRECISION
                    ))
                } else {
                    rate.truncate(rate.len() - 1); // Removes the trailing decimal point
                    mantissa
                }
            }
        };

        let mantissa_length = mantissa.len();
        let integer = rate;

        let zeros = vec!['0'].repeat(Self::PRECISION as usize - mantissa_length);
        let zeros = String::from_iter(zeros.into_iter());
        let integer = u64::from_str(&format!("{}{}{}", integer, mantissa, zeros))
            .context("Rate is unexpectedly large")?;
        Rate::new(integer)
    }
}

/// Spread: percentage to be added on top of a rate or amount with
/// a maximum precision of 2 decimals
#[derive(Clone, Copy, Debug)]
pub struct Spread(u16);

impl Spread {
    /// Input is the spread in percent, with 2 digits after the decimal point:
    /// 5% => 500
    /// 23.14% => 2314
    /// 0.001% => Not allowed
    /// 200% => Not allowed
    pub fn new(spread: u16) -> anyhow::Result<Spread> {
        if spread > 10000 {
            anyhow::bail!("Spread must be between 0% and 100%");
        }

        Ok(Spread(spread))
    }

    pub fn apply(&self, rate: Rate) -> anyhow::Result<Rate> {
        let ten_thousand = BigUint::from(10_000u16);
        let integer = rate.integer() * (ten_thousand.clone() + self.0);
        // Now divide by 10e4 because of the spread
        let (rate, _remainder) = integer.div_rem(&ten_thousand);
        let rate = rate
            .to_u64()
            .ok_or_else(|| anyhow::anyhow!("Result is unexpectedly large"))?;
        Rate::new(rate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn from_f64_and_new_matches_1() {
        let rate_from_f64 = Rate::try_from(123.456).unwrap();
        let rate_new = Rate::new(123_456_000_000).unwrap();
        assert_eq!(rate_from_f64, rate_new);
    }

    #[test]
    fn from_f64_and_new_matches_2() {
        let rate_from_f64 = Rate::try_from(10.0).unwrap();
        let rate_new = Rate::new(10_000_000_000).unwrap();
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
        let new_rate = spread.apply(rate).unwrap();

        assert_eq!(new_rate, Rate::try_from(30.0).unwrap())
    }

    #[test]
    fn apply_spread_3() {
        let spread = Spread::new(2000).unwrap();
        let rate = Rate::try_from(25.0).unwrap();
        let new_rate = spread.apply(rate).unwrap();

        assert_eq!(new_rate, Rate::try_from(30.0).unwrap())
    }

    #[test]
    fn apply_spread_zero_doesnt_change_rate() {
        let spread = Spread::new(0).unwrap();
        let rate = Rate::try_from(123456.789).unwrap();
        let res = spread.apply(rate).unwrap();
        assert_eq!(rate, res);
    }

    proptest! {
        #[test]
        fn spread_new_doesnt_panic(f in any::<u16>()) {
            let _ = Spread::new(f);
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

    proptest! {
        #[test]
        fn spread_apply_doesnt_panic(s in any::<u16>(), r in any::<f64>()) {
            let spread = Spread::new(s);
            let rate = Rate::try_from(r);
            if let (Ok(rate), Ok(spread)) = (rate, spread) {
                let _ = spread.apply(rate);
            }
        }
    }

    proptest! {
        #[test]
        fn spread_zero_doesnt_change_rate(r in any::<f64>()) {
            let spread = Spread::new(0).unwrap();
            let rate = Rate::try_from(r);
            if let Ok(rate) = rate {
                let res = spread.apply(rate).unwrap();
                assert_eq!(res, rate)
            }
        }
    }
}
