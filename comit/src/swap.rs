pub mod actions;
pub mod hbit_herc20;
pub mod herc20_hbit;

pub use crate::{ethereum, *};
pub use hbit_herc20::{hbit_herc20_alice, hbit_herc20_bob};
pub use herc20_hbit::{herc20_hbit_alice, herc20_hbit_bob};

use crate::swap::actions::{CallContract, DeployContract, SendToAddress, SpendOutput};
use std::error::Error as StdError;
use thiserror::Error;

#[derive(Debug, Clone)]
pub enum Action {
    Herc20Deploy(DeployContract),
    Herc20Fund(CallContract),
    Herc20Redeem(CallContract, Secret),
    HbitFund(SendToAddress),
    HbitRedeem(SpendOutput, Secret),
}

#[derive(Debug, Clone, Copy, Error)]
pub enum Error<A, B>
where
    A: StdError + 'static,
    B: StdError + 'static,
{
    #[error("alpha ledger was incorrectly funded")]
    AlphaIncorrectlyFunded(#[source] A),
    #[error("beta ledger was incorrectly funded")]
    BetaIncorrectlyFunded(#[source] B),
}
