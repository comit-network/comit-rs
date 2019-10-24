use crate::swap_protocols::ledger::{Ledger, LedgerKind};
use ethereum_support::{Address, ChainId, Network, Transaction};
use serde::Deserialize;

/// `network` is only kept for backward compatibility with client
/// and must be removed with issue #TODO
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Hash)]
pub struct Ethereum {
    pub network: Network,
    pub chain_id: ChainId,
}

impl Ethereum {
    pub fn new(chain: ChainId) -> Self {
        Ethereum {
            network: chain.into(),
            chain_id: chain,
        }
    }
}

impl Default for Ethereum {
    fn default() -> Self {
        Ethereum {
            network: ChainId::regtest().into(),
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
