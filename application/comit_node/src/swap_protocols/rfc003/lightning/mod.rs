use misc::Seconds;
use secp256k1_support::PublicKey;
use swap_protocols::{ledger::Lightning, rfc003::Ledger};

impl Ledger for Lightning {
    type LockDuration = Seconds;
    type HtlcLocation = ();
    type HtlcIdentity = PublicKey;
}
