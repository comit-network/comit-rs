use bip39::{Language, Mnemonic};
use serde::{de, export::fmt, Deserializer};
use std::str::FromStr;

pub fn deserialize<'de, D>(deserializer: D) -> Result<Mnemonic, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visitor;

    impl<'de> de::Visitor<'de> for Visitor {
        type Value = Mnemonic;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a base58-encoded bip32 extended private key")
        }

        fn visit_str<E>(self, raw_mnemonic: &str) -> Result<Mnemonic, E>
        where
            E: de::Error,
        {
            // No passphrase
            Mnemonic::from_string(raw_mnemonic, Language::English, "").map_err(E::custom)
        }
    }

    deserializer.deserialize_str(Visitor)
}
