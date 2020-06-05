use bitcoin::secp256k1::SecretKey;
use comit::{asset, ledger, Timestamp};

pub use crate::hbit::*;

/// Data known by the party funding the HTLC in the Hbit protocol, after the
/// swap has been finalized.
///
/// The funder of the HTLC knows the following identities:
/// - `transient_redeem_identity`: the public identity of the redeemer.
/// - `transient_refund_identity`: their own secret identity, from which their
///   public identity can be derived, and which can be used to produce a
///   signature that will enable the refund action.
/// -`final_refund_identity`: the address where the HTLC funds will go if the
///   refund action is executed.
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

/// Data known by the party redeeming the HTLC in the Hbit protocol, after the
/// swap has been finalized.
///
/// The redeemer of the HTLC knows the following identities:
/// - `transient_refund_identity`: the public identity of the funder.
/// - `transient_redeem_identity`: their own secret identity, from which their
///   public identity can be derived, and which can be used to produce a
///   signature that will enable the redeem action.
/// -`final_refund_identity`: the address where the HTLC funds will go if the
///   redeem action is executed.
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
