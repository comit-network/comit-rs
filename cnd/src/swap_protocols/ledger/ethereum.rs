use crate::swap_protocols::ledger::{Ledger, LedgerKind};
use core::convert::TryFrom;
use ethereum_support::{Address, Network, Transaction};
use serde::{Deserialize, Serialize};

/// `network` is only kept for backward compatibility with client
/// and must be removed with issue #TODO
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Hash)]
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

impl Ledger for Ethereum {
    type Identity = Address;
    type Transaction = Transaction;
}

impl From<Ethereum> for LedgerKind {
    fn from(ethereum: Ethereum) -> Self {
        LedgerKind::Ethereum(ethereum)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct ChainId(u32);

impl ChainId {
    pub fn new(chain_id: u32) -> ChainId {
        ChainId(chain_id)
    }
    pub fn mainnet() -> ChainId {
        ChainId(1)
    }

    pub fn ropsten() -> ChainId {
        ChainId(3)
    }

    pub fn regtest() -> ChainId {
        ChainId(17)
    }
}

impl From<ChainId> for Network {
    fn from(chain: ChainId) -> Self {
        Network::from_network_id(chain.0.to_string())
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
