use crate::comit_api::LedgerKind;

macro_rules! declare_ledger {
    ($ledger:ident) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub struct $ledger;
    };
}
declare_ledger!(Mainnet);
declare_ledger!(Testnet);
declare_ledger!(Regtest);

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

macro_rules! impl_network {
    ($ledger:ty, $network:expr) => {
        impl Network for $ledger {
            fn network() -> bitcoin::Network {
                $network
            }
        }
    };
}
impl_network!(Mainnet, bitcoin::Network::Bitcoin);
impl_network!(Testnet, bitcoin::Network::Testnet);
impl_network!(Regtest, bitcoin::Network::Regtest);

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
