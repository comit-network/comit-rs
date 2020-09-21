use crate::{
    config::{Bitcoind, Data, Geth},
    ethereum,
    ethereum::ChainId,
};
use comit::ledger;
use libp2p::core::Multiaddr;
use log::LevelFilter;
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsStr,
    net::SocketAddr,
    path::{Path, PathBuf},
};

/// This struct aims to represent the configuration file as it appears on disk.
///
/// Most importantly, optional elements of the configuration file are
/// represented as `Option`s` here. This allows us to create a dedicated step
/// for filling in default values for absent configuration options.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct File {
    pub network: Option<Network>,
    pub http_api: Option<HttpApi>,
    pub data: Option<Data>,
    pub logging: Option<Logging>,
    pub bitcoin: Option<Bitcoin>,
    pub ethereum: Option<Ethereum>,
    pub lightning: Option<Lightning>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Network {
    pub listen: Vec<Multiaddr>,
    pub peer_addresses: Option<Vec<Multiaddr>>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Bitcoin {
    pub network: ledger::Bitcoin,
    pub bitcoind: Option<Bitcoind>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Ethereum {
    pub chain_id: ChainId,
    pub geth: Option<Geth>,
    pub tokens: Option<Tokens>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Tokens {
    pub dai: Option<ethereum::Address>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Lightning {
    pub network: ledger::Bitcoin,
    pub lnd: Option<Lnd>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Lnd {
    pub rest_api_url: reqwest::Url,
    pub dir: PathBuf,
}

impl File {
    pub fn default() -> Self {
        File {
            network: Option::None,
            http_api: Option::None,
            data: Option::None,
            logging: Option::None,
            bitcoin: Option::None,
            ethereum: Option::None,
            lightning: Option::None,
        }
    }

    pub fn read<D>(config_file: D) -> Result<Self, ::config::ConfigError>
    where
        D: AsRef<OsStr>,
    {
        let config_file = Path::new(&config_file);

        let mut config = ::config::Config::new();
        config.merge(::config::File::from(config_file))?;
        config.try_into()
    }
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Logging {
    pub level: Option<Level>,
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum Level {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl From<LevelFilter> for Level {
    fn from(level: LevelFilter) -> Self {
        match level {
            LevelFilter::Off => Level::Error, // We don't support suppressing all logs.
            LevelFilter::Error => Level::Error,
            LevelFilter::Warn => Level::Warn,
            LevelFilter::Info => Level::Info,
            LevelFilter::Debug => Level::Debug,
            LevelFilter::Trace => Level::Trace,
        }
    }
}

impl From<Level> for LevelFilter {
    fn from(level: Level) -> Self {
        match level {
            Level::Error => LevelFilter::Error,
            Level::Warn => LevelFilter::Warn,
            Level::Info => LevelFilter::Info,
            Level::Debug => LevelFilter::Debug,
            Level::Trace => LevelFilter::Trace,
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct HttpApi {
    pub socket: SocketAddr,
    pub cors: Option<Cors>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Cors {
    pub allowed_origins: AllowedOrigins,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(untagged)]
pub enum AllowedOrigins {
    All(All),
    None(None),
    Some(Vec<String>),
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum All {
    All,
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum None {
    None,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Bitcoind, Geth, Settings};
    use reqwest::Url;
    use spectral::prelude::*;
    use std::{
        net::{IpAddr, Ipv4Addr, SocketAddr},
        path::PathBuf,
    };

    #[derive(serde::Deserialize, PartialEq, Debug)]
    struct LoggingOnlyConfig {
        logging: Logging,
    }

    #[test]
    fn network_deserializes_correctly() {
        let file_contents = vec![
            r#"
            listen = ["/ip4/0.0.0.0/tcp/9939"]
            peer_addresses = [ "/ip4/1.1.1.1/tcp/9939" ]
            "#,
            r#"
            listen = ["/ip4/0.0.0.0/tcp/9939", "/ip4/127.0.0.1/tcp/9939"]
            peer_addresses = [
                "/ip4/1.1.1.1/tcp/9939",
                "/ip4/2.2.2.2/tcp/3456"
            ]
            "#,
        ];

        let expected = vec![
            Network {
                listen: vec!["/ip4/0.0.0.0/tcp/9939".parse().unwrap()],
                peer_addresses: Some(vec!["/ip4/1.1.1.1/tcp/9939".parse().unwrap()]),
            },
            Network {
                listen: (vec![
                    "/ip4/0.0.0.0/tcp/9939".parse().unwrap(),
                    "/ip4/127.0.0.1/tcp/9939".parse().unwrap(),
                ]),
                peer_addresses: Some(vec![
                    "/ip4/1.1.1.1/tcp/9939".parse().unwrap(),
                    "/ip4/2.2.2.2/tcp/3456".parse().unwrap(),
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
    fn cors_deserializes_correctly() {
        let file_contents = vec![
            r#"
            allowed_origins = "all"
            "#,
            r#"
             allowed_origins = "none"
            "#,
            r#"
             allowed_origins = ["http://localhost:8000", "https://192.168.1.55:3000"]
            "#,
        ];

        let expected = vec![
            Cors {
                allowed_origins: AllowedOrigins::All(All::All),
            },
            Cors {
                allowed_origins: AllowedOrigins::None(None::None),
            },
            Cors {
                allowed_origins: AllowedOrigins::Some(vec![
                    String::from("http://localhost:8000"),
                    String::from("https://192.168.1.55:3000"),
                ]),
            },
        ];

        let actual = file_contents
            .into_iter()
            .map(toml::from_str)
            .collect::<Result<Vec<Cors>, toml::de::Error>>()
            .unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn full_config_deserializes_correctly() {
        let contents = r#"
[network]
listen = ["/ip4/0.0.0.0/tcp/9939"]

[http_api]
socket = "127.0.0.1:8000"

[http_api.cors]
allowed_origins = "all"

[data]
dir = "/tmp/comit/"

[logging]
level = "Debug"

[bitcoin]
network = "regtest"

[bitcoin.bitcoind]
node_url = "http://localhost:18443/"

[ethereum]
chain_id = 1337

[ethereum.geth]
node_url = "http://localhost:8545/"

[ethereum.tokens]
dai = "0x6b175474e89094c44da98b954eedeac495271d0f"

[lightning]
network = "regtest"

[lightning.lnd]
rest_api_url = "https://localhost:8080"
dir = "/foo/bar"
"#;
        let file = File {
            network: Some(Network {
                listen: vec!["/ip4/0.0.0.0/tcp/9939".parse().unwrap()],
                peer_addresses: None,
            }),
            http_api: Some(HttpApi {
                socket: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8000),
                cors: Some(Cors {
                    allowed_origins: AllowedOrigins::All(All::All),
                }),
            }),
            data: Some(Data {
                dir: PathBuf::from("/tmp/comit/"),
            }),
            logging: Some(Logging {
                level: Some(Level::Debug),
            }),
            bitcoin: Some(Bitcoin {
                network: ledger::Bitcoin::Regtest,
                bitcoind: Some(Bitcoind {
                    node_url: "http://localhost:18443".parse().unwrap(),
                }),
            }),
            ethereum: Some(Ethereum {
                chain_id: ChainId::GETH_DEV,
                geth: Some(Geth {
                    node_url: "http://localhost:8545".parse().unwrap(),
                }),
                tokens: Some(Tokens {
                    dai: Some(
                        "0x6b175474e89094c44da98b954eedeac495271d0f"
                            .parse()
                            .unwrap(),
                    ),
                }),
            }),
            lightning: Some(Lightning {
                network: ledger::Bitcoin::Regtest,
                lnd: Some(Lnd {
                    rest_api_url: "https://localhost:8080".parse().unwrap(),
                    dir: PathBuf::from("/foo/bar"),
                }),
            }),
        };

        let config = toml::from_str::<File>(contents);
        assert_that(&config).is_ok().is_equal_to(file);
    }

    #[test]
    fn config_with_defaults_roundtrip() {
        // we start with the default config file
        let default_file = File::default();

        // convert to settings, this populates all empty fields with defaults
        let effective_settings =
            Settings::from_config_file_and_defaults(default_file, None).unwrap();

        // write settings back to file
        let file_with_effective_settings = File::from(effective_settings);

        let serialized = toml::to_string(&file_with_effective_settings).unwrap();
        let file = toml::from_str::<File>(&serialized).unwrap();

        assert_eq!(file, file_with_effective_settings)
    }

    #[test]
    fn bitcoin_deserializes_correctly() {
        let file_contents = vec![
            r#"
            network = "mainnet"
            [bitcoind]
            node_url = "http://example.com:8332"
            "#,
            r#"
            network = "testnet"
            [bitcoind]
            node_url = "http://example.com:18332"
            "#,
            r#"
            network = "regtest"
            [bitcoind]
            node_url = "http://example.com:18443"
            "#,
        ];

        let expected = vec![
            Bitcoin {
                network: ledger::Bitcoin::Mainnet,
                bitcoind: Some(Bitcoind {
                    node_url: Url::parse("http://example.com:8332").unwrap(),
                }),
            },
            Bitcoin {
                network: ledger::Bitcoin::Testnet,
                bitcoind: Some(Bitcoind {
                    node_url: Url::parse("http://example.com:18332").unwrap(),
                }),
            },
            Bitcoin {
                network: ledger::Bitcoin::Regtest,
                bitcoind: Some(Bitcoind {
                    node_url: Url::parse("http://example.com:18443").unwrap(),
                }),
            },
        ];

        let actual = file_contents
            .into_iter()
            .map(toml::from_str)
            .collect::<Result<Vec<Bitcoin>, toml::de::Error>>()
            .unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn ethereum_deserializes_correctly() {
        let file_contents = vec![
            r#"
            chain_id = 42
            [geth]
            node_url = "http://example.com:8545"
            [tokens]
            dai = "0xc4375b7de8af5a38a93548eb8453a498222c4ff2"
            "#,
            r#"
            chain_id = 3
            [geth]
            node_url = "http://example.com:8545"
            [tokens]
            dai = "0xaD6D458402F60fD3Bd25163575031ACDce07538D"
            "#,
            r#"
            chain_id = 1
            [geth]
            node_url = "http://example.com:8545"
            [tokens]
            dai = "0x6b175474e89094c44da98b954eedeac495271d0f"
            "#,
        ];

        let expected = vec![
            Ethereum {
                chain_id: ChainId::KOVAN,
                geth: Some(Geth {
                    node_url: Url::parse("http://example.com:8545").unwrap(),
                }),
                tokens: Some(Tokens {
                    dai: Some(
                        "0xc4375b7de8af5a38a93548eb8453a498222c4ff2"
                            .parse()
                            .unwrap(),
                    ),
                }),
            },
            Ethereum {
                chain_id: ChainId::ROPSTEN,
                geth: Some(Geth {
                    node_url: Url::parse("http://example.com:8545").unwrap(),
                }),
                tokens: Some(Tokens {
                    dai: Some(
                        "0xaD6D458402F60fD3Bd25163575031ACDce07538D"
                            .parse()
                            .unwrap(),
                    ),
                }),
            },
            Ethereum {
                chain_id: ChainId::MAINNET,
                geth: Some(Geth {
                    node_url: Url::parse("http://example.com:8545").unwrap(),
                }),
                tokens: Some(Tokens {
                    dai: Some(
                        "0x6b175474e89094c44da98b954eedeac495271d0f"
                            .parse()
                            .unwrap(),
                    ),
                }),
            },
        ];

        let actual = file_contents
            .into_iter()
            .map(toml::from_str)
            .collect::<Result<Vec<Ethereum>, toml::de::Error>>()
            .unwrap();

        assert_eq!(actual, expected);
    }
}
