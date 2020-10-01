use crate::bitcoin::*;
use serde::{de, Deserialize, Deserializer, Serializer};
use std::{convert::TryFrom, fmt};

pub mod btc_as_optional_float {
    use super::*;

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Amount>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Option<Amount>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an amount in bitcoin")
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(None)
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                let s: Option<f64> = Option::deserialize(deserializer)?;
                match s {
                    Some(value) => {
                        let amount = Amount::from_btc(value).map_err(serde::de::Error::custom)?;
                        Ok(Some(amount))
                    }
                    None => Ok(None),
                }
            }

            fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Some(Amount::from_btc(value).map_err(E::custom)?))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let value = u32::try_from(value).map_err(E::custom)?;
                let value = f64::try_from(value).map_err(E::custom)?;
                Ok(Some(Amount::from_btc(value).map_err(E::custom)?))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let value = i32::try_from(value).map_err(E::custom)?;
                let value = f64::try_from(value).map_err(E::custom)?;
                Ok(Some(Amount::from_btc(value).map_err(E::custom)?))
            }
        }

        deserializer.deserialize_any(Visitor)
    }

    pub fn serialize<S>(value: &Option<Amount>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(value) => serializer.serialize_f64(value.as_btc()),
            None => serializer.serialize_none(),
        }
    }
}

pub mod sat_as_optional_unsigned_int {
    use super::*;

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Amount>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Option<Amount>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an amount in satoshi")
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(None)
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                let s: Option<u64> = Option::deserialize(deserializer)?;
                match s {
                    Some(value) => {
                        let amount = Amount::from_sat(value);
                        Ok(Some(amount))
                    }
                    None => Ok(None),
                }
            }

            fn visit_u32<E>(self, value: u32) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Some(Amount::from_sat(value as u64)))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let value = i32::try_from(value).map_err(E::custom)?;
                let value = u64::try_from(value).map_err(E::custom)?;
                Ok(Some(Amount::from_sat(value)))
            }
        }

        deserializer.deserialize_any(Visitor)
    }

    pub fn serialize<S>(value: &Option<Amount>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(value) => serializer.serialize_u64(value.as_sat()),
            None => serializer.serialize_none(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Serialize, Deserialize)]
    struct BtcOption {
        #[serde(with = "crate::config::serde::bitcoin_amount::btc_as_optional_float")]
        pub amount: Option<Amount>,
    }

    #[test]
    fn deserialize_btc_float_option_some() {
        let str = "amount = 1.12345678";

        let value: BtcOption = toml::from_str(str).unwrap();

        assert_eq!(value.amount.unwrap(), Amount::from_sat(112345678));
    }

    #[test]
    fn serialize_btc_float_option_some() {
        let value = BtcOption {
            amount: Some(Amount::from_sat(112345678)),
        };

        let str = toml::to_string(&value).unwrap();

        assert_eq!(str, "amount = 1.12345678\n".to_string());
    }

    #[test]
    fn serialize_btc_float_option_none() {
        let value = BtcOption { amount: None };

        let str = toml::to_string(&value).unwrap();

        assert_eq!(str, "".to_string());
    }
}
