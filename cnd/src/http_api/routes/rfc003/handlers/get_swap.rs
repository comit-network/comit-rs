use crate::{
    db::{DetermineTypes, Retrieve},
    http_api::swap_resource::{build_rfc003_siren_entity, IncludeState, OnFail},
    swap_protocols::{rfc003::SwapId, Rfc003Facade},
};

pub async fn handle_get_swap(
    dependencies: Rfc003Facade,
    id: SwapId,
) -> anyhow::Result<siren::Entity> {
    let swap = Retrieve::get(&dependencies, &id).await?;
    let types = dependencies.determine_types(&id).await?;

    build_rfc003_siren_entity(&dependencies, swap, types, IncludeState::Yes, OnFail::Error).await
}
