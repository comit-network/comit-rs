use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use uuid::Uuid;

/// This is a swap identifier created by Bob and shared with Alice via the
/// network communication protocols.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SharedSwapId(Uuid);

#[cfg(test)]
impl SharedSwapId {
    pub fn nil() -> Self {
        SharedSwapId(Uuid::nil())
    }
}

impl Default for SharedSwapId {
    fn default() -> Self {
        SharedSwapId(Uuid::new_v4())
    }
}

impl FromStr for SharedSwapId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::from_str(s).map(SharedSwapId)
    }
}

impl fmt::Display for SharedSwapId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        self.0.fmt(f)
    }
}
