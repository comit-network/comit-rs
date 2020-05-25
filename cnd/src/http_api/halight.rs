use crate::swap_protocols::halight;
use comit::{asset, identity, RelativeTime};

#[derive(Clone, Copy, Debug)]
pub struct HalightFinalized {
    pub halight_asset: asset::Bitcoin,
    pub halight_refund_identity: identity::Lightning,
    pub halight_redeem_identity: identity::Lightning,
    pub cltv_expiry: RelativeTime,
    pub halight_state: halight::State,
}
