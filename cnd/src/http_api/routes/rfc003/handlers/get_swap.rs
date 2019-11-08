use crate::{
    http_api::{
        problem,
        swap_resource::{build_rfc003_siren_entity, IncludeState},
    },
    swap_protocols::{rfc003::state_store::StateStore, MetadataStore, SwapId},
};
use http_api_problem::HttpApiProblem;

pub fn handle_get_swap<D: MetadataStore + StateStore>(
    dependencies: D,
    id: SwapId,
) -> Result<siren::Entity, HttpApiProblem> {
    let metadata = MetadataStore::get(&dependencies, id)?.ok_or_else(problem::swap_not_found)?;
    build_rfc003_siren_entity(&dependencies, id, metadata, IncludeState::Yes)
}
