use serde::{de, Deserialize, Deserializer, Serialize};
use std::fmt;

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    strum_macros::Display,
    strum_macros::EnumString,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum Bitcoin {
    Mainnet,
    Testnet,
    Regtest,
}

impl From<Bitcoin> for ::bitcoin::Network {
    fn from(bitcoin: Bitcoin) -> ::bitcoin::Network {
        match bitcoin {
            Bitcoin::Mainnet => ::bitcoin::Network::Bitcoin,
            Bitcoin::Testnet => ::bitcoin::Network::Testnet,
            Bitcoin::Regtest => ::bitcoin::Network::Regtest,
        }
    }
}

impl From<::bitcoin::Network> for Bitcoin {
    fn from(network: ::bitcoin::Network) -> Self {
        match network {
            bitcoin::Network::Bitcoin => Bitcoin::Mainnet,
            bitcoin::Network::Testnet => Bitcoin::Testnet,
            bitcoin::Network::Regtest => Bitcoin::Regtest,
        }
    }
}

/// Module specifically desgined for use with the `serde(with)` attribute.
///
/// # Usage
///
/// ```rust
/// use comit::ledger;
///
/// #[derive(serde::Deserialize, PartialEq, Debug)]
/// #[serde(transparent)]
/// struct Container(#[serde(with = "ledger::bitcoin::bitcoind_jsonrpc_network")] ledger::Bitcoin);
///
/// let container = Container(ledger::Bitcoin::Mainnet);
/// let network = r#""main""#;
///
/// assert_eq!(
///     container,
///     serde_json::from_str::<Container>(network).unwrap()
/// )
/// ```
pub mod bitcoind_jsonrpc_network {
    use super::*;

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Bitcoin, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Bitcoin;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a bitcoin network")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match v {
                    "main" => Ok(Bitcoin::Mainnet),
                    "test" => Ok(Bitcoin::Testnet),
                    "regtest" => Ok(Bitcoin::Regtest),
                    unknown => Err(E::custom(format!("unknown bitcoin network {}", unknown))),
                }
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectral::prelude::*;

    #[test]
    fn bitcoin_serializes_as_expected() {
        let ledger = Bitcoin::Mainnet;
        let want = r#""mainnet""#.to_string();
        let got = serde_json::to_string(&ledger).expect("failed to serialize");

        assert_that(&got).is_equal_to(&want);
    }

    #[test]
    fn bitcoin_serialization_roundtrip() {
        let ledger = Bitcoin::Mainnet;
        let json = serde_json::to_string(&ledger).expect("failed to serialize");
        let rinsed: Bitcoin = serde_json::from_str(&json).expect("failed to deserialize");

        assert_eq!(ledger, rinsed);
    }
}
