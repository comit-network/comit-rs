use crate::float_maths::string_int_to_float;
use comit::{asset::Bitcoin, Quantity};

pub const SATS_IN_BITCOIN_EXP: u16 = 8;

#[derive(Debug, Copy, Clone, Ord, PartialOrd, PartialEq, Eq, Default)]
pub struct Amount(::bitcoin::Amount);

impl Amount {
    pub const ZERO: Self = Self(::bitcoin::Amount::ZERO);

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

    pub fn checked_add(self, rhs: Amount) -> Option<Amount> {
        self.0.checked_add(rhs.0).map(Amount)
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

impl From<::bitcoin::Amount> for Amount {
    fn from(amount: ::bitcoin::Amount) -> Self {
        Amount { 0: amount }
    }
}

impl From<Amount> for ::bitcoin::Amount {
    fn from(from: Amount) -> Self {
        from.0
    }
}

impl From<Amount> for comit::asset::Bitcoin {
    fn from(from: Amount) -> Self {
        Self::from_sat(from.as_sat())
    }
}

impl From<comit::asset::Bitcoin> for Amount {
    fn from(from: comit::asset::Bitcoin) -> Self {
        Self(from.into())
    }
}

impl From<comit::Quantity<comit::asset::Bitcoin>> for Amount {
    fn from(from: comit::Quantity<comit::asset::Bitcoin>) -> Self {
        Self::from_sat(from.sats())
    }
}

impl Into<comit::Quantity<comit::asset::Bitcoin>> for Amount {
    fn into(self) -> Quantity<Bitcoin> {
        Quantity::new(comit::asset::Bitcoin::from_sat(self.as_sat()))
    }
}

impl std::fmt::Display for Amount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let bitcoin = string_int_to_float(self.as_sat().to_string(), SATS_IN_BITCOIN_EXP as usize);
        write!(f, "{} BTC", bitcoin)
    }
}

#[cfg(test)]
pub fn btc(btc: f64) -> Amount {
    Amount::from_btc(btc).unwrap()
}

#[cfg(test)]
pub fn some_btc(btc: f64) -> Option<Amount> {
    Some(Amount::from_btc(btc).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_bitcoin_displays_as_one_btc() {
        let bitcoin = Amount::from_sat(100_000_000);

        assert_eq!(bitcoin.to_string(), "1 BTC".to_string())
    }

    #[test]
    fn one_sat_displays_as_ten_nano_btc() {
        let bitcoin = Amount::from_sat(1);

        assert_eq!(bitcoin.to_string(), "0.00000001 BTC".to_string())
    }
}
