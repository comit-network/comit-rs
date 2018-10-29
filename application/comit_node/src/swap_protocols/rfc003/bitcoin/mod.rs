use bitcoin_support::{Blocks, OutPoint};
use secp256k1_support::KeyPair;
use swap_protocols::{ledger::Bitcoin, rfc003::Ledger};

mod htlc;

pub use self::htlc::{Htlc, UnlockingError};

impl Ledger for Bitcoin {
    type LockDuration = Blocks;
    type HtlcLocation = OutPoint;
    type HtlcIdentity = KeyPair;
}
