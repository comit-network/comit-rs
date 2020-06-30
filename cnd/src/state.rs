use crate::LocalSwapId;
use async_trait::async_trait;

#[async_trait]
pub trait Insert<S>: Send + Sync + 'static {
    async fn insert(&self, key: LocalSwapId, value: S);
}

#[async_trait]
pub trait Get<S>: Send + Sync + 'static {
    async fn get(&self, key: &LocalSwapId) -> anyhow::Result<Option<S>>;
}

#[async_trait]
pub trait Update<E>: Send + Sync + 'static {
    async fn update(&self, key: &LocalSwapId, update: E);
}
