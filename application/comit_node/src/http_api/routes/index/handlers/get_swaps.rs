use crate::{
    http_api::swap_resource::{new_rfc003_siren_entity, IncludeState},
    swap_protocols::{rfc003::state_store::StateStore, MetadataStore, SwapId},
};
use http_api_problem::HttpApiProblem;

pub fn handle_get_swaps<T: MetadataStore<SwapId>, S: StateStore>(
    metadata_store: &T,
    state_store: &S,
) -> Result<siren::Entity, HttpApiProblem> {
    let mut entity = siren::Entity::default();

    for (id, metadata) in metadata_store.all()?.into_iter() {
        let sub_entity = new_rfc003_siren_entity(state_store, id, metadata, IncludeState::No)?;
        entity.push_sub_entity(siren::SubEntity::from_entity(sub_entity, &["item"]));
    }

    Ok(entity)
}
