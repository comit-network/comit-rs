//! Code that could be upstreamed to COMIT lib.

pub mod hbit;
mod hbit_herc20;
pub mod herc20;
mod herc20_hbit;

pub use comit::{ethereum, *};
pub use hbit_herc20::{hbit_herc20_alice, hbit_herc20_bob};
pub use herc20_hbit::{herc20_hbit_alice, herc20_hbit_bob};

use std::fmt::Debug;
use thiserror::Error;

/// Indicates that a swap failed AND that we should refund as a result.
///
/// The contained event holds the necessary information to refund.
#[derive(Clone, Copy, Debug, Error)]
#[error("swap execution failed")]
pub struct SwapFailedShouldRefund<E: Debug>(pub E);

/// Indicates that a swap failed but we don't have to refund.
#[derive(Clone, Copy, Debug, Error)]
#[error("swap execution failed")]
pub struct SwapFailedNoRefund;
