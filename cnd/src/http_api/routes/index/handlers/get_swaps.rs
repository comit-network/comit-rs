use crate::{
    http_api::swap_resource::{build_rfc003_siren_entity, IncludeState},
    metadata_store::MetadataStore,
    state_store::StateStore,
};
use http_api_problem::HttpApiProblem;

pub fn handle_get_swaps<T: MetadataStore, S: StateStore>(
    metadata_store: &T,
    state_store: &S,
) -> Result<siren::Entity, HttpApiProblem> {
    let mut entity = siren::Entity::default().with_class_member("swaps");

    for metadata in metadata_store.all()?.into_iter() {
        let id = metadata.swap_id;
        let sub_entity = build_rfc003_siren_entity(state_store, id, metadata, IncludeState::No)?;
        entity.push_sub_entity(siren::SubEntity::from_entity(sub_entity, &["item"]));
    }

    Ok(entity)
}
