mod serde;

use bitcoin_support::{ExtendedPrivKey, Network};
use config::{Config, ConfigError, File};
use ethereum_support;
use secp256k1_support::KeyPair;
use serde::Deserialize;
use std::{ffi::OsStr, net::SocketAddr, path::Path, time::Duration};
use url;

#[derive(Debug, Deserialize)]
pub struct ComitNodeSettings {
    pub ethereum: Ethereum,
    pub bitcoin: Bitcoin,
    pub swap: Swap,
    pub comit: Comit,
    pub http_api: HttpApi,
    pub ledger_query_service: LedgerQueryService,
}

#[derive(Debug, Deserialize)]
pub struct Ethereum {
    pub network_id: u8,
    #[serde(with = "serde::url")]
    pub node_url: url::Url,
    pub gas_price: u64,
    #[serde(with = "serde::keypair")]
    // TODO: Replace with mnemonics and derive keys. See #185
    pub private_key: KeyPair,
}

#[derive(Debug, Deserialize)]
pub struct Bitcoin {
    pub network: Network,
    pub satoshi_per_byte: f64,
    #[serde(with = "serde::url")]
    pub node_url: url::Url,
    pub node_username: String,
    pub node_password: String,
    #[serde(with = "serde::extended_privkey")]
    pub extended_private_key: ExtendedPrivKey,
}

#[derive(Debug, Deserialize)]
pub struct Swap {
    //TODO this should be generated on the fly per swap from the ethereum private key with #185
    pub eth_refund_address: ethereum_support::Address,
}

#[derive(Debug, Deserialize)]
pub struct Comit {
    #[serde(with = "serde::socket_addr")]
    pub remote_comit_node_url: SocketAddr,
    #[serde(with = "serde::socket_addr")]
    pub comit_listen: SocketAddr,
}

#[derive(Debug, Deserialize)]
pub struct HttpApi {
    #[serde(with = "serde::socket_addr")]
    pub socket_address: SocketAddr,
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
    pub fn new<D: AsRef<OsStr>, R: AsRef<OsStr>>(
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

    #[test]
    fn can_read_default_config() {
        let settings = ComitNodeSettings::new("./config/default.toml", "./config/development.toml");

        assert_that(&settings).is_ok();
    }

    #[test]
    fn can_read_nested_parameters() {
        let settings = ComitNodeSettings::new("./config/default.toml", "./config/development.toml");

        assert_that(&settings).is_ok();
        assert_that(
            &settings
                .unwrap()
                .ledger_query_service
                .ethereum
                .poll_interval_secs,
        ).is_equal_to(&Duration::from_secs(20));
    }

}
