use crate::bitcoin::{self, SATS_IN_BITCOIN_EXP};
use crate::float_maths::{divide_pow_ten_trunc, multiple_pow_ten, truncate};
use crate::publish::WorthIn;
use crate::rate::Rate;
use conquer_once::Lazy;
use num::{pow::Pow, BigUint, ToPrimitive};

pub const ATTOS_IN_DAI_EXP: u16 = 18;
pub static DAI_DEC: Lazy<BigUint> = Lazy::new(|| BigUint::from(10u16).pow(ATTOS_IN_DAI_EXP));

#[derive(Clone, Ord, PartialOrd, PartialEq, Eq)]
pub struct Amount(BigUint);

impl Amount {
    /// Rounds the value received to a 9 digits mantissa.
    pub fn from_dai_trunc(dai: f64) -> anyhow::Result<Self> {
        if dai.is_sign_negative() {
            anyhow::bail!("Passed value is negative")
        }

        if !dai.is_finite() {
            anyhow::bail!("Passed value is not finite")
        }

        let dai = truncate(dai, ATTOS_IN_DAI_EXP);

        let u_int_value = multiple_pow_ten(dai, ATTOS_IN_DAI_EXP).expect("It is truncated");

        Ok(Amount(u_int_value))
    }

    pub fn from_atto(atto: BigUint) -> Self {
        Amount(atto)
    }

    pub fn as_atto(&self) -> BigUint {
        self.0.clone()
    }
}

impl std::fmt::Debug for Amount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::Display for Amount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// The rate input is for dai to bitcoin but we applied it to attodai so we need to:
// - divide to get dai (18)
// - divide to adjust for rate (9)
// - multiple to get satoshis (8)
// = - 19
const ADJUSTEMENT_EXP: i32 =
    SATS_IN_BITCOIN_EXP as i32 - ATTOS_IN_DAI_EXP as i32 - Rate::PRECISION as i32;

impl WorthIn<crate::bitcoin::Amount> for Amount {
    fn worth_in(&self, dai_to_btc_rate: Rate) -> anyhow::Result<bitcoin::Amount> {
        // Get the integer part of the rate
        let uint_rate = dai_to_btc_rate.integer();

        // Apply the rate
        let worth = uint_rate * self.as_atto();

        let inv_exp = ADJUSTEMENT_EXP.abs() as usize;
        let sats = divide_pow_ten_trunc(worth, inv_exp)
            .to_u64()
            .ok_or_else(|| anyhow::anyhow!("Result is unexpectedly large"))?;

        Ok(bitcoin::Amount::from_sat(sats))
    }
}

impl std::ops::Sub for Amount {
    type Output = Amount;

    fn sub(self, rhs: Self) -> Self::Output {
        Amount(self.0 - rhs.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::str::FromStr;

    #[test]
    fn given_float_dai_amount_less_precise_than_attodai_then_exact_value_is_stored() {
        let some_dai = Amount::from_dai_trunc(1.555_555_555).unwrap();
        let same_amount = Amount::from_atto(BigUint::from(1_555_555_555_000_000_000u64));

        assert_eq!(some_dai, same_amount);
    }

    #[test]
    fn given_float_dai_amount_more_precise_than_attodai_then_stored_value_is_truncated() {
        let some_dai = Amount::from_dai_trunc(0.000_000_555_555_555_555_5).unwrap();
        let same_amount = Amount::from_atto(BigUint::from(555_555_555_555u64));

        assert_eq!(some_dai, same_amount);
    }

    #[test]
    fn using_rate_returns_correct_result() {
        let dai = Amount::from_dai_trunc(1.0).unwrap();
        let rate = Rate::from_f64(0.001_234).unwrap();

        let res: bitcoin::Amount = dai.worth_in(rate).unwrap();

        let btc = bitcoin::Amount::from_btc(0.001_234).unwrap();
        assert_eq!(res, btc);
    }

    #[test]
    fn worth_in_result_truncated_1() {
        let dai = Amount::from_dai_trunc(101.0).unwrap();
        let rate = Rate::from_f64(0.000_123_456).unwrap();

        let res: bitcoin::Amount = dai.worth_in(rate).unwrap();

        // Result is 0.012469056 btc or 1246905.6 satoshis
        let btc = bitcoin::Amount::from_btc(0.012_469_05).unwrap();
        assert_eq!(res, btc);
    }

    #[test]
    fn worth_in_result_truncated_2() {
        let dai = Amount::from_dai_trunc(100_001.0).unwrap();
        let rate = Rate::from_f64(0.000_001_234).unwrap();

        let res: bitcoin::Amount = dai.worth_in(rate).unwrap();

        // Result is 12,340,123.4 satoshis
        let btc = bitcoin::Amount::from_sat(12_340_123);
        assert_eq!(res, btc);
    }

    proptest! {
        #[test]
        fn doesnt_panic(f in any::<f64>()) {
               let _ = Amount::from_dai_trunc(f);
        }
    }

    proptest! {
        #[test]
        fn worth_in_bitcoin_doesnt_panic(s in "[0-9]+", r in any::< f64>()) {
            let uint = BigUint::from_str(&s);
            let rate = Rate::from_f64(r);
            if let (Ok(uint), Ok(rate)) = (uint, rate) {
                let amount = Amount::from_atto(uint);
                let _: anyhow::Result<bitcoin::Amount> = amount.worth_in(rate);
            }
        }
    }
}
