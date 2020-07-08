pub mod file;
mod serde;
pub mod settings;
pub mod validation;

use crate::{bitcoin, dai, Spread};
use ::serde::{Deserialize, Serialize};
use comit::ethereum::ChainId;
use libp2p::Multiaddr;
use std::path::PathBuf;
use url::Url;

pub use self::{file::File, settings::Settings};
use crate::dai::DaiContractAddress;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Data {
    pub dir: PathBuf,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Network {
    pub listen: Vec<Multiaddr>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Bitcoin {
    #[serde(with = "crate::config::serde::bitcoin_network")]
    pub network: ::bitcoin::Network,
    pub bitcoind: Bitcoind,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Bitcoind {
    pub node_url: Url,
}

impl Default for Bitcoin {
    fn default() -> Self {
        Self {
            network: ::bitcoin::Network::Regtest,
            bitcoind: Bitcoind {
                node_url: Url::parse("http://localhost:18443")
                    .expect("static string to be a valid url"),
            },
        }
    }
}

impl From<Bitcoin> for file::Bitcoin {
    fn from(bitcoin: Bitcoin) -> Self {
        file::Bitcoin {
            network: bitcoin.network,
            bitcoind: Some(bitcoin.bitcoind),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Ethereum {
    pub chain_id: ChainId,
    pub node_url: Url,
    pub dai_contract_address: comit::ethereum::Address,
}

impl From<Ethereum> for file::Ethereum {
    fn from(ethereum: Ethereum) -> Self {
        let dai_contract_address = DaiContractAddress::from_public_chain_id(ethereum.chain_id);
        match dai_contract_address {
            None => file::Ethereum {
                chain_id: ethereum.chain_id,
                node_url: Some(ethereum.node_url),
                local_dai_contract_address: Some(ethereum.dai_contract_address),
            },
            Some(_) => file::Ethereum {
                chain_id: ethereum.chain_id,
                node_url: Some(ethereum.node_url),
                local_dai_contract_address: None,
            },
        }
    }
}

impl Default for Ethereum {
    fn default() -> Self {
        Self {
            chain_id: ChainId::mainnet(),
            node_url: Url::parse("http://localhost:8545").expect("static string to be a valid url"),
            dai_contract_address: DaiContractAddress::Mainnet.into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Maker {
    pub max_sell: MaxSell,
    pub spread: Spread,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MaxSell {
    #[serde(default)]
    #[serde(with = "crate::config::serde::bitcoin_amount")]
    pub bitcoin: Option<bitcoin::Amount>,
    #[serde(default)]
    #[serde(with = "crate::config::serde::dai_amount")]
    pub dai: Option<dai::Amount>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_deserializes_correctly() {
        let file_contents = vec![
            r#"
            listen = ["/ip4/0.0.0.0/tcp/9939"]
            "#,
            r#"
            listen = ["/ip4/0.0.0.0/tcp/9939", "/ip4/127.0.0.1/tcp/9939"]
            "#,
        ];

        let expected = vec![
            Network {
                listen: vec!["/ip4/0.0.0.0/tcp/9939".parse().unwrap()],
            },
            Network {
                listen: (vec![
                    "/ip4/0.0.0.0/tcp/9939".parse().unwrap(),
                    "/ip4/127.0.0.1/tcp/9939".parse().unwrap(),
                ]),
            },
        ];

        let actual = file_contents
            .into_iter()
            .map(toml::from_str)
            .collect::<Result<Vec<Network>, toml::de::Error>>()
            .unwrap();

        assert_eq!(actual, expected);
    }
}
