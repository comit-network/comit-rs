use secp256k1_support::KeyPair;
use serde::{de, export::fmt, Deserializer};

pub fn deserialize<'de, D>(deserializer: D) -> Result<KeyPair, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visitor;

    impl<'de> de::Visitor<'de> for Visitor {
        type Value = KeyPair;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a hex-encoded Secp256k1 private key")
        }

        fn visit_str<E>(self, value: &str) -> Result<KeyPair, E>
        where
            E: de::Error,
        {
            KeyPair::from_secret_key_hex(value).map_err(E::custom)
        }
    }

    deserializer.deserialize_str(Visitor)
}
