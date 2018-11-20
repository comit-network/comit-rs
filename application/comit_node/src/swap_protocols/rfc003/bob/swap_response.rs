use swap_protocols::{
    ledger::{Bitcoin, Ethereum},
    rfc003::Ledger,
};

#[derive(Clone, Debug, PartialEq)]
pub enum SwapResponse<SL: Ledger, TL: Ledger> {
    Accept {
        target_ledger_refund_identity: TL::Identity,
        source_ledger_success_identity: SL::Identity,
        target_ledger_lock_duration: TL::LockDuration,
    },
    Decline,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SwapResponseKind {
    BitcoinEthereum(SwapResponse<Bitcoin, Ethereum>),
}
