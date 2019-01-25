use crate::{
    ledger_query_service::{FirstMatch, QueryIdCache},
    swap_protocols::{
        asset::Asset,
        dependencies::LedgerEventDependencies,
        ledger::{Bitcoin, Ethereum},
        rfc003::{
            events::{LedgerEvents, LqsEvents, LqsEventsForErc20},
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
        Box::new(LqsEvents::new(
            QueryIdCache::wrap(Arc::clone(&self.lqs_client)),
            FirstMatch::new(Arc::clone(&self.lqs_client), self.lqs_bitcoin_poll_interval),
        ))
    }
}

impl CreateLedgerEvents<Ethereum, EtherQuantity> for LedgerEventDependencies {
    fn create_ledger_events(&self) -> Box<dyn LedgerEvents<Ethereum, EtherQuantity>> {
        Box::new(LqsEvents::new(
            QueryIdCache::wrap(Arc::clone(&self.lqs_client)),
            FirstMatch::new(
                Arc::clone(&self.lqs_client),
                self.lqs_ethereum_poll_interval,
            ),
        ))
    }
}

impl CreateLedgerEvents<Ethereum, Erc20Token> for LedgerEventDependencies {
    fn create_ledger_events(&self) -> Box<dyn LedgerEvents<Ethereum, Erc20Token>> {
        Box::new(LqsEventsForErc20::new(
            QueryIdCache::wrap(Arc::clone(&self.lqs_client)),
            FirstMatch::new(
                Arc::clone(&self.lqs_client),
                self.lqs_ethereum_poll_interval,
            ),
        ))
    }
}
