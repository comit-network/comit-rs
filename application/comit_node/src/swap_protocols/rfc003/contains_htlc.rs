use swap_protocols::{
    asset::Asset,
    rfc003::{state_machine::HtlcParams, Ledger},
};

#[derive(Debug, PartialEq)]
pub enum Error<A: Asset> {
    UnexpectedAsset { found: A, expected: A },
    WrongTransaction,
}

pub trait ContainsHtlc<L, LA>: Send + Sync
where
    L: Ledger,
    LA: Asset,
{
    fn contains_htlc(&self, htlc_params: &HtlcParams<L, LA>) -> Result<L::HtlcLocation, Error<LA>>;
}
