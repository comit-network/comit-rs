pub mod block;
pub mod transaction;

pub use self::{block::BlockQuery, transaction::TransactionQuery};
use bitcoin_support::{FromHex, Sha256dHash, Transaction};
use serde::Serialize;

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum PayloadKind {
    Id { id: Sha256dHash },
    Transaction { transaction: Transaction },
}

fn to_sha256d_hash<S: AsRef<str>>(id: S) -> Option<Sha256dHash> {
    let id = id.as_ref();
    Sha256dHash::from_hex(id)
        .map_err(|e| log::warn!("skipping {} because it is invalid hex: {:?}", id, e))
        .ok()
}
