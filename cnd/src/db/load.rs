use crate::{
    db::{CreatedSwap, InProgressSwap, Sqlite},
    swap_protocols::{halight, han, herc20, LocalSwapId},
};
use async_trait::async_trait;

/// Load data from the database.
#[async_trait]
pub trait Load<T>: Send + Sync + 'static {
    async fn load(&self, swap_id: LocalSwapId) -> anyhow::Result<Option<T>>;
}

#[async_trait]
impl Load<InProgressSwap<han::InProgressSwap, halight::InProgressSwap>> for Sqlite {
    async fn load(
        &self,
        _: LocalSwapId,
    ) -> anyhow::Result<Option<InProgressSwap<han::InProgressSwap, halight::InProgressSwap>>> {
        // NOTE: When implementing this be sure to return Ok(None) if the
        // communication protocols have not been finalized and the data saved.
        unimplemented!()
    }
}

#[async_trait]
impl Load<InProgressSwap<herc20::InProgressSwap, halight::InProgressSwap>> for Sqlite {
    async fn load(
        &self,
        _: LocalSwapId,
    ) -> anyhow::Result<Option<InProgressSwap<herc20::InProgressSwap, halight::InProgressSwap>>>
    {
        unimplemented!()
    }
}

#[async_trait]
impl Load<CreatedSwap<han::CreatedSwap, halight::CreatedSwap>> for Sqlite {
    async fn load(
        &self,
        _: LocalSwapId,
    ) -> anyhow::Result<Option<CreatedSwap<han::CreatedSwap, halight::CreatedSwap>>> {
        unimplemented!()
    }
}

#[async_trait]
impl Load<CreatedSwap<herc20::CreatedSwap, halight::CreatedSwap>> for Sqlite {
    async fn load(
        &self,
        _: LocalSwapId,
    ) -> anyhow::Result<Option<CreatedSwap<herc20::CreatedSwap, halight::CreatedSwap>>> {
        unimplemented!()
    }
}
