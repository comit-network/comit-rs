use ethereum_support::Bytes;
use serde::{Deserialize, Serialize};

pub mod rfc003;

#[derive(Deserialize, Serialize, Debug)]
pub struct ByteCode(pub String);

impl Into<Bytes> for ByteCode {
    fn into(self) -> Bytes {
        Bytes(hex::decode(self.0).unwrap())
    }
}
