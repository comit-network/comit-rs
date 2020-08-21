mod file;
mod serde_bitcoin_network;
mod settings;
mod validation;

use crate::{ethereum::ChainId, fs};
use anyhow::{Context, Result};
use conquer_once::Lazy;
use libp2p::Multiaddr;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, path::PathBuf, str::FromStr};

pub use self::{
    file::File,
    settings::{AllowedOrigins, Settings},
    validation::validate_connection_to_network,
};

static BITCOIND_RPC_MAINNET: Lazy<Url> = Lazy::new(|| parse_unchecked("http://localhost:8332"));
static BITCOIND_RPC_TESTNET: Lazy<Url> = Lazy::new(|| parse_unchecked("http://localhost:18332"));
static BITCOIND_RPC_REGTEST: Lazy<Url> = Lazy::new(|| parse_unchecked("http://localhost:18443"));

static LND_URL: Lazy<Url> = Lazy::new(|| parse_unchecked("https://localhost:8080"));

static WEB3_URL: Lazy<Url> = Lazy::new(|| parse_unchecked("http://localhost:8545"));

static COMIT_SOCKET: Lazy<Multiaddr> = Lazy::new(|| parse_unchecked("/ip4/0.0.0.0/tcp/9939"));

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Data {
    pub dir: PathBuf,
}

impl Data {
    pub fn default() -> Result<Self> {
        Ok(Self {
            dir: fs::data_dir().context("unable to determine default data path")?,
        })
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Network {
    pub listen: Vec<Multiaddr>,
}

impl Default for Network {
    fn default() -> Self {
        Self {
            listen: vec![COMIT_SOCKET.clone()],
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Bitcoin {
    #[serde(with = "crate::config::serde_bitcoin_network")]
    pub network: bitcoin::Network,
    pub bitcoind: Bitcoind,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Bitcoind {
    pub node_url: Url,
}

impl Bitcoin {
    fn new(network: bitcoin::Network) -> Self {
        Self {
            network,
            bitcoind: Bitcoind::new(network),
        }
    }

    fn from_file(bitcoin: file::Bitcoin, comit_network: Option<comit::Network>) -> Result<Self> {
        if let Some(comit_network) = comit_network {
            let inferred = bitcoin::Network::from(comit_network);
            if inferred != bitcoin.network {
                anyhow::bail!(
                    "inferred Bitcoin network {} from CLI argument {} but config file says {}",
                    inferred,
                    comit_network,
                    bitcoin.network
                );
            }
        }

        let network = bitcoin.network;
        let bitcoind = bitcoin.bitcoind.unwrap_or_else(|| Bitcoind::new(network));

        Ok(Bitcoin { network, bitcoind })
    }
}

impl Bitcoind {
    fn new(network: bitcoin::Network) -> Self {
        let node_url = match network {
            bitcoin::Network::Bitcoin => BITCOIND_RPC_MAINNET.clone(),
            bitcoin::Network::Testnet => BITCOIND_RPC_TESTNET.clone(),
            bitcoin::Network::Regtest => BITCOIND_RPC_REGTEST.clone(),
        };

        Bitcoind { node_url }
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
    pub geth: Geth,
}

impl Ethereum {
    fn new(chain_id: ChainId) -> Self {
        Self {
            chain_id,
            geth: Geth::new(),
        }
    }

    fn from_file(ethereum: file::Ethereum, comit_network: Option<comit::Network>) -> Result<Self> {
        if let Some(comit_network) = comit_network {
            let inferred = ChainId::from(comit_network);
            if inferred != ethereum.chain_id {
                anyhow::bail!(
                    "inferred Ethereum chain ID {} from CLI argument {} but config file says {}",
                    inferred,
                    comit_network,
                    ethereum.chain_id
                );
            }
        }

        let chain_id = ethereum.chain_id;
        let geth = ethereum.geth.unwrap_or_else(Geth::new);

        Ok(Ethereum { chain_id, geth })
    }
}

impl From<Ethereum> for file::Ethereum {
    fn from(ethereum: Ethereum) -> Self {
        file::Ethereum {
            chain_id: ethereum.chain_id,
            geth: Some(ethereum.geth),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Geth {
    pub node_url: Url,
}

impl Geth {
    fn new() -> Self {
        Self {
            node_url: WEB3_URL.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Lightning {
    pub network: bitcoin::Network,
    pub lnd: Lnd,
}

impl Lightning {
    fn new(network: bitcoin::Network) -> Self {
        Self {
            network,
            lnd: Lnd::new(network),
        }
    }

    fn from_file(
        lightning: file::Lightning,
        comit_network: Option<comit::Network>,
    ) -> Result<Self> {
        if let Some(comit_network) = comit_network {
            let inferred = bitcoin::Network::from(comit_network);
            if inferred != lightning.network {
                anyhow::bail!(
                    "inferred Lightning network {} from CLI argument {} but config file says {}",
                    inferred,
                    comit_network,
                    lightning.network
                );
            }
        }

        let network = lightning.network;
        let lnd = lightning.lnd.map_or_else::<Result<Lnd>, _, _>(
            || Ok(Lnd::new(network)),
            |file| Lnd::from_file(file, network),
        )?;

        Ok(Lightning { network, lnd })
    }
}

impl From<Lightning> for file::Lightning {
    fn from(lightning: Lightning) -> Self {
        file::Lightning {
            lnd: Some(file::Lnd {
                rest_api_url: lightning.lnd.rest_api_url,
                dir: lightning.lnd.dir,
            }),
            network: lightning.network,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Lnd {
    pub rest_api_url: Url,
    pub dir: PathBuf,
    pub cert_path: PathBuf,
    pub readonly_macaroon_path: PathBuf,
}

impl Lnd {
    fn new(network: bitcoin::Network) -> Self {
        Self::from_url_dir_and_network(LND_URL.clone(), default_lnd_dir(), network)
    }

    fn from_file(file: file::Lnd, network: bitcoin::Network) -> Result<Self> {
        let rest_api_url = assert_lnd_url_https(file.rest_api_url)?;

        Ok(Self::from_url_dir_and_network(
            rest_api_url,
            file.dir,
            network,
        ))
    }

    fn from_url_dir_and_network(
        rest_api_url: Url,
        dir: PathBuf,
        network: bitcoin::Network,
    ) -> Self {
        Lnd {
            rest_api_url,
            dir: dir.clone(),
            cert_path: default_lnd_cert_path(dir.clone()),
            readonly_macaroon_path: default_lnd_readonly_macaroon_path(dir, network),
        }
    }
}

fn assert_lnd_url_https(lnd_url: Url) -> Result<Url> {
    if lnd_url.scheme() == "https" {
        Ok(lnd_url)
    } else {
        Err(anyhow::anyhow!("HTTPS scheme is expected for lnd url."))
    }
}

fn default_lnd_dir() -> PathBuf {
    fs::lnd_dir().expect("no home directory")
}

fn default_lnd_cert_path(lnd_dir: PathBuf) -> PathBuf {
    lnd_dir.join("tls.cert")
}

fn default_lnd_readonly_macaroon_path(lnd_dir: PathBuf, network: bitcoin::Network) -> PathBuf {
    let network_dir = match network {
        bitcoin::Network::Bitcoin => "mainnet",
        bitcoin::Network::Testnet => "testnet",
        bitcoin::Network::Regtest => "regtest",
    };
    lnd_dir
        .join("data")
        .join("chain")
        .join("bitcoin")
        .join(network_dir)
        .join("readonly.macaroon")
}

fn parse_unchecked<T>(str: &'static str) -> T
where
    T: FromStr + Debug,
    <T as FromStr>::Err: Send + Sync + 'static + std::error::Error,
{
    str.parse()
        .with_context(|| format!("failed to parse static string '{}' into T", str))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectral::prelude::*;

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
        let actual = toml::from_str(
            r#"
            rest_api_url = "https://localhost:8080"
            dir = "~/.local/share/comit/lnd"
            "#,
        );

        let expected = file::Lnd {
            rest_api_url: LND_URL.clone(),
            dir: PathBuf::from("~/.local/share/comit/lnd"),
        };

        assert_eq!(actual, Ok(expected));
    }

    #[test]
    fn lightning_deserializes_correctly() {
        let actual = toml::from_str(
            r#"
            network = "regtest"
            [lnd]
            rest_api_url = "https://localhost:8080"
            dir = "/path/to/lnd"
            "#,
        );

        let expected = file::Lightning {
            network: bitcoin::Network::Regtest,
            lnd: Some(file::Lnd {
                rest_api_url: LND_URL.clone(),
                dir: PathBuf::from("/path/to/lnd"),
            }),
        };

        assert_eq!(actual, Ok(expected));
    }

    #[test]
    fn given_network_on_cli_when_config_disagrees_then_error() {
        let comit_network = comit::Network::Main;
        let config_file = file::Bitcoin {
            network: bitcoin::Network::Testnet,
            bitcoind: None,
        };

        let result = Bitcoin::from_file(config_file, Some(comit_network));

        assert_that(&result).is_err();
    }

    #[test]
    fn given_no_network_on_cli_then_use_config() {
        let config_file = file::Bitcoin {
            network: bitcoin::Network::Testnet,
            bitcoind: None,
        };

        let result = Bitcoin::from_file(config_file, None);

        assert_that(&result)
            .is_ok()
            .map(|b| &b.network)
            .is_equal_to(bitcoin::Network::Testnet);
    }

    #[test]
    fn given_network_on_cli_when_config_specifies_the_same_then_ok() {
        let comit_network = comit::Network::Main;
        let config_file = file::Bitcoin {
            network: bitcoin::Network::Bitcoin,
            bitcoind: None,
        };

        let result = Bitcoin::from_file(config_file, Some(comit_network));

        assert_that(&result).is_ok();
    }
}
