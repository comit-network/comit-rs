mod actions;

pub use self::actions::btc_eth;

use bitcoin_support::{self, BitcoinQuantity};
use swap_protocols::{
    ledger::Bitcoin,
    rfc003::{self, messages::AcceptResponse, state_machine::Start, Ledger, SecretHash},
};

pub fn bitcoin_htlc<TL: Ledger, TA: Clone, S: Into<SecretHash> + Clone>(
    start: &Start<Bitcoin, TL, BitcoinQuantity, TA, S>,
    response: &AcceptResponse<Bitcoin, TL>,
) -> rfc003::bitcoin::Htlc {
    rfc003::bitcoin::Htlc::new(
        response.source_ledger_success_identity,
        start.source_identity,
        start.secret.clone().into(),
        start.source_ledger_lock_duration.into(),
    )
}

pub fn bitcoin_htlc_address<TL: Ledger, TA: Clone, S: Into<SecretHash> + Clone>(
    start: &Start<Bitcoin, TL, BitcoinQuantity, TA, S>,
    response: &AcceptResponse<Bitcoin, TL>,
) -> bitcoin_support::Address {
    bitcoin_htlc(start, response).compute_address(start.source_ledger.network())
}
