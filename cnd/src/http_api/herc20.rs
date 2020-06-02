use crate::{asset, identity, swap_protocols::herc20, Timestamp};

pub use crate::swap_protocols::herc20::*;

#[derive(Clone, Debug)]
pub struct Finalized {
    pub asset: asset::Erc20,
    pub refund_identity: identity::Ethereum,
    pub redeem_identity: identity::Ethereum,
    pub expiry: Timestamp,
    pub state: herc20::State,
}
