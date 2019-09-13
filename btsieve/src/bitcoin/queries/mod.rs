pub mod transaction;

pub use self::transaction::TransactionQuery;
use bitcoin_support::Transaction;
use serde::Serialize;

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum PayloadKind {
    Id { id: String },
    Transaction { transaction: Transaction },
}
