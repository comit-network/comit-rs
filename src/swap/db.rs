#[async_trait::async_trait]
pub trait Load<T>: Send + Sync + 'static {
    async fn load(&self, swap_id: u8) -> anyhow::Result<Option<T>>;
}
