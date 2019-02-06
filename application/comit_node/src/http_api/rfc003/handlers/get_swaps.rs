use crate::swap_protocols::{
    ledger::{Bitcoin, Ethereum},
    metadata_store::RoleKind,
    rfc003::{state_store::StateStore, Alice, Bob},
    Metadata, MetadataStore, SwapId,
};
use ethereum_support::Erc20Token;
use http_api_problem::HttpApiProblem;
use rustic_hal::HalResource;

#[derive(Serialize, Debug)]
pub struct EmbeddedSwapResource {
    state: String,
    protocol: String,
}

use crate::http_api::rfc003::routes::{swap_path, PROTOCOL_NAME};

pub fn handle_get_swaps<T: MetadataStore<SwapId>, S: StateStore<SwapId>>(
    metadata_store: &T,
    state_store: &S,
) -> Result<Vec<HalResource>, HttpApiProblem> {
    let mut resources = vec![];
    for (id, metadata) in metadata_store.all()?.into_iter() {
        with_swap_types!(
            &metadata,
            (|| -> Result<(), HttpApiProblem> {
                let state = state_store.get::<Role>(&id)?;

                match state {
                    Some(state) => {
                        let swap = EmbeddedSwapResource {
                            state: state.name(),
                            protocol: PROTOCOL_NAME.into(),
                        };

                        let mut hal_resource = HalResource::new(swap);
                        hal_resource.with_link("self", swap_path(id));
                        resources.push(hal_resource);
                    }
                    None => error!("Couldn't find state for {} despite having the metadata", id),
                };
                Ok(())
            })
        )?;
    }

    Ok(resources)
}
