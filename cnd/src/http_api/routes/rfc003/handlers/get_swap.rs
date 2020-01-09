use crate::{
    db::{DetermineTypes, Retrieve},
    http_api::swap_resource::{build_rfc003_siren_entity, IncludeState},
    swap_protocols::{Facade, SwapId},
};

pub async fn handle_get_swap<S>(
    dependencies: Facade<S>,
    id: SwapId,
) -> anyhow::Result<siren::Entity>
where
    S: Send + Sync + 'static,
{
    let swap = Retrieve::get(&dependencies, &id).await?;
    let types = dependencies.determine_types(&id).await?;

    build_rfc003_siren_entity(&dependencies, swap, types, IncludeState::Yes)
}
