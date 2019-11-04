use btsieve::{bitcoin::BitcoindConnector, ethereum::Web3Connector};
use std::fmt;

#[allow(missing_debug_implementations)]
#[derive(Clone)]
pub struct LedgerConnectors {
    pub bitcoin_connector: BitcoindConnector,
    pub ethereum_connector: Web3Connector,
}

impl fmt::Debug for LedgerConnectors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<Ledger Event Dependencies>")
    }
}
