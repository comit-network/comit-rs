use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SwapId(Uuid);

impl SwapId {
    pub fn random() -> Self {
        SwapId(Uuid::new_v4())
    }
}
