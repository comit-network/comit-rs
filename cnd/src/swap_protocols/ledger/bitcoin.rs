use crate::comit_api::LedgerKind;
use bitcoin::Network;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Mainnet;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Testnet;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Regtest;

pub trait Bitcoin:
    Sized + Default + std::fmt::Debug + std::hash::Hash + Eq + Sync + Copy + Send
{
    fn into_ledger_kind(self) -> LedgerKind;
    fn network() -> ::bitcoin::Network;
}

impl Bitcoin for Mainnet {
    fn into_ledger_kind(self) -> LedgerKind {
        LedgerKind::BitcoinMainnet
    }

    fn network() -> Network {
        ::bitcoin::Network::Bitcoin
    }
}
impl Bitcoin for Testnet {
    fn into_ledger_kind(self) -> LedgerKind {
        LedgerKind::BitcoinTestnet
    }

    fn network() -> Network {
        ::bitcoin::Network::Testnet
    }
}
impl Bitcoin for Regtest {
    fn into_ledger_kind(self) -> LedgerKind {
        LedgerKind::BitcoinRegtest
    }

    fn network() -> Network {
        ::bitcoin::Network::Regtest
    }
}

impl<B: Bitcoin> From<B> for LedgerKind {
    fn from(ledger: B) -> Self {
        ledger.into_ledger_kind()
    }
}
