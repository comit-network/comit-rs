mod serde;

use bitcoin_support::Network;
use config::{Config, ConfigError, File};
use std::{ffi::OsStr, net::IpAddr, path::Path, time::Duration};
use url;

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub http_api: HttpApi,
    pub bitcoin: Option<Bitcoin>,
    pub ethereum: Option<Ethereum>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct HttpApi {
    pub address_bind: IpAddr,
    pub port_bind: u16,
    #[serde(with = "serde::url")]
    pub external_url: url::Url,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Bitcoin {
    pub network: Network,
    pub zmq_endpoint: String,
    // Below could be options
    #[serde(with = "serde::url")]
    pub node_url: url::Url,
    pub node_username: String,
    pub node_password: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Ethereum {
    #[serde(with = "serde::url")]
    pub node_url: url::Url,
    #[serde(with = "serde::duration")]
    pub poll_interval_secs: Duration,
}

impl Settings {
    pub fn new<D: AsRef<OsStr>>(default_config: D) -> Result<Self, ConfigError> {
        let mut config = Config::new();

        let default_config_file = Path::new(&default_config);

        // Start off by merging in the "default" configuration file
        config.merge(File::from(default_config_file))?;

        // Add in a local configuration file
        // This file shouldn't be checked in to git
        config.merge(File::with_name("config/local").required(false))?;

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
        let settings = Settings::new("./config/default.toml");

        assert_that(&settings).is_ok();
    }

}
