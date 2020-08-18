use crate::{btsieve::LatestBlock, Timestamp};
pub use ethbloom::{Bloom as H2048, Input};
use hex::FromHexError;
pub use primitive_types::U256;
use serde::{Deserialize, Deserializer, Serialize};
use std::{
    fmt,
    fmt::{Display, Formatter},
    str::FromStr,
};

pub async fn latest_time<C>(connector: &C) -> anyhow::Result<Timestamp>
where
    C: LatestBlock<Block = Block>,
{
    let timestamp = connector.latest_block().await?.timestamp.into();

    Ok(timestamp)
}

#[derive(
    Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct Address(#[serde(with = "serde_hex_data")] [u8; 20]);

impl Address {
    pub fn as_bytes(&self) -> &[u8; 20] {
        &self.0
    }

    /// Generates a random address for use in tests where the actual value
    /// doesn't / shouldn't matter.
    pub fn random() -> Address {
        use rand::RngCore;

        let mut buffer = [0u8; 20];
        rand::thread_rng().fill_bytes(&mut buffer);

        Address(buffer)
    }
}

impl From<[u8; 20]> for Address {
    fn from(bytes: [u8; 20]) -> Self {
        Address(bytes)
    }
}

impl From<Address> for [u8; 20] {
    fn from(s: Address) -> Self {
        s.0
    }
}

impl FromStr for Address {
    type Err = FromHexError;

    fn from_str(hex: &str) -> Result<Self, Self::Err> {
        let mut address = [0u8; 20];
        hex::decode_to_slice(hex.trim_start_matches("0x"), &mut address)?;

        Ok(Address(address))
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "0x")?;
        for i in &self.0 {
            write!(f, "{:02x}", i)?;
        }
        Ok(())
    }
}

impl From<Address> for Hash {
    fn from(address: Address) -> Self {
        let mut h256 = Hash([0u8; 32]);
        h256.0[(32 - 20)..32].copy_from_slice(&address.0);
        h256
    }
}

#[derive(
    Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct Hash(#[serde(with = "serde_hex_data")] [u8; 32]);

impl From<[u8; 32]> for Hash {
    fn from(bytes: [u8; 32]) -> Self {
        Hash(bytes)
    }
}

impl From<Hash> for [u8; 32] {
    fn from(s: Hash) -> Self {
        s.0
    }
}

impl Hash {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl Display for Hash {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "0x")?;
        for i in &self.0 {
            write!(f, "{:02x}", i)?;
        }
        Ok(())
    }
}

impl FromStr for Hash {
    type Err = FromHexError;

    fn from_str(hex: &str) -> Result<Self, Self::Err> {
        let mut hash = [0u8; 32];
        hex::decode_to_slice(hex.trim_start_matches("0x"), &mut hash)?;

        Ok(Hash(hash))
    }
}

/// "Receipt" of an executed transaction: details of its execution.
#[derive(Debug, Default, Clone, PartialEq, Deserialize)]
pub struct TransactionReceipt {
    /// Contract address created, or `None` if not a deployment.
    #[serde(rename = "contractAddress")]
    pub contract_address: Option<Address>,
    /// Logs generated within this transaction.
    pub logs: Vec<Log>,
    /// Status: Whether or not the transaction executed successfully
    #[serde(rename = "status", deserialize_with = "deserialize_status")]
    pub successful: bool,
}

fn deserialize_status<'de, D>(deserializer: D) -> Result<bool, <D as Deserializer<'de>>::Error>
where
    D: Deserializer<'de>,
{
    let hex_string = String::deserialize(deserializer)?;

    Ok(&hex_string == "0x1")
}

/// Description of a Transaction, pending or in the chain.
#[derive(Debug, Default, Clone, PartialEq, Deserialize)]
pub struct Transaction {
    /// Hash
    pub hash: Hash,
    /// Recipient (None when contract creation)
    pub to: Option<Address>,
    /// Transfered value
    pub value: U256,
    /// Input data
    #[serde(with = "serde_hex_data")]
    pub input: Vec<u8>,
}

/// A log produced by a transaction.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Log {
    /// H160
    pub address: Address,
    /// Topics
    pub topics: Vec<Hash>,
    /// Data
    #[serde(with = "serde_hex_data")]
    pub data: Vec<u8>,
}

/// The block returned from RPC calls.
///
/// This type contains only the fields we are actually using.
#[derive(Debug, Default, Clone, PartialEq, Deserialize)]
pub struct Block {
    /// Hash of the block
    pub hash: Hash,
    /// Hash of the parent
    #[serde(rename = "parentHash")]
    pub parent_hash: Hash,
    /// Logs bloom
    #[serde(rename = "logsBloom")]
    pub logs_bloom: H2048,
    /// Timestamp
    pub timestamp: U256,
    /// Transactions
    pub transactions: Vec<Transaction>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct ChainId(u32);

impl ChainId {
    pub const MAINNET: Self = ChainId(1);
    pub const ROPSTEN: Self = ChainId(3);
    pub const KOVAN: Self = ChainId(42);
    pub const GETH_DEV: Self = ChainId(1337);
}

impl fmt::Display for ChainId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            &Self::MAINNET => write!(f, "MAINNET"),
            &Self::ROPSTEN => write!(f, "ROPSTEN"),
            &Self::GETH_DEV => write!(f, "GETH-DEV"),
            other => write!(f, "UNKNOWN ({})", other.0),
        }
    }
}

impl From<ChainId> for u32 {
    fn from(chain_id: ChainId) -> Self {
        chain_id.0
    }
}

impl From<u32> for ChainId {
    fn from(id: u32) -> Self {
        ChainId(id)
    }
}

/// A serde module for formatting bytes according to Ethereum's convention for
/// "data".
///
/// See https://eth.wiki/json-rpc/API#hex-value-encoding for more details.
pub mod serde_hex_data {
    use super::*;
    use hex::FromHex;
    use serde::{de::Error, Deserialize, Deserializer, Serializer};

    pub fn serialize<S, V>(value: &V, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        V: AsRef<[u8]>,
    {
        serializer.serialize_str(&format!("0x{}", hex::encode(value.as_ref())))
    }

    pub fn deserialize<'de, D, V>(deserializer: D) -> Result<V, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
        V: FromHex,
        <V as FromHex>::Error: Display,
    {
        let string = String::deserialize(deserializer)?;
        let value = V::from_hex(string.trim_start_matches("0x")).map_err(D::Error::custom)?;

        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proptest::*;
    use proptest::prelude::*;

    #[test]
    fn deserialise_address() {
        let json =
            serde_json::Value::String("0xc5549e335b2786520f4c5d706c76c9ee69d0a028".to_owned());
        let _: Address = Address::deserialize(&json).unwrap();
    }

    #[test]
    fn from_string_address() {
        let json =
            serde_json::Value::String("0xc5549e335b2786520f4c5d706c76c9ee69d0a028".to_owned());
        let deserialized: Address = Address::deserialize(&json).unwrap();

        let from_string = Address::from_str("0xc5549e335b2786520f4c5d706c76c9ee69d0a028").unwrap();

        assert_eq!(from_string, deserialized);
    }

    #[test]
    fn deserialise_hash() {
        let json = serde_json::Value::String(
            "0x3ae3b6ffb04204f52dee42000e8b971c0f7c2b4aa8dd9455e41a30ee4b31e8a9".to_owned(),
        );
        let _: Hash = Hash::deserialize(&json).unwrap();
    }

    #[test]
    fn deserialise_hash_when_not_using_reference_to_deserialize_fails() {
        // This is due to a bug in serde-jex, keep this test until https://github.com/fspmarshall/serde-hex/pull/8
        // is fixed.
        let json = serde_json::Value::String(
            "0x3ae3b6ffb04204f52dee42000e8b971c0f7c2b4aa8dd9455e41a30ee4b31e8a9".to_owned(),
        );

        let deserialized = serde_json::from_value::<Hash>(json);
        matches!(deserialized, Err(_));
    }

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

        assert_eq!(receipt.successful, true);
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

        assert_eq!(receipt.successful, false);
    }

    proptest! {
        #[test]
        fn address_from_hex_doesnt_panic(string in any::<String>()) {
            let _ = Address::from_str(&string);
        }
    }

    proptest! {
        #[test]
        fn address_to_string_from_str_is_uniform(address in ethereum::address()) {
            let displayed = format!("{}", address);
            let constructed = Address::from_str(&displayed).unwrap();

            assert_eq!(constructed, address);
        }
    }

    proptest! {
        #[test]
        fn hash_from_hex_doesnt_panic(string in any::<String>()) {
            let _ = Hash::from_str(&string);
        }
    }

    proptest! {
        #[test]
        fn hash_to_string_from_str_is_uniform(hash in ethereum::hash()) {
            let displayed = format!("{}", hash);
            let constructed = Hash::from_str(&displayed).unwrap();

            assert_eq!(constructed, hash);
        }
    }
}
