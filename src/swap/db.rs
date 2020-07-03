use crate::SwapId;

#[async_trait::async_trait]
pub trait Load<T>: Send + Sync + 'static {
    async fn load(&self, swap_id: SwapId) -> anyhow::Result<Option<T>>;
}

#[async_trait::async_trait]
pub trait Save<T>: Send + Sync + 'static {
    async fn save(&self, event: T, swap_id: SwapId) -> anyhow::Result<()>;
}
