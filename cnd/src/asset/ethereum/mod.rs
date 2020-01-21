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
    type Err;
    fn try_from_wei(wei: W) -> Result<Self, Self::Err>;
}

#[derive(Clone, Copy, Debug, thiserror::Error, PartialEq)]
pub enum Error {
    #[error("value provided overflows")]
    Overflow,
}
