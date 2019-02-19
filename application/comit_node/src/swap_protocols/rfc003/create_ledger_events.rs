use crate::{
    btsieve::{FirstMatch, QueryIdCache},
    swap_protocols::{
        asset::Asset,
        dependencies::LedgerEventDependencies,
        ledger::{Bitcoin, Ethereum},
        rfc003::{
            events::{BtsieveEvents, BtsieveEventsForErc20, LedgerEvents},
            Ledger,
        },
    },
};
use bitcoin_support::BitcoinQuantity;
use ethereum_support::{Erc20Token, EtherQuantity};
use std::sync::Arc;

pub trait CreateLedgerEvents<L: Ledger, A: Asset> {
    fn create_ledger_events(&self) -> Box<dyn LedgerEvents<L, A>>;
}

impl CreateLedgerEvents<Bitcoin, BitcoinQuantity> for LedgerEventDependencies {
    fn create_ledger_events(&self) -> Box<dyn LedgerEvents<Bitcoin, BitcoinQuantity>> {
        Box::new(BtsieveEvents::new(
            QueryIdCache::wrap(Arc::clone(&self.btsieve_client)),
            FirstMatch::new(
                Arc::clone(&self.btsieve_client),
                self.btsieve_bitcoin_poll_interval,
            ),
        ))
    }
}

impl CreateLedgerEvents<Ethereum, EtherQuantity> for LedgerEventDependencies {
    fn create_ledger_events(&self) -> Box<dyn LedgerEvents<Ethereum, EtherQuantity>> {
        Box::new(BtsieveEvents::new(
            QueryIdCache::wrap(Arc::clone(&self.btsieve_client)),
            FirstMatch::new(
                Arc::clone(&self.btsieve_client),
                self.btsieve_ethereum_poll_interval,
            ),
        ))
    }
}

impl CreateLedgerEvents<Ethereum, Erc20Token> for LedgerEventDependencies {
    fn create_ledger_events(&self) -> Box<dyn LedgerEvents<Ethereum, Erc20Token>> {
        Box::new(BtsieveEventsForErc20::new(
            QueryIdCache::wrap(Arc::clone(&self.btsieve_client)),
            FirstMatch::new(
                Arc::clone(&self.btsieve_client),
                self.btsieve_ethereum_poll_interval,
            ),
        ))
    }
}
