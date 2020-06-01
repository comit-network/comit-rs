use crate::{asset, identity, swap_protocols::halight, RelativeTime};

pub use crate::swap_protocols::halight::*;

#[derive(Clone, Copy, Debug)]
pub struct Finalized {
    pub asset: asset::Bitcoin,
    pub refund_identity: identity::Lightning,
    pub redeem_identity: identity::Lightning,
    pub cltv_expiry: RelativeTime,
    pub state: halight::State,
}
