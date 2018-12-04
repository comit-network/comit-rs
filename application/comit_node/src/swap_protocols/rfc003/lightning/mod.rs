use bitcoin_support::Blocks;
use secp256k1_support::PublicKey;
use swap_protocols::{ledger::Lightning, rfc003::Ledger};
mod actions;

pub use self::actions::*;

impl Ledger for Lightning {
    type LockDuration = Blocks;
    type HtlcLocation = ();
    type HtlcIdentity = PublicKey;
}
