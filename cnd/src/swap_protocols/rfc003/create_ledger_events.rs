use crate::swap_protocols::{
    asset::Asset,
    ledger::{Bitcoin, Ethereum},
    rfc003::{
        events::{LedgerEventFutures, LedgerEvents},
        Ledger,
    },
    LedgerConnectors,
};
use bitcoin::Amount;
use ethereum_support::{Erc20Token, EtherQuantity};

pub trait CreateLedgerEvents<L: Ledger, A: Asset> {
    fn create_ledger_events(&self) -> Box<dyn LedgerEvents<L, A>>;
}

impl CreateLedgerEvents<Bitcoin, Amount> for LedgerConnectors {
    fn create_ledger_events(&self) -> Box<dyn LedgerEvents<Bitcoin, Amount>> {
        Box::new(LedgerEventFutures::new(Box::new(
            self.bitcoin_connector.clone(),
        )))
    }
}

impl CreateLedgerEvents<Ethereum, EtherQuantity> for LedgerConnectors {
    fn create_ledger_events(&self) -> Box<dyn LedgerEvents<Ethereum, EtherQuantity>> {
        Box::new(LedgerEventFutures::new(Box::new(
            self.ethereum_connector.clone(),
        )))
    }
}

impl CreateLedgerEvents<Ethereum, Erc20Token> for LedgerConnectors {
    fn create_ledger_events(&self) -> Box<dyn LedgerEvents<Ethereum, Erc20Token>> {
        Box::new(LedgerEventFutures::new(Box::new(
            self.ethereum_connector.clone(),
        )))
    }
}
