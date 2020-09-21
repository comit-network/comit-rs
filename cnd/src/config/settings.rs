use crate::config::{file, Bitcoin, Data, Ethereum, File, Lightning, COMIT_SOCKET};
use libp2p::core::Multiaddr;
use log::LevelFilter;
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
            network: Some(file::Network {
                listen: network.listen,
                peer_addresses: Some(network.peer_addresses),
            }),
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
pub struct Network {
    pub listen: Vec<Multiaddr>,
    pub peer_addresses: Vec<Multiaddr>,
}

impl Default for Network {
    fn default() -> Self {
        Self {
            listen: vec![COMIT_SOCKET.clone()],
            peer_addresses: vec![],
        }
    }
}

impl From<file::Network> for Network {
    fn from(network: file::Network) -> Self {
        let listen = network.listen;
        let peer_addresses = network.peer_addresses.unwrap_or_default();

        Self {
            listen,
            peer_addresses,
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

impl From<file::HttpApi> for HttpApi {
    fn from(http_api: file::HttpApi) -> Self {
        let socket = http_api.socket;
        let cors = http_api.cors.map_or_else(Cors::default, Cors::from);

        HttpApi { socket, cors }
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

impl From<file::Cors> for Cors {
    fn from(cors: file::Cors) -> Self {
        let allowed_origins = match cors.allowed_origins {
            file::AllowedOrigins::All(_) => AllowedOrigins::All,
            file::AllowedOrigins::None(_) => AllowedOrigins::None,
            file::AllowedOrigins::Some(origins) => AllowedOrigins::Some(origins),
        };

        Cors { allowed_origins }
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

impl From<file::Logging> for Logging {
    fn from(logging: file::Logging) -> Self {
        match logging {
            file::Logging { level: None } => Logging::default(),
            file::Logging { level: Some(level) } => Logging {
                level: level.into(),
            },
        }
    }
}

impl Settings {
    pub fn from_config_file_and_defaults(
        config_file: File,
        comit_network: Option<comit::Network>,
    ) -> anyhow::Result<Self> {
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
            network: network.map_or_else(Network::default, Network::from),
            http_api: http_api.map_or_else(HttpApi::default, HttpApi::from),
            data: data.map_or_else(Data::default, Ok)?,
            logging: logging.map_or_else(Logging::default, Logging::from),

            bitcoin: bitcoin.map_or_else(
                || Ok(Bitcoin::new(comit_network.unwrap_or_default().into())),
                |file| Bitcoin::from_file(file, comit_network),
            )?,
            ethereum: ethereum.map_or_else(
                || Ethereum::new(comit_network.unwrap_or_default().into()),
                |file| Ethereum::from_file(file, comit_network),
            )?,
            lightning: lightning.map_or_else(
                || Ok(Lightning::new(comit_network.unwrap_or_default().into())),
                |file| Lightning::from_file(file, comit_network),
            )?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{file, Bitcoind, Geth, Lnd, Tokens, DAI_MAINNET},
        ethereum::ChainId,
    };
    use comit::ledger;
    use spectral::prelude::*;
    use std::net::IpAddr;

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
    fn cors_section_defaults_to_no_allowed_foreign_origins() {
        let config_file = File {
            http_api: Some(file::HttpApi {
                socket: SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8000),
                cors: None,
            }),
            ..File::default()
        };

        let settings = Settings::from_config_file_and_defaults(config_file, None);

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

        let settings = Settings::from_config_file_and_defaults(config_file, None);

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

        let settings = Settings::from_config_file_and_defaults(config_file, None);

        assert_that(&settings)
            .is_ok()
            .map(|settings| &settings.network)
            .is_equal_to(Network {
                listen: vec!["/ip4/0.0.0.0/tcp/9939".parse().unwrap()],
                peer_addresses: vec![],
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
                chain_id: ChainId::MAINNET,
                geth: Geth {
                    node_url: "http://localhost:8545".parse().unwrap(),
                },
                tokens: Tokens { dai: *DAI_MAINNET },
            })
    }

    #[test]
    fn lightning_section_defaults() {
        let config_file = File {
            lightning: None,
            ..File::default()
        };

        let settings = Settings::from_config_file_and_defaults(config_file, None);

        assert_that(&settings)
            .is_ok()
            .map(|settings| &settings.lightning)
            .is_equal_to(Lightning::new(ledger::Bitcoin::Mainnet))
    }

    #[test]
    fn lightning_lnd_section_defaults() {
        let config_file = File {
            lightning: Some(file::Lightning {
                network: ledger::Bitcoin::Regtest,
                lnd: None,
            }),
            ..File::default()
        };

        let settings = Settings::from_config_file_and_defaults(config_file, None);

        assert_that(&settings)
            .is_ok()
            .map(|settings| &settings.lightning)
            .is_equal_to(Lightning {
                network: ledger::Bitcoin::Regtest,
                lnd: Lnd::new(ledger::Bitcoin::Regtest),
            })
    }

    #[test]
    fn error_on_http_url_for_lnd() {
        let config_file = File {
            lightning: Some(file::Lightning {
                network: ledger::Bitcoin::Regtest,
                lnd: Some(file::Lnd {
                    rest_api_url: "http://localhost:8000/".parse().unwrap(),
                    dir: Default::default(),
                }),
            }),
            ..File::default()
        };

        let settings = Settings::from_config_file_and_defaults(config_file, None);

        assert_that(&settings).is_err();
    }
}
