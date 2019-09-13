mod serde_log;

use bitcoin_support::Network;
use config::{Config, ConfigError, File};
use log::LevelFilter;
use serde::Deserialize;
use std::{ffi::OsStr, net::IpAddr, path::Path};

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    #[serde(default = "default_log_levels")]
    pub log_levels: LogLevels,
    pub http_api: HttpApi,
    pub bitcoin: Option<Bitcoin>,
    pub ethereum: Option<Ethereum>,
}

fn default_log() -> LevelFilter {
    LevelFilter::Info
}

#[derive(Debug, Clone, Deserialize)]
pub struct LogLevels {
    #[serde(with = "self::serde_log", default = "default_log")]
    pub btsieve: LevelFilter,
}

#[derive(Debug, Deserialize, Clone)]
pub struct HttpApi {
    pub address_bind: IpAddr,
    pub port_bind: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Bitcoin {
    pub network: Network,
    pub node_url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Ethereum {
    pub node_url: url::Url,
}

fn default_log_levels() -> LogLevels {
    LogLevels {
        btsieve: default_log(),
    }
}

impl Settings {
    pub fn read<D: AsRef<OsStr>>(config_file: D) -> Result<Self, ConfigError> {
        let config_file = Path::new(&config_file);

        let mut config = Config::new();
        config.merge(File::from(config_file))?;
        config.try_into()
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use spectral::prelude::*;

    #[test]
    fn can_read_default_config() {
        let settings = Settings::read("./config/btsieve.toml");

        assert_that(&settings).is_ok();
    }

    #[test]
    fn can_read_config_with_bitcoin_missing() -> Result<(), failure::Error> {
        let settings = Settings::read("./config/ethereum_only.toml");

        let settings = settings?;
        assert_that(&settings.ethereum.is_some()).is_true();
        assert_that(&settings.bitcoin.is_some()).is_false();

        Ok(())
    }

    #[test]
    fn can_read_config_with_ethereum_missing() -> Result<(), failure::Error> {
        let settings = Settings::read("./config/bitcoin_only.toml");

        let settings = settings?;
        assert_that(&settings.ethereum.is_some()).is_false();
        assert_that(&settings.bitcoin.is_some()).is_true();

        Ok(())
    }

    #[test]
    fn can_read_config_without_log_levels() -> Result<(), failure::Error> {
        let settings = Settings::read("./config/bitcoin_only.toml");

        let settings = settings?;

        assert_that(&settings.log_levels.btsieve).is_equal_to(LevelFilter::Info);

        Ok(())
    }

    #[test]
    fn can_deserialize_log_level_other_then_default() -> Result<(), failure::Error> {
        let settings = Settings::read("./config/btsieve.toml");

        let settings = settings?;
        assert_that(&settings.log_levels.btsieve).is_equal_to(LevelFilter::Debug);
        assert_that(&settings.bitcoin.is_some()).is_true();

        Ok(())
    }
}
