use crate::{
    bitcoin,
    config::{
        file, file::EthereumGasPriceService, Bitcoind, BtcDai, Data, EstimateMode, File, Network,
    },
    ethereum, Spread,
};
use anyhow::{Context, Result};
use comit::ledger;
use conquer_once::Lazy;
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
    pub sentry: Option<Sentry>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Bitcoin {
    pub network: ledger::Bitcoin,
    pub bitcoind: Bitcoind,
    pub fees: BitcoinFees,
}

impl Bitcoin {
    pub fn default_from_network(network: ledger::Bitcoin) -> Self {
        Self {
            network,
            bitcoind: Bitcoind::new(network),
            fees: Default::default(),
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
        let fees = bitcoin
            .fees
            .map_or_else(BitcoinFees::default, BitcoinFees::from);

        Ok(Bitcoin {
            network,
            bitcoind,
            fees,
        })
    }
}

#[cfg(test)]
impl crate::StaticStub for Bitcoin {
    fn static_stub() -> Self {
        Bitcoin {
            network: ledger::Bitcoin::Regtest,
            bitcoind: Bitcoind::new(ledger::Bitcoin::Regtest),
            fees: BitcoinFees::static_stub(),
        }
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
            fees: Some(bitcoin.fees.into()),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Ethereum {
    pub node_url: Url,
    pub chain: ethereum::Chain,
    pub gas_price: EthereumGasPrice,
}

impl Ethereum {
    fn default_from_chain_id(chain_id: ethereum::ChainId) -> Result<Self> {
        let chain = ethereum::Chain::from_public_chain_id(chain_id)?;
        let node_url = "http://localhost:8545"
            .parse()
            .expect("to be valid static string");

        Ok(Ethereum {
            node_url,
            chain,
            gas_price: Default::default(),
        })
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

        let gas_price = ethereum.gas_price.map_or_else(Default::default, From::from);

        Ok(Ethereum {
            node_url,
            chain,
            gas_price,
        })
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
                gas_price: Some(ethereum.gas_price.into()),
            },
            _ => file::Ethereum {
                chain_id: ethereum.chain.chain_id(),
                node_url: Some(ethereum.node_url),
                local_dai_contract_address: None,
                gas_price: Some(ethereum.gas_price.into()),
            },
        }
    }
}

impl Default for Ethereum {
    fn default() -> Self {
        Self {
            node_url: Url::parse("http://localhost:8545").expect("static string to be a valid url"),
            chain: ethereum::Chain::Mainnet,
            gas_price: Default::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Maker {
    /// Maximum quantities per order
    pub btc_dai: BtcDai,
    /// Spread to apply to the mid-market rate, format is permyriad. E.g. 5.20
    /// is 5.2% spread
    pub spread: Spread,
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

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum BitcoinFees {
    SatsPerByte(bitcoin::Amount),
    BitcoindEstimateSmartfee {
        mode: EstimateMode,
        max_sat_per_vbyte: bitcoin::Amount,
    },
}

impl BitcoinFees {
    pub fn max_tx_fee(&self) -> bitcoin::Amount {
        let rate_per_byte = match self {
            BitcoinFees::SatsPerByte(fee) => fee,
            BitcoinFees::BitcoindEstimateSmartfee {
                max_sat_per_vbyte, ..
            } => max_sat_per_vbyte,
        };

        *rate_per_byte * crate::bitcoin::MAX_EXPECTED_TRANSACTION_VBYTE_WEIGHT
    }
}

static DEFAULT_BITCOIN_STATIC_FEE_SAT: Lazy<bitcoin::Amount> =
    // Low value that would allow inclusion in ~6 blocks:
    // https://txstats.com/dashboard/db/fee-estimation?orgId=1&panelId=2&fullscreen&from=now-6M&to=now&var-source=blockcypher
    Lazy::new(|| bitcoin::Amount::from_sat(50));

static DEFAULT_MAX_BITCOIN_FEE_SAT_PER_VBYTE: Lazy<bitcoin::Amount> =
// Bitcoind's highest estimate in the past year:
// https://txstats.com/dashboard/db/fee-estimation?orgId=1&panelId=5&fullscreen&from=now-1y&to=now
    Lazy::new(|| bitcoin::Amount::from_sat(200));

#[cfg(test)]
impl crate::StaticStub for BitcoinFees {
    fn static_stub() -> Self {
        Self::SatsPerByte(bitcoin::Amount::ZERO)
    }
}

/// Defaults to static fee mode
/// Default value for static mode is 10 sat per byte
impl From<file::BitcoinFees> for BitcoinFees {
    fn from(file: file::BitcoinFees) -> Self {
        file.strategy
            .map_or_else(Default::default, |strategy| match strategy {
                file::BitcoinFeeStrategy::Static => Self::SatsPerByte(
                    file.sat_per_vbyte
                        .unwrap_or(*DEFAULT_BITCOIN_STATIC_FEE_SAT),
                ),
                file::BitcoinFeeStrategy::Bitcoind => Self::BitcoindEstimateSmartfee {
                    mode: file.estimate_mode.unwrap_or_else(EstimateMode::default),
                    max_sat_per_vbyte: Default::default(),
                },
            })
    }
}

impl Default for BitcoinFees {
    fn default() -> Self {
        Self::BitcoindEstimateSmartfee {
            mode: EstimateMode::Economical,
            max_sat_per_vbyte: *DEFAULT_MAX_BITCOIN_FEE_SAT_PER_VBYTE,
        }
    }
}

impl From<BitcoinFees> for file::BitcoinFees {
    fn from(settings: BitcoinFees) -> Self {
        match settings {
            BitcoinFees::SatsPerByte(fee) => Self {
                strategy: Some(file::BitcoinFeeStrategy::Static),
                sat_per_vbyte: Some(fee),
                estimate_mode: None,
                max_sat_per_vbyte: None,
            },
            BitcoinFees::BitcoindEstimateSmartfee {
                mode,
                max_sat_per_vbyte,
            } => Self {
                strategy: Some(file::BitcoinFeeStrategy::Bitcoind),
                sat_per_vbyte: None,
                estimate_mode: Some(mode),
                max_sat_per_vbyte: Some(max_sat_per_vbyte),
            },
        }
    }
}

static DEFAULT_ETH_GAS_STATION_URL: Lazy<url::Url> = Lazy::new(|| {
    "https://ethgasstation.info/api/ethgasAPI.json"
        .parse()
        .expect("Valid url")
});

#[derive(Clone, Debug, PartialEq)]
pub enum EthereumGasPrice {
    Geth(url::Url),
    EthGasStation(url::Url),
}

impl From<file::EthereumGasPrice> for EthereumGasPrice {
    fn from(file: file::EthereumGasPrice) -> Self {
        match file.service {
            EthereumGasPriceService::Geth => Self::Geth(file.url),
            EthereumGasPriceService::EthGasStation => Self::EthGasStation(file.url),
        }
    }
}

impl From<EthereumGasPrice> for file::EthereumGasPrice {
    fn from(settings: EthereumGasPrice) -> Self {
        match settings {
            EthereumGasPrice::Geth(url) => Self {
                service: EthereumGasPriceService::Geth,
                url,
            },
            EthereumGasPrice::EthGasStation(url) => Self {
                service: EthereumGasPriceService::EthGasStation,
                url,
            },
        }
    }
}

impl Default for EthereumGasPrice {
    fn default() -> Self {
        Self::EthGasStation(DEFAULT_ETH_GAS_STATION_URL.clone())
    }
}

impl Maker {
    fn from_file(file: file::Maker) -> Self {
        Self {
            btc_dai: file.btc_dai.unwrap_or_default(),
            spread: file
                .spread
                .unwrap_or_else(|| Spread::new(500).expect("500 is a valid spread value")),
            kraken_api_host: file
                .kraken_api_host
                .map_or_else(KrakenApiHost::default, KrakenApiHost),
        }
    }
}

impl Default for Maker {
    fn default() -> Self {
        Self {
            btc_dai: BtcDai::default(),
            spread: Spread::new(500).expect("500 is a valid spread value"),
            kraken_api_host: KrakenApiHost::default(),
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Sentry {
    pub url: Url,
}

impl Sentry {
    fn from_file(sentry: file::Sentry) -> Self {
        Sentry { url: sentry.url }
    }
}

impl From<Sentry> for file::Sentry {
    fn from(sentry: Sentry) -> Self {
        file::Sentry { url: sentry.url }
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
            sentry,
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
            sentry: sentry.map(file::Sentry::from),
        }
    }
}

impl From<Maker> for file::Maker {
    fn from(maker: Maker) -> file::Maker {
        file::Maker {
            btc_dai: match maker.btc_dai {
                BtcDai {
                    max_buy_quantity: None,
                    max_sell_quantity: None,
                } => None,
                max_sell => Some(max_sell),
            },
            spread: Some(maker.spread),
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
            sentry,
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
                || {
                    Ok(Bitcoin::default_from_network(
                        comit_network.unwrap_or_default().into(),
                    ))
                },
                |file| Bitcoin::from_file(file, comit_network),
            )?,
            ethereum: ethereum.map_or_else(
                || Ethereum::default_from_chain_id(comit_network.unwrap_or_default().into()),
                |file| Ethereum::from_file(file, comit_network),
            )?,
            sentry: sentry.map(Sentry::from_file),
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
                fees: BitcoinFees::BitcoindEstimateSmartfee {
                    mode: EstimateMode::Economical,
                    max_sat_per_vbyte: bitcoin::Amount::from_sat(200),
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
                    fees: Default::default(),
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
                gas_price: EthereumGasPrice::EthGasStation(DEFAULT_ETH_GAS_STATION_URL.clone()),
            })
    }
}
