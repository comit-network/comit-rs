use secp256k1_support::PublicKey;
use std_ext::time::Seconds;
use swap_protocols::{
    ledger::Lightning,
    rfc003::{
        secret::{Secret, SecretHash},
        Ledger, RedeemTransaction,
    },
};

impl Ledger for Lightning {
    type LockDuration = Seconds;
    type HtlcLocation = ();
    type HtlcIdentity = PublicKey;

    fn extract_secret(
        _payment: &RedeemTransaction<Self>,
        _secret_hash: &SecretHash,
    ) -> Option<Secret> {
        unimplemented!()
    }
}
