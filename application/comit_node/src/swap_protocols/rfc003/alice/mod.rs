mod actions;

pub use self::actions::btc_eth;

use bitcoin_support::{self, BitcoinQuantity};
use swap_protocols::{
    ledger::Bitcoin,
    rfc003::{
        self,
        messages::AcceptResponse,
        state_machine::{OngoingSwap, Start},
        Ledger, SecretHash,
    },
};

pub fn bitcoin_htlc<TL: Ledger, TA: Clone, S: Into<SecretHash> + Clone>(
    swap: &OngoingSwap<Bitcoin, TL, BitcoinQuantity, TA, S>,
) -> rfc003::bitcoin::Htlc {
    rfc003::bitcoin::Htlc::new(
        swap.source_ledger_success_identity,
        swap.source_identity,
        swap.secret.clone().into(),
        swap.source_ledger_lock_duration.into(),
    )
}

pub fn bitcoin_htlc_address<TL: Ledger, TA: Clone, S: Into<SecretHash> + Clone>(
    swap: &OngoingSwap<Bitcoin, TL, BitcoinQuantity, TA, S>,
) -> bitcoin_support::Address {
    bitcoin_htlc(swap).compute_address(swap.source_ledger.network())
}
