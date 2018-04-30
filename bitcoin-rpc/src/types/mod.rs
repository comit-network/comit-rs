#[macro_use]
mod from_str;
mod transaction;
mod address;
mod block;
mod script;

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct BlockHash(String);

#[derive(Deserialize, Serialize, Debug)]
pub struct Account(String);

from_str!(BlockHash);
from_str!(Account);

pub use self::address::*;
pub use self::transaction::*;
pub use self::block::*;
pub use self::script::*;
