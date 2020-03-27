pub mod behaviour;
pub mod handler;
pub mod protocol;

use libp2p::{multihash, multihash::Multihash};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SwapDigest {
    // TODO: should this be public?
    pub inner: Multihash,
}

impl fmt::Display for SwapDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", multihash::to_hex(self.inner.to_vec().as_slice()))
    }
}

impl Serialize for SwapDigest {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        unimplemented!()
    }
}

impl<'de> Deserialize<'de> for SwapDigest {
    fn deserialize<D>(_deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        unimplemented!()
    }
}
