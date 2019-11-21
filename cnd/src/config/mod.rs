pub mod file;
mod serde_bitcoin_network;
mod serde_duration;
pub mod settings;

use libp2p::Multiaddr;
use serde::{Deserialize, Serialize};
use std::{net::IpAddr, path::PathBuf};

pub use self::{file::File, settings::Settings};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Database {
    pub sqlite: PathBuf,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Network {
    pub listen: Vec<Multiaddr>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Socket {
    pub address: IpAddr,
    pub port: u16,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Bitcoin {
    #[serde(with = "crate::config::serde_bitcoin_network")]
    pub network: bitcoin::Network,
    #[serde(with = "url_serde")]
    pub node_url: reqwest::Url,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Ethereum {
    #[serde(with = "url_serde")]
    pub node_url: reqwest::Url,
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::Url;

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

    #[test]
    fn bitcoin_deserializes_correctly() {
        let file_contents = vec![
            r#"
            network = "mainnet"
            node_url = "http://example.com:8545"
            "#,
            r#"
            network = "testnet"
            node_url = "http://example.com:8545"
            "#,
            r#"
            network = "regtest"
            node_url = "http://example.com:8545"
            "#,
        ];

        let expected = vec![
            Bitcoin {
                network: bitcoin::Network::Bitcoin,
                node_url: Url::parse("http://example.com:8545").unwrap(),
            },
            Bitcoin {
                network: bitcoin::Network::Testnet,
                node_url: Url::parse("http://example.com:8545").unwrap(),
            },
            Bitcoin {
                network: bitcoin::Network::Regtest,
                node_url: Url::parse("http://example.com:8545").unwrap(),
            },
        ];

        let actual = file_contents
            .into_iter()
            .map(toml::from_str)
            .collect::<Result<Vec<Bitcoin>, toml::de::Error>>()
            .unwrap();

        assert_eq!(actual, expected);
    }
}
