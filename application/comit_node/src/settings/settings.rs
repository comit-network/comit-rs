use config::{Config, ConfigError, File};
use std::{
    path::Path,
};

#[derive(Debug, Deserialize)]
pub struct Ethereum {
    pub network_id: u8,
    pub node_url: String,
    pub gas_price: u64,
    pub private_key: String,
}

#[derive(Debug, Deserialize)]
pub struct Bitcoin {
    pub network_id: String,
    pub satoshi_per_byte: f64,
    pub node_url: String,
    pub node_username: String,
    pub node_password: String,
    pub private_key: String,
}

#[derive(Debug, Deserialize)]
pub struct Swap {
    pub btc_redeem_address: String, //TODO this should be generated on the fly per swap from the master key
    pub eth_refund_address: String, //TODO this should be generated on the fly per swap from the master key
}

#[derive(Debug, Deserialize)]
pub struct Comit {
    pub remote_comit_node_url: String,
    pub comit_listen: String,
}

#[derive(Debug, Deserialize)]
pub struct ComitNodeSettings {
    pub ethereum: Ethereum,
    pub bitcoin: Bitcoin,
    pub swap: Swap,
    pub comit: Comit,
}

impl ComitNodeSettings {
    pub fn new(default_config : String, run_mode_config: String) -> Result<Self, ConfigError> {
        let mut config = Config::new();

        let default_config_file = Path::new(default_config.as_str());

        // Add in the current environment file
        // Note that this file is optional, and can be used to hold keys by run_mode
        let environment_config_file = Path::new(run_mode_config.as_str());

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
