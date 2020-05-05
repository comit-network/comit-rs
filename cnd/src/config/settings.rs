use crate::config::{
    default_lnd_cert_path, default_lnd_readonly_macaroon_path, file, Bitcoin, Bitcoind, Data,
    Ethereum, File, Lightning, Lnd, Network, Parity,
};
use anyhow::Context;
use log::LevelFilter;
use reqwest::Url;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

/// This structs represents the settings as they are used through out the code.
///
/// An optional setting (represented in this struct as an `Option`) has semantic
/// meaning in cnd. Contrary to that, many configuration values are optional in
/// the config file but may be replaced by default values when the `Settings`
/// are created from a given `Config`.
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub network: Network,
    pub http_api: HttpApi,
    pub data: Data,
    pub logging: Logging,
    pub bitcoin: Bitcoin,
    pub ethereum: Ethereum,
    pub lightning: Lightning,
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
            let node_url = match ethereum.parity {
                None => {
                    // default is always localhost:8545
                    "http://localhost:8545"
                        .parse()
                        .expect("to be valid static string")
                }
                Some(parity) => parity.node_url,
            };
            Ethereum {
                chain_id: ethereum.chain_id,
                parity: Parity { node_url },
            }
        }
    }
}

fn check_url_lnd(lnd_url: Url) -> anyhow::Result<Url> {
    if lnd_url.scheme() == "https" {
        Ok(lnd_url)
    } else {
        Err(anyhow::anyhow!("HTTPS scheme is expected for lnd url."))
    }
}

impl From<Settings> for File {
    fn from(settings: Settings) -> Self {
        let Settings {
            network,
            http_api: HttpApi { socket, cors },
            data,
            logging: Logging { level },
            bitcoin,
            ethereum,
            lightning,
        } = settings;

        File {
            network: Some(network),
            http_api: Some(file::HttpApi {
                socket,
                cors: Some(file::Cors {
                    allowed_origins: match cors.allowed_origins {
                        AllowedOrigins::All => file::AllowedOrigins::All(file::All::All),
                        AllowedOrigins::None => file::AllowedOrigins::None(file::None::None),
                        AllowedOrigins::Some(origins) => file::AllowedOrigins::Some(origins),
                    },
                }),
            }),
            data: Some(data),
            logging: Some(file::Logging {
                level: Some(level.into()),
            }),
            bitcoin: Some(bitcoin.into()),
            ethereum: Some(ethereum.into()),
            lightning: Some(lightning.into()),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct HttpApi {
    pub socket: SocketAddr,
    pub cors: Cors,
}

impl Default for HttpApi {
    fn default() -> Self {
        Self {
            socket: SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8000),
            cors: Cors::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Cors {
    pub allowed_origins: AllowedOrigins,
}

impl Default for Cors {
    fn default() -> Self {
        Self {
            allowed_origins: AllowedOrigins::None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum AllowedOrigins {
    All,
    None,
    Some(Vec<String>),
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
            http_api,
            data,
            logging,
            bitcoin,
            ethereum,
            lightning,
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
            http_api: http_api
                .map(|file::HttpApi { socket, cors }| {
                    let cors = cors
                        .map(|cors| {
                            let allowed_origins = match cors.allowed_origins {
                                file::AllowedOrigins::All(_) => AllowedOrigins::All,
                                file::AllowedOrigins::None(_) => AllowedOrigins::None,
                                file::AllowedOrigins::Some(origins) => {
                                    AllowedOrigins::Some(origins)
                                }
                            };

                            Cors { allowed_origins }
                        })
                        .unwrap_or_default();

                    HttpApi { socket, cors }
                })
                .unwrap_or_default(),
            data: {
                let default_data_dir =
                    crate::data_dir().context("unable to determine default data path")?;
                data.unwrap_or_else(|| Data {
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
            lightning: match lightning {
                None => Lightning::default(),
                Some(lightning) => Lightning {
                    network: lightning.network,
                    lnd: match lightning.lnd {
                        None => Lnd::default(),
                        Some(lnd) => Lnd {
                            rest_api_url: check_url_lnd(lnd.rest_api_url)?,
                            dir: lnd.dir.clone(),
                            cert_path: default_lnd_cert_path(lnd.dir.clone()),
                            readonly_macaroon_path: default_lnd_readonly_macaroon_path(
                                lnd.dir,
                                lightning.network,
                            ),
                        },
                    },
                },
            },
        })
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::{config::file, swap_protocols::ledger::ethereum};
    use spectral::prelude::*;
    use std::net::IpAddr;

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
    fn cors_section_defaults_to_no_allowed_foreign_origins() {
        let config_file = File {
            http_api: Some(file::HttpApi {
                socket: SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8000),
                cors: None,
            }),
            ..File::default()
        };

        let settings = Settings::from_config_file_and_defaults(config_file);

        assert_that(&settings)
            .is_ok()
            .map(|settings| &settings.http_api.cors)
            .is_equal_to(Cors {
                allowed_origins: AllowedOrigins::None,
            })
    }

    #[test]
    fn http_api_section_defaults() {
        let config_file = File {
            http_api: None,
            ..File::default()
        };

        let settings = Settings::from_config_file_and_defaults(config_file);

        assert_that(&settings)
            .is_ok()
            .map(|settings| &settings.http_api)
            .is_equal_to(HttpApi {
                socket: SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8000),
                cors: Cors {
                    allowed_origins: AllowedOrigins::None,
                },
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
                chain_id: ethereum::ChainId::regtest(),
                parity: Parity {
                    node_url: "http://localhost:8545".parse().unwrap(),
                },
            })
    }

    #[test]
    fn ethereum_defaults_chain_id_only() {
        let defaults = vec![
            (ethereum::ChainId::mainnet(), "http://localhost:8545"),
            (ethereum::ChainId::ropsten(), "http://localhost:8545"),
            (ethereum::ChainId::regtest(), "http://localhost:8545"),
        ];

        for (chain_id, url) in defaults {
            let ethereum = Some(file::Ethereum {
                chain_id,
                parity: None,
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
                    parity: Parity {
                        node_url: url.parse().unwrap(),
                    },
                })
        }
    }

    #[test]
    fn lightning_section_defaults() {
        let config_file = File {
            lightning: None,
            ..File::default()
        };

        let settings = Settings::from_config_file_and_defaults(config_file);

        assert_that(&settings)
            .is_ok()
            .map(|settings| &settings.lightning)
            .is_equal_to(Lightning::default())
    }

    #[test]
    fn lightning_lnd_section_defaults() {
        let config_file = File {
            lightning: Some(file::Lightning {
                network: bitcoin::Network::Regtest,
                lnd: None,
            }),
            ..File::default()
        };

        let settings = Settings::from_config_file_and_defaults(config_file);

        assert_that(&settings)
            .is_ok()
            .map(|settings| &settings.lightning)
            .is_equal_to(Lightning {
                network: bitcoin::Network::Regtest,
                lnd: Lnd::default(),
            })
    }

    #[test]
    fn error_on_http_url_for_lnd() {
        let config_file = File {
            lightning: Some(file::Lightning {
                network: bitcoin::Network::Regtest,
                lnd: Some(file::Lnd {
                    rest_api_url: "http://localhost:8000/".parse().unwrap(),
                    dir: Default::default(),
                }),
            }),
            ..File::default()
        };

        let settings = Settings::from_config_file_and_defaults(config_file);

        assert_that(&settings).is_err();
    }
}
