mod transaction;
mod address;

#[derive(Deserialize, Serialize, Debug)]
pub struct BlockHash(String);

#[derive(Deserialize, Serialize, Debug)]
pub struct Account(String);

pub use self::address::*;
pub use self::transaction::*;
