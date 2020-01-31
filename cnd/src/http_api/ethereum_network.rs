use comit::swap_protocols::ledger::ethereum::ChainId;
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
}

#[derive(Debug, thiserror::Error)]
#[error("chain with id {0} is unknown")]
pub struct UnknownChainId(String);

impl Network {
    pub fn from_chain_id(s: &str) -> Result<Self, UnknownChainId> {
        Ok(match s {
            "1" => Network::Mainnet,
            "3" => Network::Ropsten,
            "17" => Network::Regtest,
            _ => return Err(UnknownChainId(s.to_string())),
        })
    }
}

impl TryFrom<ChainId> for Network {
    type Error = UnknownChainId;

    fn try_from(value: ChainId) -> Result<Self, Self::Error> {
        let value = u32::from(value).to_string();
        Network::from_chain_id(value.as_str())
    }
}

impl From<Network> for ChainId {
    fn from(network: Network) -> Self {
        match network {
            Network::Mainnet => ChainId::mainnet(),
            Network::Regtest => ChainId::regtest(),
            Network::Ropsten => ChainId::ropsten(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use spectral::prelude::*;
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
        assert_that(&Network::from_chain_id("1")).is_ok_containing(Network::Mainnet);
        assert_that(&Network::from_chain_id("3")).is_ok_containing(Network::Ropsten);
        assert_that(&Network::from_chain_id("17")).is_ok_containing(Network::Regtest);
        assert_that(&Network::from_chain_id("-1")).is_err();
    }

    fn assert_display<T: Display>(_t: T) {}

    #[test]
    fn test_derives_display() {
        assert_display(Network::Regtest);
    }
}
