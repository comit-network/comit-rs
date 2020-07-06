use crate::bitcoin::*;
use serde::{Deserialize, Deserializer, Serializer};

// pub fn deserialize<'de, D>(deserializer: D) -> Result<Amount, D::Error>
// where
//     D: Deserializer<'de>,
// {
//     struct Visitor;
//
//     impl<'de> de::Visitor<'de> for Visitor {
//         type Value = Amount;
//
//         fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
//             formatter.write_str("an amount in bitcoin")
//         }
//
//         fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
//         where
//             E: de::Error,
//         {
//             Amount::from_btc(value).map_err(|error| {
//                 E::custom(format!("Could not deserialize bitcoin amount: {:?}", error))
//             })
//         }
//     }
//
//     deserializer.deserialize_f64(Visitor)
// }

pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Amount>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<f64> = Option::deserialize(deserializer)?;
    match s {
        Some(value) => {
            let amount = Amount::from_btc(value).map_err(|error| {
                serde::de::Error::custom(format!(
                    "Could not deserialize bitcoin amount: {:?}",
                    error
                ))
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
        Some(value) => serializer.serialize_f64(value.as_btc()),
        None => serializer.serialize_none(),
    }
}
