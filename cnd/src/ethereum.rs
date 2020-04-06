#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![forbid(unsafe_code)]

pub use ethbloom::{Bloom as H2048, Input};
pub use primitive_types::{H160, H256, U128, U256};
use serde::{Deserialize, Serialize};
use serde_hex::{CompactPfx, SerHex, SerHexSeq, StrictPfx};

pub type Address = H160;

#[derive(Debug, Default, Copy, Clone, PartialEq, Deserialize)]
pub struct H64(#[serde(with = "SerHex::<StrictPfx>")] [u8; 8]);

/// "Receipt" of an executed transaction: details of its execution.
#[derive(Debug, Default, Clone, PartialEq, Deserialize)]
pub struct TransactionReceipt {
    /// Contract address created, or `None` if not a deployment.
    #[serde(rename = "contractAddress")]
    pub contract_address: Option<H160>,
    /// Logs generated within this transaction.
    pub logs: Vec<Log>,
    /// Status: either 1 (success) or 0 (failure).
    #[serde(with = "SerHex::<CompactPfx>")]
    pub status: u8,
}

impl TransactionReceipt {
    pub fn is_status_ok(&self) -> bool {
        self.status == 1
    }
}

/// Description of a Transaction, pending or in the chain.
#[derive(Debug, Default, Clone, PartialEq, Deserialize)]
pub struct Transaction {
    /// Hash
    pub hash: H256,
    /// Recipient (None when contract creation)
    pub to: Option<H160>,
    /// Transfered value
    pub value: U256,
    /// Input data
    pub input: Bytes,
}

/// A log produced by a transaction.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Log {
    /// H160
    pub address: H160,
    /// Topics
    pub topics: Vec<H256>,
    /// Data
    pub data: Bytes,
}

/// The block returned from RPC calls.
///
/// This type contains only the fields we are actually using.
#[derive(Debug, Default, Clone, PartialEq, Deserialize)]
pub struct Block {
    /// Hash of the block
    pub hash: Option<H256>,
    /// Hash of the parent
    #[serde(rename = "parentHash")]
    pub parent_hash: H256,
    /// Logs bloom
    #[serde(rename = "logsBloom")]
    pub logs_bloom: H2048,
    /// Timestamp
    pub timestamp: U256,
    /// Transactions
    pub transactions: Vec<Transaction>,
}

/// Raw bytes wrapper
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Bytes(#[serde(with = "SerHexSeq::<StrictPfx>")] pub Vec<u8>);

impl AsRef<[u8]> for Bytes {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl<T: Into<Vec<u8>>> From<T> for Bytes {
    fn from(data: T) -> Self {
        Bytes(data.into())
    }
}

#[cfg(test)]
mod tests {
    use super::Log;
    use crate::ethereum::TransactionReceipt;

    #[test]
    fn deserialise_log() {
        let json = r#"
            {
                "address": "0xc5549e335b2786520f4c5d706c76c9ee69d0a028",
                "blockHash": "0x3ae3b6ffb04204f52dee42000e8b971c0f7c2b4aa8dd9455e41a30ee4b31e8a9",
                "blockNumber": "0x856ca0",
                "data": "0x0000000000000000000000000000000000000000000000000000000ba43b7400",
                "logIndex": "0x81",
                "removed": false,
                "topics": [
                    "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef",
                    "0x000000000000000000000000fb303a1fba5b4804863131145bc27256d3ab6692",
                    "0x000000000000000000000000d50fb7d948426633ec126aeea140ce4dd0979682"
                ],
                "transactionHash": "0x5ffd218c617f76c73aa49ee636027440b58eb022778c5e75794563c0d60fcb88",
                "transactionIndex": "0x93"
            }"#;

        let _: Log = serde_json::from_str(json).unwrap();
    }

    #[test]
    fn deserialize_receipt_with_status_1() {
        let json = r#"
        {
          "contractAddress": null,
          "logs": [],
          "status": "0x1"
        }
        "#;

        let receipt = serde_json::from_str::<TransactionReceipt>(json).unwrap();

        assert_eq!(receipt.status, 1);
    }

    #[test]
    fn deserialize_receipt_with_status_0() {
        let json = r#"
        {
          "contractAddress": null,
          "logs": [],
          "status": "0x0"
        }
        "#;

        let receipt = serde_json::from_str::<TransactionReceipt>(json).unwrap();

        assert_eq!(receipt.status, 0);
    }
}
