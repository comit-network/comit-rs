use crate::{
    db::{DetermineTypes, Retrieve},
    http_api::{
        problem,
        swap_resource::{build_rfc003_siren_entity, IncludeState},
    },
    swap_protocols::rfc003::state_store::StateStore,
};
use http_api_problem::HttpApiProblem;

pub fn handle_get_swaps<D: DetermineTypes + Retrieve + StateStore>(
    dependencies: D,
) -> Result<siren::Entity, HttpApiProblem> {
    let mut entity = siren::Entity::default().with_class_member("swaps");

    for swap in Retrieve::all(&dependencies)
        .map_err(problem::internal_error)?
        .into_iter()
    {
        let types = dependencies
            .determine_types(&swap.swap_id)
            .map_err(problem::internal_error)?;

        let sub_entity = build_rfc003_siren_entity(&dependencies, swap, types, IncludeState::No)?;
        entity.push_sub_entity(siren::SubEntity::from_entity(sub_entity, &["item"]));
    }

    Ok(entity)
}
