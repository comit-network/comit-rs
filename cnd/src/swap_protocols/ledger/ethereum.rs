use crate::swap_protocols::ledger::{Ledger, LedgerKind};
use ethereum_support::{Address, Network, Transaction};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Ethereum {
    pub network: Network,
}

impl Ethereum {
    pub fn new(network: Network) -> Self {
        Ethereum { network }
    }
}

impl Default for Ethereum {
    fn default() -> Self {
        Ethereum {
            network: Network::Regtest,
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
