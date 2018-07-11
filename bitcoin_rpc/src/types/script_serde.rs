use bitcoin::blockdata::script::Script as BitcoinScript;
use serde::Deserializer;
use serde::Serializer;
use serde::de;
use serde::export::fmt;
use std_hex;

pub fn deserialize<'de, D>(deserializer: D) -> Result<BitcoinScript, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visitor;

    impl<'de> de::Visitor<'de> for Visitor {
        type Value = BitcoinScript;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("Bitcoin script in hex format")
        }

        fn visit_str<E>(self, value: &str) -> Result<BitcoinScript, E>
        where
            E: de::Error,
        {
            let hex = std_hex::decode(value)
                .map_err(|err| E::custom(format!("Could not decode hex: {}", err)))?;
            Ok(BitcoinScript::from(hex))
        }
    }

    deserializer.deserialize_str(Visitor)
}

pub fn serialize<S>(script: &BitcoinScript, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(format!("{:x}", script).as_str())
}
