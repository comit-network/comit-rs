use num::bigint::ParseBigIntError;

mod erc20;
mod ether;

pub use self::{
    erc20::{Erc20, Erc20Quantity},
    ether::Ether,
};

pub trait FromWei<W> {
    fn from_wei(wei: W) -> Self;
}

pub trait TryFromWei<W>
where
    Self: std::marker::Sized,
{
    fn try_from_wei(wei: W) -> anyhow::Result<Self>;
}

#[derive(Clone, Copy, Debug, thiserror::Error, PartialEq)]
#[error("value provided overflows")]
pub struct ValueOverflow;

#[derive(Clone, Debug, thiserror::Error, PartialEq)]
#[error("parsing error encountered")]
pub struct ParseError(#[from] ParseBigIntError);
