pub mod bitcoin;
pub mod ethereum;

pub use self::{bitcoin::Bitcoin, ethereum::Ethereum};

use derivative::Derivative;

// TODO: Move this were it is used (comit-api)
#[derive(Clone, Copy, Derivative, PartialEq)]
#[derivative(Debug = "transparent")]
pub enum LedgerKind {
    BitcoinMainnet,
    BitcoinTestnet,
    BitcoinRegtest,
    Ethereum(Ethereum),
}
