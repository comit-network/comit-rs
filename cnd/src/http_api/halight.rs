use crate::swap_protocols::halight;
use comit::{asset, identity, RelativeTime};

pub use crate::swap_protocols::halight::*;

#[derive(Clone, Copy, Debug)]
pub struct Finalized {
    pub asset: asset::Bitcoin,
    pub refund_identity: identity::Lightning,
    pub redeem_identity: identity::Lightning,
    pub cltv_expiry: RelativeTime,
    pub state: halight::State,
}
