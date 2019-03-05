use bitcoin;
use bitcoin_bech32;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    Mainnet,
    Regtest,
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
