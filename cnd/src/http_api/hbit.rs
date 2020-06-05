use comit::{asset, ledger, Timestamp};

pub use crate::hbit::*;
use bitcoin::secp256k1::SecretKey;

#[derive(Clone, Debug)]
pub struct FinalizedAsFunder {
    pub asset: asset::Bitcoin,
    pub network: ledger::Bitcoin,
    pub transient_redeem_identity: identity::Bitcoin,
    pub final_refund_identity: comit::bitcoin::Address,
    pub transient_refund_identity: SecretKey,
    pub expiry: Timestamp,
    pub state: State,
}

#[derive(Clone, Debug)]
pub struct FinalizedAsRedeemer {
    pub asset: asset::Bitcoin,
    pub network: ledger::Bitcoin,
    pub final_redeem_identity: comit::bitcoin::Address,
    pub transient_redeem_identity: SecretKey,
    pub transient_refund_identity: identity::Bitcoin,
    pub expiry: Timestamp,
    pub state: State,
}
