use crate::{
    http_api::swap_resource::{build_rfc003_siren_entity, IncludeState},
    swap_protocols::{rfc003::state_store::StateStore, MetadataStore},
};
use http_api_problem::HttpApiProblem;

pub fn handle_get_swaps<D: MetadataStore + StateStore>(
    dependencies: &D,
) -> Result<siren::Entity, HttpApiProblem> {
    let mut entity = siren::Entity::default().with_class_member("swaps");

    for metadata in MetadataStore::all(dependencies)?.into_iter() {
        let id = metadata.swap_id;
        let sub_entity = build_rfc003_siren_entity(dependencies, id, metadata, IncludeState::No)?;
        entity.push_sub_entity(siren::SubEntity::from_entity(sub_entity, &["item"]));
    }

    Ok(entity)
}
