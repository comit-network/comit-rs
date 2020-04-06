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

/// This is an identifier created, and used locally, by a node to identify a
/// swap created by this node i.e., when a node is acting in the role of Alice
/// we need an identifier.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NodeLocalSwapId(pub Uuid);

impl Default for NodeLocalSwapId {
    fn default() -> Self {
        NodeLocalSwapId(Uuid::new_v4())
    }
}

impl FromStr for NodeLocalSwapId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::from_str(s).map(NodeLocalSwapId)
    }
}

impl From<Uuid> for NodeLocalSwapId {
    fn from(uuid: Uuid) -> Self {
        NodeLocalSwapId(uuid)
    }
}

impl From<NodeLocalSwapId> for Uuid {
    fn from(swap_id: NodeLocalSwapId) -> Self {
        swap_id.0
    }
}

impl fmt::Display for NodeLocalSwapId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        self.0.fmt(f)
    }
}
