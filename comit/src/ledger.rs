use crate::ethereum::ChainId;
use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Bitcoin {
    Mainnet,
    Testnet,
    Regtest,
}

impl Default for Bitcoin {
    fn default() -> Self {
        Self::Regtest
    }
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

impl Serialize for Bitcoin {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let str = match self {
            Bitcoin::Mainnet => "mainnet",
            Bitcoin::Testnet => "testnet",
            Bitcoin::Regtest => "regtest",
        };

        serializer.serialize_str(str)
    }
}

impl<'de> Deserialize<'de> for Bitcoin {
    fn deserialize<D>(deserializer: D) -> Result<Bitcoin, D::Error>
    where
        D: Deserializer<'de>,
    {
        let network = match String::deserialize(deserializer)?.as_str() {
            "mainnet" => Bitcoin::Mainnet,
            "testnet" => Bitcoin::Testnet,
            "regtest" => Bitcoin::Regtest,

            network => {
                return Err(<D as Deserializer<'de>>::Error::custom(format!(
                    "not regtest: {}",
                    network
                )))
            }
        };

        Ok(network)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Ethereum {
    pub chain_id: ChainId,
}

impl Ethereum {
    pub fn new(chain: ChainId) -> Self {
        Ethereum { chain_id: chain }
    }
}

impl From<u32> for Ethereum {
    fn from(chain_id: u32) -> Self {
        Ethereum::new(chain_id.into())
    }
}

impl Default for Ethereum {
    fn default() -> Self {
        Ethereum {
            chain_id: ChainId::regtest(),
        }
    }
}
