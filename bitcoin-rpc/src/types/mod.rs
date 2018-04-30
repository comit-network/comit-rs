mod transaction;
mod address;
mod block;

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct BlockHash(String);

impl<'a> From<&'a str> for BlockHash {
    fn from(string: &str) -> Self {
        BlockHash(string.to_string())
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Account(String);

pub use self::address::*;
pub use self::transaction::*;
pub use self::block::*;
use std::str::FromStr;
