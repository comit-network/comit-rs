use crate::{
    connector::Connect,
    http_api::{
        problem,
        swap_resource::{build_rfc003_siren_entity, IncludeState},
    },
    swap_protocols::{MetadataStore, SwapId},
};
use http_api_problem::HttpApiProblem;

pub fn handle_get_swap<C: Connect>(con: C, id: SwapId) -> Result<siren::Entity, HttpApiProblem> {
    let metadata = MetadataStore::get(&con, id)?.ok_or_else(problem::swap_not_found)?;
    build_rfc003_siren_entity(&con, id, metadata, IncludeState::Yes)
}
