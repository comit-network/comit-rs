use crate::{
    db::{DetermineTypes, Retrieve},
    http_api::swap_resource::{build_rfc003_siren_entity, IncludeState, OnFail},
    swap_protocols::{Facade, SwapId},
};

pub async fn handle_get_swap(facade: Facade, id: SwapId) -> anyhow::Result<siren::Entity> {
    let swap = Retrieve::get(&facade, &id).await?;
    let types = facade.determine_types(&id).await?;

    build_rfc003_siren_entity(&facade, swap, types, IncludeState::Yes, OnFail::Error)
}
