use crate::dai;
use crate::dai::ATTOS_IN_DAI_EXP;
use crate::float_maths::divide_pow_ten_trunc;
use crate::publish::WorthIn;
use crate::rate::Rate;
use anyhow::anyhow;
use bitcoin::hashes::core::cmp::Ordering;
use num::pow::Pow;
use num::BigUint;
use std::convert::TryFrom;

pub const SATS_IN_BITCOIN_EXP: u16 = 8;

#[derive(Debug, Copy, Clone, Ord, PartialOrd, PartialEq, Eq)]
pub struct Amount(::bitcoin::Amount);

impl Amount {
    pub fn from_btc(btc: f64) -> anyhow::Result<Amount> {
        Ok(Amount(::bitcoin::Amount::from_btc(btc)?))
    }

    pub fn from_sat(sat: u64) -> Self {
        Amount(::bitcoin::Amount::from_sat(sat))
    }

    pub fn as_sat(self) -> u64 {
        self.0.as_sat()
    }

    pub fn as_btc(self) -> f64 {
        self.0.as_btc()
    }
}

impl WorthIn<dai::Amount> for Amount {
    fn worth_in(&self, btc_to_dai_rate: Rate) -> anyhow::Result<dai::Amount> {
        // Get the integer part of the rate
        let uint_rate = BigUint::from(btc_to_dai_rate.integer());

        // Apply the rate
        let worth = uint_rate * self.as_sat();

        // The rate input is for bitcoin to dai but we applied to satoshis so we need to:
        // - divide to get bitcoins
        // - divide to adjust for rate (we used integer part only).
        // - multiple to get attodai
        let sats_in_bitcoin = i32::from(SATS_IN_BITCOIN_EXP);
        let rate_exp = i32::try_from(btc_to_dai_rate.inverse_decimal_exponent())
            .map_err(|_| anyhow!("Exponent is unexpectedly large."))?;
        let attos_in_dai = i32::from(ATTOS_IN_DAI_EXP);
        let adjustment_exp = -sats_in_bitcoin - rate_exp + attos_in_dai;

        let atto_dai = match adjustment_exp.cmp(&0) {
            Ordering::Less => {
                let inv_exp = usize::try_from(adjustment_exp.abs())
                    .map_err(|_| anyhow!("Exponent is unexpectedly large."))?;
                divide_pow_ten_trunc(worth, inv_exp)
            }
            Ordering::Equal => worth,
            Ordering::Greater => {
                let exp = u16::try_from(adjustment_exp)
                    .map_err(|_| anyhow!("Exponent is unexpectedly large."))?;
                worth * BigUint::from(10u16).pow(BigUint::from(exp))
            }
        };

        Ok(dai::Amount::from_atto(atto_dai))
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

    #[test]
    fn using_rate_returns_correct_result() {
        let btc = Amount::from_btc(1.0).unwrap();

        let res: dai::Amount = btc.worth_in(Rate::from_f64(1000.1234).unwrap()).unwrap();

        assert_eq!(res, dai::Amount::from_dai_trunc(1000.1234).unwrap());
    }

    proptest! {
        #[test]
        fn worth_in_dai_doesnt_panic(u in any::<u64>(), r in any::<f64>()) {
            let amount = Amount::from_sat(u);
            let rate = Rate::from_f64(r);
            if let Ok(rate) = rate {
                let _: anyhow::Result<dai::Amount> = amount.worth_in(rate);
            }
        }
    }
}
