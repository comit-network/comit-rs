use bitcoin;
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
    strum_macros::EnumString,
    strum_macros::Display,
)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    #[strum(serialize = "main")]
    Mainnet,
    #[strum(serialize = "regtest")]
    Regtest,
    #[strum(serialize = "test")]
    Testnet,
}

impl From<bitcoin::network::constants::Network> for Network {
    fn from(item: bitcoin::network::constants::Network) -> Network {
        match item {
            bitcoin::network::constants::Network::Bitcoin => Network::Mainnet,
            bitcoin::network::constants::Network::Regtest => Network::Regtest,
            bitcoin::network::constants::Network::Testnet => Network::Testnet,
        }
    }
}

impl From<Network> for bitcoin::Network {
    fn from(item: Network) -> bitcoin::Network {
        match item {
            Network::Regtest => bitcoin::Network::Regtest,
            Network::Testnet => bitcoin::Network::Testnet,
            Network::Mainnet => bitcoin::Network::Bitcoin,
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
        let testnet: &'static str = Network::Testnet.into();

        assert_eq!(mainnet, "main");
        assert_eq!(regtest, "regtest");
        assert_eq!(testnet, "test");
    }

    fn assert_display<T: Display>(_t: T) {}

    #[test]
    fn test_derives_display() {
        assert_display(Network::Regtest);
    }
}
