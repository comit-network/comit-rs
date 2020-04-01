pub mod behaviour;
pub mod handler;
pub mod protocol;

use libp2p::{multihash, multihash::Multihash};
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SwapDigest(Multihash);

impl SwapDigest {
    pub fn new(multihash: Multihash) -> Self {
        Self(multihash)
    }
}

impl fmt::Display for SwapDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", multihash::to_hex(self.0.to_vec().as_slice()))
    }
}

impl Serialize for SwapDigest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let bytes = self.0.to_vec();
        let hex = multihash::to_hex(bytes.as_slice());

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
