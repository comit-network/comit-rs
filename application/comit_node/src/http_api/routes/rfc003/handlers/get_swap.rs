use crate::{
    http_api::{
        problem,
        swap_resource::{build_rfc003_siren_entity, IncludeState},
    },
    swap_protocols::{rfc003::state_store::StateStore, MetadataStore, SwapId},
};
use http_api_problem::HttpApiProblem;

pub fn handle_get_swap<T: MetadataStore<SwapId>, S: StateStore>(
    metadata_store: &T,
    state_store: &S,
    id: SwapId,
) -> Result<siren::Entity, HttpApiProblem> {
    let metadata = metadata_store
        .get(&id)?
        .ok_or_else(problem::swap_not_found)?;

    build_rfc003_siren_entity(state_store, id, metadata, IncludeState::Yes)
}
