use crate::swap_protocols::{
	ledger::Bitcoin,
	rfc003::{state_machine::HtlcParams, Ledger},
};
use bitcoin_support::{Address, BitcoinQuantity, OutPoint};

mod actions;
mod extract_secret;
mod htlc;
mod htlc_events;

pub use self::{
	actions::*,
	htlc::{Htlc, UnlockingError},
	htlc_events::*,
};

impl Ledger for Bitcoin {
	type HtlcLocation = OutPoint;
}

impl From<HtlcParams<Bitcoin, BitcoinQuantity>> for Htlc {
	fn from(htlc_params: HtlcParams<Bitcoin, BitcoinQuantity>) -> Self {
		Htlc::new(
			htlc_params.redeem_identity,
			htlc_params.refund_identity,
			htlc_params.secret_hash,
			htlc_params.expiry,
		)
	}
}

impl HtlcParams<Bitcoin, BitcoinQuantity> {
	pub fn compute_address(&self) -> Address {
		Htlc::from(self.clone()).compute_address(self.ledger.network)
	}
}
