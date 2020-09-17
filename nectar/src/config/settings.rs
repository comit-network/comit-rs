use crate::{
    bitcoin,
    config::{file, Bitcoind, Data, File, MaxSell, Network},
    ethereum, Spread,
};
use anyhow::{Context, Result};
use comit::ledger;
use log::LevelFilter;
use url::Url;

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub maker: Maker,
    pub network: Network,
    pub data: Data,
    pub logging: Logging,
    pub bitcoin: Bitcoin,
    pub ethereum: Ethereum,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Bitcoin {
    pub network: ledger::Bitcoin,
    pub bitcoind: Bitcoind,
}

impl Bitcoin {
    pub fn new(network: ledger::Bitcoin) -> Self {
        Self {
            network,
            bitcoind: Bitcoind::new(network),
        }
    }

    fn from_file(bitcoin: file::Bitcoin, comit_network: Option<comit::Network>) -> Result<Self> {
        if let Some(comit_network) = comit_network {
            let inferred = ledger::Bitcoin::from(comit_network);
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
    fn new(network: ledger::Bitcoin) -> Self {
        let node_url = match network {
            ledger::Bitcoin::Mainnet => {
                Url::parse("http://localhost:8332").expect("static string to be a valid url")
            }
            ledger::Bitcoin::Testnet => {
                Url::parse("http://localhost:18332").expect("static string to be a valid url")
            }
            ledger::Bitcoin::Regtest => {
                Url::parse("http://localhost:18443").expect("static string to be a valid url")
            }
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

#[derive(Clone, Debug, PartialEq)]
pub struct Ethereum {
    pub node_url: Url,
    pub chain: ethereum::Chain,
}

impl Ethereum {
    fn new(chain_id: ethereum::ChainId) -> Result<Self> {
        let chain = ethereum::Chain::from_public_chain_id(chain_id)?;
        let node_url = "http://localhost:8545"
            .parse()
            .expect("to be valid static string");

        Ok(Ethereum { node_url, chain })
    }

    fn from_file(ethereum: file::Ethereum, comit_network: Option<comit::Network>) -> Result<Self> {
        if let Some(comit_network) = comit_network {
            let inferred = ethereum::ChainId::from(comit_network);
            if inferred != ethereum.chain_id {
                anyhow::bail!(
                    "inferred Ethereum chain ID {} from CLI argument {} but config file says {}",
                    inferred,
                    comit_network,
                    ethereum.chain_id
                );
            }
        }

        let node_url = match ethereum.node_url {
            None => {
                // default is always localhost:8545
                "http://localhost:8545"
                    .parse()
                    .expect("to be valid static string")
            }
            Some(node_url) => node_url,
        };

        let chain = match (ethereum.chain_id, ethereum.local_dai_contract_address) {
            (chain_id, Some(address)) => ethereum::Chain::new(chain_id, address),
            (chain_id, None) => ethereum::Chain::from_public_chain_id(chain_id)?,
        };

        Ok(Ethereum { node_url, chain })
    }
}

impl From<Ethereum> for file::Ethereum {
    fn from(ethereum: Ethereum) -> Self {
        match ethereum.chain {
            ethereum::Chain::Local {
                chain_id,
                dai_contract_address,
            } => file::Ethereum {
                chain_id: chain_id.into(),
                node_url: Some(ethereum.node_url),
                local_dai_contract_address: Some(dai_contract_address),
            },
            _ => file::Ethereum {
                chain_id: ethereum.chain.chain_id(),
                node_url: Some(ethereum.node_url),
                local_dai_contract_address: None,
            },
        }
    }
}

impl Default for Ethereum {
    fn default() -> Self {
        Self {
            node_url: Url::parse("http://localhost:8545").expect("static string to be a valid url"),
            chain: ethereum::Chain::Mainnet,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Maker {
    /// Maximum amount to sell per order
    pub max_sell: MaxSell,
    /// Spread to apply to the mid-market rate, format is permyriad. E.g. 5.20
    /// is 5.2% spread
    pub spread: Spread,
    /// Maximum possible network fee to consider when calculating the available
    /// balance. Fees are in the nominal native currency and per
    /// transaction.
    pub maximum_possible_fee: Fees,
    pub kraken_api_host: KrakenApiHost,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KrakenApiHost(Url);

impl KrakenApiHost {
    pub fn with_trading_pair(&self, trading_pair: &str) -> Result<Url> {
        let url = self
            .0
            .join(&format!("/0/public/Ticker?pair={}", trading_pair))?;

        Ok(url)
    }
}

impl Default for KrakenApiHost {
    fn default() -> Self {
        let url = "https://api.kraken.com"
            .parse()
            .expect("static url always parses correctly");

        Self(url)
    }
}

impl Maker {
    fn from_file(file: file::Maker) -> Self {
        Self {
            max_sell: file.max_sell.unwrap_or_default(),
            spread: file
                .spread
                .unwrap_or_else(|| Spread::new(500).expect("500 is a valid spread value")),
            maximum_possible_fee: file
                .maximum_possible_fee
                .map_or_else(Fees::default, Fees::from_file),
            kraken_api_host: file
                .kraken_api_host
                .map_or_else(KrakenApiHost::default, KrakenApiHost),
        }
    }
}

impl Default for Maker {
    fn default() -> Self {
        Self {
            max_sell: MaxSell::default(),
            spread: Spread::new(500).expect("500 is a valid spread value"),
            maximum_possible_fee: Fees::default(),
            kraken_api_host: KrakenApiHost::default(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Fees {
    pub bitcoin: bitcoin::Amount,
}

impl Fees {
    fn from_file(file: file::Fees) -> Self {
        Self {
            bitcoin: file.bitcoin.unwrap_or_else(Self::default_bitcoin_fee),
        }
    }

    // ~265 vbytes (2 inputs 2 outputs segwit transaction)
    // * 35 sat/vbytes (Looking at https://bitcoinfees.github.io/#1m)
    fn default_bitcoin_fee() -> bitcoin::Amount {
        bitcoin::Amount::from_sat(265 * 35)
    }
}

impl Default for Fees {
    fn default() -> Self {
        Fees {
            bitcoin: Self::default_bitcoin_fee(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, derivative::Derivative)]
#[derivative(Default)]
pub struct Logging {
    #[derivative(Default(value = "LevelFilter::Info"))]
    pub level: LevelFilter,
}

impl From<Settings> for File {
    fn from(settings: Settings) -> Self {
        let Settings {
            maker,
            network,
            data,
            logging: Logging { level },
            bitcoin,
            ethereum,
        } = settings;

        File {
            maker: Some(maker.into()),
            network: Some(network),
            data: Some(data),
            logging: Some(file::Logging {
                level: Some(level.into()),
            }),
            bitcoin: Some(bitcoin.into()),
            ethereum: Some(ethereum.into()),
        }
    }
}

impl From<Maker> for file::Maker {
    fn from(maker: Maker) -> file::Maker {
        file::Maker {
            max_sell: match maker.max_sell {
                MaxSell {
                    bitcoin: None,
                    dai: None,
                } => None,
                max_sell => Some(max_sell),
            },
            spread: Some(maker.spread),
            maximum_possible_fee: Some(file::Fees {
                bitcoin: Some(maker.maximum_possible_fee.bitcoin),
            }),
            kraken_api_host: Some(maker.kraken_api_host.0),
        }
    }
}

impl Settings {
    pub fn from_config_file_and_defaults(
        config_file: File,
        comit_network: Option<comit::Network>,
    ) -> anyhow::Result<Self> {
        let File {
            maker,
            network,
            data,
            logging,
            bitcoin,
            ethereum,
        } = config_file;

        Ok(Self {
            maker: maker.map_or_else(Maker::default, Maker::from_file),
            network: network.unwrap_or_else(|| {
                let default_socket = "/ip4/0.0.0.0/tcp/9939"
                    .parse()
                    .expect("cnd listen address could not be parsed");

                Network {
                    listen: vec![default_socket],
                }
            }),
            data: {
                let default_data_dir =
                    crate::fs::data_dir().context("unable to determine default data path")?;
                data.unwrap_or(Data {
                    dir: default_data_dir,
                })
            },

            logging: {
                match logging {
                    None => Logging::default(),
                    Some(inner) => match inner {
                        file::Logging { level: None } => Logging::default(),
                        file::Logging { level: Some(level) } => Logging {
                            level: level.into(),
                        },
                    },
                }
            },
            bitcoin: bitcoin.map_or_else(
                || Ok(Bitcoin::new(comit_network.unwrap_or_default().into())),
                |file| Bitcoin::from_file(file, comit_network),
            )?,
            ethereum: ethereum.map_or_else(
                || Ethereum::new(comit_network.unwrap_or_default().into()),
                |file| Ethereum::from_file(file, comit_network),
            )?,
        })
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::config::file;
    use spectral::prelude::*;

    #[test]
    fn logging_section_defaults_to_info() {
        let config_file = File {
            logging: None,
            ..File::default()
        };

        let settings = Settings::from_config_file_and_defaults(config_file, None);

        assert_that(&settings)
            .is_ok()
            .map(|settings| &settings.logging)
            .is_equal_to(Logging {
                level: LevelFilter::Info,
            })
    }

    #[test]
    fn network_section_defaults() {
        let config_file = File {
            network: None,
            ..File::default()
        };

        let settings = Settings::from_config_file_and_defaults(config_file, None);

        assert_that(&settings)
            .is_ok()
            .map(|settings| &settings.network)
            .is_equal_to(Network {
                listen: vec!["/ip4/0.0.0.0/tcp/9939".parse().unwrap()],
            })
    }

    #[test]
    fn bitcoin_defaults() {
        let config_file = File { ..File::default() };

        let settings = Settings::from_config_file_and_defaults(config_file, None);

        assert_that(&settings)
            .is_ok()
            .map(|settings| &settings.bitcoin)
            .is_equal_to(Bitcoin {
                network: ledger::Bitcoin::Mainnet,
                bitcoind: Bitcoind {
                    node_url: "http://localhost:8332".parse().unwrap(),
                },
            })
    }

    #[test]
    fn bitcoin_defaults_network_only() {
        let defaults = vec![
            (ledger::Bitcoin::Mainnet, "http://localhost:8332"),
            (ledger::Bitcoin::Testnet, "http://localhost:18332"),
            (ledger::Bitcoin::Regtest, "http://localhost:18443"),
        ];

        for (network, url) in defaults {
            let config_file = File {
                bitcoin: Some(file::Bitcoin {
                    network,
                    bitcoind: None,
                }),
                ..File::default()
            };

            let settings = Settings::from_config_file_and_defaults(config_file, None);

            assert_that(&settings)
                .is_ok()
                .map(|settings| &settings.bitcoin)
                .is_equal_to(Bitcoin {
                    network,
                    bitcoind: Bitcoind {
                        node_url: url.parse().unwrap(),
                    },
                })
        }
    }

    #[test]
    fn ethereum_defaults() {
        let config_file = File { ..File::default() };

        let settings = Settings::from_config_file_and_defaults(config_file, None);

        assert_that(&settings)
            .is_ok()
            .map(|settings| &settings.ethereum)
            .is_equal_to(Ethereum {
                node_url: "http://localhost:8545".parse().unwrap(),
                chain: ethereum::Chain::Mainnet,
            })
    }
}
