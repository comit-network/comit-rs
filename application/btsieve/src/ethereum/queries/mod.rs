pub mod block;
pub mod event;
pub mod transaction;

pub use self::{block::BlockQuery, event::EventQuery, transaction::TransactionQuery};
use crate::route_factory::Error;
use ethereum_support::{
    web3::{transports::Http, Web3},
    Transaction, TransactionId, TransactionReceipt,
};
use ethereum_types::{clean_0x, H256};
use futures::Future;

fn to_h256<S: AsRef<str>>(tx_id: S) -> Option<H256> {
    let tx_id = tx_id.as_ref();

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
        transaction: Box<Transaction>,
    },
    Receipt {
        receipt: Box<TransactionReceipt>,
    },
    TransactionAndReceipt {
        transaction: Box<Transaction>,
        receipt: Box<TransactionReceipt>,
    },
}

// TODO: Investigate under which circumstances the ethereum node fails to return
// a transaction/receipt
pub fn create_transaction_future(
    client: &Web3<Http>,
    id: H256,
) -> impl Future<Item = Option<Box<Transaction>>, Error = Error> {
    client
        .eth()
        .transaction(TransactionId::Hash(id))
        .map(move |maybe_transaction| {
            maybe_transaction.map(Box::new).or_else(|| {
                warn!(
                    "received empty response for eth_getTransactionByHash({:x?})",
                    id
                );
                None
            })
        })
        .map_err(Error::Web3)
}

pub fn create_receipt_future(
    client: &Web3<Http>,
    id: H256,
) -> impl Future<Item = Option<Box<TransactionReceipt>>, Error = Error> {
    client
        .eth()
        .transaction_receipt(id)
        .map(move |maybe_receipt| {
            maybe_receipt.map(Box::new).or_else(|| {
                warn!(
                    "received empty response for eth_getTransactionReceipt({:x?})",
                    id
                );
                None
            })
        })
        .map_err(Error::Web3)
}
