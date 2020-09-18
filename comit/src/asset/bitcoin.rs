pub use bitcoin::Amount as Bitcoin;

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
