use crate::float_maths::string_int_to_float;
use crate::{
    bitcoin::{self, SATS_IN_BITCOIN_EXP},
    float_maths::{divide_pow_ten_trunc, multiply_pow_ten, truncate},
    Rate,
};
use comit::{
    asset::{ethereum::FromWei, Erc20, Erc20Quantity},
    ethereum::{Address, ChainId},
};
use conquer_once::Lazy;
use ethereum_types::U256;
use num::{pow::Pow, BigUint, CheckedAdd, ToPrimitive, Zero};
use std::str::FromStr;

pub const ATTOS_IN_DAI_EXP: u16 = 18;
pub static DAI_DEC: Lazy<BigUint> = Lazy::new(|| BigUint::from(10u16).pow(ATTOS_IN_DAI_EXP));

/// As per https://github.com/makerdao/developerguides/blob/804bb1f4d1ea737f0287cbf6480a570b888dd547/dai/dai-token/dai-token.md
/// Dai Version 1.0.8
static DAI_CONTRACT_ADDRESS_MAINNET: Lazy<Address> = Lazy::new(|| {
    "0x6B175474E89094C44Da98b954EedeAC495271d0F"
        .parse()
        .unwrap()
});
/// Dai Version 1.0.8
static DAI_CONTRACT_ADDRESS_KOVAN: Lazy<Address> = Lazy::new(|| {
    "0x4f96fe3b7a6cf9725f59d353f723c1bdb64ca6aa"
        .parse()
        .unwrap()
});
/// Dai Version 1.0.4
static DAI_CONTRACT_ADDRESS_RINKEBY: Lazy<Address> = Lazy::new(|| {
    "0x6A9865aDE2B6207dAAC49f8bCba9705dEB0B0e6D"
        .parse()
        .unwrap()
});
/// Dai Version 1.0.4
static DAI_CONTRACT_ADDRESS_ROPSTEN: Lazy<Address> = Lazy::new(|| {
    "0x31F42841c2db5173425b5223809CF3A38FEde360"
        .parse()
        .unwrap()
});

// TODO: There is duplicated information between `contract_address`
// and `chain_id` that can be avoided.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asset {
    pub amount: Amount,
    pub contract_address: DaiContractAddress,
    pub chain_id: ChainId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DaiContractAddress {
    Mainnet,
    Kovan,
    Rinkeby,
    Ropsten,
    Local(Address),
}

impl From<DaiContractAddress> for Address {
    fn from(var: DaiContractAddress) -> Self {
        match var {
            DaiContractAddress::Mainnet => *DAI_CONTRACT_ADDRESS_MAINNET,
            DaiContractAddress::Kovan => *DAI_CONTRACT_ADDRESS_KOVAN,
            DaiContractAddress::Rinkeby => *DAI_CONTRACT_ADDRESS_RINKEBY,
            DaiContractAddress::Ropsten => *DAI_CONTRACT_ADDRESS_ROPSTEN,
            DaiContractAddress::Local(address) => address,
        }
    }
}

impl DaiContractAddress {
    // TODO: What if we pass the token contract address of, for
    // example, the Mainnet token contract. I think this should be
    // renamed, together with the Local variant of DaiContractAddress
    pub fn local(dai_contract_address: Address) -> Self {
        Self::Local(dai_contract_address)
    }

    pub fn from_public_chain_id(chain_id: ChainId) -> Option<Self> {
        use DaiContractAddress::*;
        match chain_id.into() {
            1 => Some(Mainnet),
            3 => Some(Ropsten),
            4 => Some(Rinkeby),
            42 => Some(Kovan),
            _ => None,
        }
    }
}

#[derive(Clone, Ord, PartialOrd, PartialEq, Eq, Default)]
pub struct Amount(BigUint);

impl Amount {
    pub fn zero() -> Self {
        Self(BigUint::zero())
    }

    // The rate input is for dai to bitcoin but we applied it to attodai so we need to:
    // - divide to get dai (18)
    // - divide to adjust for rate (9)
    // - multiply to get satoshis (8)
    // = - 19
    const ADJUSTEMENT_EXP: i32 =
        SATS_IN_BITCOIN_EXP as i32 - ATTOS_IN_DAI_EXP as i32 - Rate::PRECISION as i32;

    /// Rounds the value received to a 9 digits mantissa.
    pub fn from_dai_trunc(dai: f64) -> anyhow::Result<Self> {
        if dai.is_sign_negative() {
            anyhow::bail!("Passed value is negative")
        }

        if !dai.is_finite() {
            anyhow::bail!("Passed value is not finite")
        }

        let dai = truncate(dai, ATTOS_IN_DAI_EXP);

        let u_int_value =
            multiply_pow_ten(&dai.to_string(), ATTOS_IN_DAI_EXP).expect("It is truncated");

        Ok(Amount(u_int_value))
    }

    /// Rounds to 2 digits after decimal point
    pub fn as_dai_rounded(&self) -> f64 {
        let mut str = self.0.to_string();
        let precision: usize = 2;

        let truncate: usize = ATTOS_IN_DAI_EXP as usize - precision;
        if str.len() > truncate {
            str.truncate(str.len() - truncate);
            let str = match str.len() {
                1 => {
                    let mut prefix = String::from("0.0");
                    prefix.push_str(&str);
                    prefix
                }
                2 => {
                    let mut prefix = String::from("0.");
                    prefix.push_str(&str);
                    prefix
                }
                _ => {
                    str.insert(str.len() - precision, '.');
                    str
                }
            };
            f64::from_str(&str).expect("float")
        } else {
            0.0
        }
    }

    pub fn from_atto(atto: BigUint) -> Self {
        Amount(atto)
    }

    pub fn as_atto(&self) -> BigUint {
        self.0.clone()
    }

    /// Allow to know the worth of self in bitcoin asset using the given conversion rate.
    /// Truncation may be done during the conversion to allow a result in satoshi
    pub fn worth_in(&self, dai_to_btc_rate: Rate) -> anyhow::Result<bitcoin::Amount> {
        // Get the integer part of the rate
        let uint_rate = dai_to_btc_rate.integer();

        // Apply the rate
        let worth = uint_rate * self.as_atto();

        let inv_exp = Self::ADJUSTEMENT_EXP.abs() as usize;
        let sats = divide_pow_ten_trunc(worth, inv_exp)
            .to_u64()
            .ok_or_else(|| anyhow::anyhow!("Result is unexpectedly large"))?;

        Ok(bitcoin::Amount::from_sat(sats))
    }

    pub fn checked_add(self, rhs: Amount) -> Option<Amount> {
        self.0.checked_add(&rhs.0).map(Amount)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes_le()
    }
}

impl std::fmt::Debug for Amount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::Display for Amount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = self.as_atto().to_string();
        let dai = string_int_to_float(str, ATTOS_IN_DAI_EXP as usize);

        write!(f, "{} DAI", dai)
    }
}

impl std::ops::Add for Amount {
    type Output = Amount;
    fn add(self, rhs: Self) -> Self::Output {
        Amount(self.0 + rhs.0)
    }
}

impl std::ops::Sub for Amount {
    type Output = Amount;

    fn sub(self, rhs: Self) -> Self::Output {
        Amount(self.0 - rhs.0)
    }
}

impl From<Erc20> for Amount {
    fn from(erc20: Erc20) -> Self {
        erc20.quantity.into()
    }
}

impl From<Erc20Quantity> for Amount {
    fn from(erc20_quantity: Erc20Quantity) -> Self {
        let quantity = BigUint::from_bytes_le(erc20_quantity.to_bytes().as_slice());
        Amount(quantity)
    }
}

impl From<Amount> for Erc20Quantity {
    fn from(amount: Amount) -> Self {
        let buf = amount.0.to_bytes_be();
        let wei = U256::from_big_endian(&buf);

        Self::from_wei(wei)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::convert::TryFrom;
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
        let rate = Rate::try_from(0.001_234).unwrap();

        let res: bitcoin::Amount = dai.worth_in(rate).unwrap();

        let btc = bitcoin::Amount::from_btc(0.001_234).unwrap();
        assert_eq!(res, btc);
    }

    #[test]
    fn worth_in_result_truncated_1() {
        let dai = Amount::from_dai_trunc(101.0).unwrap();
        let rate = Rate::try_from(0.000_123_456).unwrap();

        let res: bitcoin::Amount = dai.worth_in(rate).unwrap();

        // Result is 0.012469056 btc or 1246905.6 satoshis
        let btc = bitcoin::Amount::from_btc(0.012_469_05).unwrap();
        assert_eq!(res, btc);
    }

    #[test]
    fn worth_in_result_truncated_2() {
        let dai = Amount::from_dai_trunc(100_001.0).unwrap();
        let rate = Rate::try_from(0.000_001_234).unwrap();

        let res: bitcoin::Amount = dai.worth_in(rate).unwrap();

        // Result is 12,340,123.4 satoshis
        let btc = bitcoin::Amount::from_sat(12_340_123);
        assert_eq!(res, btc);
    }

    #[test]
    fn given_amount_has_2_digits_after_decimal_return_same_amount() {
        let dai = Amount::from_dai_trunc(1.23).unwrap();
        let dai = dai.as_dai_rounded();

        assert!(dai - 1.23 < 1e-10)
    }

    #[test]
    fn given_amount_has_3_digits_after_decimal_return_rounded_down_amount() {
        let dai = Amount::from_dai_trunc(1.234).unwrap();
        let dai = dai.as_dai_rounded();

        assert!(dai - 1.23 < 1e-10)
    }

    #[test]
    fn given_amount_has_3_digits_after_decimal_return_rounded_up_amount() {
        let dai = Amount::from_dai_trunc(1.235).unwrap();
        let dai = dai.as_dai_rounded();

        assert!(dai - 1.24 < 1e-10)
    }

    #[test]
    fn given_amount_is_less_than_milli_dai_return_0() {
        let dai = Amount::from_dai_trunc(0.001).unwrap();
        let dai = dai.as_dai_rounded();

        assert!(dai - 0.0 < 1e-10)
    }

    #[test]
    fn given_amount_is_centi_dai_return_centi_dai() {
        let dai = Amount::from_dai_trunc(0.1).unwrap();
        let dai = dai.as_dai_rounded();

        assert!(dai - 0.1 < 1e-10)
    }

    #[test]
    fn given_amount_is_deci_dai_return_deci_dai() {
        let dai = Amount::from_dai_trunc(0.01).unwrap();
        let dai = dai.as_dai_rounded();

        assert!(dai - 0.01 < 1e-10)
    }

    #[test]
    fn given_amount_is_one_atto_dai_as_dai_returns_one_atto_dai() {
        let dai = Amount::from_atto(1u64.into());

        assert_eq!(dai.to_string(), "0.000000000000000001 DAI".to_string())
    }

    #[test]
    fn given_amount_is_one_tenth_of_a_dai_as_dai_returns_one_tenth_of_a_dai() {
        let dai = Amount::from_dai_trunc(0.1).unwrap();

        assert_eq!(dai.to_string(), "0.1 DAI".to_string())
    }

    #[test]
    fn given_amount_is_one_dai_as_dai_returns_one_dai() {
        let dai = Amount::from_dai_trunc(1.0).unwrap();

        assert_eq!(dai.to_string(), "1 DAI".to_string())
    }

    #[test]
    fn given_amount_is_ten_dai_as_dai_returns_ten_dai() {
        let dai = Amount::from_dai_trunc(10.0).unwrap();

        assert_eq!(dai.to_string(), "10 DAI".to_string())
    }

    proptest! {
        #[test]
        fn as_dai_rounded_return_2_digits_or_less_after_decimal(s in "[0-9]+") {
            let uint = BigUint::from_str(&s).unwrap();
            let dai = Amount::from_atto(uint);
            let dai = dai.as_dai_rounded();
            let dai = dai.to_string();
            let decimal_index = dai.find('.');

            // If there is no decimal point then the test pass
            if let Some(decimal_index) = decimal_index {
                // Decimal needs to be within 2 digit of the last char (len - 1)
                assert!(decimal_index >= dai.len() - 1 - 2)
            }
        }
    }

    proptest! {
        #[test]
        fn as_dai_rounded_doesnt_panic(s in "[0-9]+") {
            let uint = BigUint::from_str(&s).unwrap();
            let dai = Amount::from_atto(uint);
            let _ = dai.as_dai_rounded();
        }
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
            let rate = Rate::try_from(r);
            if let (Ok(uint), Ok(rate)) = (uint, rate) {
                let amount = Amount::from_atto(uint);
                let _: anyhow::Result<bitcoin::Amount> = amount.worth_in(rate);
            }
        }
    }
}
