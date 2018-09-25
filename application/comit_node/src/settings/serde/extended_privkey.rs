use bitcoin_support::ExtendedPrivKey;
use serde::{de, export::fmt, Deserializer};
use std::str::FromStr;

pub fn deserialize<'de, D>(deserializer: D) -> Result<ExtendedPrivKey, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visitor;

    impl<'de> de::Visitor<'de> for Visitor {
        type Value = ExtendedPrivKey;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a base58-encoded bip32 extended private key")
        }

        fn visit_str<E>(self, value: &str) -> Result<ExtendedPrivKey, E>
        where
            E: de::Error,
        {
            ExtendedPrivKey::from_str(value).map_err(E::custom)
        }
    }

    deserializer.deserialize_str(Visitor)
}
