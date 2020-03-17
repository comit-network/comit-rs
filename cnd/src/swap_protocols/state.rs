use crate::swap_protocols::swap_id::SwapId;
use async_trait::async_trait;

#[async_trait]
pub trait Insert<S>: Send + Sync + 'static {
    async fn insert(&self, key: SwapId, value: S);
}

#[async_trait]
pub trait Get<S>: Send + Sync + 'static {
    async fn get(&self, key: &SwapId) -> anyhow::Result<Option<S>>;
}

#[async_trait]
pub trait Update<S, E>: Send + Sync + 'static {
    async fn update(&self, key: &SwapId, update: E);
}
