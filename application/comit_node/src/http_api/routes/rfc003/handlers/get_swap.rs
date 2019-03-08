use crate::{
    http_api::{
        problem,
        swap_resource::{new_rfc003_hal_swap_resource, IncludeState},
    },
    swap_protocols::{rfc003::state_store::StateStore, MetadataStore, SwapId},
};
use http_api_problem::HttpApiProblem;
use rustic_hal::HalResource;

pub fn handle_get_swap<T: MetadataStore<SwapId>, S: StateStore>(
    metadata_store: &T,
    state_store: &S,
    id: SwapId,
) -> Result<HalResource, HttpApiProblem> {
    let metadata = metadata_store
        .get(&id)?
        .ok_or_else(problem::swap_not_found)?;

    new_rfc003_hal_swap_resource(state_store, id, metadata, IncludeState::Yes)
}
