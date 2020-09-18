use crate::ethereum::dai::*;
use serde::{de, Deserialize, Deserializer, Serializer};
use std::{convert::TryFrom, fmt};

pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Amount>, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visitor;

    impl<'de> de::Visitor<'de> for Visitor {
        type Value = Option<Amount>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("an amount in dai")
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
                    let amount = Amount::from_dai_trunc(value).map_err(|error| {
                        serde::de::Error::custom(format!(
                            "Could not deserialize dai amount: {:#}",
                            error
                        ))
                    })?;
                    Ok(Some(amount))
                }
                None => Ok(None),
            }
        }

        fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(Amount::from_dai_trunc(value).map_err(|error| {
                E::custom(format!("Could not deserialize dai amount: {:#}", error))
            })?))
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            let value = u32::try_from(value).map_err(|error| {
                E::custom(format!("Could not deserialize dai amount: {:#}", error))
            })?;
            let value = f64::try_from(value).map_err(|error| {
                E::custom(format!("Could not deserialize dai amount: {:#}", error))
            })?;
            Ok(Some(Amount::from_dai_trunc(value).map_err(|error| {
                E::custom(format!("Could not deserialize dai amount: {:#}", error))
            })?))
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            let value = i32::try_from(value).map_err(|error| {
                E::custom(format!("Could not deserialize dai amount: {:#}", error))
            })?;
            let value = f64::try_from(value).map_err(|error| {
                E::custom(format!("Could not deserialize dai amount: {:#}", error))
            })?;
            Ok(Some(Amount::from_dai_trunc(value as f64).map_err(
                |error| E::custom(format!("Could not deserialize dai amount: {:#}", error)),
            )?))
        }
    }

    deserializer.deserialize_any(Visitor)
}

pub fn serialize<S>(value: &Option<Amount>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(value) => serializer.serialize_f64(value.as_dai_rounded()),
        None => serializer.serialize_none(),
    }
}
