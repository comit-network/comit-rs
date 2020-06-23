use crate::dai;
use crate::dai::ATTOS_IN_DAI_EXP;
use crate::publish::WorthIn;
use crate::rate::Rate;
use num::pow::Pow;
use num::BigUint;

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

// The rate input is for bitcoin to dai but we applied to satoshis so we need to:
// - divide to get bitcoins (8)
// - divide to adjust for rate (9)
// - multiply to get attodai (18)
// = 1
const ADJUSTEMENT_EXP: u16 = ATTOS_IN_DAI_EXP - SATS_IN_BITCOIN_EXP - Rate::PRECISION;

impl WorthIn<dai::Amount> for Amount {
    fn worth_in(&self, btc_to_dai_rate: Rate) -> anyhow::Result<dai::Amount> {
        // Get the integer part of the rate
        let uint_rate = btc_to_dai_rate.integer();

        // Apply the rate
        let worth = uint_rate * self.as_sat();

        let atto_dai = worth * BigUint::from(10u16).pow(BigUint::from(ADJUSTEMENT_EXP));

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
    use std::convert::TryFrom;

    #[test]
    fn worth_in_1() {
        let btc = Amount::from_btc(1.0).unwrap();
        let rate = Rate::try_from(1_000.123_4).unwrap();

        let res: dai::Amount = btc.worth_in(rate).unwrap();

        let dai = dai::Amount::from_dai_trunc(1_000.123_4).unwrap();
        assert_eq!(res, dai);
    }

    #[test]
    fn worth_in_2() {
        let btc = Amount::from_btc(0.345_678_9).unwrap();
        let rate = Rate::try_from(9_123.456_7).unwrap();

        let res: dai::Amount = btc.worth_in(rate).unwrap();

        let dai = dai::Amount::from_dai_trunc(3_153.786_476_253_63).unwrap();
        assert_eq!(res, dai);
    }

    #[test]
    fn worth_in_3() {
        let btc = Amount::from_btc(0.010_7).unwrap();
        let rate = Rate::try_from(9_355.38).unwrap();

        let res: dai::Amount = btc.worth_in(rate).unwrap();

        let dai = dai::Amount::from_dai_trunc(100.102_566).unwrap();
        assert_eq!(res, dai);
    }

    #[test]
    fn worth_in_4() {
        let btc = Amount::from_btc(9999.0).unwrap();
        let rate = Rate::try_from(10.0).unwrap();

        let res: dai::Amount = btc.worth_in(rate).unwrap();

        let dai = dai::Amount::from_dai_trunc(99990.0).unwrap();
        assert_eq!(res, dai);
    }

    proptest! {
        #[test]
        fn worth_in_dai_doesnt_panic(u in any::<u64>(), r in any::<f64>()) {
            let amount = Amount::from_sat(u);
            let rate = Rate::try_from(r);
            if let Ok(rate) = rate {
                let _: anyhow::Result<dai::Amount> = amount.worth_in(rate);
            }
        }
    }
}
