use crate::ethereum::ChainId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Bitcoin {
    Mainnet,
    Testnet,
    Regtest,
}

impl Default for Bitcoin {
    fn default() -> Self {
        Self::Regtest
    }
}

impl From<Bitcoin> for ::bitcoin::Network {
    fn from(bitcoin: Bitcoin) -> ::bitcoin::Network {
        match bitcoin {
            Bitcoin::Mainnet => ::bitcoin::Network::Bitcoin,
            Bitcoin::Testnet => ::bitcoin::Network::Testnet,
            Bitcoin::Regtest => ::bitcoin::Network::Regtest,
        }
    }
}

impl From<::bitcoin::Network> for Bitcoin {
    fn from(network: ::bitcoin::Network) -> Self {
        match network {
            bitcoin::Network::Bitcoin => Bitcoin::Mainnet,
            bitcoin::Network::Testnet => Bitcoin::Testnet,
            bitcoin::Network::Regtest => Bitcoin::Regtest,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Ethereum {
    pub chain_id: ChainId,
}

impl Ethereum {
    pub fn new(chain: ChainId) -> Self {
        Ethereum { chain_id: chain }
    }
}

impl Default for Ethereum {
    fn default() -> Self {
        Ethereum {
            chain_id: ChainId::regtest(),
        }
    }
}
