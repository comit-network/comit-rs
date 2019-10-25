use crate::swap_protocols::ledger::{Ledger, LedgerKind};
use ethereum_support::{Address, Transaction};
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

impl From<ChainId> for u32 {
    fn from(chain_id: ChainId) -> Self {
        chain_id.0
    }
}
