use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SwapId(pub Uuid);

impl Default for SwapId {
    fn default() -> Self {
        SwapId(Uuid::new_v4())
    }
}

impl FromStr for SwapId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::from_str(s).map(SwapId)
    }
}

impl From<Uuid> for SwapId {
    fn from(uuid: Uuid) -> Self {
        SwapId(uuid)
    }
}

impl From<SwapId> for Uuid {
    fn from(swap_id: SwapId) -> Self {
        swap_id.0
    }
}

impl fmt::Display for SwapId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        self.0.fmt(f)
    }
}
