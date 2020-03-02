pub mod file;
mod serde_bitcoin_network;
pub mod settings;

use crate::swap_protocols::ledger::ethereum;
use libp2p::Multiaddr;
use serde::{Deserialize, Serialize};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
};

pub use self::{file::File, settings::Settings};
use reqwest::Url;

lazy_static::lazy_static! {
    pub static ref LND_SOCKET: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
}

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
    #[serde(with = "crate::config::serde_bitcoin_network")]
    pub network: bitcoin::Network,
    pub bitcoind: Bitcoind,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Bitcoind {
    pub node_url: reqwest::Url,
}

impl Default for Bitcoin {
    fn default() -> Self {
        Self {
            network: bitcoin::Network::Regtest,
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
    pub chain_id: ethereum::ChainId,
    pub parity: Parity,
}

impl From<Ethereum> for file::Ethereum {
    fn from(ethereum: Ethereum) -> Self {
        file::Ethereum {
            chain_id: ethereum.chain_id,
            parity: Some(ethereum.parity),
        }
    }
}

impl Default for Ethereum {
    fn default() -> Self {
        Self {
            chain_id: ethereum::ChainId::regtest(),
            parity: Parity {
                node_url: Url::parse("http://localhost:8545")
                    .expect("static string to be a valid url"),
            },
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Parity {
    pub node_url: reqwest::Url,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Lightning {
    pub network: bitcoin::Network,
    pub lnd: Option<Lnd>,
}

impl Default for Lightning {
    fn default() -> Self {
        Self {
            network: bitcoin::Network::Regtest,
            lnd: Some(Lnd::default()),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Lnd {
    pub rest_api_socket: Option<SocketAddr>,
    pub dir: Option<PathBuf>,
}

impl Default for Lnd {
    fn default() -> Self {
        Self {
            rest_api_socket: Some(*LND_SOCKET),
            dir: Some(default_lnd_dir()),
        }
    }
}

fn default_lnd_dir() -> PathBuf {
    // Only fails if we don't have a home directory.
    crate::lnd_dir().unwrap()
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

    #[test]
    fn lnd_deserializes_correctly() {
        let file_contents = vec![
            r#"
            rest_api_socket = "127.0.0.1:8080"
            dir = "~/.local/share/comit/lnd"
            "#,
            r#"
            rest_api_socket = "127.0.0.1:8080"
            "#,
            r#"
            dir = "~/.local/share/comit/lnd"
            "#,
        ];

        let expected = vec![
            Lnd {
                rest_api_socket: Some(*LND_SOCKET),
                dir: Some(PathBuf::from("~/.local/share/comit/lnd")),
            },
            Lnd {
                rest_api_socket: Some(*LND_SOCKET),
                dir: None,
            },
            Lnd {
                rest_api_socket: None,
                dir: Some(PathBuf::from("~/.local/share/comit/lnd")),
            },
        ];

        let actual = file_contents
            .into_iter()
            .map(toml::from_str)
            .collect::<Result<Vec<Lnd>, toml::de::Error>>()
            .unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn lightning_deserializes_correctly() {
        let file_contents = vec![
            r#"
            network = "regtest"
            "#,
            r#"
            network = "regtest"
            [lnd]
            rest_api_socket = "127.0.0.1:8080"
            dir = "~/.lnd"
            "#,
        ];

        let expected = vec![
            Lightning {
                network: bitcoin::Network::Regtest,
                lnd: None,
            },
            Lightning {
                network: bitcoin::Network::Regtest,
                lnd: Some(Lnd {
                    rest_api_socket: Some(*LND_SOCKET),
                    dir: Some(PathBuf::from("~/.lnd")),
                }),
            },
        ];

        let actual = file_contents
            .into_iter()
            .map(toml::from_str)
            .collect::<Result<Vec<Lightning>, toml::de::Error>>()
            .unwrap();

        assert_eq!(actual, expected);
    }
}
