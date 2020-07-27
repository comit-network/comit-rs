use num::bigint::ParseBigIntError;

mod erc20;
mod ether;

pub use self::{
    erc20::{Dai, Erc20, Erc20Quantity},
    ether::Ether,
};

pub trait FromWei<W> {
    fn from_wei(wei: W) -> Self;
}

pub trait TryFromWei<W>
where
    Self: std::marker::Sized,
{
    fn try_from_wei(wei: W) -> Result<Self, Error>;
}

#[derive(Clone, Debug, thiserror::Error, PartialEq)]
pub enum Error {
    #[error("value provided overflows")]
    Overflow,
    #[error("parsing error encountered")]
    Parse(#[from] ParseBigIntError),
}
