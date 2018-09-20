use keypair::KeyPair;
use serde::{de, export::fmt, Deserializer, Serializer};

pub fn deserialize<'de, D>(deserializer: D) -> Result<KeyPair, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visitor;

    impl<'de> de::Visitor<'de> for Visitor {
        type Value = KeyPair;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("KeyPair in hex format")
        }

        fn visit_str<E>(self, value: &str) -> Result<KeyPair, E>
        where
            E: de::Error,
        {
            KeyPair::from_secret_key_hex(value)
                .map_err(|err| E::custom(format!("Could not decode keypair: {:?}", err)))
        }
    }

    deserializer.deserialize_str(Visitor)
}

pub fn serialize<S>(key_pair: &KeyPair, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(format!("{:?}", key_pair).as_str())
}

//TODO write tests
