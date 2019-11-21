pub mod file;
mod serde_bitcoin_network;
mod serde_duration;
mod settings;

use config as config_rs;
use libp2p::Multiaddr;
use std::{
    ffi::OsStr,
    net::IpAddr,
    path::{Path, PathBuf},
};

pub use self::{
    file::File,
    settings::{AllowedOrigins, Settings},
};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Database {
    pub sqlite: PathBuf,
}

use serde::{Deserialize, Serialize};

impl File {
    pub fn read<D: AsRef<OsStr>>(config_file: D) -> Result<Self, config_rs::ConfigError> {
        let config_file = Path::new(&config_file);

        let mut config = config_rs::Config::new();
        config.merge(config_rs::File::from(config_file))?;
        config.try_into()
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Network {
    pub listen: Vec<Multiaddr>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Socket {
    pub address: IpAddr,
    pub port: u16,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Bitcoin {
    #[serde(with = "crate::config::serde_bitcoin_network")]
    pub network: bitcoin::Network,
    #[serde(with = "url_serde")]
    pub node_url: reqwest::Url,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Ethereum {
    #[serde(with = "url_serde")]
    pub node_url: reqwest::Url,
}
