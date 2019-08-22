mod extract_secret;
mod htlc_events;

use crate::swap_protocols::{
    ledger::Bitcoin,
    rfc003::{state_machine::HtlcParams, Ledger},
};
use bitcoin_support::{Address, BitcoinQuantity, OutPoint};
use blockchain_contracts::bitcoin::rfc003::bitcoin_htlc::BitcoinHtlc;

pub use self::htlc_events::*;

impl Ledger for Bitcoin {
    type HtlcLocation = OutPoint;
}

impl From<HtlcParams<Bitcoin, BitcoinQuantity>> for BitcoinHtlc {
    fn from(htlc_params: HtlcParams<Bitcoin, BitcoinQuantity>) -> Self {
        BitcoinHtlc::new(
            htlc_params.expiry.into(),
            htlc_params.redeem_identity.into(),
            htlc_params.refund_identity.into(),
            htlc_params.secret_hash.into_raw(),
        )
    }
}

impl HtlcParams<Bitcoin, BitcoinQuantity> {
    pub fn compute_address(&self) -> Address {
        BitcoinHtlc::from(self.clone()).compute_address(self.ledger.network.into())
    }
}
