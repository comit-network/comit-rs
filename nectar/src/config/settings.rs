use crate::{
    bitcoin,
    config::{file, Bitcoind, Data, File, MaxSell, Network},
    ethereum, Spread,
};
use anyhow::Context;
use comit::Role;
use log::LevelFilter;
use std::convert::{TryFrom, TryInto};
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
    pub network: bitcoin::Network,
    pub bitcoind: Bitcoind,
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

#[derive(Clone, Debug, PartialEq)]
pub struct Ethereum {
    pub node_url: Url,
    pub chain: ethereum::Chain,
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

impl TryFrom<Option<file::Ethereum>> for Ethereum {
    type Error = anyhow::Error;

    fn try_from(file_ethereum: Option<file::Ethereum>) -> anyhow::Result<Ethereum> {
        match file_ethereum {
            None => Ok(Ethereum::default()),
            Some(file_ethereum) => {
                let node_url = match file_ethereum.node_url {
                    None => {
                        // default is always localhost:8545
                        "http://localhost:8545"
                            .parse()
                            .expect("to be valid static string")
                    }
                    Some(node_url) => node_url,
                };

                let chain = match (
                    file_ethereum.chain_id,
                    file_ethereum.local_dai_contract_address,
                ) {
                    (chain_id, Some(address)) => ethereum::Chain::new(chain_id, address),
                    (chain_id, None) => ethereum::Chain::from_public_chain_id(chain_id)?,
                };

                Ok(Ethereum { node_url, chain })
            }
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
    /// Role that the maker wishes to take in the swap protocol. The role of
    /// Alice means the maker will deploy the HTLC first on the blockchain.
    /// If Bob fails to proceed with the swap, then the transaction fees to
    /// deploy the contract are wasted.
    pub role: Role,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Fees {
    pub bitcoin: bitcoin::Amount,
}

impl Default for Fees {
    fn default() -> Self {
        Fees {
            // ~265 vbytes (2 inputs 2 outputs segwit transaction)
            // * 35 sat/vbytes (Looking at https://bitcoinfees.github.io/#1m)
            bitcoin: bitcoin::Amount::from_sat(265 * 35),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, derivative::Derivative)]
#[derivative(Default)]
pub struct Logging {
    #[derivative(Default(value = "LevelFilter::Info"))]
    pub level: LevelFilter,
}

fn derive_url_bitcoin(bitcoin: Option<file::Bitcoin>) -> Bitcoin {
    match bitcoin {
        None => Bitcoin::default(),
        Some(bitcoin) => {
            let node_url = match bitcoin.bitcoind {
                Some(bitcoind) => bitcoind.node_url,
                None => match bitcoin.network {
                    bitcoin::Network::Bitcoin => "http://localhost:8332"
                        .parse()
                        .expect("to be valid static string"),
                    bitcoin::Network::Testnet => "http://localhost:18332"
                        .parse()
                        .expect("to be valid static string"),
                    bitcoin::Network::Regtest => "http://localhost:18443"
                        .parse()
                        .expect("to be valid static string"),
                },
            };
            Bitcoin {
                network: bitcoin.network,
                bitcoind: Bitcoind { node_url },
            }
        }
    }
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
            role: Some(maker.role),
        }
    }
}

impl Settings {
    pub fn from_config_file_and_defaults(config_file: File) -> anyhow::Result<Self> {
        let File {
            maker,
            network,
            data,
            logging,
            bitcoin,
            ethereum,
        } = config_file;

        Ok(Self {
            maker: Maker {
                max_sell: if let Some(file::Maker {
                    max_sell: Some(ref max_sell),
                    ..
                }) = maker
                {
                    max_sell.clone()
                } else {
                    MaxSell {
                        bitcoin: None,
                        dai: None,
                    }
                },
                spread: match maker {
                    Some(file::Maker {
                        spread: Some(spread),
                        ..
                    }) => spread,
                    _ => Spread::new(500).expect("500 is a valid spread value"),
                },
                maximum_possible_fee: {
                    if let Some(file::Maker {
                        maximum_possible_fee:
                            Some(file::Fees {
                                bitcoin: Some(bitcoin),
                            }),
                        ..
                    }) = maker
                    {
                        Fees { bitcoin }
                    } else {
                        Fees::default()
                    }
                },
                role: {
                    match maker {
                        // If the role is not specified, default to Bob to prevent loss of money
                        Some(maker) => match maker.role {
                            Some(role) => role,
                            None => Role::Bob,
                        },
                        None => Role::Bob,
                    }
                },
            },
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
            bitcoin: derive_url_bitcoin(bitcoin),
            ethereum: ethereum.try_into()?,
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

        let settings = Settings::from_config_file_and_defaults(config_file);

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

        let settings = Settings::from_config_file_and_defaults(config_file);

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

        let settings = Settings::from_config_file_and_defaults(config_file);

        assert_that(&settings)
            .is_ok()
            .map(|settings| &settings.bitcoin)
            .is_equal_to(Bitcoin {
                network: ::bitcoin::Network::Regtest,
                bitcoind: Bitcoind {
                    node_url: "http://localhost:18443".parse().unwrap(),
                },
            })
    }

    #[test]
    fn bitcoin_defaults_network_only() {
        let defaults = vec![
            (::bitcoin::Network::Bitcoin, "http://localhost:8332"),
            (::bitcoin::Network::Testnet, "http://localhost:18332"),
            (::bitcoin::Network::Regtest, "http://localhost:18443"),
        ];

        for (network, url) in defaults {
            let config_file = File {
                bitcoin: Some(file::Bitcoin {
                    network,
                    bitcoind: None,
                }),
                ..File::default()
            };

            let settings = Settings::from_config_file_and_defaults(config_file);

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

        let settings = Settings::from_config_file_and_defaults(config_file);

        assert_that(&settings)
            .is_ok()
            .map(|settings| &settings.ethereum)
            .is_equal_to(Ethereum {
                node_url: "http://localhost:8545".parse().unwrap(),
                chain: ethereum::Chain::Mainnet,
            })
    }
}
