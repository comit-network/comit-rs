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

fn to_sha256d_hash<S: AsRef<str>>(id: S) -> Option<Sha256dHash> {
    let id = id.as_ref();
    Sha256dHash::from_hex(id)
        .map_err(|e| warn!("skipping {} because it is invalid hex: {:?}", id, e))
        .ok()
}
