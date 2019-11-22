use serde::{de, export::fmt, Deserializer, Serializer};

pub fn deserialize<'de, D>(deserializer: D) -> Result<bitcoin::Network, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visitor;

    impl<'de> de::Visitor<'de> for Visitor {
        type Value = bitcoin::Network;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a bitcoin network")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            match v {
                "mainnet" => Ok(bitcoin::Network::Bitcoin),
                "testnet" => Ok(bitcoin::Network::Testnet),
                "regtest" => Ok(bitcoin::Network::Regtest),
                unknown => Err(E::custom(format!("unknown bitcoin network {}", unknown))),
            }
        }
    }

    deserializer.deserialize_str(Visitor)
}

// This is the API serde expects, can't do much about the trivial copy :(
#[allow(clippy::trivially_copy_pass_by_ref)]
pub fn serialize<S: Serializer>(
    value: &bitcoin::Network,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(match value {
        bitcoin::Network::Bitcoin => "mainnet",
        bitcoin::Network::Testnet => "testnet",
        bitcoin::Network::Regtest => "regtest",
    })
}
