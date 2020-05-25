use crate::swap_protocols::herc20;
use comit::{asset, identity, Timestamp};

#[derive(Clone, Debug)]
pub struct Herc20Finalized {
    pub herc20_asset: asset::Erc20,
    pub herc20_refund_identity: identity::Ethereum,
    pub herc20_redeem_identity: identity::Ethereum,
    pub herc20_expiry: Timestamp,
    pub herc20_state: herc20::State,
}
