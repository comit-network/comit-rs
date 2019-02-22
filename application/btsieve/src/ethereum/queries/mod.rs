pub mod block_query;
pub mod event_query;
pub mod transaction_query;

pub use self::{
    block_query::BlockQuery, event_query::EventQuery, transaction_query::TransactionQuery,
};
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
