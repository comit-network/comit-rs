use config::{Config, ConfigError, File};
use std::{
    self,
    env::{self, var},
    path::Path,
};

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
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = Config::new();

        let config_path = var_or_exit("COMIT_NODE_CONFIG_PATH");
        let default_settings = format!("{}/{}", config_path.trim(), "default");
        //"./application/comit_node/config/default.toml"

        let default_config_file = Path::new(default_settings.as_str());

        // Add in the current environment file
        // Default to 'development' env
        // Note that this file is optional, and can be used to hold keys by run_mode
        let env = env::var("RUN_MODE").unwrap_or("development".into()); //add new file for the keysc
        let environment_settings = format!("{}/{}", config_path.trim(), env);
        let environment_config_file = Path::new(environment_settings.as_str());

        // Start off by merging in the "default" configuration file
        s.merge(File::from(default_config_file))?;

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

fn var_or_exit(name: &str) -> String {
    match var(name) {
        Ok(value) => {
            info!("Set {}={}", name, value);
            value
        }
        Err(_) => {
            eprintln!("{} is not set but is required", name);
            std::process::exit(1)
        }
    }
}
