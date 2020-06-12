mod extract_secret;
mod htlc_events;

use crate::{
    asset, identity,
    swap_protocols::{ledger, rfc003::create_swap::HtlcParams},
};
use ::bitcoin::{
    hashes::{hash160, Hash},
    Address,
};
use blockchain_contracts::bitcoin::rfc003::bitcoin_htlc::BitcoinHtlc;

pub use self::htlc_events::*;

impl From<HtlcParams<ledger::Bitcoin, asset::Bitcoin, identity::Bitcoin>> for BitcoinHtlc {
    fn from(htlc_params: HtlcParams<ledger::Bitcoin, asset::Bitcoin, identity::Bitcoin>) -> Self {
        let refund_public_key = ::bitcoin::PublicKey::from(htlc_params.refund_identity);
        let redeem_public_key = ::bitcoin::PublicKey::from(htlc_params.redeem_identity);

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

impl HtlcParams<ledger::Bitcoin, asset::Bitcoin, identity::Bitcoin> {
    pub fn compute_address(&self) -> Address {
        BitcoinHtlc::from(*self).compute_address(self.ledger.into())
    }
}
