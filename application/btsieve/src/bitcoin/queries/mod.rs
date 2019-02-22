mod block;
mod transaction;

pub use self::{block::BlockQuery, transaction::TransactionQuery};
use bitcoin_support::{Sha256dHash, Transaction};

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum PayloadKind {
    Id { id: Sha256dHash },
    Transaction { transaction: Transaction },
}
