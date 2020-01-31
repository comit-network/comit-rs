use crate::{
    db::{DetermineTypes, Retrieve},
    http_api::swap_resource::{build_rfc003_siren_entity_err, IncludeState},
    swap_protocols::{Facade, SwapId},
};

pub async fn handle_get_swap(dependencies: Facade, id: SwapId) -> anyhow::Result<siren::Entity> {
    let swap = Retrieve::get(&dependencies, &id).await?;
    let types = dependencies.determine_types(&id).await?;

    build_rfc003_siren_entity_err(&dependencies, swap, types, IncludeState::Yes)
}
