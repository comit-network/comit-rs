use crate::{
    db::{DetermineTypes, Retrieve},
    http_api::{
        problem,
        swap_resource::{build_rfc003_siren_entity, IncludeState},
    },
    swap_protocols::{rfc003::state_store::StateStore, SwapId},
};
use http_api_problem::HttpApiProblem;

pub fn handle_get_swap<D: Retrieve + StateStore + DetermineTypes>(
    dependencies: D,
    id: SwapId,
) -> Result<siren::Entity, HttpApiProblem> {
    let swap = Retrieve::get(&dependencies, &id).map_err(problem::internal_error)?;

    let types = dependencies
        .determine_types(&id)
        .map_err(problem::internal_error)?;

    build_rfc003_siren_entity(&dependencies, swap, types, IncludeState::Yes)
}
