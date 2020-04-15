pub mod behaviour;
pub mod handler;
pub mod protocol;

use digest::IntoDigestInput;
use libp2p::multihash::{self, Multihash};
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

#[derive(Clone, Debug, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct SwapDigest(Multihash);

impl SwapDigest {
    pub fn new(multihash: Multihash) -> Self {
        Self(multihash)
    }
}

impl IntoDigestInput for SwapDigest {
    fn into_digest_input(self) -> Vec<u8> {
        self.0.into_bytes()
    }
}

impl digest::Hash for SwapDigest {
    fn hash(bytes: &[u8]) -> Self {
        Self(multihash::Sha3_256::digest(bytes))
    }
}

impl fmt::Display for SwapDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0.as_bytes()))
    }
}

impl Serialize for SwapDigest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let hex = hex::encode(self.0.as_bytes());

        serializer.serialize_str(&hex)
    }
}

impl<'de> Deserialize<'de> for SwapDigest {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        let hex = String::deserialize(deserializer)?;
        let bytes = hex::decode(hex).map_err(D::Error::custom)?;
        let multihash = multihash::Multihash::from_bytes(bytes).map_err(D::Error::custom)?;

        Ok(SwapDigest::new(multihash))
    }
}
