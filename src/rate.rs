use std::convert::TryFrom;
use std::str::FromStr;

/// Represent a rate. Note this is designed to support Bitcoin/Dai buy and sell rates (Bitcoin being in the range of 10k-100kDai)
// The rate is equal to integer * 10e-inv_dec_exp
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rate {
    integer: u32,
    inv_dec_exp: usize,
}

impl Rate {
    const MAX_PRECISION: usize = 9;

    pub fn integer(&self) -> u32 {
        self.integer
    }

    pub fn inverse_decimal_exponent(&self) -> usize {
        self.inv_dec_exp
    }

    pub fn new(integer: u64, inverse_decimal_exponent: usize) -> anyhow::Result<Self> {
        let (integer, inv_dec_exp) = Rate::reduce(integer, inverse_decimal_exponent);

        let integer =
            u32::try_from(integer).map_err(|_| anyhow::anyhow!("Value is unexpectedly large."))?;
        Ok(Rate {
            integer,
            inv_dec_exp,
        })
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
        match decimal_index {
            None => {
                let rate = u64::from_str(&rate)
                    .map_err(|_| anyhow::anyhow!("Rate is unexpectedly large."))?;
                Rate::new(rate, 0)
            }
            Some(decimal_index) => {
                let mantissa = rate.split_off(decimal_index + 1);
                if mantissa.len() > Self::MAX_PRECISION {
                    anyhow::bail!("Precision of the rate is too high.")
                } else {
                    // Removes the trailing decimal point
                    rate.truncate(rate.len() - 1);
                    let integer = rate;

                    let inv_dec_exp = mantissa.len();

                    let integer = u64::from_str(&format!("{}{}", integer, mantissa))
                        .map_err(|_| anyhow::anyhow!("Rate is unexpectedly large"))?;
                    Rate::new(integer, inv_dec_exp)
                }
            }
        }
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
        // Goes to u64 to avoid overflow until it's reduced.
        let integer = rate.integer as u64 * (10_000 + self.0 as u64);
        let inv_dec_exp = rate.inv_dec_exp + 4;
        Rate::new(integer, inv_dec_exp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

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

        assert_eq!(
            new_rate,
            Rate {
                integer: 30,
                inv_dec_exp: 0
            }
        )
    }

    #[test]
    fn apply_spread_3() {
        let spread = Spread::new(2000).unwrap();
        let rate = Rate::from_f64(25.0).unwrap();
        let new_rate = spread.apply(rate).unwrap();

        assert_eq!(
            new_rate,
            Rate {
                integer: 30,
                inv_dec_exp: 0
            }
        )
    }

    proptest! {
        #[test]
        fn spread_new_doesnt_panic(f in any::<u16>()) {
            let _ = Spread::new(f);
        }
    }

    proptest! {
        #[test]
        fn rate_new_doesnt_panic(i in any::<u64>(), u in any::<usize>()) {
            let _ = Rate::new(i, u);
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
}
