use bitcoin_support::{Address, BitcoinQuantity, Blocks, OutPoint};
use secp256k1_support::KeyPair;
use swap_protocols::{ledger::Bitcoin, rfc003::Ledger};

mod htlc;
mod queries;

pub use self::{
    htlc::{Htlc, UnlockingError},
    queries::*,
};
use swap_protocols::{
    asset::Asset,
    rfc003::{state_machine::OngoingSwap, IntoSecretHash},
};

impl Ledger for Bitcoin {
    type LockDuration = Blocks;
    type HtlcLocation = OutPoint;
    type HtlcIdentity = KeyPair;
}

pub fn bitcoin_htlc<TL: Ledger, TA: Asset, S: IntoSecretHash>(
    swap: &OngoingSwap<Bitcoin, TL, BitcoinQuantity, TA, S>,
) -> Htlc {
    Htlc::new(
        swap.source_ledger_success_identity,
        swap.source_ledger_refund_identity,
        swap.secret.clone().into(),
        swap.source_ledger_lock_duration.into(),
    )
}

pub fn bitcoin_htlc_address<TL: Ledger, TA: Asset, S: IntoSecretHash>(
    swap: &OngoingSwap<Bitcoin, TL, BitcoinQuantity, TA, S>,
) -> Address {
    bitcoin_htlc(swap).compute_address(swap.source_ledger.network)
}
