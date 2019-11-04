use crate::{
    connector::Connect,
    http_api::swap_resource::{build_rfc003_siren_entity, IncludeState},
    swap_protocols::MetadataStore,
};
use http_api_problem::HttpApiProblem;

pub fn handle_get_swaps<C: Connect>(con: C) -> Result<siren::Entity, HttpApiProblem> {
    let mut entity = siren::Entity::default().with_class_member("swaps");

    for metadata in MetadataStore::all(&con)?.into_iter() {
        let id = metadata.swap_id;
        let sub_entity = build_rfc003_siren_entity(&con, id, metadata, IncludeState::No)?;
        entity.push_sub_entity(siren::SubEntity::from_entity(sub_entity, &["item"]));
    }

    Ok(entity)
}
