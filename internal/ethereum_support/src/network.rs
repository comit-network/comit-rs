use serde::{Deserialize, Serialize};

#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    Deserialize,
    Serialize,
    Hash,
    strum_macros::IntoStaticStr,
    strum_macros::Display,
)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    #[strum(serialize = "mainnet")]
    Mainnet,
    #[strum(serialize = "regtest")]
    Regtest,
    #[strum(serialize = "ropsten")]
    Ropsten,
    #[strum(serialize = "unknown")]
    Unknown,
}

impl Network {
    pub fn from_network_id(s: String) -> Self {
        match s.as_str() {
            "1" => Network::Mainnet,
            "3" => Network::Ropsten,
            "17" => Network::Regtest,
            _ => Network::Unknown,
        }
    }
}
