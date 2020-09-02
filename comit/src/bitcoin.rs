//! This module is the home of bitcoin-specific types and functionality that is
//! needed across several places in cnd.
//!
//! This involves:
//!     - Creating wrapper types to allow for more ergonomic APIs of upstream
//!       libraries
//!     - Common functionality that is not (yet) available upstream

use crate::{
    btsieve::{BlockByHash, LatestBlock},
    Timestamp,
};
use bitcoin::secp256k1;
use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::{fmt, str::FromStr};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PublicKey(bitcoin::PublicKey);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Address(bitcoin::Address);

impl PublicKey {
    pub fn from_secret_key<C>(
        secp: &secp256k1::Secp256k1<C>,
        secret_key: &secp256k1::SecretKey,
    ) -> Self
    where
        C: secp256k1::Signing,
    {
        secp256k1::PublicKey::from_secret_key(secp, secret_key).into()
    }
    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes()
    }
}

impl fmt::Display for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<secp256k1::PublicKey> for PublicKey {
    fn from(key: secp256k1::PublicKey) -> Self {
        Self(bitcoin::PublicKey {
            compressed: true, // we always want the PublicKey to be serialized in a compressed way
            key,
        })
    }
}

impl From<PublicKey> for bitcoin::PublicKey {
    fn from(pubkey: PublicKey) -> bitcoin::PublicKey {
        pubkey.0
    }
}

impl From<Address> for bitcoin::Address {
    fn from(address: Address) -> bitcoin::Address {
        address.0
    }
}

impl From<bitcoin::util::key::PublicKey> for PublicKey {
    fn from(key: bitcoin::util::key::PublicKey) -> Self {
        Self(key)
    }
}

impl From<bitcoin::util::address::Address> for Address {
    fn from(address: bitcoin::util::address::Address) -> Self {
        Self(address)
    }
}

impl FromStr for PublicKey {
    type Err = bitcoin::util::key::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(bitcoin::PublicKey::from_str(s)?.into())
    }
}

impl FromStr for Address {
    type Err = bitcoin::util::address::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(bitcoin::Address::from_str(s)?.into())
    }
}

impl Serialize for PublicKey {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for PublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        struct PublicKeyVisitor;

        impl<'de> Visitor<'de> for PublicKeyVisitor {
            type Value = PublicKey;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "a hex-encoded, compressed public key")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                v.parse().map_err(E::custom)
            }
        }

        deserializer.deserialize_str(PublicKeyVisitor)
    }
}

/// Median time in Bitcoin is defined as the median of the blocktimes from the
/// last 11 blocks.
pub async fn median_time_past<C>(connector: &C) -> anyhow::Result<Timestamp>
where
    C: LatestBlock<Block = bitcoin::Block>
        + BlockByHash<Block = bitcoin::Block, BlockHash = bitcoin::BlockHash>,
{
    let mut block_times = vec![];

    let mut current = connector.latest_block().await?;
    block_times.push(current.header.time);

    for _ in 0..10 {
        let prev = current.header.prev_blockhash;
        current = connector.block_by_hash(prev).await?;
        block_times.push(current.header.time);
    }

    block_times.sort_unstable();
    let median = block_times[5];

    Ok(Timestamp::from(median))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn serialize_bitcoin_identity() {
        let secp_pubkey = secp256k1::PublicKey::from_str(
            "02c2a8efce029526d364c2cf39d89e3cdda05e5df7b2cbfc098b4e3d02b70b5275",
        )
        .unwrap();
        let pubkey = PublicKey::from(secp_pubkey);

        let str = serde_json::to_string(&pubkey).unwrap();
        assert_eq!(
            str,
            "\"02c2a8efce029526d364c2cf39d89e3cdda05e5df7b2cbfc098b4e3d02b70b5275\""
        )
    }

    #[test]
    fn deserialize_bitcoin_identity() {
        let pubkey_str = "\"02c2a8efce029526d364c2cf39d89e3cdda05e5df7b2cbfc098b4e3d02b70b5275\"";
        let pubkey = serde_json::from_str::<PublicKey>(pubkey_str).unwrap();

        let expected_pubkey = secp256k1::PublicKey::from_str(
            "02c2a8efce029526d364c2cf39d89e3cdda05e5df7b2cbfc098b4e3d02b70b5275",
        )
        .unwrap();
        let expected_pubkey = PublicKey::from(expected_pubkey);

        assert_eq!(pubkey, expected_pubkey);
    }
}
