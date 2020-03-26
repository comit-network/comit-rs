use crate::{
    db::{DetermineTypes, Retrieve},
    http_api::swap_resource::{build_rfc003_siren_entity, IncludeState, OnFail},
    swap_protocols::Facade,
};

pub async fn handle_get_swaps(dependencies: Facade) -> anyhow::Result<siren::Entity> {
    let mut entity = siren::Entity::default().with_class_member("swaps");

    for swap in Retrieve::all(&dependencies).await?.into_iter() {
        let types = dependencies.determine_types(&swap.swap_id).await?;

        let sub_entity = build_rfc003_siren_entity(
            &dependencies,
            swap,
            types,
            IncludeState::No,
            OnFail::NoAction,
        )
        .await?;
        entity.push_sub_entity(siren::SubEntity::from_entity(sub_entity, &["item"]));
    }

    Ok(entity)
}
