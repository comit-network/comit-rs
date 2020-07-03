use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct SwapId(Uuid);

impl SwapId {
    pub fn random() -> Self {
        SwapId(Uuid::new_v4())
    }
}
