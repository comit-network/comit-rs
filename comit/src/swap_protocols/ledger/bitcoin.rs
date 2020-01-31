use bitcoin::Network;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Mainnet;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Testnet;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Regtest;

pub trait Bitcoin:
    Sized + Default + std::fmt::Debug + std::hash::Hash + Eq + Sync + Copy + Send + 'static
{
    fn network() -> ::bitcoin::Network;
}

impl Bitcoin for Mainnet {
    fn network() -> Network {
        ::bitcoin::Network::Bitcoin
    }
}
impl Bitcoin for Testnet {
    fn network() -> Network {
        ::bitcoin::Network::Testnet
    }
}
impl Bitcoin for Regtest {
    fn network() -> Network {
        ::bitcoin::Network::Regtest
    }
}
