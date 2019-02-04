mod serde;

use crate::seed::Seed;
use config::{Config, ConfigError, File};
use log::LevelFilter;

use std::{
    ffi::OsStr,
    net::{IpAddr, SocketAddr},
    path::Path,
    time::Duration,
};

#[derive(Debug, Deserialize)]
pub struct ComitNodeSettings {
    pub comit: Comit,
    pub http_api: HttpApi,
    pub ledger_query_service: LedgerQueryService,
    #[serde(with = "serde::log_level", default = "default_log")]
    pub log_level: LevelFilter,
}

fn default_log() -> LevelFilter {
    LevelFilter::Debug
}

#[derive(Debug, Deserialize)]
pub struct Comit {
    pub comit_listen: SocketAddr,
    pub secret_seed: Seed,
}

#[derive(Debug, Deserialize)]
pub struct HttpApi {
    pub address: IpAddr,
    pub port: u16,
    pub logging: bool,
}

#[derive(Debug, Deserialize)]
pub struct LedgerQueryService {
    #[serde(with = "serde::url")]
    pub url: url::Url,
    pub bitcoin: PollParameters,
    pub ethereum: PollParameters,
}

#[derive(Debug, Deserialize)]
pub struct PollParameters {
    #[serde(with = "serde::duration")]
    pub poll_interval_secs: Duration,
}

impl ComitNodeSettings {
    pub fn create<D: AsRef<OsStr>, R: AsRef<OsStr>>(
        default_config: D,
        run_mode_config: R,
    ) -> Result<Self, ConfigError> {
        let mut config = Config::new();

        let default_config_file = Path::new(&default_config);

        // Add in the current environment file
        // Note that this file is optional, and can be used to hold keys by run_mode
        let environment_config_file = Path::new(&run_mode_config);

        // Start off by merging in the "default" configuration file
        config.merge(File::from(default_config_file))?;

        // Add in the current environment file
        // Default to 'development' env
        // Note that this file is _optional, in our case this holds all the keys
        config.merge(File::from(environment_config_file).required(false))?;

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

    fn comit_settings() -> Result<ComitNodeSettings, ConfigError> {
        ComitNodeSettings::create("./config/default.toml", "./config/development.toml")
    }

    #[test]
    fn can_read_default_config() {
        let settings = comit_settings();

        assert_that(&settings).is_ok();
    }

    #[test]
    fn can_read_nested_parameters() {
        let settings = comit_settings();

        assert_that(&settings).is_ok();
        assert_that(
            &settings
                .unwrap()
                .ledger_query_service
                .ethereum
                .poll_interval_secs,
        )
        .is_equal_to(&Duration::from_secs(20));
    }

}
