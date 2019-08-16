use bitcoin;
use bitcoin_bech32;
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

impl From<Network> for bitcoin::network::constants::Network {
    fn from(item: Network) -> bitcoin::network::constants::Network {
        match item {
            Network::Mainnet => bitcoin::network::constants::Network::Bitcoin,
            Network::Regtest => bitcoin::network::constants::Network::Regtest,
            Network::Testnet => bitcoin::network::constants::Network::Testnet,
        }
    }
}

impl From<Network> for bitcoin_bech32::constants::Network {
    fn from(item: Network) -> bitcoin_bech32::constants::Network {
        match item {
            Network::Regtest => bitcoin_bech32::constants::Network::Regtest,
            Network::Testnet => bitcoin_bech32::constants::Network::Testnet,
            Network::Mainnet => bitcoin_bech32::constants::Network::Bitcoin,
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
