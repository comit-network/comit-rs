use crate::ethereum::Address;
use serde::{de, export::fmt, Deserialize, Deserializer, Serializer};
use std::str::FromStr;

pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Address>, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visitor;

    impl<'de> de::Visitor<'de> for Visitor {
        type Value = Option<Address>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("an ethereum address")
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
            let s: Option<&str> = Option::deserialize(deserializer)?;
            match s {
                Some(value) => Ok(Some(Address::from_str(value).map_err(|error| {
                    serde::de::Error::custom(format!(
                        "Could not deserialize ethereum address: {:#}",
                        error
                    ))
                })?)),
                None => Ok(None),
            }
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(Address::from_str(v).map_err(|error| {
                E::custom(format!(
                    "Could not deserialize ethereum address: {:#}",
                    error
                ))
            })?))
        }
    }

    deserializer.deserialize_any(Visitor)
}

pub fn serialize<S>(value: &Option<Address>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(value) => serializer.serialize_str(&value.to_string()),
        None => serializer.serialize_none(),
    }
}
