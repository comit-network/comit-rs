use bitcoin_support::{Blocks, OutPoint};
use secp256k1_support::KeyPair;
use swap_protocols::{ledger::Bitcoin, rfc003::Ledger};

mod htlc;
mod queries;

pub use self::{
    htlc::{Htlc, UnlockingError},
    queries::*,
};

impl Ledger for Bitcoin {
    type LockDuration = Blocks;
    type HtlcLocation = OutPoint;
    type HtlcIdentity = KeyPair;
}
