use crate::swap_protocols::{asset, identity};

/// Htlc Lightning Bitcoin atomic swap protocol (HLNBTC).

/// Data required to create a swap that involves a bitcoin on the lightning
/// network.
#[derive(Clone, Debug)]
pub struct CreatedSwap {
    pub amount: asset::Bitcoin,
    pub identity: identity::Lightning,
    pub network: String,
    pub cltv_expiry: u32,
}
