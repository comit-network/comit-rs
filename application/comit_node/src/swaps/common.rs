use std::{fmt, str::FromStr};
use uuid::{ParseError, Uuid};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SwapId(Uuid);

impl Default for SwapId {
    fn default() -> Self {
        SwapId(Uuid::new_v4())
    }
}

impl FromStr for SwapId {
    type Err = ParseError;
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
    fn from(trade_id: SwapId) -> Self {
        trade_id.0
    }
}

impl fmt::Display for SwapId {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.0.fmt(f)
    }
}
