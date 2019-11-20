use serde::{de, export::fmt, Deserializer, Serialize, Serializer};

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

// reference: serde_url crate.

/// Serialises `value` with a given serializer.
// We need this in order to use
// `#[serde(with = "super::serde_bitcoin_network")]`
pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    for<'a> Ser<'a, T>: Serialize,
{
    Ser::new(value).serialize(serializer)
}

// A wrapper so we can implement custom serialize of inner type.
#[derive(Debug)]
pub struct Ser<'a, T>(&'a T);

impl<'a, T> Ser<'a, T>
where
    Ser<'a, T>: Serialize,
{
    #[inline(always)]
    pub fn new(value: &'a T) -> Self {
        Ser(value)
    }
}

/// Does the actual serialization of the bitcoin Network into a `serde` stream.
impl<'a> Serialize for Ser<'a, bitcoin::Network> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Ser(bitcoin::network::constants::Network::Bitcoin) => {
                serializer.serialize_str("mainnet")
            }
            Ser(bitcoin::network::constants::Network::Testnet) => {
                serializer.serialize_str("testnet")
            }
            Ser(bitcoin::network::constants::Network::Regtest) => {
                serializer.serialize_str("regtest")
            }
        }
    }
}
