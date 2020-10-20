//! Code that could be upstreamed to COMIT lib.

pub mod hbit;
pub mod hbit_herc20;
pub mod herc20;
pub mod herc20_hbit;

pub use comit::{ethereum, *};
pub use hbit_herc20::{hbit_herc20_alice, hbit_herc20_bob};
pub use herc20_hbit::herc20_hbit_bob;

use async_trait::async_trait;
use clarity::Uint256;
use std::fmt::Debug;
use thiserror::Error;

/// Indicates that a swap failed AND that we should refund as a result.
///
/// The contained event holds the necessary information to refund.
#[derive(Clone, Copy, Debug, Error)]
#[error("swap execution failed")]
pub struct SwapFailedShouldRefund<E: Debug>(pub E);

#[async_trait]
pub trait EstimateBitcoinFee {
    // TODO: Encode in the type signature that is this sats/vKB
    async fn estimate_bitcoin_fee(&self, block_target: u8) -> ::bitcoin::Amount;
}

#[async_trait]
pub trait EstimateEthereumGasPrice {
    async fn estimate_ethereum_gas_price(&self, block_target: u8) -> Uint256;
}
