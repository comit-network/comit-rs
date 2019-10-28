use crate::swap_protocols::ledger::ethereum::ChainId;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

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

impl From<ChainId> for Network {
    fn from(chain: ChainId) -> Self {
        Network::from_network_id(u32::from(chain).to_string())
    }
}

impl TryFrom<Network> for ChainId {
    type Error = ();

    fn try_from(network: Network) -> Result<Self, ()> {
        match network {
            Network::Mainnet => Ok(ChainId::mainnet()),
            Network::Regtest => Ok(ChainId::regtest()),
            Network::Ropsten => Ok(ChainId::ropsten()),
            Network::Unknown => Err(()),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::fmt::Display;

    #[test]
    fn string_serialize() {
        let mainnet: &'static str = Network::Mainnet.into();
        let regtest: &'static str = Network::Regtest.into();
        let ropsten: &'static str = Network::Ropsten.into();

        assert_eq!(mainnet, "mainnet");
        assert_eq!(regtest, "regtest");
        assert_eq!(ropsten, "ropsten");
    }

    #[test]
    fn from_version() {
        assert_eq!(
            Network::from_network_id(String::from("1")),
            Network::Mainnet
        );
        assert_eq!(
            Network::from_network_id(String::from("3")),
            Network::Ropsten
        );
        assert_eq!(
            Network::from_network_id(String::from("17")),
            Network::Regtest
        );
        assert_eq!(
            Network::from_network_id(String::from("-1")),
            Network::Unknown
        );
    }

    fn assert_display<T: Display>(_t: T) {}

    #[test]
    fn test_derives_display() {
        assert_display(Network::Regtest);
    }
}
