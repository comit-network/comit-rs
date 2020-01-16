mod bitcoin;
pub mod ethereum;

pub use self::{bitcoin::Bitcoin, ethereum::Ethereum};

use derivative::Derivative;

#[derive(Clone, Copy, Derivative, PartialEq)]
#[derivative(Debug = "transparent")]
pub enum LedgerKind {
    Bitcoin(Bitcoin),
    Ethereum(Ethereum),
}
