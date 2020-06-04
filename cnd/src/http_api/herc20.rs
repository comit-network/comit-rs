use crate::{asset, ethereum::ChainId, identity, Timestamp};

pub use crate::herc20::*;

#[derive(Clone, Debug)]
pub struct Finalized {
    pub asset: asset::Erc20,
    pub chain_id: ChainId,
    pub refund_identity: identity::Ethereum,
    pub redeem_identity: identity::Ethereum,
    pub expiry: Timestamp,
    pub state: State,
}
