mod behaviour;
mod handler;
mod protocol;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

// FIXME: Where should this go?
#[derive(Clone, Debug)]
pub struct SwapDigest {
    inner: libp2p::multihash::Multihash,
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
