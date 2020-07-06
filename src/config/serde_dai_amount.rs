use crate::dai::*;
use serde::{Deserialize, Deserializer, Serializer};

pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Amount>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<f64> = Option::deserialize(deserializer)?;
    match s {
        Some(value) => {
            let amount = Amount::from_dai_trunc(value).map_err(|error| {
                serde::de::Error::custom(format!("Could not deserialize dai amount: {:?}", error))
            })?;
            Ok(Some(amount))
        }
        None => Ok(None),
    }
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
