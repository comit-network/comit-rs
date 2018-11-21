use swap_protocols::{
    ledger::{Bitcoin, Ethereum},
    rfc003::Ledger,
};

#[derive(Clone, Debug, PartialEq)]
pub enum SwapResponse<AL: Ledger, BL: Ledger> {
    Accept {
        beta_ledger_refund_identity: BL::Identity,
        alpha_ledger_success_identity: AL::Identity,
        beta_ledger_lock_duration: BL::LockDuration,
    },
    Decline,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SwapResponseKind {
    BitcoinEthereum(SwapResponse<Bitcoin, Ethereum>),
}
