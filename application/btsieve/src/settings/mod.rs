mod serde_duration;
mod serde_log;

use config::{Config, ConfigError, File};
use log::LevelFilter;
use serde::Deserialize;
use std::{ffi::OsStr, net::IpAddr, path::Path, time::Duration};

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    #[serde(with = "self::serde_log", default = "default_log")]
    pub log_level: LevelFilter,
    pub http_api: HttpApi,
    pub bitcoin: Option<Bitcoin>,
    pub ethereum: Option<Ethereum>,
}

fn default_log() -> LevelFilter {
    LevelFilter::Info
}

#[derive(Debug, Deserialize, Clone)]
pub struct HttpApi {
    pub address_bind: IpAddr,
    pub port_bind: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Bitcoin {
    pub zmq_endpoint: String,
    #[serde(with = "url_serde")]
    pub node_url: url::Url,
    pub node_username: String,
    pub node_password: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Ethereum {
    #[serde(with = "url_serde")]
    pub node_url: url::Url,
    #[serde(with = "serde_duration")]
    pub poll_interval_secs: Duration,
}

impl Settings {
    pub fn read<D: AsRef<OsStr>>(config_file: D) -> Result<Self, ConfigError> {
        let mut config = Config::new();

        let config_file = Path::new(&config_file);

        config.merge(File::from(config_file))?;

        // You can deserialize (and thus freeze) the entire configuration as
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
    fn can_deserialize_log_level() -> Result<(), failure::Error> {
        let settings = Settings::read("./config/btsieve.toml");

        let settings = settings?;
        assert_that(&settings.log_level).is_equal_to(LevelFilter::Info);
        assert_that(&settings.bitcoin.is_some()).is_true();

        Ok(())
    }

}
