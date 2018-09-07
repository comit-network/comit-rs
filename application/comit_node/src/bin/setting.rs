use config::{Config, ConfigError, Environment, File};
use std::{env, path::Path};

#[derive(Debug, Deserialize)]
pub struct Ethereum {
    pub network_id: String,
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
    pub btc_bob_redeem_address: String,
    pub eth_bob_refund_address: String,
    pub alice_refund_address: String,
    pub alice_success_address: String,
    pub alice_sender_address: String,
}

#[derive(Debug, Deserialize)]
pub struct Comit {
    pub remote_comit_node_url: String,
    pub comit_listen: String,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub ethereum: Ethereum,
    pub bitcoin: Bitcoin,
    pub swap: Swap,
    pub comit: Comit,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = Config::new();

        let defaul_config_file = Path::new("./application/comit_node/config/default.toml");
        // Add in the current environment file
        // Default to 'development' env
        // Note that this file is _optional, in our case this holds all the keys
        let env = env::var("RUN_MODE").unwrap_or("development".into()); //add new file for the keys
        let path = format!("./application/comit_node/config/{}", env);
        let environment_config_file = Path::new(path.as_str());

        // Start off by merging in the "default" configuration file
        s.merge(File::from(defaul_config_file))?;

        // Add in the current environment file
        // Default to 'development' env
        // Note that this file is _optional, in our case this holds all the keys
        s.merge(File::from(environment_config_file).required(false))?;

        // Add in a local configuration file
        // This file shouldn't be checked in to git
        s.merge(File::with_name("config/local").required(false))?;

        // You can deserialize (and thus freeze) the entire configuration as
        s.try_into()
    }
}
