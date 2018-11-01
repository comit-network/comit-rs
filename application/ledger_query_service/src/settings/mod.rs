mod serde;

use bitcoin_support::Network;
use config::{Config, ConfigError, File};
use serde::Deserialize;
use std::{ffi::OsStr, path::Path};
use url;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub bitcoin: Bitcoin,
    //   pub http_api: HttpApi,
}

#[derive(Debug, Deserialize)]
pub struct Bitcoin {
    pub network: Network,
    #[serde(with = "serde::url")]
    pub node_url: url::Url,
    pub node_username: String,
    pub node_password: String,
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
