#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![forbid(unsafe_code)]

pub use web3::types::{
    Address, Block, BlockId, BlockNumber, Bytes, Log, Transaction, TransactionReceipt,
    TransactionRequest, H160, H2048, H256, U128, U256,
};

pub trait IsStatusOk {
    fn is_status_ok(&self) -> bool;
}

impl IsStatusOk for TransactionReceipt {
    fn is_status_ok(&self) -> bool {
        const TRANSACTION_STATUS_OK: u32 = 1;
        self.status == Some(TRANSACTION_STATUS_OK.into())
    }
}
