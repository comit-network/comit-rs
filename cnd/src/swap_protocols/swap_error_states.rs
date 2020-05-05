use crate::swap_protocols::swap_id::SwapId;
use async_trait::async_trait;
use std::collections::HashMap;
use tokio::sync::Mutex;

#[derive(Default, Debug)]
pub struct SwapErrorStates(Mutex<HashMap<SwapId, bool>>);

impl SwapErrorStates {
    pub async fn has_failed(&self, id: &SwapId) -> bool {
        *self.0.lock().await.get(id).unwrap_or(&false)
    }
}

#[async_trait]
pub trait InsertFailedSwap {
    async fn insert_failed_swap(&self, id: &SwapId);
}

#[async_trait]
impl InsertFailedSwap for SwapErrorStates {
    async fn insert_failed_swap(&self, id: &SwapId) {
        let _ = self.0.lock().await.insert(*id, true);
    }
}
