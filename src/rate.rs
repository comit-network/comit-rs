use anyhow::bail;
use num::{BigUint, Integer, ToPrimitive};
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
    pub fn integer(&self) -> u64 {
        self.0
    }

    pub fn from_f64(rate: f64) -> anyhow::Result<Rate> {
        if rate.is_sign_negative() {
            anyhow::bail!("Spread must be positive");
        }

        if !rate.is_finite() {
            anyhow::bail!("Spread must be finite")
        }

        let mut rate = rate.to_string();
        let decimal_index = rate.find('.');
        let mantissa = match decimal_index {
            None => String::new(),
            Some(decimal_index) => {
                let mantissa = rate.split_off(decimal_index + 1);
                if mantissa.len() > Self::PRECISION as usize {
                    anyhow::bail!("Precision of the rate is too high.")
                } else {
                    // Removes the trailing decimal point
                    rate.truncate(rate.len() - 1);
                    mantissa
                }
            }
        };

        let mantissa_length = mantissa.len();
        let integer = rate;

        let zeros = vec!['0'].repeat(Self::PRECISION as usize - mantissa_length);
        let zeros = String::from_iter(zeros.into_iter());
        let integer = u64::from_str(&format!("{}{}{}", integer, mantissa, zeros))
            .map_err(|_| anyhow::anyhow!("Rate is unexpectedly large"))?;
        Rate::new(integer)
    }

    /// If the integer part ends with 0 and the inverse_decimal_exponent is not null
    /// then we can reduce the representation by removing zero and decrementing the inverse
    /// exponent. For example:
    /// Rate { integer: 1000, inv_dec_exp: 1 } becomes
    /// Rate { integer: 100, inv_dec_exp: 0 }.
    fn reduce(integer: u64, inv_dec_exp: usize) -> (u64, usize) {
        let mut integer_str = integer.to_string();
        if integer_str.len() > 1 && integer_str.ends_with('0') && inv_dec_exp != 0 {
            integer_str.truncate(integer_str.len() - 1);
            let inv_dec_exp = inv_dec_exp - 1;
            let integer = u64::from_str(&integer_str).expect("an integer");
            Rate::reduce(integer, inv_dec_exp)
        } else {
            (integer, inv_dec_exp)
        }
    }
}

/// Spread: percentage to be added on top of a rate or amount with
/// a maximum precision of 2 decimals
// 0 is the rate * 100.
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
        let integer = BigUint::from(rate.integer()) * (ten_thousand.clone() + self.0);
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
        let rate_from_f64 = Rate::from_f64(123.456).unwrap();
        let rate_new = Rate::new(123_456_000_000).unwrap();
        assert_eq!(rate_from_f64, rate_new);
    }

    #[test]
    fn from_f64_and_new_matches_2() {
        let rate_from_f64 = Rate::from_f64(10.0).unwrap();
        let rate_new = Rate::new(10_000_000_000).unwrap();
        assert_eq!(rate_from_f64, rate_new);
    }

    #[test]
    fn rate_error_on_negative_rate() {
        let rate = Rate::from_f64(-1.0);
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
        let rate = Rate::from_f64(25.0).unwrap();
        let new_rate = spread.apply(rate).unwrap();

        assert_eq!(new_rate, Rate::from_f64(30.0).unwrap())
    }

    #[test]
    fn apply_spread_3() {
        let spread = Spread::new(2000).unwrap();
        let rate = Rate::from_f64(25.0).unwrap();
        let new_rate = spread.apply(rate).unwrap();

        assert_eq!(new_rate, Rate::from_f64(30.0).unwrap())
    }

    #[test]
    fn apply_spread_zero_doesnt_change_rate() {
        let spread = Spread::new(0).unwrap();
        let rate = Rate::from_f64(123456.789).unwrap();
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
            let _ = Rate::from_f64(f);
        }
    }

    proptest! {
        #[test]
        fn spread_apply_doesnt_panic(s in any::<u16>(), r in any::<f64>()) {
            let spread = Spread::new(s);
            let rate = Rate::from_f64(r);
            if let (Ok(rate), Ok(spread)) = (rate, spread) {
                let _ = spread.apply(rate);
            }
        }
    }

    proptest! {
        #[test]
        fn spread_zero_doesnt_change_rate(r in any::<f64>()) {
            let spread = Spread::new(0).unwrap();
            let rate = Rate::from_f64(r);
            if let Ok(rate) = rate {
                let res = spread.apply(rate).unwrap();
                assert_eq!(res, rate)
            }
        }
    }
}
