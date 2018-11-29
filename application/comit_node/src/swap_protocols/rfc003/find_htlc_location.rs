use std::fmt::Debug;
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

// Not all assets are PartialOrd, that is why the bound is explicitly listed here
pub fn compare_assets<A: Asset + PartialOrd, L: Debug>(
    location: L,
    given: A,
    expected: A,
) -> Result<L, Error<A>> {
    info!("Value of HTLC at {:?} is {}", location, given);

    let has_enough_money = given >= expected;

    debug!("{} >= {} -> {}", given, expected, has_enough_money);

    if given < expected {
        return Err(Error::UnexpectedAsset {
            found: given,
            expected,
        });
    }

    Ok(location)
}
