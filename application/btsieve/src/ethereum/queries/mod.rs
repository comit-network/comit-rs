pub mod block;
pub mod event;
pub mod transaction;

pub use self::{block::BlockQuery, event::EventQuery, transaction::TransactionQuery};
use ethereum_support::{Transaction, TransactionReceipt};
use ethereum_types::{clean_0x, H256};

fn to_h256(tx_id: &String) -> Option<H256> {
    match hex::decode(clean_0x(tx_id)) {
        Ok(bytes) => Some(H256::from_slice(&bytes)),
        Err(e) => {
            warn!("skipping {} because it is not valid hex: {:?}", tx_id, e);
            None
        }
    }
}

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum PayloadKind {
    Id {
        id: H256,
    },
    Transaction {
        transaction: Transaction,
    },
    Receipt {
        receipt: TransactionReceipt,
    },
    TransactionAndReceipt {
        transaction: Transaction,
        receipt: TransactionReceipt,
    },
}
