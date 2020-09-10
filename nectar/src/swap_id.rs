use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SwapId(Uuid);

impl SwapId {
    pub fn as_bytes(&self) -> &[u8; 16] {
        self.0.as_bytes()
    }
}

impl Default for SwapId {
    fn default() -> Self {
        SwapId(Uuid::new_v4())
    }
}

impl fmt::Display for SwapId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.to_string())
    }
}

#[cfg(test)]
mod arbitrary {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for SwapId {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let mut bytes = [0u8; 16];
            for byte in &mut bytes {
                *byte = u8::arbitrary(g);
            }
            let uuid = Uuid::from_bytes(bytes);
            SwapId(uuid)
        }
    }
}
