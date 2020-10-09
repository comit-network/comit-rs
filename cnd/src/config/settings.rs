use crate::config::{
    file, Bitcoind, Data, Ethereum, File, Lightning, COMIT_SOCKET, CYPHERBLOCK_MAINNET_URL,
    CYPHERBLOCK_TESTNET_URL, FEERATE_SAT_PER_VBYTE,
};
use anyhow::Result;
use comit::ledger;
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

#[derive(Clone, Debug, PartialEq)]
pub struct Bitcoin {
    pub network: ledger::Bitcoin,
    pub bitcoind: Bitcoind,
    pub fees: BitcoinFees,
}

impl Bitcoin {
    fn default_from_network(network: ledger::Bitcoin) -> Self {
        Self {
            network,
            bitcoind: Bitcoind::new(network),
            fees: BitcoinFees::default_from_network(network),
        }
    }

    pub fn from_file(
        bitcoin: file::Bitcoin,
        comit_network: Option<comit::Network>,
    ) -> Result<Self> {
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
        let fees = bitcoin.fees.map_or_else(
            || Ok(BitcoinFees::default_from_network(network)),
            |file| BitcoinFees::from_file(file, network),
        )?;

        Ok(Bitcoin {
            network,
            bitcoind,
            fees,
        })
    }
}

impl From<Bitcoin> for file::Bitcoin {
    fn from(settings: Bitcoin) -> Self {
        Self {
            network: settings.network,
            bitcoind: Some(settings.bitcoind),
            fees: Some(settings.fees.into()),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum BitcoinFees {
    StaticSatPerVbyte(bitcoin::Amount),
    CypherBlock(url::Url),
}

impl BitcoinFees {
    fn default_from_network(network: ledger::Bitcoin) -> Self {
        use ledger::Bitcoin::*;

        match network {
            Mainnet => BitcoinFees::CypherBlock(CYPHERBLOCK_MAINNET_URL.clone()),
            Testnet => BitcoinFees::CypherBlock(CYPHERBLOCK_TESTNET_URL.clone()),
            Regtest => BitcoinFees::StaticSatPerVbyte(*FEERATE_SAT_PER_VBYTE),
        }
    }

    fn from_file(file: file::BitcoinFees, network: ledger::Bitcoin) -> Result<Self> {
        use file::BitcoinFeesStrategy::*;

        match (file.strategy, file.r#static, file.cypherblock) {
            (Static, _, Some(_)) => anyhow::bail!(
                "bitcoin.fees.cypherblock must not be present if the static strategy is selected."
            ),
            (Static, Some(file::Static { sat_per_vbyte }), None) => {
                Ok(Self::StaticSatPerVbyte(sat_per_vbyte))
            }
            (Static, None, None) => Ok(Self::StaticSatPerVbyte(*FEERATE_SAT_PER_VBYTE)),
            (CypherBlock, Some(_), _) => anyhow::bail!(
                "bitcoin.fees.static must not be present if the cypherblock strategy is selected."
            ),
            (
                CypherBlock,
                None,
                Some(file::CypherBlock {
                    blockchain_endpoint_url,
                }),
            ) => Ok(Self::CypherBlock(blockchain_endpoint_url)),
            (CypherBlock, None, None) => Ok(Self::default_from_network(network)),
        }
    }
}

// TODO: move all this impl under `file.rs` to make things neater
impl From<BitcoinFees> for file::BitcoinFees {
    fn from(settings: BitcoinFees) -> Self {
        match settings {
            BitcoinFees::StaticSatPerVbyte(sat_per_vbyte) => Self {
                strategy: file::BitcoinFeesStrategy::Static,
                r#static: Some(file::Static { sat_per_vbyte }),
                cypherblock: None,
            },
            BitcoinFees::CypherBlock(blockchain_endpoint_url) => Self {
                strategy: file::BitcoinFeesStrategy::CypherBlock,
                r#static: None,
                cypherblock: Some(file::CypherBlock {
                    blockchain_endpoint_url,
                }),
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
                || {
                    Ok(Bitcoin::default_from_network(
                        comit_network.unwrap_or_default().into(),
                    ))
                },
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
                fees: BitcoinFees::CypherBlock(CYPHERBLOCK_MAINNET_URL.clone()),
            })
    }

    #[test]
    fn bitcoin_defaults_network_only() {
        let defaults = vec![
            (
                ledger::Bitcoin::Mainnet,
                "http://localhost:8332",
                BitcoinFees::CypherBlock("http://api.blockcypher.com/v1/btc/main".parse().unwrap()),
            ),
            (
                ledger::Bitcoin::Testnet,
                "http://localhost:18332",
                BitcoinFees::CypherBlock(
                    "http://api.blockcypher.com/v1/btc/test3".parse().unwrap(),
                ),
            ),
            (
                ledger::Bitcoin::Regtest,
                "http://localhost:18443",
                BitcoinFees::StaticSatPerVbyte(bitcoin::Amount::from_sat(50)),
            ),
        ];

        for (network, url, fees) in defaults {
            let config_file = File {
                bitcoin: Some(file::Bitcoin {
                    network,
                    bitcoind: None,
                    fees: None,
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
                    fees,
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

    #[test]
    fn given_network_on_cli_when_config_disagrees_then_error() {
        let comit_network = comit::Network::Main;
        let config_file = file::Bitcoin {
            network: ledger::Bitcoin::Testnet,
            bitcoind: None,
            fees: None,
        };

        let result = Bitcoin::from_file(config_file, Some(comit_network));

        assert_that(&result).is_err();
    }

    #[test]
    fn given_no_network_on_cli_then_use_config() {
        let config_file = file::Bitcoin {
            network: ledger::Bitcoin::Testnet,
            bitcoind: None,
            fees: None,
        };

        let result = Bitcoin::from_file(config_file, None);

        assert_that(&result)
            .is_ok()
            .map(|b| &b.network)
            .is_equal_to(ledger::Bitcoin::Testnet);
    }

    #[test]
    fn given_network_on_cli_when_config_specifies_the_same_then_ok() {
        let comit_network = comit::Network::Main;
        let config_file = file::Bitcoin {
            network: ledger::Bitcoin::Mainnet,
            bitcoind: None,
            fees: None,
        };

        let result = Bitcoin::from_file(config_file, Some(comit_network));

        assert_that(&result).is_ok();
    }

    #[test]
    fn given_bitcoin_fees_static_strategy_and_cypherblock_config_present_then_error() {
        let config_file = file::BitcoinFees {
            strategy: file::BitcoinFeesStrategy::Static,
            cypherblock: Some(file::CypherBlock {
                blockchain_endpoint_url: "http://1.1.1.1:123".parse().unwrap(),
            }),
            r#static: None,
        };

        let result = BitcoinFees::from_file(config_file, ledger::Bitcoin::Mainnet);

        assert_that(&result).is_err();
    }

    #[test]
    fn given_bitcoin_fees_cypherblock_strategy_and_cypherblock_config_present_then_error() {
        let config_file = file::BitcoinFees {
            strategy: file::BitcoinFeesStrategy::CypherBlock,
            cypherblock: None,
            r#static: Some(file::Static {
                sat_per_vbyte: bitcoin::Amount::from_sat(10),
            }),
        };

        let result = BitcoinFees::from_file(config_file, ledger::Bitcoin::Mainnet);

        assert_that(&result).is_err();
    }
}
