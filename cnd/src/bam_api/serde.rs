use bitcoin_support::Amount;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

#[derive(Clone, Copy, Debug)]
pub struct Serde<T>(pub T);

impl Serialize for Serde<Amount> {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{}", self.0.as_sat()))
    }
}

impl<'de> Deserialize<'de> for Serde<Amount> {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'vde> de::Visitor<'vde> for Visitor {
            type Value = Serde<Amount>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
                formatter.write_str("a Bitcoin amount in Satoshi")
            }

            fn visit_str<E>(self, v: &str) -> Result<Serde<Amount>, E>
            where
                E: de::Error,
            {
                let int: u64 = v.parse().map_err(|_| {
                    de::Error::invalid_value(de::Unexpected::Str(v), &"integer Satoshi amount")
                })?;
                Ok(Serde(bitcoin_support::Amount::from_sat(int)))
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

impl From<bitcoin_support::Amount> for Serde<Amount> {
    fn from(amount: bitcoin_support::Amount) -> Self {
        Serde(amount)
    }
}

impl From<Serde<Amount>> for bitcoin_support::Amount {
    fn from(serde_amount: Serde<Amount>) -> Self {
        serde_amount.0
    }
}
