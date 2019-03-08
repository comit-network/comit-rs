use crate::{
    http_api::swap_resource::{new_rfc003_hal_swap_resource, IncludeState},
    swap_protocols::{rfc003::state_store::StateStore, MetadataStore, SwapId},
};
use http_api_problem::HttpApiProblem;
use rustic_hal::HalResource;

pub fn handle_get_swaps<T: MetadataStore<SwapId>, S: StateStore>(
    metadata_store: &T,
    state_store: &S,
) -> Result<Vec<HalResource>, HttpApiProblem> {
    let mut resources = vec![];
    for (id, metadata) in metadata_store.all()?.into_iter() {
        let hal_swap_resource =
            new_rfc003_hal_swap_resource(state_store, id, metadata, IncludeState::No);
        resources.push(hal_swap_resource);
    }

    Ok(resources.into_iter().flatten().collect())
}
