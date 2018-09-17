use ethereum_support::Bytes;
use hex;

pub use self::{erc20_htlc::*, ether_htlc::*};

mod erc20_htlc;
mod ether_htlc;

#[derive(Deserialize, Serialize, Debug)]
pub struct ByteCode(pub String);

impl Into<Bytes> for ByteCode {
    fn into(self) -> Bytes {
        Bytes(hex::decode(self.0).unwrap())
    }
}

pub trait Htlc {
    fn compile_to_hex(&self) -> ByteCode;
}
