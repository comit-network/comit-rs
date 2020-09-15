use crate::ethereum::ChainId;
use fmt::Display;
use serde::{Deserialize, Serialize};
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Ethereum {
    pub chain_id: ChainId,
}

impl Ethereum {
    pub fn new(chain: ChainId) -> Self {
        Ethereum { chain_id: chain }
    }
}

impl Display for Ethereum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let chain_id = u32::from(self.chain_id);
        let s = match chain_id {
            1 => "Mainnet",
            3 => "Ropsten",
            4 => "Rinkeby",
            5 => "Goerli",
            42 => "Kovan",
            _ => "Devnet",
        };

        write!(f, "{}", s)
    }
}

impl From<u32> for Ethereum {
    fn from(chain_id: u32) -> Self {
        Ethereum::new(chain_id.into())
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

    #[test]
    fn ethereum_serializes_as_expected() {
        let ledger = Ethereum::from(1);
        let want = r#"{"chain_id":1}"#.to_string();
        let got = serde_json::to_string(&ledger).expect("failed to serialize");

        assert_that(&got).is_equal_to(&want);
    }

    #[test]
    fn ethereum_serialization_roundtrip() {
        let ledger = Ethereum::from(1);
        let json = serde_json::to_string(&ledger).expect("failed to serialize");
        let rinsed: Ethereum = serde_json::from_str(&json).expect("failed to deserialize");

        assert_eq!(ledger, rinsed);
    }
}
