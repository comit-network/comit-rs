use crate::swap_protocols::ledger::{Ledger, LedgerKind};
use bitcoin_support::{Network, Transaction};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Bitcoin {
    pub network: Network,
}

impl Bitcoin {
    pub fn new(network: Network) -> Self {
        Bitcoin { network }
    }
}

impl Default for Bitcoin {
    fn default() -> Self {
        Bitcoin {
            network: Network::Regtest,
        }
    }
}

impl Ledger for Bitcoin {
    type Identity = crate::bitcoin::PublicKey;
    type Transaction = Transaction;
}

impl From<Bitcoin> for LedgerKind {
    fn from(bitcoin: Bitcoin) -> Self {
        LedgerKind::Bitcoin(bitcoin)
    }
}
