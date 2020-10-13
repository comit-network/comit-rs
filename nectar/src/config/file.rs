use crate::{
    config::{Bitcoind, BtcDai, Data, EstimateMode, Network},
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
#[serde(deny_unknown_fields)]
pub struct File {
    pub maker: Option<Maker>,
    pub network: Option<Network>,
    pub data: Option<Data>,
    pub logging: Option<Logging>,
    pub bitcoin: Option<Bitcoin>,
    pub ethereum: Option<Ethereum>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Maker {
    pub spread: Option<Spread>,
    pub kraken_api_host: Option<Url>,
    pub btc_dai: Option<BtcDai>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Bitcoin {
    pub network: ledger::Bitcoin,
    pub bitcoind: Option<Bitcoind>,
    #[serde(default)]
    pub fees: Option<BitcoinFees>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Ethereum {
    pub chain_id: ChainId,
    pub node_url: Option<Url>,
    #[serde(default)]
    #[serde(with = "crate::config::serde::ethereum_address")]
    pub local_dai_contract_address: Option<comit::ethereum::Address>,
    #[serde(default)]
    pub gas_price: Option<EthereumGasPrice>,
}

#[derive(Copy, Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BitcoinFees {
    /// Select the strategy to use to set Bitcoin fees
    pub strategy: Option<BitcoinFeeStrategy>,
    /// The static value to use if the selected strategy is "static"
    #[serde(default)]
    #[serde(with = "::bitcoin::util::amount::serde::as_sat::opt")]
    pub sat_per_vbyte: Option<bitcoin::Amount>,
    /// The estimate mode to use if the selected strategy is "bitcoind estimate
    /// smart fee"
    pub estimate_mode: Option<EstimateMode>,
    /// The Maximum rate we would expected bitcoind estimatesmartfee to return.
    /// This is used as a safeguard to be sure we can always fund Bitcoin if we
    /// committed to it.
    #[serde(default)]
    #[serde(with = "::bitcoin::util::amount::serde::as_sat::opt")]
    pub max_sat_per_vbyte: Option<bitcoin::Amount>,
}

#[derive(Copy, Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BitcoinFeeStrategy {
    Static,
    Bitcoind,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub struct EthereumGasPrice {
    pub service: EthereumGasPriceService,
    pub url: url::Url,
}

#[derive(Copy, Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EthereumGasPriceService {
    Geth,
    EthGasStation,
}

impl File {
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

impl Default for File {
    fn default() -> Self {
        File {
            maker: None,
            network: None,
            data: None,
            logging: None,
            bitcoin: None,
            ethereum: None,
        }
    }
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
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
kraken_api_host = "https://api.kraken.com"

[maker.btc_dai]
max_buy_quantity = 1.23456
max_sell_quantity = 1.23456

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

[bitcoin.fees]
strategy = "bitcoind"
max_sat_per_vbyte = 25

[ethereum]
chain_id = 1337
node_url = "http://localhost:8545/"
local_dai_contract_address = "0x6A9865aDE2B6207dAAC49f8bCba9705dEB0B0e6D"

[ethereum.gas_price]
service = "eth_gas_station"
url = "https://ethgasstation.info/api/ethgasAPI.json?api-key=XXAPI_Key_HereXXX"
"#;
        let expected = File {
            maker: Some(Maker {
                btc_dai: Some(BtcDai {
                    max_buy_quantity: Some(bitcoin::Amount::from_btc(1.23456).unwrap()),
                    max_sell_quantity: Some(bitcoin::Amount::from_btc(1.23456).unwrap()),
                }),
                spread: Some(Spread::new(1000).unwrap()),
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
                fees: Some(BitcoinFees {
                    strategy: Some(BitcoinFeeStrategy::Bitcoind),
                    sat_per_vbyte: None,
                    estimate_mode: None,
                    max_sat_per_vbyte: Some(bitcoin::Amount::from_sat(25)),
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
                gas_price: Some(EthereumGasPrice {
                    service: EthereumGasPriceService::EthGasStation,
                    url: "https://ethgasstation.info/api/ethgasAPI.json?api-key=XXAPI_Key_HereXXX"
                        .parse()
                        .unwrap(),
                }),
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
                btc_dai: Some(BtcDai {
                    max_buy_quantity: Some(bitcoin::Amount::from_btc(1.23456).unwrap()),
                    max_sell_quantity: Some(bitcoin::Amount::from_btc(1.23456).unwrap()),
                }),
                spread: Some(Spread::new(1000).unwrap()),
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
                fees: Some(BitcoinFees {
                    strategy: Some(BitcoinFeeStrategy::Bitcoind),
                    sat_per_vbyte: None,
                    estimate_mode: Some(EstimateMode::Conservative),
                    max_sat_per_vbyte: Some(bitcoin::Amount::from_sat(34)),
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
                gas_price: Some(EthereumGasPrice {
                    service: EthereumGasPriceService::EthGasStation,
                    url: "https://ethgasstation.info/api/ethgasAPI.json?api-key=XXAPI_Key_HereXXX"
                        .parse()
                        .unwrap(),
                }),
            }),
        };

        let expected = r#"[maker]
spread = 1000
kraken_api_host = "https://api.kraken.com/"

[maker.btc_dai]
max_buy_quantity = 1.23456
max_sell_quantity = 1.23456

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

[bitcoin.fees]
strategy = "bitcoind"
estimate_mode = "conservative"
max_sat_per_vbyte = 34

[ethereum]
chain_id = 1337
node_url = "http://localhost:8545/"
local_dai_contract_address = "0x6a9865ade2b6207daac49f8bcba9705deb0b0e6d"

[ethereum.gas_price]
service = "eth_gas_station"
url = "https://ethgasstation.info/api/ethgasAPI.json?api-key=XXAPI_Key_HereXXX"
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
            [fees]
            strategy = "bitcoind"
            estimate_mode = "unset"
            sat_per_vbyte = 12
            max_sat_per_vbyte = 34
            "#,
        ];

        let expected = vec![
            Bitcoin {
                network: ledger::Bitcoin::Mainnet,
                bitcoind: Some(Bitcoind {
                    node_url: Url::parse("http://example.com:8332").unwrap(),
                }),
                fees: None,
            },
            Bitcoin {
                network: ledger::Bitcoin::Testnet,
                bitcoind: Some(Bitcoind {
                    node_url: Url::parse("http://example.com:18332").unwrap(),
                }),
                fees: None,
            },
            Bitcoin {
                network: ledger::Bitcoin::Regtest,
                bitcoind: Some(Bitcoind {
                    node_url: Url::parse("http://example.com:18443").unwrap(),
                }),
                fees: Some(BitcoinFees {
                    strategy: Some(BitcoinFeeStrategy::Bitcoind),
                    sat_per_vbyte: Some(bitcoin::Amount::from_sat(12)),
                    estimate_mode: Some(EstimateMode::Unset),
                    max_sat_per_vbyte: Some(bitcoin::Amount::from_sat(34)),
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
            [gas_price]
            service = "geth"
            url = "http://example.com:1234"
            "#,
            r#"
            chain_id = 3
            node_url = "http://example.com:8545"
            [gas_price]
            service = "eth_gas_station"
            url = "http://example.url:5678"
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
                gas_price: Some(EthereumGasPrice {
                    service: EthereumGasPriceService::Geth,
                    url: "http://example.com:1234".parse().unwrap(),
                }),
            },
            Ethereum {
                chain_id: ChainId::ROPSTEN,
                node_url: Some(Url::parse("http://example.com:8545").unwrap()),
                local_dai_contract_address: None,
                gas_price: Some(EthereumGasPrice {
                    service: EthereumGasPriceService::EthGasStation,
                    url: "http://example.url:5678".parse().unwrap(),
                }),
            },
            Ethereum {
                chain_id: ChainId::MAINNET,
                node_url: Some(Url::parse("http://example.com:8545").unwrap()),
                local_dai_contract_address: None,
                gas_price: None,
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
    fn max_deserializes_correctly() {
        let file_contents = vec![
            r#"
            max_buy_quantity = 1.2345
            max_sell_quantity = 1.2345
            "#,
            r#"
            max_buy_quantity = 0
            max_sell_quantity = 1.2345
            "#,
            r#"
            max_buy_quantity = 123
            max_sell_quantity = 0
            "#,
            r#"
            max_buy_quantity = 123
            "#,
            r#"
            max_sell_quantity = 123
            "#,
            r#"
            "#,
        ];

        let expected = vec![
            BtcDai {
                max_buy_quantity: Some(bitcoin::Amount::from_btc(1.2345).unwrap()),
                max_sell_quantity: Some(bitcoin::Amount::from_btc(1.2345).unwrap()),
            },
            BtcDai {
                max_buy_quantity: Some(bitcoin::Amount::from_btc(0.0).unwrap()),
                max_sell_quantity: Some(bitcoin::Amount::from_btc(1.2345).unwrap()),
            },
            BtcDai {
                max_buy_quantity: Some(bitcoin::Amount::from_btc(123.0).unwrap()),
                max_sell_quantity: Some(bitcoin::Amount::from_btc(0.0).unwrap()),
            },
            BtcDai {
                max_buy_quantity: Some(bitcoin::Amount::from_btc(123.0).unwrap()),
                max_sell_quantity: None,
            },
            BtcDai {
                max_buy_quantity: None,
                max_sell_quantity: Some(bitcoin::Amount::from_btc(123.0).unwrap()),
            },
            BtcDai {
                max_buy_quantity: None,
                max_sell_quantity: None,
            },
        ];

        let actual = file_contents
            .into_iter()
            .map(toml::from_str)
            .collect::<Result<Vec<BtcDai>, toml::de::Error>>()
            .unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn bitcoin_fee_strategies_deserializes_correctly() {
        let file_contents = vec![
            r#"
            strategy = "static"
            sat_per_vbyte = 10
            "#,
            r#"
            strategy = "bitcoind"
            estimate_mode = "unset"
            max_sat_per_vbyte = 34
            "#,
            r#"
            strategy = "bitcoind"
            max_sat_per_vbyte = 50
            "#,
            r#"
            strategy = "bitcoind"
            estimate_mode = "economical"
            "#,
        ];

        let expected = vec![
            BitcoinFees {
                strategy: Some(BitcoinFeeStrategy::Static),
                sat_per_vbyte: Some(bitcoin::Amount::from_sat(10)),
                estimate_mode: None,
                max_sat_per_vbyte: None,
            },
            BitcoinFees {
                strategy: Some(BitcoinFeeStrategy::Bitcoind),
                sat_per_vbyte: None,
                estimate_mode: Some(EstimateMode::Unset),
                max_sat_per_vbyte: Some(bitcoin::Amount::from_sat(34)),
            },
            BitcoinFees {
                strategy: Some(BitcoinFeeStrategy::Bitcoind),
                sat_per_vbyte: None,
                estimate_mode: None,
                max_sat_per_vbyte: Some(bitcoin::Amount::from_sat(50)),
            },
            BitcoinFees {
                strategy: Some(BitcoinFeeStrategy::Bitcoind),
                sat_per_vbyte: None,
                estimate_mode: Some(EstimateMode::Economical),
                max_sat_per_vbyte: None,
            },
        ];

        let actual = file_contents
            .into_iter()
            .map(toml::from_str)
            .collect::<Result<Vec<BitcoinFees>, toml::de::Error>>()
            .unwrap();

        assert_eq!(actual, expected);
    }
}
