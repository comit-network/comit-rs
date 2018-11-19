mod serde;

use bitcoin_support;
use config::{Config, ConfigError, File};
use ethereum_support;
use secp256k1_support::KeyPair;
use serde::Deserialize;
use std::{
    ffi::OsStr,
    net::{IpAddr, SocketAddr},
    path::Path,
    time::Duration,
};
use url;

#[derive(Debug, Deserialize)]
pub struct ComitNodeSettings {
    pub ethereum: Ethereum,
    pub bitcoin: Bitcoin,
    pub swap: Swap,
    pub comit: Comit,
    pub http_api: HttpApi,
    pub ledger_query_service: LedgerQueryService,
    pub tokens: Vec<ethereum_support::Erc20Token>,
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
    pub network: bitcoin_support::Network,
    pub satoshi_per_byte: f64,
    #[serde(with = "serde::url")]
    pub node_url: url::Url,
    pub node_username: String,
    pub node_password: String,
    #[serde(with = "serde::extended_privkey")]
    pub extended_private_key: bitcoin_support::ExtendedPrivKey,
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
    pub fn new<D: AsRef<OsStr>, R: AsRef<OsStr>, S: AsRef<OsStr>>(
        default_config: D,
        run_mode_config: R,
        erc20_config: S,
    ) -> Result<Self, ConfigError> {
        let mut config = Config::new();

        let default_config_file = Path::new(&default_config);

        // Add in the current environment file
        // Note that this file is optional, and can be used to hold keys by run_mode
        let environment_config_file = Path::new(&run_mode_config);

        // Create erc20 token config file path
        let erc20_config_file = Path::new(&erc20_config);

        // Start off by merging in the "default" configuration file
        config.merge(File::from(default_config_file))?;

        // Add in the current environment file
        // Default to 'development' env
        // Note that this file is _optional, in our case this holds all the keys
        config.merge(File::from(environment_config_file).required(false))?;

        // Load erc20 token config file
        config.merge(File::from(erc20_config_file))?;

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
    use std::str::FromStr;

    fn comit_settings() -> Result<ComitNodeSettings, ConfigError> {
        ComitNodeSettings::new(
            "./config/default.toml",
            "./config/development.toml",
            "./config/erc20.toml",
        )
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

    #[test]
    fn can_get_erc20token_list_from_config_file() {
        let settings = comit_settings();

        assert_that(&settings).is_ok();
        let settings = settings.unwrap();

        assert_that(&settings.tokens.len()).is_equal_to(1);
        let token = &settings.tokens[0];
        assert_that(token).is_equal_to(ethereum_support::Erc20Token {
            symbol: String::from("PAY"),
            decimals: 18,
            address: ethereum_support::Address::from_str(
                "B97048628DB6B661D4C2aA833e95Dbe1A905B280",
            )
            .unwrap(),
        });
    }

}
