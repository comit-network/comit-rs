mod erc20_htlc;
mod ether_htlc;

pub use self::{erc20_htlc::Erc20Htlc, ether_htlc::EtherHtlc};
use crate::ethereum::ByteCode;

pub trait Htlc {
    fn compile_to_hex(&self) -> ByteCode;
}
