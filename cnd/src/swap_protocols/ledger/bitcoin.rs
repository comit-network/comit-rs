use crate::comit_api::LedgerKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Mainnet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Testnet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Regtest;

pub trait Bitcoin:
    Sized + std::fmt::Debug + std::hash::Hash + Eq + Sync + Copy + Send + Into<LedgerKind> + 'static
{
}

impl Bitcoin for Mainnet {}
impl Bitcoin for Testnet {}
impl Bitcoin for Regtest {}

pub trait Network {
    fn network() -> bitcoin::Network;
}

impl Network for Mainnet {
    fn network() -> bitcoin::Network {
        bitcoin::Network::Bitcoin
    }
}
impl Network for Testnet {
    fn network() -> bitcoin::Network {
        bitcoin::Network::Testnet
    }
}
impl Network for Regtest {
    fn network() -> bitcoin::Network {
        bitcoin::Network::Regtest
    }
}

impl From<Mainnet> for LedgerKind {
    fn from(_: Mainnet) -> Self {
        LedgerKind::BitcoinMainnet
    }
}

impl From<Testnet> for LedgerKind {
    fn from(_: Testnet) -> Self {
        LedgerKind::BitcoinTestnet
    }
}

impl From<Regtest> for LedgerKind {
    fn from(_: Regtest) -> Self {
        LedgerKind::BitcoinRegtest
    }
}
