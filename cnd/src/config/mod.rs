pub mod file;
mod serde_bitcoin_network;
mod serde_duration;
mod settings;

use libp2p::Multiaddr;
use serde::{Deserialize, Serialize};
use std::{net::IpAddr, path::PathBuf};

pub use self::{
    file::File,
    settings::{AllowedOrigins, Settings},
};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Database {
    pub sqlite: PathBuf,
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
