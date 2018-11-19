use swap_protocols::{
    self,
    asset::Asset,
    rfc003::{state_machine::HtlcParams, Ledger},
};

#[derive(Debug, PartialEq)]
pub enum Error<A: Asset> {
    UnexpectedAsset { found: A, expected: A },
    WrongTransaction,
}

pub trait IsContainedInTransaction<L>: Send + Sync
where
    L: Ledger,
    Self: Asset,
{
    fn is_contained_in_transaction(
        htlc_params: &HtlcParams<L, Self>,
        transaction: <L as swap_protocols::Ledger>::Transaction,
    ) -> Result<L::HtlcLocation, Error<Self>>;
}
