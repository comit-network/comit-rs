use crate::swap_protocols::ledger::LedgerKind;
use bitcoin::Network;

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

impl From<Bitcoin> for LedgerKind {
    fn from(bitcoin: Bitcoin) -> Self {
        LedgerKind::Bitcoin(bitcoin)
    }
}
