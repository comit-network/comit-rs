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

macro_rules! impl_from_for_ledgerkind {
    ($ledger:ty, $ledger_kind:expr) => {
        impl From<$ledger> for LedgerKind {
            fn from(_: Mainnet) -> Self {
                $ledger_kind
            }
        }
    };
}
impl_from_for_ledgerkind!(Mainnet, LedgerKind::BitcoinMainnet);
impl_from_for_ledgerkind!(Testnet, LedgerKind::BitcoinTestnet);
impl_from_for_ledgerkind!(Regtest, LedgerKind::BitcoinRegtest);
