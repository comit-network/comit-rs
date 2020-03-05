#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![forbid(unsafe_code)]

pub use primitive_types::{H160, H256, U128, U256};

pub use ethbloom::{Bloom as H2048, Input};

use hex::{FromHex, ToHex};
use serde::{
    de::{Error, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::fmt;

mod hex_serde;

pub type Index = U128;
pub type Address = H160;

#[derive(Debug, Default, Copy, Clone, PartialEq, Deserialize)]
pub struct H64(#[serde(with = "hex_serde")] [u8; 8]);

/// "Receipt" of an executed transaction: details of its execution.
#[derive(Debug, Default, Clone, PartialEq, Deserialize)]
pub struct TransactionReceipt {
    /// Transaction hash.
    #[serde(rename = "transactionHash")]
    pub transaction_hash: H256,
    /// Index within the block.
    #[serde(rename = "transactionIndex")]
    pub transaction_index: Index,
    /// Hash of the block this transaction was included within.
    #[serde(rename = "blockHash")]
    pub block_hash: H256,
    /// Number of the block this transaction was included within.
    #[serde(rename = "blockNumber")]
    pub block_number: U256,
    /// Cumulative gas used within the block after this was executed.
    #[serde(rename = "cumulativeGasUsed")]
    pub cumulative_gas_used: U256,
    /// Gas used by this transaction alone.
    ///
    /// Gas used is `None` if the the client is running in light client mode.
    #[serde(rename = "gasUsed")]
    pub gas_used: U256,
    /// Contract address created, or `None` if not a deployment.
    #[serde(rename = "contractAddress")]
    pub contract_address: Option<H160>,
    /// Logs generated within this transaction.
    pub logs: Vec<Log>,
    /// Status: either 1 (success) or 0 (failure).
    pub status: U256,
    /// Logs bloom
    #[serde(rename = "logsBloom")]
    pub logs_bloom: H2048,
}

pub trait IsStatusOk {
    fn is_status_ok(&self) -> bool;
}

impl IsStatusOk for TransactionReceipt {
    fn is_status_ok(&self) -> bool {
        const TRANSACTION_STATUS_OK: u32 = 1;
        self.status == TRANSACTION_STATUS_OK.into()
    }
}

/// Description of a Transaction, pending or in the chain.
#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct Transaction {
    /// Hash
    pub hash: H256,
    /// Nonce
    pub nonce: U256,
    /// Block hash. None when pending.
    #[serde(rename = "blockHash")]
    pub block_hash: H256,
    /// Block number. None when pending.
    #[serde(rename = "blockNumber")]
    pub block_number: U256,
    /// Transaction Index. None when pending.
    #[serde(rename = "transactionIndex")]
    pub transaction_index: Index,
    /// Sender
    pub from: H160,
    /// Recipient (None when contract creation)
    pub to: Option<H160>,
    /// Transfered value
    pub value: U256,
    /// Gas Price
    #[serde(rename = "gasPrice")]
    pub gas_price: U256,
    /// Gas amount
    pub gas: U256,
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
    /// Block Hash
    #[serde(rename = "blockHash")]
    pub block_hash: Option<H256>,
    /// Block Number
    #[serde(rename = "blockNumber")]
    pub block_number: Option<U256>,
    /// Transaction Hash
    #[serde(rename = "transactionHash")]
    pub transaction_hash: Option<H256>,
    /// Transaction Index
    #[serde(rename = "transactionIndex")]
    pub transaction_index: Option<U256>,
}

/// The block returned from RPC calls.
#[derive(Debug, Default, Clone, PartialEq, Deserialize)]
pub struct Block {
    /// Hash of the block
    pub hash: Option<H256>,
    /// Hash of the parent
    #[serde(rename = "parentHash")]
    pub parent_hash: H256,
    /// Hash of the uncles
    #[serde(rename = "sha3Uncles")]
    pub uncles_hash: H256,
    /// Miner/author's address.
    #[serde(rename = "miner")]
    pub author: H160,
    /// State root hash
    #[serde(rename = "stateRoot")]
    pub state_root: H256,
    /// Transactions root hash
    #[serde(rename = "transactionsRoot")]
    pub transactions_root: H256,
    /// Transactions receipts root hash
    #[serde(rename = "receiptsRoot")]
    pub receipts_root: H256,
    /// Block number. None if pending.
    pub number: Option<U128>,
    /// Gas Used
    #[serde(rename = "gasUsed")]
    pub gas_used: U256,
    /// Gas Limit
    #[serde(rename = "gasLimit")]
    pub gas_limit: U256,
    /// Extra data
    #[serde(rename = "extraData")]
    pub extra_data: Bytes,
    /// Logs bloom
    #[serde(rename = "logsBloom")]
    pub logs_bloom: H2048,
    /// Timestamp
    pub timestamp: U256,
    /// Difficulty
    pub difficulty: U256,
    /// Total difficulty
    #[serde(rename = "totalDifficulty")]
    pub total_difficulty: U256,
    /// Seal fields
    #[serde(default, rename = "sealFields")]
    pub seal_fields: Vec<Bytes>,
    /// Uncles' hashes
    pub uncles: Vec<H256>,
    /// Transactions
    pub transactions: Vec<Transaction>,
    /// Size in bytes
    pub size: Option<U256>,
    /// Mix Hash
    #[serde(rename = "mixHash")]
    pub mix_hash: Option<H256>,
    /// Nonce
    pub nonce: Option<H64>,
}

/// Used to specify a block. Can currently only specify the latest block but
/// other variants can be added as required
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BlockNumber {
    Latest,
}

impl Serialize for BlockNumber {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            BlockNumber::Latest => serializer.serialize_str("latest"),
        }
    }
}

/// Raw bytes wrapper
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Bytes(pub Vec<u8>);

impl<T: Into<Vec<u8>>> From<T> for Bytes {
    fn from(data: T) -> Self {
        Bytes(data.into())
    }
}

impl Serialize for Bytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut serialized = "0x".to_owned();
        serialized.push_str(self.0.encode_hex::<String>().as_ref());
        serializer.serialize_str(serialized.as_ref())
    }
}

impl<'a> Deserialize<'a> for Bytes {
    fn deserialize<D>(deserializer: D) -> Result<Bytes, D::Error>
    where
        D: Deserializer<'a>,
    {
        deserializer.deserialize_identifier(BytesVisitor)
    }
}

struct BytesVisitor;

impl<'a> Visitor<'a> for BytesVisitor {
    type Value = Bytes;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "a 0x-prefixed hex-encoded vector of bytes")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        if value.len() >= 2 && &value[0..2] == "0x" && value.len() & 1 == 0 {
            Ok(Bytes(
                FromHex::from_hex(&value[2..]).map_err(|_| Error::custom("invalid hex"))?,
            ))
        } else {
            Err(Error::custom("invalid format"))
        }
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_str(value.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::Log;
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
}
