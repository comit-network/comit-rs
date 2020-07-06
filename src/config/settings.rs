use crate::config::{file, Bitcoin, Bitcoind, Data, Ethereum, File, Geth, Network};
use anyhow::Context;
use log::LevelFilter;

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub network: Network,
    pub data: Data,
    pub logging: Logging,
    pub bitcoin: Bitcoin,
    pub ethereum: Ethereum,
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

fn derive_url_ethereum(ethereum: Option<file::Ethereum>) -> Ethereum {
    match ethereum {
        None => Ethereum::default(),
        Some(ethereum) => {
            let node_url = match ethereum.geth {
                None => {
                    // default is always localhost:8545
                    "http://localhost:8545"
                        .parse()
                        .expect("to be valid static string")
                }
                Some(geth) => geth.node_url,
            };
            Ethereum {
                chain_id: ethereum.chain_id,
                geth: Geth { node_url },
            }
        }
    }
}

impl From<Settings> for File {
    fn from(settings: Settings) -> Self {
        let Settings {
            network,
            data,
            logging: Logging { level },
            bitcoin,
            ethereum,
        } = settings;

        File {
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

#[derive(Clone, Copy, Debug, PartialEq, derivative::Derivative)]
#[derivative(Default)]
pub struct Logging {
    #[derivative(Default(value = "LevelFilter::Info"))]
    pub level: LevelFilter,
}

impl Settings {
    pub fn from_config_file_and_defaults(config_file: File) -> anyhow::Result<Self> {
        let File {
            network,
            data,
            logging,
            bitcoin,
            ethereum,
        } = config_file;

        Ok(Self {
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
                    crate::data_dir().context("unable to determine default data path")?;
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
            ethereum: derive_url_ethereum(ethereum),
        })
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::config::file;
    use comit::ethereum::ChainId;
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
                network: bitcoin::Network::Regtest,
                bitcoind: Bitcoind {
                    node_url: "http://localhost:18443".parse().unwrap(),
                },
            })
    }

    #[test]
    fn bitcoin_defaults_network_only() {
        let defaults = vec![
            (bitcoin::Network::Bitcoin, "http://localhost:8332"),
            (bitcoin::Network::Testnet, "http://localhost:18332"),
            (bitcoin::Network::Regtest, "http://localhost:18443"),
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
                chain_id: ChainId::regtest(),
                geth: Geth {
                    node_url: "http://localhost:8545".parse().unwrap(),
                },
            })
    }

    #[test]
    fn ethereum_defaults_chain_id_only() {
        let defaults = vec![
            (ChainId::mainnet(), "http://localhost:8545"),
            (ChainId::ropsten(), "http://localhost:8545"),
            (ChainId::regtest(), "http://localhost:8545"),
        ];

        for (chain_id, url) in defaults {
            let ethereum = Some(file::Ethereum {
                chain_id,
                geth: None,
            });
            let config_file = File {
                ethereum,
                ..File::default()
            };

            let settings = Settings::from_config_file_and_defaults(config_file);

            assert_that(&settings)
                .is_ok()
                .map(|settings| &settings.ethereum)
                .is_equal_to(Ethereum {
                    chain_id,
                    geth: Geth {
                        node_url: url.parse().unwrap(),
                    },
                })
        }
    }
}
