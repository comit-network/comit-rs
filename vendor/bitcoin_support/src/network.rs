use bitcoin::network::constants::Network as ExternNetwork;
use serde::Deserialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    Mainnet,
    Regtest,
    Testnet,
}

impl From<ExternNetwork> for Network {
    fn from(item: ExternNetwork) -> Network {
        match item {
            ExternNetwork::Bitcoin => Network::Mainnet,
            ExternNetwork::Regtest => Network::Regtest,
            ExternNetwork::Testnet => Network::Testnet,
        }
    }
}

impl From<Network> for ExternNetwork {
    fn from(item: Network) -> ExternNetwork {
        match item {
            Network::Mainnet => ExternNetwork::Bitcoin,
            Network::Regtest => ExternNetwork::Regtest,
            Network::Testnet => ExternNetwork::Testnet,
        }
    }
}
