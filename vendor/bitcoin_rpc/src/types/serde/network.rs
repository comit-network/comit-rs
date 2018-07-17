use bitcoin::network::constants::Network;
use serde::{de, Deserializer};
use std::fmt;

pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<Network, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visitor;

    impl<'de> de::Visitor<'de> for Visitor {
        type Value = Network;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("Bitcoin network: `main`, `test` or `regtest`")
        }

        fn visit_str<E>(self, value: &str) -> Result<Network, E>
        where
            E: de::Error,
        {
            match value {
                "test" => Ok(Network::Testnet),
                "regtest" => Ok(Network::BitcoinCoreRegtest),
                "main" => Ok(Network::Bitcoin),
                _ => Err(E::custom(format!("Unexpect value for Network: {}", value))),
            }
        }
    }

    deserializer.deserialize_str(Visitor)
}
