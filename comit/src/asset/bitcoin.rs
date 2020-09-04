use bitcoin::{util::amount::Denomination, Amount};
use std::{
    fmt,
    ops::{AddAssign, Sub, SubAssign},
};

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash, Default)]
pub struct Bitcoin(Amount);

impl Bitcoin {
    pub const ZERO: Bitcoin = Bitcoin(Amount::ZERO);
    pub const ONE: Bitcoin = Bitcoin(Amount::ONE_BTC);

    pub fn from_sat(sat: u64) -> Bitcoin {
        Bitcoin(Amount::from_sat(sat))
    }

    pub fn as_sat(self) -> u64 {
        Amount::as_sat(self.0)
    }

    pub fn to_le_bytes(self) -> [u8; 8] {
        self.0.as_sat().to_le_bytes()
    }
}

impl From<Bitcoin> for Amount {
    fn from(bitcoin: Bitcoin) -> Self {
        Amount::from_sat(bitcoin.as_sat())
    }
}

impl fmt::Display for Bitcoin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        let bitcoin = self.0.to_string_in(Denomination::Bitcoin);
        write!(f, "{} BTC", bitcoin)
    }
}

impl AddAssign for Bitcoin {
    fn add_assign(&mut self, rhs: Self) {
        self.0.add_assign(rhs.0);
    }
}

impl SubAssign for Bitcoin {
    fn sub_assign(&mut self, rhs: Self) {
        self.0.sub_assign(rhs.0);
    }
}

impl Sub for Bitcoin {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0.sub(rhs.0))
    }
}

/// Module specifically desgined for use with the `serde(with)` attribute.
///
/// # Usage
///
/// ```rust
/// use comit::asset;
///
/// #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
/// #[serde(transparent)]
/// struct Container(#[serde(with = "asset::bitcoin::sats_as_string")] asset::Bitcoin);
///
/// let container = Container(asset::Bitcoin::from_sat(1000));
/// let json_sats = r#""1000""#;
///
/// assert_eq!(json_sats, serde_json::to_string(&container).unwrap());
/// assert_eq!(
///     container,
///     serde_json::from_str::<Container>(json_sats).unwrap()
/// )
/// ```
pub mod sats_as_string {
    use super::*;
    use serde::{de::Error, Deserialize, Deserializer, Serializer};
    use std::str::FromStr;

    pub fn serialize<S>(value: &Bitcoin, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.as_sat().to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Bitcoin, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        let value =
            u64::from_str(value.as_str()).map_err(<D as Deserializer<'de>>::Error::custom)?;
        let amount = Bitcoin::from_sat(value);

        Ok(amount)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_bitcoin() {
        assert_eq!(
            Bitcoin::from_sat(900_000_000_000).to_string(),
            "9000.00000000 BTC"
        );
    }
}
