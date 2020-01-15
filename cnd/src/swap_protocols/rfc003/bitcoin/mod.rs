mod extract_secret;
mod htlc_events;

use crate::swap_protocols::{
    ledger::Bitcoin,
    rfc003::{create_swap::HtlcParams, Ledger},
};
use bitcoin::{
    hashes::{hash160, Hash},
    Address, Amount, OutPoint, Transaction,
};
use blockchain_contracts::bitcoin::rfc003::bitcoin_htlc::BitcoinHtlc;

pub use self::htlc_events::*;
use crate::bitcoin::PublicKey;

impl Ledger for Bitcoin {
    type HtlcLocation = OutPoint;
    type Identity = PublicKey;
    type Transaction = Transaction;
}

impl From<HtlcParams<Bitcoin, Amount>> for BitcoinHtlc {
    fn from(htlc_params: HtlcParams<Bitcoin, Amount>) -> Self {
        let refund_public_key = htlc_params.refund_identity.into_inner();
        let redeem_public_key = htlc_params.redeem_identity.into_inner();

        let refund_identity = hash160::Hash::hash(&refund_public_key.key.serialize());
        let redeem_identity = hash160::Hash::hash(&redeem_public_key.key.serialize());

        BitcoinHtlc::new(
            htlc_params.expiry.into(),
            refund_identity,
            redeem_identity,
            htlc_params.secret_hash.into_raw(),
        )
    }
}

impl HtlcParams<Bitcoin, Amount> {
    pub fn compute_address(&self) -> Address {
        BitcoinHtlc::from(*self).compute_address(self.ledger.network)
    }
}
