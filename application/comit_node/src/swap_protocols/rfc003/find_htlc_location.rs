use swap_protocols::{
    asset::Asset,
    rfc003::{state_machine::HtlcParams, Ledger},
};

#[derive(Debug, PartialEq)]
pub enum Error<A: Asset> {
    UnexpectedAsset { found: A, expected: A },
    WrongTransaction,
}

pub trait FindHtlcLocation<L, A>: Send + Sync
where
    L: Ledger,
    A: Asset,
{
    fn find_htlc_location(
        &self,
        htlc_params: &HtlcParams<L, A>,
    ) -> Result<L::HtlcLocation, Error<A>>;
}
