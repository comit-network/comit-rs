use bitcoin::{util::amount::Denomination, Amount};
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, str::FromStr};

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Bitcoin(Amount);

impl Bitcoin {
    pub fn from_sat(sat: u64) -> Bitcoin {
        Bitcoin(Amount::from_sat(sat))
    }

    pub fn as_sat(self) -> u64 {
        Amount::as_sat(self.0)
    }

    pub fn to_le_bytes(self) -> [u8; 8] {
        self.0.as_sat().to_le_bytes()
    }

    #[cfg(test)]
    pub fn meaningless_test_value() -> Self {
        Bitcoin::from_sat(1_000u64)
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

impl Serialize for Bitcoin {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let precision = 8;
        let base: u64 = 10;
        let remainder = self.as_sat() % base.pow(precision);
        let integer = (self.as_sat() - remainder) / base.pow(precision);
        let string = format!("{}.{}", integer, remainder);

        serializer.serialize_str(&string)
    }
}

impl<'de> Deserialize<'de> for Bitcoin {
    /// Decimal precision of 8 is selected because it is the largest possible
    /// precision for BTC
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        let precision = 8;
        let string = String::deserialize(deserializer)?;
        let v: Vec<&str> = string.as_str().split('.').collect();

        let integer = *v.first().unwrap();
        let decimals = *v.last().unwrap();
        if decimals.len() > precision {
            return Err(Error::custom(format!(
                "Bitcoin does not support a decimal precision of {}, expected {}",
                decimals.len(),
                precision
            )));
        }
        let trailing_zeros = precision - decimals.len();

        let zero_vec = vec!['0'; trailing_zeros];
        let zeros: String = zero_vec.into_iter().collect();

        let result = format!("{}{}{}", integer, decimals, &zeros);

        let sat = u64::from_str(&result).unwrap();

        Ok(Bitcoin::from_sat(sat))
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
    use spectral::*;

    #[test]
    fn display_bitcoin() {
        assert_eq!(
            Bitcoin::from_sat(900_000_000_000).to_string(),
            "9000.00000000 BTC"
        );
    }
    #[test]
    fn btc_serialization_success() {
        let expected = "\"0.12345678\"".to_string();
        let bitcoin = Bitcoin::from_sat(12345678);
        let actual = serde_json::to_string(&bitcoin).expect("failed to serialise bitcoin");
        assert_that(&actual).is_equal_to(expected);
    }
    #[test]
    fn btc_deserialization_fail_too_many_decimals() {
        let expected = "\"0.123456789\"".to_string();
        assert!(serde_json::from_str::<Bitcoin>(&expected).is_err());
    }
    #[test]
    fn btc_deserialization_success() {
        let expected = "\"0.12345678\"".to_string();
        assert!(serde_json::from_str::<Bitcoin>(&expected).is_ok());
    }
}
