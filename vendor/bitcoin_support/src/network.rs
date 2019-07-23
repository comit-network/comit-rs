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
    Main,
    #[strum(serialize = "regtest")]
    Regtest,
    #[strum(serialize = "test")]
    Test,
}

impl From<bitcoin::network::constants::Network> for Network {
    fn from(item: bitcoin::network::constants::Network) -> Network {
        match item {
            bitcoin::network::constants::Network::Bitcoin => Network::Main,
            bitcoin::network::constants::Network::Regtest => Network::Regtest,
            bitcoin::network::constants::Network::Testnet => Network::Test,
        }
    }
}

impl From<Network> for bitcoin::network::constants::Network {
    fn from(item: Network) -> bitcoin::network::constants::Network {
        match item {
            Network::Main => bitcoin::network::constants::Network::Bitcoin,
            Network::Regtest => bitcoin::network::constants::Network::Regtest,
            Network::Test => bitcoin::network::constants::Network::Testnet,
        }
    }
}

impl From<Network> for bitcoin_bech32::constants::Network {
    fn from(item: Network) -> bitcoin_bech32::constants::Network {
        match item {
            Network::Regtest => bitcoin_bech32::constants::Network::Regtest,
            Network::Test => bitcoin_bech32::constants::Network::Testnet,
            Network::Main => bitcoin_bech32::constants::Network::Bitcoin,
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
        let mainnet: &'static str = Network::Main.into();
        let regtest: &'static str = Network::Regtest.into();
        let testnet: &'static str = Network::Test.into();

        assert_eq!(mainnet, "main");
        assert_eq!(regtest, "regtest");
        assert_eq!(testnet, "test");
    }

    fn assert_display<T: Display>(t: T) {}

    #[test]
    fn test_derives_display() {
        assert_display(Network::Regtest);
    }

    #[test]
    fn string_serialize_using_serde() {
        serialize_and_compare(Network::Main, "\"main\"");
        serialize_and_compare(Network::Test, "\"test\"");
        serialize_and_compare(Network::Regtest, "\"regtest\"");
    }

    fn serialize_and_compare(network: Network, expected_json: &str) {
        let actual_json = serde_json::to_string(&network);

        assert_that(&actual_json).is_ok_containing(expected_json.to_string());
    }
}
