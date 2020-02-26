use crate::comit_api::LedgerKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Mainnet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Testnet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Regtest;

pub trait Bitcoin:
    Sized + std::fmt::Debug + std::hash::Hash + Eq + Sync + Copy + Send + 'static
{
}

pub trait Network {
    fn network() -> ::bitcoin::Network;
}

impl Bitcoin for Mainnet {}

impl From<Mainnet> for LedgerKind {
    fn from(_: Mainnet) -> Self {
        LedgerKind::BitcoinMainnet
    }
}
impl Network for Mainnet {
    fn network() -> ::bitcoin::Network {
        ::bitcoin::Network::Bitcoin
    }
}
impl Bitcoin for Testnet {}

impl From<Testnet> for LedgerKind {
    fn from(_: Testnet) -> Self {
        LedgerKind::BitcoinTestnet
    }
}
impl Network for Testnet {
    fn network() -> ::bitcoin::Network {
        ::bitcoin::Network::Testnet
    }
}

impl Bitcoin for Regtest {}
impl From<Regtest> for LedgerKind {
    fn from(_: Regtest) -> Self {
        LedgerKind::BitcoinRegtest
    }
}
impl Network for Regtest {
    fn network() -> ::bitcoin::Network {
        ::bitcoin::Network::Regtest
    }
}
