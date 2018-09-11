extern crate chrono;
extern crate common_types;
extern crate ethereum_support;
extern crate hex;
#[macro_use]
extern crate log;
extern crate serde;
#[macro_use]
extern crate serde_derive;

use ethereum_support::Bytes;

pub use erc20_htlc::*;
pub use ether_htlc::*;

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
