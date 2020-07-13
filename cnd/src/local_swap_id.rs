use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use uuid::Uuid;

/// This is an identifier created, and used locally, by a node to identify a
/// swap to this node.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct LocalSwapId(Uuid);

impl LocalSwapId {
    /// Creates a new random swap id.
    pub fn random() -> Self {
        LocalSwapId(Uuid::new_v4())
    }
}

impl LocalSwapId {
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl Default for LocalSwapId {
    fn default() -> Self {
        LocalSwapId::random()
    }
}

impl FromStr for LocalSwapId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::from_str(s).map(LocalSwapId)
    }
}

impl From<Uuid> for LocalSwapId {
    fn from(uuid: Uuid) -> Self {
        LocalSwapId(uuid)
    }
}

impl From<LocalSwapId> for Uuid {
    fn from(swap_id: LocalSwapId) -> Self {
        swap_id.0
    }
}

impl fmt::Display for LocalSwapId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        self.0.fmt(f)
    }
}
