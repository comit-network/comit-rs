use crate::{
    bitcoin,
    config::{Bitcoind, Data, MaxSell, Network},
    Spread,
};
use comit::{ethereum::ChainId, ledger};
use config as config_rs;
use log::LevelFilter;
use serde::{Deserialize, Serialize};
use std::{ffi::OsStr, path::Path};
use url::Url;

/// This struct aims to represent the configuration file as it appears on disk.
///
/// Most importantly, optional elements of the configuration file are
/// represented as `Option`s` here. This allows us to create a dedicated step
/// for filling in default values for absent configuration options.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct File {
    pub maker: Option<Maker>,
    pub network: Option<Network>,
    pub data: Option<Data>,
    pub logging: Option<Logging>,
    pub bitcoin: Option<Bitcoin>,
    pub ethereum: Option<Ethereum>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Maker {
    pub spread: Option<Spread>,
    pub kraken_api_host: Option<Url>,
    pub max_sell: Option<MaxSell>,
    pub maximum_possible_fee: Option<Fees>,
}

#[derive(Copy, Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Fees {
    #[serde(default)]
    #[serde(with = "crate::config::serde::bitcoin_amount")]
    pub bitcoin: Option<bitcoin::Amount>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Bitcoin {
    pub network: ledger::Bitcoin,
    pub bitcoind: Option<Bitcoind>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Ethereum {
    pub chain_id: ChainId,
    pub node_url: Option<Url>,
    #[serde(default)]
    #[serde(with = "crate::config::serde::ethereum_address")]
    pub local_dai_contract_address: Option<comit::ethereum::Address>,
}

impl File {
    pub fn default() -> Self {
        File {
            maker: None,
            network: None,
            data: None,
            logging: None,
            bitcoin: None,
            ethereum: None,
        }
    }

    pub fn read<D>(config_file: D) -> Result<Self, config_rs::ConfigError>
    where
        D: AsRef<OsStr>,
    {
        let config_file = Path::new(&config_file);

        let mut config = config_rs::Config::new();
        config.merge(config_rs::File::from(config_file))?;
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
    use crate::{
        bitcoin,
        config::{Bitcoind, Settings},
        ethereum::dai,
    };
    use spectral::prelude::*;
    use std::{io::Write, path::PathBuf};
    use tempfile::TempDir;

    #[derive(serde::Deserialize, PartialEq, Debug)]
    struct LoggingOnlyConfig {
        logging: Logging,
    }

    #[test]
    fn full_config_deserializes_correctly() {
        let contents = r#"
[maker]
# 1000 is 10.00% spread
spread = 1000
maximum_possible_fee = { bitcoin = 0.01 }
kraken_api_host = "https://api.kraken.com"

[maker.max_sell]
bitcoin = 1.23456
dai = 9876.54321

[network]
listen = ["/ip4/0.0.0.0/tcp/9939"]

[data]
dir = "/tmp/nectar/"

[logging]
level = "Debug"

[bitcoin]
network = "regtest"

[bitcoin.bitcoind]
node_url = "http://localhost:18443/"

[ethereum]
chain_id = 1337
node_url = "http://localhost:8545/"
local_dai_contract_address = "0x6A9865aDE2B6207dAAC49f8bCba9705dEB0B0e6D"
"#;
        let expected = File {
            maker: Some(Maker {
                max_sell: Some(MaxSell {
                    bitcoin: Some(bitcoin::Amount::from_btc(1.23456).unwrap()),
                    dai: Some(dai::Amount::from_dai_trunc(9876.54321).unwrap()),
                }),
                spread: Some(Spread::new(1000).unwrap()),
                maximum_possible_fee: Some(Fees {
                    bitcoin: Some(bitcoin::Amount::from_btc(0.01).unwrap()),
                }),
                kraken_api_host: Some("https://api.kraken.com".parse().unwrap()),
            }),
            network: Some(Network {
                listen: vec!["/ip4/0.0.0.0/tcp/9939".parse().unwrap()],
            }),
            data: Some(Data {
                dir: PathBuf::from("/tmp/nectar/"),
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
                node_url: Some("http://localhost:8545".parse().unwrap()),
                local_dai_contract_address: Some(
                    "0x6A9865aDE2B6207dAAC49f8bCba9705dEB0B0e6D"
                        .parse()
                        .unwrap(),
                ),
            }),
        };

        let tmp_dir = TempDir::new().unwrap();
        let file_path = tmp_dir.path().join("config.toml");

        let mut file = std::fs::File::create(&file_path).unwrap();
        file.write_all(contents.as_bytes()).unwrap();

        let file = File::read(&file_path);

        assert_that(&file).is_ok().is_equal_to(expected);
    }

    #[test]
    fn full_config_serializes_correctly() {
        let file = File {
            maker: Some(Maker {
                max_sell: Some(MaxSell {
                    bitcoin: Some(bitcoin::Amount::from_btc(1.23456).unwrap()),
                    dai: Some(dai::Amount::from_dai_trunc(9876.54321).unwrap()),
                }),
                spread: Some(Spread::new(1000).unwrap()),
                maximum_possible_fee: Some(Fees {
                    bitcoin: Some(bitcoin::Amount::from_btc(0.01).unwrap()),
                }),
                kraken_api_host: Some("https://api.kraken.com".parse().unwrap()),
            }),
            network: Some(Network {
                listen: vec!["/ip4/0.0.0.0/tcp/9939".parse().unwrap()],
            }),
            data: Some(Data {
                dir: PathBuf::from("/tmp/nectar/"),
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
                node_url: Some("http://localhost:8545".parse().unwrap()),
                local_dai_contract_address: Some(
                    "0x6A9865aDE2B6207dAAC49f8bCba9705dEB0B0e6D"
                        .parse()
                        .unwrap(),
                ),
            }),
        };

        let expected = r#"[maker]
spread = 1000
kraken_api_host = "https://api.kraken.com/"

[maker.max_sell]
bitcoin = 1.23456
dai = 9876.54

[maker.maximum_possible_fee]
bitcoin = 0.01

[network]
listen = ["/ip4/0.0.0.0/tcp/9939"]

[data]
dir = "/tmp/nectar/"

[logging]
level = "Debug"

[bitcoin]
network = "regtest"

[bitcoin.bitcoind]
node_url = "http://localhost:18443/"

[ethereum]
chain_id = 1337
node_url = "http://localhost:8545/"
local_dai_contract_address = "0x6a9865ade2b6207daac49f8bcba9705deb0b0e6d"
"#;

        let serialized = toml::to_string(&file);

        assert_that(&serialized)
            .is_ok()
            .is_equal_to(expected.to_string());
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
            chain_id = 1337
            node_url = "http://example.com:8545"
            local_dai_contract_address = "0x31F42841c2db5173425b5223809CF3A38FEde360"
            "#,
            r#"
            chain_id = 3
            node_url = "http://example.com:8545"
            "#,
            r#"
            chain_id = 1
            node_url = "http://example.com:8545"
            "#,
        ];

        let expected = vec![
            Ethereum {
                chain_id: ChainId::GETH_DEV,
                node_url: Some(Url::parse("http://example.com:8545").unwrap()),
                local_dai_contract_address: Some(
                    "0x31F42841c2db5173425b5223809CF3A38FEde360"
                        .parse()
                        .unwrap(),
                ),
            },
            Ethereum {
                chain_id: ChainId::ROPSTEN,
                node_url: Some(Url::parse("http://example.com:8545").unwrap()),
                local_dai_contract_address: None,
            },
            Ethereum {
                chain_id: ChainId::MAINNET,
                node_url: Some(Url::parse("http://example.com:8545").unwrap()),
                local_dai_contract_address: None,
            },
        ];

        let actual = file_contents
            .into_iter()
            .map(toml::from_str)
            .collect::<Result<Vec<Ethereum>, toml::de::Error>>()
            .unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn max_sell_deserializes_correctly() {
        let file_contents = vec![
            r#"
            bitcoin = 1.2345
            dai = 91234.123
            "#,
            r#"
            bitcoin = 0
            dai = 9999
            "#,
            r#"
            bitcoin = 123
            dai = 0
            "#,
            r#"
            dai = 9999
            "#,
            r#"
            bitcoin = 123
            "#,
            r#"
            "#,
        ];

        let expected = vec![
            MaxSell {
                bitcoin: Some(bitcoin::Amount::from_btc(1.2345).unwrap()),
                dai: Some(dai::Amount::from_dai_trunc(91234.123).unwrap()),
            },
            MaxSell {
                bitcoin: Some(bitcoin::Amount::from_btc(0.0).unwrap()),
                dai: Some(dai::Amount::from_dai_trunc(9999.0).unwrap()),
            },
            MaxSell {
                bitcoin: Some(bitcoin::Amount::from_btc(123.0).unwrap()),
                dai: Some(dai::Amount::from_dai_trunc(0.0).unwrap()),
            },
            MaxSell {
                bitcoin: None,
                dai: Some(dai::Amount::from_dai_trunc(9999.0).unwrap()),
            },
            MaxSell {
                bitcoin: Some(bitcoin::Amount::from_btc(123.0).unwrap()),
                dai: None,
            },
            MaxSell {
                bitcoin: None,
                dai: None,
            },
        ];

        let actual = file_contents
            .into_iter()
            .map(toml::from_str)
            .collect::<Result<Vec<MaxSell>, toml::de::Error>>()
            .unwrap();

        assert_eq!(actual, expected);
    }
}
